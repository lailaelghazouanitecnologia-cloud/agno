//! Graph Operations — gap analysis, plan parsing, and project graph updates.
//!
//! Core supervisor logic that bridges the Plan Graph (features + specs)
//! and the Project Graph (modules + files from roska scans).
//!
//! ```text
//! Plan Graph (what we WANT)
//!      ↕ gap analysis ↕
//! Project Graph (what we HAVE)
//!      ↓
//! Actions (prioritized)
//! ```

use crate::supervisor::{Action, ActionKind, Gap, GapStatus};
use crate::util::{camel_to_hyphen, sanitize_id, extract_json};
use knowledge_core::graph::*;
use knowledge_graph::KnowledgeGraph;
use roska_descriptor::Depth;
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════
// Plan Parsing — model response → Feature nodes in graph
// ═══════════════════════════════════════════════════════════════════════

/// Parse the model's plan response and populate the knowledge graph.
/// Returns the list of feature node IDs created.
///
/// Handles multiple plan schemas:
/// - Standard: `{"features": [{"id", "name", "files", "depends_on", "criteria"}]}`
/// - Module-based: `{"modules": [{"id", "name", "file", "dependencies"}]}` + optional `features`
pub fn parse_plan_into_graph(
    response: &str,
    task: &str,
    graph: &mut KnowledgeGraph,
) -> Vec<String> {
    let json_str = extract_json(response);
    let parsed: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut feature_ids = Vec::new();

    // Create intent node — root of the plan
    let intent_id = format!("intent-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let intent_node = Node::new(&intent_id, task, NodeKind::Concept)
        .with_tag("intent")
        .with_tag("plan-root")
        .with_weight(1.0)
        .with_description(format!("User request: {}", task));
    let _ = graph.add_node(intent_node);

    // Try "modules" array first (more implementation-oriented), then "features"
    let items = if let Some(modules) = parsed.get("modules").and_then(|m| m.as_array()) {
        if !modules.is_empty() { modules.clone() } else {
            match parsed.get("features").and_then(|f| f.as_array()) {
                Some(f) if !f.is_empty() => f.clone(),
                _ => return Vec::new(),
            }
        }
    } else {
        match parsed.get("features").and_then(|f| f.as_array()) {
            Some(f) if !f.is_empty() => f.clone(),
            _ => return Vec::new(),
        }
    };

    // Build a name→node_id mapping for dependency resolution
    let mut name_to_id: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // First pass: create all feature nodes
    for item in &items {
        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or(id);
        let desc = item.get("description").and_then(|v| v.as_str()).unwrap_or("");

        let node_id = format!("plan-{}", sanitize_id(id));

        // Collect files from multiple possible fields/formats
        let mut file_list = Vec::new();
        if let Some(files) = item.get("files").and_then(|f| f.as_array()) {
            for f in files {
                if let Some(s) = f.as_str() {
                    file_list.push(s.to_string());
                } else if let Some(p) = f.get("path").and_then(|p| p.as_str()) {
                    file_list.push(p.to_string());
                }
            }
        }
        if let Some(file) = item.get("file").and_then(|f| f.as_str()) {
            if !file_list.iter().any(|f| f == file) {
                file_list.push(file.to_string());
            }
        }
        if let Some(subs) = item.get("submodules").and_then(|s| s.as_array()) {
            for sub in subs {
                if let Some(sf) = sub.get("file").and_then(|f| f.as_str()) {
                    file_list.push(sf.to_string());
                }
            }
        }

        let mut node = Node::new(&node_id, name, NodeKind::Feature)
            .with_tag("plan")
            .with_description(desc.to_string());

        if !file_list.is_empty() {
            node = node.with_meta("files", &file_list.join(","));
        }

        let deps_array = item.get("depends_on").and_then(|d| d.as_array())
            .or_else(|| item.get("dependencies").and_then(|d| d.as_array()));
        if let Some(deps) = deps_array {
            let deps_str: Vec<String> = deps
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| format!("plan-{}", sanitize_id(s)))
                .collect();
            node = node.with_meta("depends_on", &deps_str.join(","));
        }

        let _ = graph.add_node(node);
        let _ = graph.add_edge(Edge::new(&intent_id, &node_id, EdgeRelation::Parent));

        if let Some(criteria) = item.get("criteria").and_then(|c| c.as_array()) {
            let mut spec = Spec::new(&node_id, format!("{} spec", name));
            for criterion in criteria {
                if let Some(c) = criterion.as_str() {
                    spec.add_criterion(c);
                }
            }
            let _ = graph.add_spec(spec);
        }

        name_to_id.insert(name.to_string(), node_id.clone());
        name_to_id.insert(id.to_string(), node_id.clone());
        feature_ids.push(node_id);
    }

    // Second pass: add dependency edges
    for item in &items {
        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let node_id = format!("plan-{}", sanitize_id(id));

        let deps_array = item.get("depends_on").and_then(|d| d.as_array())
            .or_else(|| item.get("dependencies").and_then(|d| d.as_array()));

        if let Some(deps) = deps_array {
            for dep in deps {
                if let Some(dep_str) = dep.as_str() {
                    let dep_node_id = {
                        let direct = format!("plan-{}", sanitize_id(dep_str));
                        if feature_ids.contains(&direct) {
                            direct
                        } else if let Some(mapped) = name_to_id.get(dep_str) {
                            mapped.clone()
                        } else {
                            continue;
                        }
                    };

                    let existing = graph.edges_from(&node_id);
                    let already = existing.iter().any(|e| {
                        e.to == dep_node_id && e.relation == EdgeRelation::DependsOn
                    });
                    if !already {
                        let _ = graph.add_edge(Edge::new(
                            &node_id,
                            &dep_node_id,
                            EdgeRelation::DependsOn,
                        ));
                    }
                }
            }
        }
    }

    feature_ids
}

