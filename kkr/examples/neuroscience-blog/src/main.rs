//! Neuroscience Blog Agent
//!
//! A full KKR agent that uses GLM-4.7 via baseten.co (OpenAI-compatible)
//! to autonomously create a neuroscience blog project with files.

use kkr_core::agent::{Agent, AgentConfig, AgentEvent, ApprovalPolicy};
use kkr_core::capsule::CapsuleBuilder;
use kkr_core::Task;
use kkr_core::workspace::Workspace;
use kkr_provider_openai::{OpenAI, OpenAIConfig};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -- Provider config --
    let config = OpenAIConfig::new("5K435aS6.7VDpoc8x2oxQ303Ppx9nNY08QHNKrjMI")
        .base_url("https://inference.baseten.co/v1")
        .model_name("zai-org/GLM-4.7")
        .max_tokens(8192)
        .temperature(0.7);

    let provider = OpenAI::new(config);

    // -- Workspace --
    let workspace_dir = "/tmp/neuroscience-blog";
    std::fs::create_dir_all(workspace_dir)?;
    let workspace = Workspace::new(workspace_dir);

    // -- Agent config --
    let agent_config = AgentConfig::new("NeuroBlogAgent")
        .with_instructions(
            "Eres un agente autónomo experto en neurociencia y desarrollo web. \
             Tu tarea es crear un proyecto de blog sobre neurociencia. \
             DEBES usar las herramientas disponibles para crear los archivos. \
             \n\nHerramientas disponibles:\n\
             - fs_write_file: Crea/escribe archivos (params: path, content)\n\
             - fs_read_file: Lee archivos (params: path)\n\
             - fs_mkdir: Crea directorios (params: path)\n\
             - fs_list_dir: Lista directorios (params: path)\n\n\
             IMPORTANTE: Crea UN SOLO archivo por turno de respuesta. \
             Primero mkdir, luego un write_file, espera confirmación, y sigue con el siguiente. \
             Usa paths relativos al workspace."
        )
        .with_max_iterations(15)
        .with_approval_policy(ApprovalPolicy::Never);

    // -- Build capsule with filesystem tools --
    let fs_capsule = CapsuleBuilder::new("fs")
        .description("Filesystem operations for creating blog files")
        .scope(".")
        .tool(Box::new(kkr_tool_fs::WriteFileTool::new()))
        .tool(Box::new(kkr_tool_fs::ReadFileTool::new()))
        .tool(Box::new(kkr_tool_fs::MkdirTool::new()))
        .tool(Box::new(kkr_tool_fs::ListDirTool::new()))
        .build();

    // -- Create agent --
    let mut agent = Agent::new(agent_config, workspace, Box::new(provider));
    agent.add_capsule(fs_capsule);

    println!("=== KKR Neuroscience Blog Agent ===");
    println!("Provider: baseten.co (OpenAI-compatible)");
    println!("Model: zai-org/GLM-4.7");
    println!("Workspace: {}", workspace_dir);
    println!("Tools: fs_write_file, fs_read_file, fs_mkdir, fs_list_dir");
    println!("====================================\n");

    // -- Event channel for real-time output --
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(100);

    // -- Task --
    let task = Task::new(
        "Crea un proyecto de blog de neurociencia con los siguientes archivos:\n\
         1. index.html - Página principal del blog con CSS inline, título 'NeuroSciencia Blog', \
            navegación, y links a los posts\n\
         2. posts/plasticidad-sinaptica.html - Artículo sobre plasticidad sináptica y LTP\n\
         3. posts/hipocampo-memoria.html - Artículo sobre el hipocampo y la formación de memorias\n\
         4. posts/tecnicas-estudio.html - Artículo sobre técnicas de estudio basadas en neurociencia\n\
         5. about.html - Página 'Sobre nosotros' del blog\n\n\
         Cada archivo debe ser HTML completo con CSS inline, contenido científico real y preciso, \
         en español. Usa mkdir para crear el directorio posts/ primero, luego write_file para cada archivo."
    );

    // -- Spawn event listener --
    let event_handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                AgentEvent::RunStarted { run_id } => {
                    println!("[AGENT] Run iniciado: {}", &run_id[..8]);
                }
                AgentEvent::ToolCallStarted { tool_name, .. } => {
                    println!("[TOOL] Ejecutando: {}", tool_name);
                }
                AgentEvent::ToolCallCompleted { tool_call_id, success } => {
                    let status = if success { "OK" } else { "ERROR" };
                    let id_preview = if tool_call_id.len() > 8 { &tool_call_id[..8] } else { &tool_call_id };
                    println!("[TOOL] Completado: {} [{}]", id_preview, status);
                }
                AgentEvent::MessageAdded { message } => {
                    if message.role == kkr_core::Role::Assistant && !message.content.is_empty() {
                        println!("\n[GLM-4.7 dice]:\n{}\n", message.content);
                    }
                }
                AgentEvent::RunCompleted { output } => {
                    println!("\n[AGENT] Run completado - Status: {:?}", output.status);
                    if let Some(ref result) = output.result {
                        if !result.is_empty() {
                            println!("\n[RESULTADO FINAL]:\n{}", result);
                        }
                    }
                    if let Some(ref error) = output.error {
                        println!("[ERROR]: {}", error);
                    }
                }
                AgentEvent::Error { error } => {
                    println!("[ERROR]: {}", error);
                }
                _ => {}
            }
        }
    });

    // -- Run the agent --
    println!("[Iniciando agente...]\n");
    let output = agent.run_with_events(task, Some(tx)).await?;

    // Wait for all events to be processed
    let _ = event_handle.await;

    // -- Summary --
    let usage = agent.total_usage();
    println!("\n====================================");
    println!("[Resumen]");
    println!("  Status: {:?}", output.status);
    println!("  Tokens - prompt: {}, completion: {}, total: {}",
        usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);

    // -- List created files --
    println!("\n[Archivos creados en {}]:", workspace_dir);
    fn list_files(dir: &str, indent: usize) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let name = entry.file_name().to_string_lossy().to_string();
                let path = entry.path();
                if path.is_dir() {
                    println!("{}{}/", " ".repeat(indent), name);
                    list_files(&path.to_string_lossy(), indent + 2);
                } else {
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    println!("{}{} ({}B)", " ".repeat(indent), name, size);
                }
            }
        }
    }
    list_files(workspace_dir, 2);

    Ok(())
}
