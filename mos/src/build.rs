//! Build Supervisor — graph-driven multi-phase build orchestrator.
//!
//! Extracts the `cmd_build` logic into a modular supervisor that wires
//! together the previously disconnected subsystems:
//!
//! - **TaskQueue** — action management, loop detection, budget tracking
//! - **TierBudget** — per-tier token allocation (micro/coder/architect/oracle)
//! - **ContextAssembler** — targeted context injection from roska + graph
//! - **DecisionEngine** — optional LLM-driven action selection (fallback: heuristic)
//! - **PlanGraph** — typed plan hierarchy (workspace → feature → context)
//! - **RuleSet** — conditional context/instruction injection
//!
//! ```text
//! BuildSupervisor::run()
//!   ├─ Perception (roska scan)
//!   ├─ Plan (LLM decomposes task → PlanGraph)
//!   └─ Loop:
//!       ├─ TaskQueue.should_stop()? → break
//!       ├─ analyze_gaps() → derive_actions()
//!       ├─ TaskQueue.push_actions() → pop()
//!       ├─ TierBudget selects model tier
//!       ├─ ContextAssembler builds prompt context
//!       ├─ RuleSet injects conditional instructions
//!       ├─ Agent dispatched (KKR)
//!       ├─ Result processed → graph updated
//!       └─ TaskQueue.record_result()
//! ```

use crate::config::MosConfig;
use crate::graph_ops;
use crate::persist::{self, UserRequest};
use crate::cli;
use crate::plans::{Plan, PlanGraph, PlanKind};
use crate::prompt;
use crate::queue::{TaskQueue, QueueConfig, StopReason};
use crate::rules::{RuleSet, EvalContext};
use crate::scanner;
use crate::supervisor::{self, ActionKind, SupervisorConfig};
use crate::tiers::{self, TierBudget};
use crate::tools;

use kkr_core::agent::{AgentConfig, ApprovalPolicy};
use kkr_core::prelude::Agent;
use kkr_core::workspace::Workspace;
use kkr_core::Task;
use knowledge_graph::KnowledgeGraph;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Result of a build run.
pub struct BuildResult {
    pub request: UserRequest,
    pub stop_reason: StopReason,
    pub graph: KnowledgeGraph,
}

/// The graph-driven build supervisor.
pub struct BuildSupervisor {
    cfg: MosConfig,
    sup_config: SupervisorConfig,
    graph: KnowledgeGraph,
    request: UserRequest,
    plan_features: Vec<String>,
    plan_graph: PlanGraph,
    queue: TaskQueue,
    tier_budget: TierBudget,
    rules: RuleSet,
    feature_fail_counts: HashMap<String, u32>,
    plan_attempts: u32,
    api_key: String,
    approval: ApprovalPolicy,
    workspace_str: String,
}

impl BuildSupervisor {
    /// Create a new build supervisor.
    pub fn new(
        cfg: MosConfig,
        graph: KnowledgeGraph,
        api_key: String,
        approval: ApprovalPolicy,
        sup_config: SupervisorConfig,
    ) -> Self {
        let workspace_str = cfg.workspace.display().to_string();
        let budget = sup_config.token_budget;

        // Load or create user request
        let request_task = ""; // Will be set in run()
        let request = persist::load_active_request(&cfg.workspace)
            .unwrap_or_else(|| UserRequest::new(request_task));

        let plan_features = if !request.plan_features.is_empty() {
            request.plan_features.clone()
        } else {
            graph_ops::find_plan_features(&graph)
        };

        let queue_config = QueueConfig {
            max_repeats: 3,
            max_consecutive_failures: 5,
            history_size: 30,
            max_iterations: sup_config.max_iterations,
        };

        Self {
            cfg,
            sup_config,
            graph,
            request,
            plan_features,
            plan_graph: PlanGraph::new(),
            queue: TaskQueue::with_config(budget, queue_config),
            tier_budget: TierBudget::new(budget),
            rules: RuleSet::default_coding(),
            feature_fail_counts: HashMap::new(),
            plan_attempts: 0,
            api_key,
            approval,
            workspace_str,
        }
    }