// ═══════════════════════════════════════════════════════════════════════
// Gap Analysis — plan vs project → derive actions
// ═══════════════════════════════════════════════════════════════════════

/// Analyze gaps between plan features and current project state.
pub fn analyze_gaps(graph: &KnowledgeGraph, plan_features: &[String]) -> Vec<Gap> {
    let mut gaps = Vec::new();

    for feature_id in plan_features {
        let feature_node = match graph.get_node(feature_id) {
            Some(n) => n,
            None => continue,
        };

        let implementations = graph.related_by(feature_id, EdgeRelation::Implements);
        let tests = graph.related_by(feature_id, EdgeRelation::Tests);
        let specs = graph.specs_for(feature_id);

        let meta_status = feature_node.meta.get("status").map(|s| s.as_str());
        if meta_status == Some("verified") {
            gaps.push(Gap {
                feature_id: feature_id.clone(),
                feature_name: feature_node.name.clone(),
                status: GapStatus::Verified,
                missing: Vec::new(),
                unmet_criteria: Vec::new(),
            });
            continue;
        }

        let expected_files: Vec<String> = feature_node
            .meta
            .get("files")
            .map(|f| f.split(',').filter(|s| !s.is_empty()).map(String::from).collect())
            .unwrap_or_default();

        let all_criteria: Vec<String> = specs
            .iter()
            .flat_map(|s| s.acceptance_criteria.iter().cloned())
            .collect();

        let (status, missing, unmet) = if meta_status == Some("tested-failed") {
            (GapStatus::Tested, Vec::new(), all_criteria.clone())
        } else if meta_status == Some("implemented") || !implementations.is_empty() {
            if tests.is_empty() && meta_status != Some("tested-failed") {
                (GapStatus::Implemented, Vec::new(), all_criteria)
            } else {
                (GapStatus::Tested, Vec::new(), all_criteria)
            }
        } else {
            let impl_paths: Vec<String> = implementations
                .iter()
                .filter_map(|n| n.meta.get("path").cloned())
                .collect();

            if impl_paths.is_empty() && expected_files.is_empty() {
                (GapStatus::NotStarted, Vec::new(), all_criteria)
            } else if impl_paths.is_empty() {
                (GapStatus::NotStarted, expected_files.clone(), all_criteria)
            } else {
                let still_missing: Vec<String> = expected_files
                    .iter()
                    .filter(|f| !impl_paths.iter().any(|p| p.contains(f.as_str())))
                    .cloned()
                    .collect();
                if still_missing.is_empty() {
                    (GapStatus::Implemented, Vec::new(), all_criteria)
                } else {
                    let total = expected_files.len().max(1) as f32;
                    let ratio = 1.0 - (still_missing.len() as f32 / total);
                    (GapStatus::Partial(ratio), still_missing, all_criteria)
                }
            }
        };

        gaps.push(Gap {
            feature_id: feature_id.clone(),
            feature_name: feature_node.name.clone(),
            status,
            missing,
            unmet_criteria: unmet,
        });
    }

    gaps.sort_by(|a, b| {
        a.status
            .completion()
            .partial_cmp(&b.status.completion())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    gaps
}

/// Convert gaps into a prioritized, context-aware action queue.
pub fn derive_actions(gaps: &[Gap], graph: &KnowledgeGraph) -> Vec<Action> {
    let mut actions = Vec::new();

    if gaps.is_empty() {
        actions.push(Action {
            kind: ActionKind::Plan,
            priority: 1.0,
            context_nodes: Vec::new(),
            roska_depth: Depth::Overview,
            estimated_tokens: 2000,
        });
        return actions;
    }

    let all_not_started = gaps.iter().all(|g| g.status == GapStatus::NotStarted);
    if all_not_started {
        actions.push(Action {
            kind: ActionKind::Scaffold,
            priority: 0.95,
            context_nodes: gaps.iter().map(|g| g.feature_id.clone()).collect(),
            roska_depth: Depth::Overview,
            estimated_tokens: 3000,
        });
    }

    for gap in gaps {
        if gap.status == GapStatus::Verified {
            continue;
        }

        let action = match &gap.status {
            GapStatus::NotStarted | GapStatus::Partial(_) => {
                let deps_met = check_deps_met(&gap.feature_id, gaps, graph);
                Action {
                    kind: ActionKind::Implement {
                        feature_id: gap.feature_id.clone(),
                        feature_name: gap.feature_name.clone(),
                    },
                    priority: if deps_met { 0.9 } else { 0.3 },
                    context_nodes: feature_context_nodes(&gap.feature_id, graph),
                    roska_depth: Depth::Detail,
                    estimated_tokens: 8000,
                }
            }
            GapStatus::Implemented => Action {
                kind: ActionKind::Test {
                    feature_id: gap.feature_id.clone(),
                    feature_name: gap.feature_name.clone(),
                },
                priority: 0.7,
                context_nodes: feature_context_nodes(&gap.feature_id, graph),
                roska_depth: Depth::Structure,
                estimated_tokens: 5000,
            },
            GapStatus::Tested => Action {
                kind: ActionKind::Fix {
                    feature_id: gap.feature_id.clone(),
                    errors: gap.unmet_criteria.clone(),
                },
                priority: 0.8,
                context_nodes: feature_context_nodes(&gap.feature_id, graph),
                roska_depth: Depth::Body,
                estimated_tokens: 6000,
            },
            GapStatus::Verified => unreachable!(),
        };

        actions.push(action);
    }

    let all_impl = gaps.iter().all(|g| {
        matches!(
            g.status,
            GapStatus::Implemented | GapStatus::Tested | GapStatus::Verified
        )
    });
    if all_impl && gaps.len() > 1 {
        actions.push(Action {
            kind: ActionKind::Integrate,
            priority: 0.6,
            context_nodes: gaps.iter().map(|g| g.feature_id.clone()).collect(),
            roska_depth: Depth::Structure,
            estimated_tokens: 5000,
        });
    }

    let all_tested = gaps
        .iter()
        .all(|g| matches!(g.status, GapStatus::Tested | GapStatus::Verified));
    if all_tested {
        actions.push(Action {
            kind: ActionKind::Verify,
            priority: 0.5,
            context_nodes: gaps.iter().map(|g| g.feature_id.clone()).collect(),
            roska_depth: Depth::Overview,
            estimated_tokens: 3000,
        });
    }

    actions.sort_by(|a, b| {
        b.priority
            .partial_cmp(&a.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    actions
}

// ═══════════════════════════════════════════════════════════════════════
// Project Graph Updates — track what agents produce
// ═══════════════════════════════════════════════════════════════════════

/// After an agent runs, update the project graph with files it created.
pub fn update_project_graph(
    graph: &mut KnowledgeGraph,
    feature_id: &str,
    files_written: &[String],
    test_files: &[String],
) {
    for file_path in files_written {
        let file_name = Path::new(file_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.clone());

        let file_id = format!("file-{}", sanitize_id(&file_name));
        let node = Node::new(&file_id, &file_name, NodeKind::File)
            .with_tag("project")
            .with_meta("path", file_path)
            .with_description(format!("Source file: {}", file_path));
        let _ = graph.add_node(node);
        let _ = graph.add_edge(Edge::new(&file_id, feature_id, EdgeRelation::Implements));
    }

    for test_path in test_files {
        let test_name = Path::new(test_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| test_path.clone());

        let test_id = format!("test-{}", sanitize_id(&test_name));
        let node = Node::new(&test_id, &test_name, NodeKind::File)
            .with_tag("test")
            .with_tag("project")
            .with_meta("path", test_path)
            .with_description(format!("Test file: {}", test_path));
        let _ = graph.add_node(node);
        let _ = graph.add_edge(Edge::new(&test_id, feature_id, EdgeRelation::Tests));
    }
}

/// Mark a feature as implemented in the graph.
pub fn mark_feature_implemented(graph: &mut KnowledgeGraph, feature_id: &str) {
    if let Some(node) = graph.get_node_mut(feature_id) {
        node.meta.insert("status".to_string(), "implemented".to_string());
        node.touch();
    }
}

/// Mark a feature as tested (pass/fail) in the graph.
pub fn mark_feature_tested(graph: &mut KnowledgeGraph, feature_id: &str, passed: bool) {
    if let Some(node) = graph.get_node_mut(feature_id) {
        let status = if passed { "verified" } else { "tested-failed" };
        node.meta.insert("status".to_string(), status.to_string());
        node.touch();
    }
}

/// Record a Conversation node in the graph capturing the context injected
/// into an agent call.
pub fn record_action_context(
    graph: &mut KnowledgeGraph,
    action: &Action,
    iteration: u32,
    prompt_tokens: u32,
    completion_tokens: u32,
    duration_secs: f64,
    success: bool,
) {
    let conv_id = format!(
        "action-{}-iter{}",
        action.kind.label(),
        iteration
    );

    let context_desc = format!(
        "Action: {} | depth: {} | context_nodes: [{}] | tokens: {}in+{}out | {:.1}s | {}",
        action.kind.label(),
        action.roska_depth,
        action.context_nodes.join(", "),
        prompt_tokens,
        completion_tokens,
        duration_secs,
        if success { "OK" } else { "FAIL" },
    );

    let mut conv = Conversation::new(&conv_id, &context_desc);
    conv.add_message(MessageRole::System, format!(
        "roska_depth={} estimated_tokens={} priority={:.2}",
        action.roska_depth, action.estimated_tokens, action.priority
    ));
    conv.add_message(MessageRole::Agent, format!(
        "prompt_tokens={} completion_tokens={} duration_secs={:.1} success={}",
        prompt_tokens, completion_tokens, duration_secs, success
    ));

    let node = Node::new(&conv_id, &format!("Action: {}", action.kind.label()), NodeKind::Concept)
        .with_tag("action-context")
        .with_tag(action.kind.label())
        .with_meta("iteration", &iteration.to_string())
        .with_meta("prompt_tokens", &prompt_tokens.to_string())
        .with_meta("completion_tokens", &completion_tokens.to_string())
        .with_meta("duration_secs", &format!("{:.1}", duration_secs))
        .with_meta("roska_depth", &format!("{}", action.roska_depth))
        .with_meta("success", &success.to_string())
        .with_description(context_desc);

    let _ = graph.add_node(node);
    let _ = graph.add_conversation(conv);

    for ctx_id in &action.context_nodes {
        if graph.get_node(ctx_id).is_some() {
            let _ = graph.add_edge(Edge::new(&conv_id, ctx_id, EdgeRelation::Related));
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Status & Queries
// ═══════════════════════════════════════════════════════════════════════

/// Render a compact plan status summary.
pub fn render_plan_status(graph: &KnowledgeGraph, plan_features: &[String]) -> String {
    let gaps = analyze_gaps(graph, plan_features);
    let total = gaps.len();
    let verified = gaps.iter().filter(|g| g.status == GapStatus::Verified).count();
    let completion: f32 = if total > 0 {
        gaps.iter().map(|g| g.status.completion()).sum::<f32>() / total as f32
    } else {
        0.0
    };

    let mut out = format!(
        "Plan: {}/{} features complete ({:.0}%)\n",
        verified, total, completion * 100.0
    );
    for gap in &gaps {
        let icon = match &gap.status {
            GapStatus::NotStarted => "[ ]",
            GapStatus::Partial(p) => {
                if *p > 0.5 { "[~]" } else { "[.]" }
            }
            GapStatus::Implemented => "[*]",
            GapStatus::Tested => "[T]",
            GapStatus::Verified => "[V]",
        };
        out.push_str(&format!("  {} {}", icon, gap.feature_name));
        if !gap.missing.is_empty() {
            out.push_str(&format!(" (missing: {})", gap.missing.len()));
        }
        out.push('\n');
    }
    out
}

/// Check if all plan features are verified.
pub fn is_plan_complete(graph: &KnowledgeGraph, plan_features: &[String]) -> bool {
    if plan_features.is_empty() {
        return false;
    }
    let gaps = analyze_gaps(graph, plan_features);
    gaps.iter().all(|g| g.status == GapStatus::Verified)
}

/// Find existing plan features in the graph (for resuming).
pub fn find_plan_features(graph: &KnowledgeGraph) -> Vec<String> {
    let query = NodeQuery::new().kind(NodeKind::Feature).tag("plan");
    graph.find_nodes(&query).iter().map(|n| n.id.clone()).collect()
}

/// Extract file paths associated with a feature from the graph.
/// Returns (feature_files, dependency_files).
pub fn feature_file_context(
    feature_id: &str,
    graph: &KnowledgeGraph,
) -> (Vec<String>, Vec<String>) {
    let mut feature_files = Vec::new();
    let mut dep_files = Vec::new();

    if let Some(node) = graph.get_node(feature_id) {
        if let Some(files_meta) = node.meta.get("files") {
            for f in files_meta.split(',').filter(|s| !s.is_empty()) {
                feature_files.push(f.trim().to_string());
            }
        }
        if let Some(deps_meta) = node.meta.get("depends_on") {
            for dep_id in deps_meta.split(',').filter(|s| !s.is_empty()) {
                if let Some(dep_node) = graph.get_node(dep_id.trim()) {
                    if let Some(dep_files_meta) = dep_node.meta.get("files") {
                        for f in dep_files_meta.split(',').filter(|s| !s.is_empty()) {
                            let f = f.trim().to_string();
                            if !dep_files.contains(&f) && !feature_files.contains(&f) {
                                dep_files.push(f);
                            }
                        }
                    }
                }
            }
        }
    }

    (feature_files, dep_files)
}

/// Fallback: create features from files that already exist in the workspace.
pub fn create_features_from_files(
    task: &str,
    files: &[String],
    graph: &mut KnowledgeGraph,
) -> Vec<String> {
    if files.is_empty() {
        return Vec::new();
    }

    let intent_id = format!("intent-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let intent_node = Node::new(&intent_id, task, NodeKind::Concept)
        .with_tag("intent")
        .with_tag("plan-root")
        .with_weight(1.0)
        .with_description(format!("User request: {}", task));
    let _ = graph.add_node(intent_node);

    let mut groups: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for file in files {
        let path = std::path::Path::new(file);
        let group = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "root".to_string());
        let is_test = file.contains("test") || file.contains("spec");
        if !is_test {
            groups.entry(group).or_default().push(file.clone());
        }
    }

    let mut feature_ids = Vec::new();

    if groups.len() <= 1 && files.len() <= 5 {
        let feature_id = "plan-project".to_string();
        let files_csv: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
        let node = Node::new(&feature_id, task, NodeKind::Feature)
            .with_tag("plan")
            .with_meta("files", &files_csv.join(","))
            .with_description(format!("Full project: {} files", files.len()));
        let _ = graph.add_node(node);
        let _ = graph.add_edge(Edge::new(&intent_id, &feature_id, EdgeRelation::Parent));
        feature_ids.push(feature_id);
    } else {
        for (group_name, group_files) in &groups {
            let feature_id = format!("plan-{}", sanitize_id(group_name));
            let files_csv: Vec<&str> = group_files.iter().map(|s| s.as_str()).collect();
            let node = Node::new(&feature_id, group_name, NodeKind::Feature)
                .with_tag("plan")
                .with_meta("files", &files_csv.join(","))
                .with_description(format!("{}: {} files", group_name, group_files.len()));
            let _ = graph.add_node(node);
            let _ = graph.add_edge(Edge::new(&intent_id, &feature_id, EdgeRelation::Parent));
            feature_ids.push(feature_id);
        }
    }

    let test_files: Vec<&String> = files
        .iter()
        .filter(|f| f.contains("test") || f.contains("spec"))
        .collect();
    if !test_files.is_empty() {
        let test_feature_id = "plan-tests".to_string();
        let tf_csv: Vec<&str> = test_files.iter().map(|s| s.as_str()).collect();
        let node = Node::new(&test_feature_id, "Tests", NodeKind::Feature)
            .with_tag("plan")
            .with_tag("test")
            .with_meta("files", &tf_csv.join(","))
            .with_description(format!("Test suite: {} test files", test_files.len()));
        let _ = graph.add_node(node);
        let _ = graph.add_edge(Edge::new(&intent_id, &test_feature_id, EdgeRelation::Parent));
        feature_ids.push(test_feature_id);
    }

    feature_ids
}

/// Match workspace files to plan features and return feature IDs that have files.
pub fn match_files_to_features(
    workspace_files: &[String],
    plan_features: &[String],
    graph: &KnowledgeGraph,
) -> Vec<String> {
    let mut matched = Vec::new();

    let ws_normalized: Vec<String> = workspace_files
        .iter()
        .map(|f| {
            let p = std::path::Path::new(f);
            let components: Vec<&str> = p.components()
                .rev()
                .take(3)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|c| c.as_os_str().to_str().unwrap_or(""))
                .collect();
            components.join("/").to_lowercase()
        })
        .collect();

    for fid in plan_features {
        if let Some(node) = graph.get_node(fid) {
            if node.meta.get("status").map(|s| s.as_str()) == Some("implemented") {
                continue;
            }
            if node.meta.get("status").map(|s| s.as_str()) == Some("verified") {
                continue;
            }

            if let Some(files_meta) = node.meta.get("files") {
                let expected: Vec<&str> = files_meta.split(',')
                    .filter(|s| !s.is_empty())
                    .collect();

                if expected.is_empty() {
                    let name_lower = node.name.to_lowercase();
                    let name_hyphen = name_lower.replace(' ', "-");
                    let name_no_space = name_lower.replace(' ', "");
                    let has_match = ws_normalized.iter().any(|wf| {
                        wf.contains(&name_hyphen) || wf.contains(&name_no_space)
                            || name_lower.split_whitespace().all(|part| {
                                part.len() >= 3 && wf.contains(part)
                            })
                    });
                    if has_match {
                        matched.push(fid.clone());
                    }
                } else {
                    let found = expected.iter().filter(|ef| {
                        let ef_lower = ef.to_lowercase();
                        let ef_name = std::path::Path::new(&ef_lower)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let ef_hyphen = camel_to_hyphen(&ef_name);
                        ws_normalized.iter().any(|wf| {
                            wf.contains(&ef_lower)
                                || wf.ends_with(&ef_name)
                                || wf.ends_with(&ef_hyphen)
                        })
                    }).count();

                    if found > 0 {
                        matched.push(fid.clone());
                    }
                }
            } else {
                let name_lower = node.name.to_lowercase()
                    .replace(' ', "-")
                    .replace('_', "-");
                let has_match = ws_normalized.iter().any(|wf| {
                    wf.contains(&name_lower) || wf.contains(&name_lower.replace('-', ""))
                });
                if has_match {
                    matched.push(fid.clone());
                }
            }
        }
    }

    matched
}

/// Return a tuned max_iter for each action type.
pub fn adaptive_max_iter(action: &ActionKind, base: u32) -> u32 {
    match action {
        ActionKind::Plan => 5.min(base),
        ActionKind::Scan { .. } => 3.min(base),
        ActionKind::Scaffold => 10.min(base),
        ActionKind::Test { .. } => 15.min(base),
        ActionKind::Implement { .. } => base,
        ActionKind::Fix { .. } => 20.min(base),
        ActionKind::Integrate => 15.min(base),
        ActionKind::Verify => 10.min(base),
    }
}

// ── Helpers ──

fn check_deps_met(feature_id: &str, gaps: &[Gap], graph: &KnowledgeGraph) -> bool {
    let deps_str = graph
        .get_node(feature_id)
        .and_then(|n| n.meta.get("depends_on").cloned())
        .unwrap_or_default();

    if deps_str.is_empty() {
        return true;
    }

    for dep_id in deps_str.split(',').filter(|s| !s.is_empty()) {
        if let Some(dep_gap) = gaps.iter().find(|g| g.feature_id == dep_id) {
            if matches!(dep_gap.status, GapStatus::NotStarted | GapStatus::Partial(_)) {
                return false;
            }
        }
    }
    true
}

fn feature_context_nodes(feature_id: &str, graph: &KnowledgeGraph) -> Vec<String> {
    let mut nodes = vec![feature_id.to_string()];
    for parent in graph.parents(feature_id) {
        nodes.push(parent.id.clone());
    }
    if let Some(node) = graph.get_node(feature_id) {
        if let Some(deps) = node.meta.get("depends_on") {
            for dep in deps.split(',').filter(|s| !s.is_empty()) {
                nodes.push(dep.to_string());
            }
        }
    }
    for impl_node in graph.related_by(feature_id, EdgeRelation::Implements) {
        nodes.push(impl_node.id.clone());
    }
    nodes
}
