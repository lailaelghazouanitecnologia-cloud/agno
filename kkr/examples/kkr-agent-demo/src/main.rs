//! KKR Agent Demo
//!
//! Autonomous agent using GLM-4.7 via baseten.co (OpenAI-compatible).
//! The agent creates projects by calling filesystem tools directly.

use kkr_core::agent::{Agent, AgentConfig, AgentEvent, ApprovalPolicy};
use kkr_core::capsule::CapsuleBuilder;
use kkr_core::workspace::Workspace;
use kkr_core::Task;
use kkr_provider_openai::{OpenAI, OpenAIConfig};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -- Provider --
    // [FIX 2] max_tokens 4096 (was 8192) - prevents context explosion
    // [FIX 4] temperature 0.5 (was 0.7) - reliable tool calls + decent creativity
    let config = OpenAIConfig::new("5K435aS6.7VDpoc8x2oxQ303Ppx9nNY08QHNKrjMI")
        .base_url("https://inference.baseten.co/v1")
        .model_name("zai-org/GLM-4.7")
        .max_tokens(4096)
        .temperature(0.5);

    let provider = OpenAI::new(config);

    // -- Workspace --
    // [FIX 6] Relative output dir instead of /tmp
    let workspace_dir = std::env::current_dir()?
        .join("output")
        .to_string_lossy()
        .to_string();
    std::fs::create_dir_all(&workspace_dir)?;
    let workspace = Workspace::new(&workspace_dir);

    // -- Agent --
    // [FIX 1] No tool list in instructions - framework sends them via collect_tools()
    // [FIX 3] Verification step: agent must list_dir at the end
    // [FIX 5] Retries with exponential backoff for network resilience
    let agent_config = AgentConfig::new("KKR-Agent")
        .with_instructions(
            "Eres un agente autónomo de desarrollo de software. \
             Creas proyectos completos usando las herramientas de filesystem disponibles. \
             \n\nReglas:\n\
             1. Crea UN archivo por turno. Primero mkdir si necesitas directorios.\n\
             2. Usa paths relativos al workspace (no paths absolutos).\n\
             3. Al terminar, usa list_dir para verificar que todos los archivos existen.\n\
             4. Si un tool call falla, intenta de nuevo o ajusta el approach.\n\
             5. Escribe código funcional y bien estructurado."
        )
        .with_max_iterations(20)
        .with_retries(2)
        .with_exponential_backoff(true)
        .with_approval_policy(ApprovalPolicy::Never);

    // -- Capsule --
    let fs_capsule = CapsuleBuilder::new("fs")
        .description("Filesystem: create dirs, write/read files, list contents")
        .scope(".")
        .tool(Box::new(kkr_tool_fs::WriteFileTool::new()))
        .tool(Box::new(kkr_tool_fs::ReadFileTool::new()))
        .tool(Box::new(kkr_tool_fs::MkdirTool::new()))
        .tool(Box::new(kkr_tool_fs::ListDirTool::new()))
        .build();

    let mut agent = Agent::new(agent_config, workspace, Box::new(provider));
    agent.add_capsule(fs_capsule);

    println!("=== KKR Agent Demo ===");
    println!("Provider: baseten.co | Model: zai-org/GLM-4.7");
    println!("Workspace: {}", workspace_dir);
    println!("======================\n");

    // -- Events --
    // [FIX 7] Show tool results for Tool messages (not just assistant)
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(100);
    let event_handle = tokio::spawn(async move {
        let mut tool_count = 0u32;
        while let Some(event) = rx.recv().await {
            match event {
                AgentEvent::RunStarted { run_id } => {
                    println!("[AGENT] Run: {}", &run_id[..8]);
                }
                AgentEvent::ToolCallStarted { tool_name, .. } => {
                    tool_count += 1;
                    println!("[TOOL #{tool_count}] >> {tool_name}");
                }
                AgentEvent::ToolCallCompleted { tool_call_id, success } => {
                    let tag = if success { "OK" } else { "FAIL" };
                    let id = if tool_call_id.len() > 8 { &tool_call_id[..8] } else { &tool_call_id };
                    println!("[TOOL] << {id} [{tag}]");
                }
                AgentEvent::MessageAdded { message } => {
                    match message.role {
                        kkr_core::Role::Assistant if !message.content.is_empty() => {
                            println!("\n[GLM-4.7]:\n{}\n", message.content);
                        }
                        kkr_core::Role::Tool => {
                            let preview = if message.content.len() > 200 {
                                format!("{}...[{}B total]", &message.content[..200], message.content.len())
                            } else {
                                message.content.clone()
                            };
                            let name = message.name.as_deref().unwrap_or("?");
                            println!("[TOOL RESULT ({name})]: {preview}");
                        }
                        _ => {}
                    }
                }
                AgentEvent::RunCompleted { output } => {
                    println!("\n[AGENT] Completado: {:?}", output.status);
                    if let Some(ref err) = output.error {
                        println!("[ERROR]: {err}");
                    }
                }
                AgentEvent::Error { error } => {
                    eprintln!("[ERROR]: {error}");
                }
                _ => {}
            }
        }
    });

    // -- Task: C compiler in JavaScript --
    let task = Task::new(
        "Crea un compilador de C escrito en JavaScript. El proyecto debe incluir:\n\n\
         1. src/lexer.js - Lexer/tokenizador que convierte código C en tokens. \
            Debe soportar: keywords (int, float, char, void, if, else, while, for, return, struct), \
            operadores (+, -, *, /, =, ==, !=, <, >, <=, >=, &&, ||, !), \
            literales (enteros, flotantes, strings, chars), identificadores, \
            puntuación ({, }, (, ), ;, ,), y comentarios (// y /* */).\n\n\
         2. src/parser.js - Parser que construye un AST (Abstract Syntax Tree) \
            desde los tokens. Debe implementar recursive descent parsing para: \
            declaraciones de variables, funciones, if/else, while, for, return, \
            expresiones con precedencia de operadores, y structs.\n\n\
         3. src/codegen.js - Generador de código que recorre el AST y produce \
            JavaScript equivalente. Debe manejar: tipos C a JS, printf() a console.log(), \
            punteros simplificados, y structs a objetos JS.\n\n\
         4. src/index.js - Entry point que conecta lexer -> parser -> codegen. \
            Debe leer un archivo .c de stdin o argumento, compilar, y ejecutar el JS resultante.\n\n\
         5. examples/hello.c - Programa C de ejemplo que demuestre las capacidades del compilador.\n\n\
         6. examples/fibonacci.c - Programa C que calcule fibonacci recursivo.\n\n\
         7. package.json - Con nombre 'c-compiler-js', scripts de start y test.\n\n\
         Primero crea los directorios src/ y examples/ con mkdir, luego cada archivo con write_file. \
         Al final, verifica con list_dir que todo está correcto."
    );

    println!("[Iniciando agente...]\n");
    let output = agent.run_with_events(task, Some(tx)).await?;
    let _ = event_handle.await;

    // -- Summary --
    let usage = agent.total_usage();
    println!("\n======================");
    println!("[Resumen]");
    println!("  Status: {:?}", output.status);
    println!("  Tokens: {}k prompt + {}k completion = {}k total",
        usage.prompt_tokens / 1000,
        usage.completion_tokens / 1000,
        usage.total_tokens / 1000);

    // -- List output --
    println!("\n[Archivos en {}]:", workspace_dir);
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
                    println!("{}{} ({:.1}KB)", " ".repeat(indent), name, size as f64 / 1024.0);
                }
            }
        }
    }
    list_files(&workspace_dir, 2);

    Ok(())
}