    /// Run the build supervisor loop.
    pub async fn run(mut self, task: &str) -> BuildResult {
        // Fix request task if it was empty
        if self.request.task.is_empty() {
            self.request = UserRequest::new(task);
        }

        let is_resumed = !self.request.completed_actions.is_empty();

        // Create workspace plan in plan graph
        self.plan_graph.create_workspace_plan(task);

        eprintln!("\x1b[1;36mmos\x1b[0m v{}", cli::version());
        eprintln!("\x1b[1;36mmos\x1b[0m | \x1b[1;33mgraph-driven build\x1b[0m");
        eprintln!("\x1b[1;36mmos\x1b[0m | workspace: {}", self.workspace_str);
        eprintln!("\x1b[1;36mmos\x1b[0m | task: {}", task);
        eprintln!("\x1b[1;36mmos\x1b[0m | graph: {} nodes, {} edges", self.graph.node_count(), self.graph.edge_count());
        eprintln!("\x1b[1;36mmos\x1b[0m | rules: {} active", self.rules.len());
        eprintln!(
            "\x1b[1;36mmos\x1b[0m | request: {} ({})",
            self.request.id,
            if is_resumed { "resumed" } else { "new" }
        );

        let mut stop_reason = StopReason::MaxIterations;

        // ── Supervisor Loop ──
        for iteration in 0..self.sup_config.max_iterations {
            eprintln!();
            eprintln!("\x1b[1;34m── iteration {}/{} ──\x1b[0m", iteration + 1, self.sup_config.max_iterations);

            // Check stop conditions via TaskQueue
            let all_verified = graph_ops::is_plan_complete(&self.graph, &self.plan_features);
            if let Some(reason) = self.queue.should_stop(iteration, all_verified) {
                stop_reason = reason;
                eprintln!("\x1b[1;33mstop\x1b[0m | {}", stop_reason.label());
                break;
            }

            // Analyze gaps and derive actions
            let gaps = graph_ops::analyze_gaps(&self.graph, &self.plan_features);
            let actions = graph_ops::derive_actions(&gaps, &self.graph);

            if actions.is_empty() {
                stop_reason = StopReason::AllComplete;
                eprintln!("\x1b[1;32m=== Project complete! ===\x1b[0m");
                break;
            }

            // Show plan status
            if !self.plan_features.is_empty() {
                let status = graph_ops::render_plan_status(&self.graph, &self.plan_features);
                for line in status.lines() {
                    eprintln!("\x1b[1;35mplan\x1b[0m | {}", line);
                }
            }

            // Queue status
            eprintln!("\x1b[90m{}\x1b[0m", self.queue.render_status());

            // Push actions into queue and pop next
            self.queue.push_actions(actions.clone());
            let queued_task = match self.queue.pop() {
                Some(t) => t,
                None => {
                    stop_reason = StopReason::NoActions;
                    break;
                }
            };
            let action = queued_task.action;
            let fingerprint = queued_task.fingerprint.clone();

            // Guard plan retries
            if matches!(action.kind, ActionKind::Plan) {
                self.plan_attempts += 1;
                if self.plan_attempts > 2 {
                    if self.try_fallback_plan(task) {
                        continue;
                    }
                    stop_reason = StopReason::NoActions;
                    self.request.status = supervisor::RequestStatus::Failed(
                        "Could not create plan".to_string()
                    );
                    break;
                }
            }

            // Select tier + model
            let tier = tiers::tier_for_action(&action.kind);
            let model_profile = tiers::resolve_model(tier, &self.cfg);

            eprintln!(
                "\x1b[1;35maction\x1b[0m | {} @ {} (model: {}, depth: {})",
                action.kind.label(), tier, model_profile.model, action.roska_depth
            );

            // Build evaluation context for rules
            let eval_ctx = self.build_eval_context(&action, task, iteration);
            let rules_text = self.rules.render(&eval_ctx);

            // Targeted context injection
            let project_listing = scanner::scan_project_listing(&self.cfg.workspace);
            let file_context = self.scan_action_context(&action);

            // Compose prompt
            let action_prompt = prompt::compose_prompt(
                &action, task, &self.graph, &file_context, &project_listing,
            );

            let agent_instructions = self.build_agent_instructions(&action, &action_prompt, &rules_text);

            // Build and run agent
            let provider = tools::build_provider(model_profile, &self.api_key);
            let capsule = tools::build_tools_capsule(&self.workspace_str);
            let action_max_iter = tier.max_iterations(self.sup_config.agent_max_iter);

            let agent_cfg = AgentConfig::new("mos-build")
                .with_instructions(&agent_instructions)
                .with_max_iterations(action_max_iter as usize)
                .with_retries(4)
                .with_retry_delay(2000)
                .with_exponential_backoff(true)
                .with_approval_policy(self.approval.clone());

            let ws = Workspace::new(camino::Utf8PathBuf::from(&self.workspace_str));
            let mut agent = Agent::new(agent_cfg, ws, Box::new(provider));
            agent.add_capsule(capsule);

            let task_desc = format!("{}: {}", action.kind.label(), task);

            eprintln!(
                "\x1b[1;36mexecution\x1b[0m | dispatching (max_iter: {})",
                action_max_iter
            );

            let action_start = std::time::Instant::now();

            let (tx, mut rx) = mpsc::channel(64);
            let agent_task = Task::new(&task_desc);

            let event_handle = tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    tools::handle_event(event);
                }
            });

            let result = agent.run_with_events(agent_task, Some(tx)).await;
            let _ = event_handle.await;

            let action_duration = action_start.elapsed();
            let agent_usage = agent.total_usage();
            let tokens_used = agent_usage.total_tokens as u64;

            let success = result.as_ref().map(|o| o.is_success()).unwrap_or(false);
            let result_text = match &result {
                Ok(o) => o.result.clone().unwrap_or_default(),
                Err(e) => format!("Error: {}", e),
            };

            eprintln!(
                "\x1b[1;36mexecution\x1b[0m | {} done: {}in + {}out = {} tokens, {:.1}s {}",
                action.kind.label(),
                agent_usage.prompt_tokens, agent_usage.completion_tokens,
                agent_usage.total_tokens, action_duration.as_secs_f64(),
                if success { "\x1b[32mOK\x1b[0m" } else { "\x1b[31mFAIL\x1b[0m" },
            );

            // Record in queue + tier budget
            self.queue.record_result(&fingerprint, tokens_used, success);
            self.tier_budget.record(tier, tokens_used);

            // Record context in graph
            graph_ops::record_action_context(
                &mut self.graph, &action, iteration,
                agent_usage.prompt_tokens, agent_usage.completion_tokens,
                action_duration.as_secs_f64(), success,
            );

            // Process result → update graph
            self.process_action_result(&action, task, &result_text, success);

            // Record completed action
            self.record_completed_action(&action, iteration, &agent_usage, &action_duration, success, &result_text);

            // Save state
            persist::save_request(&self.cfg.workspace, &self.request);
            tools::save_graph(&self.cfg, &self.graph);

            eprintln!(
                "\x1b[1;36mmos\x1b[0m | tokens: {} ({:.0}% of budget)",
                self.queue.budget().used,
                self.queue.budget().utilization() * 100.0
            );
        }

        // Finalize
        self.request.status = match &stop_reason {
            StopReason::AllComplete => supervisor::RequestStatus::Completed,
            _ if matches!(self.request.status, supervisor::RequestStatus::Active) => {
                supervisor::RequestStatus::Paused
            }
            _ => self.request.status.clone(),
        };
        persist::save_request(&self.cfg.workspace, &self.request);
        tools::save_graph(&self.cfg, &self.graph);

        // Print summary
        self.print_summary(&stop_reason);

        BuildResult {
            request: self.request,
            stop_reason,
            graph: self.graph,
        }
    }

    // ── Private helpers ──

    fn build_eval_context(&self, action: &supervisor::Action, task: &str, iteration: u32) -> EvalContext {
        let files: Vec<String> = match &action.kind {
            ActionKind::Implement { feature_id, .. }
            | ActionKind::Test { feature_id, .. }
            | ActionKind::Fix { feature_id, .. } => {
                let (feat, dep) = graph_ops::feature_file_context(feature_id, &self.graph);
                [feat, dep].concat()
            }
            _ => Vec::new(),
        };

        let mut action_counts = HashMap::new();
        for ca in &self.request.completed_actions {
            *action_counts.entry(ca.action_label.clone()).or_insert(0u32) += 1;
        }

        let consecutive_errors = self.request.completed_actions.iter()
            .rev()
            .take_while(|a| !a.success)
            .count() as u32;

        EvalContext::new(&action.kind, task)
            .with_files(files)
            .with_error_count(consecutive_errors)
            .with_iteration(iteration)
            .with_action_counts(action_counts)
    }

    fn scan_action_context(&self, action: &supervisor::Action) -> String {
        match &action.kind {
            ActionKind::Implement { ref feature_id, .. }
            | ActionKind::Test { ref feature_id, .. }
            | ActionKind::Fix { ref feature_id, .. } => {
                let (feat_files, dep_files) = graph_ops::feature_file_context(feature_id, &self.graph);

                let scan_depth = match &action.kind {
                    ActionKind::Implement { .. } => roska_descriptor::Depth::Detail,
                    ActionKind::Test { .. } => roska_descriptor::Depth::Structure,
                    ActionKind::Fix { .. } => roska_descriptor::Depth::Body,
                    _ => action.roska_depth,
                };

                let ctx = scanner::scan_feature_with_deps(
                    &self.cfg.workspace, &feat_files, &dep_files, scan_depth,
                );

                if ctx.file_count > 0 {
                    eprintln!(
                        "\x1b[1;35mcontext\x1b[0m | injected {} files (~{} tokens) at {:?}",
                        ctx.file_count, ctx.estimated_tokens, scan_depth
                    );
                }
                ctx.content
            }
            _ => String::new(),
        }
    }

    fn build_agent_instructions(&self, action: &supervisor::Action, prompt: &str, rules_text: &str) -> String {
        let rules_section = if rules_text.is_empty() {
            String::new()
        } else {
            format!("\n\n{}", rules_text)
        };

        match &action.kind {
            ActionKind::Plan => format!(
                "{}\n\n## System\nYou are mos, a planning agent. Your job is to ANALYZE the task \
                 and output a structured JSON plan. Do NOT write source code files yet — \
                 only output the JSON feature decomposition as instructed above.{}",
                prompt, rules_section
            ),
            ActionKind::Verify => format!(
                "{}\n\n## System\nYou are mos, a verification agent. Run tests, check results, \
                 and output a JSON verification summary.{}",
                prompt, rules_section
            ),
            _ => format!(
                "{}\n\n## System\nYou are mos, an autonomous coding agent. You BUILD software.\n\
                 You MUST use write_file to create files. Do NOT just describe what to do.\n\
                 IMPORTANT: Source code is PRE-LOADED above. Do NOT re-read files that are \
                 already shown in the prompt. Only use read_file for files NOT listed above.\n\
                 Keep working until this action is complete.{}",
                prompt, rules_section
            ),
        }
    }

    fn process_action_result(
        &mut self,
        action: &supervisor::Action,
        task: &str,
        result_text: &str,
        success: bool,
    ) {
        let workspace_root = &self.cfg.workspace.clone();

        match &action.kind {
            ActionKind::Plan => {
                let mut features = graph_ops::parse_plan_into_graph(result_text, task, &mut self.graph);

                // Fallback: model may have written plan.json to disk
                if features.is_empty() {
                    for plan_name in &["plan.json", ".agent/plan.json"] {
                        let plan_path = workspace_root.join(plan_name);
                        if plan_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&plan_path) {
                                eprintln!("\x1b[1;35mplan\x1b[0m | found {}, parsing...", plan_name);
                                features = graph_ops::parse_plan_into_graph(&content, task, &mut self.graph);
                                if !features.is_empty() { break; }
                            }
                        }
                    }
                }

                if !features.is_empty() {
                    eprintln!("\x1b[1;35mplan\x1b[0m | created {} features in graph", features.len());

                    // Sync to plan graph
                    if let Some(root_id) = self.plan_graph.root_id().map(|s| s.to_string()) {
                        for fid in &features {
                            let name = self.graph.get_node(fid)
                                .map(|n| n.name.clone())
                                .unwrap_or_else(|| fid.clone());
                            let plan = Plan::new(PlanKind::Feature, &name)
                                .with_id(fid);
                            self.plan_graph.add_plan(plan, &root_id);
                        }
                    }

                    self.plan_features = features;
                    self.request.plan_features = self.plan_features.clone();
                } else {
                    eprintln!("\x1b[33mplan\x1b[0m | no JSON plan parsed, checking workspace...");
                    self.try_create_features_from_files(task);
                }
            }
            ActionKind::Scaffold => {
                let src_files = tools::list_source_files(workspace_root);
                if !src_files.is_empty() {
                    let matched = graph_ops::match_files_to_features(&src_files, &self.plan_features, &self.graph);
                    for fid in &matched {
                        graph_ops::mark_feature_implemented(&mut self.graph, fid);
                    }
                    if !matched.is_empty() {
                        eprintln!(
                            "\x1b[1;32mscaffold\x1b[0m | {} features matched from {} files",
                            matched.len(), src_files.len()
                        );
                    }
                }
            }
            ActionKind::Implement { ref feature_id, .. } => {
                if success {
                    graph_ops::mark_feature_implemented(&mut self.graph, feature_id);
                    eprintln!("\x1b[1;32mfeature\x1b[0m | {} → implemented", feature_id);
                }
            }
            ActionKind::Test { ref feature_id, .. } => {
                graph_ops::mark_feature_tested(&mut self.graph, feature_id, success);
                eprintln!(
                    "\x1b[1;32mfeature\x1b[0m | {} → {}",
                    feature_id,
                    if success { "verified" } else { "tested (failing)" }
                );
            }
            ActionKind::Fix { ref feature_id, .. } => {
                if success {
                    graph_ops::mark_feature_tested(&mut self.graph, feature_id, true);
                    self.feature_fail_counts.remove(feature_id);
                    eprintln!("\x1b[1;32mfeature\x1b[0m | {} → verified (fix applied)", feature_id);
                } else {
                    let count = self.feature_fail_counts.entry(feature_id.clone()).or_insert(0);
                    *count += 1;
                    eprintln!("\x1b[33mfix\x1b[0m | {} failed ({}/2 retries)", feature_id, count);
                }
            }
            ActionKind::Verify => {
                if success {
                    for fid in &self.plan_features {
                        graph_ops::mark_feature_tested(&mut self.graph, fid, true);
                    }
                    eprintln!("\x1b[1;32mverify\x1b[0m | all features verified");
                }
            }
            _ => {}
        }
    }

    fn record_completed_action(
        &mut self,
        action: &supervisor::Action,
        iteration: u32,
        usage: &kkr_core::agent::Usage,
        duration: &std::time::Duration,
        success: bool,
        result_text: &str,
    ) {
        let feature_id = match &action.kind {
            ActionKind::Implement { ref feature_id, .. }
            | ActionKind::Test { ref feature_id, .. }
            | ActionKind::Fix { ref feature_id, .. } => Some(feature_id.clone()),
            _ => None,
        };

        self.request.completed_actions.push(supervisor::CompletedAction {
            action_label: action.kind.label().to_string(),
            feature_id,
            iteration,
            tokens_used: usage.total_tokens as u64,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            duration_secs: duration.as_secs_f64(),
            success,
            summary: if result_text.len() > 200 {
                format!("{}...", &result_text[..200])
            } else {
                result_text.to_string()
            },
            context_node_ids: action.context_nodes.clone(),
            roska_depth: format!("{}", action.roska_depth),
        });
        self.request.total_tokens = self.queue.budget().used;
    }

    fn try_fallback_plan(&mut self, task: &str) -> bool {
        eprintln!("\x1b[33mplan\x1b[0m | max plan attempts reached, checking workspace files...");
        let src_files = tools::list_source_files(&self.cfg.workspace);
        if src_files.is_empty() {
            eprintln!("\x1b[31mplan\x1b[0m | no files found either, giving up on planning");
            return false;
        }

        let fallback = graph_ops::create_features_from_files(task, &src_files, &mut self.graph);
        if fallback.is_empty() {
            return false;
        }

        self.plan_features = fallback;
        self.request.plan_features = self.plan_features.clone();
        for fid in &self.plan_features {
            graph_ops::mark_feature_implemented(&mut self.graph, fid);
        }
        eprintln!(
            "\x1b[1;35mplan\x1b[0m | created {} features from {} workspace files",
            self.plan_features.len(), src_files.len()
        );
        true
    }

    fn try_create_features_from_files(&mut self, task: &str) {
        let src_files = tools::list_source_files(&self.cfg.workspace);
        if !src_files.is_empty() {
            eprintln!(
                "\x1b[1;35mplan\x1b[0m | found {} source files, creating features from workspace",
                src_files.len()
            );
            let fallback_features = graph_ops::create_features_from_files(task, &src_files, &mut self.graph);
            if !fallback_features.is_empty() {
                self.plan_features = fallback_features;
                self.request.plan_features = self.plan_features.clone();
                for fid in &self.plan_features {
                    graph_ops::mark_feature_implemented(&mut self.graph, fid);
                }
            }
        }
    }

    fn print_summary(&self, stop_reason: &StopReason) {
        let total_prompt: u64 = self.request.completed_actions.iter().map(|a| a.prompt_tokens as u64).sum();
        let total_completion: u64 = self.request.completed_actions.iter().map(|a| a.completion_tokens as u64).sum();
        let total_duration: f64 = self.request.completed_actions.iter().map(|a| a.duration_secs).sum();

        eprintln!();
        eprintln!("\x1b[1;36m── build summary ──\x1b[0m");
        eprintln!("\x1b[1;36mmos\x1b[0m | request: {}", self.request.id);
        eprintln!("\x1b[1;36mmos\x1b[0m | status: {:?}", self.request.status);
        eprintln!("\x1b[1;36mmos\x1b[0m | stop reason: {}", stop_reason.label());
        eprintln!("\x1b[1;36mmos\x1b[0m | actions completed: {}", self.request.completed_actions.len());
        eprintln!(
            "\x1b[1;36mmos\x1b[0m | tokens: {} total ({}in + {}out)",
            self.queue.budget().used, total_prompt, total_completion
        );
        eprintln!(
            "\x1b[1;36mmos\x1b[0m | duration: {:.0}s total ({:.0}s avg/action)",
            total_duration,
            if self.request.completed_actions.is_empty() { 0.0 }
            else { total_duration / self.request.completed_actions.len() as f64 }
        );
        eprintln!("\x1b[1;36mmos\x1b[0m | graph: {} nodes, {} edges", self.graph.node_count(), self.graph.edge_count());

        // Tier budget breakdown
        eprintln!("\x1b[1;36mmos\x1b[0m | tier budget:");
        for line in self.tier_budget.render().lines() {
            eprintln!("\x1b[1;36mmos\x1b[0m |{}", line);
        }

        if !self.plan_features.is_empty() {
            let status = graph_ops::render_plan_status(&self.graph, &self.plan_features);
            for line in status.lines() {
                eprintln!("\x1b[1;36mmos\x1b[0m | {}", line);
            }
        }

        let perf = prompt::render_performance_summary(&self.request.completed_actions);
        for line in perf.lines() {
            eprintln!("\x1b[1;33mperf\x1b[0m | {}", line);
        }
    }
}
