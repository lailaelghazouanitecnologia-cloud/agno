//! Neuroscience Blog Generator
//!
//! Uses the KKR OpenAI-compatible provider to call GLM-4.7 via baseten.co
//! and generate a neuroscience blog post.

use kkr_core::agent::Provider;
use kkr_core::{Message, Role};
use kkr_provider_openai::{OpenAI, OpenAIConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = OpenAIConfig::new("5K435aS6.7VDpoc8x2oxQ303Ppx9nNY08QHNKrjMI")
        .base_url("https://inference.baseten.co/v1")
        .model_name("zai-org/GLM-4.7")
        .max_tokens(2048)
        .temperature(0.7);

    let provider = OpenAI::new(config);

    println!("=== KKR Neuroscience Blog Generator ===");
    println!("Provider: baseten.co (OpenAI-compatible)");
    println!("Model: zai-org/GLM-4.7");
    println!("========================================\n");

    let messages = vec![
        Message {
            role: Role::System,
            content: "Eres un neurocientífico experto y escritor de blogs científicos. \
                      Escribes en español de forma clara, precisa y accesible para el público general. \
                      Usas terminología científica correcta pero la explicas de forma comprensible."
                .to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
        Message {
            role: Role::User,
            content: "Escribe una entrada de blog sobre la neurociencia del aprendizaje y la memoria. \
                      Incluye conceptos clave como la plasticidad sináptica, la potenciación a largo plazo (LTP), \
                      el papel del hipocampo, y cómo estos mecanismos se relacionan con técnicas de estudio efectivas. \
                      Formato: título, introducción, 3 secciones con subtítulos, y conclusión."
                .to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    println!("[Enviando request a GLM-4.7...]\n");

    match provider.generate(messages, None).await {
        Ok(response) => {
            println!("--- RESPUESTA DEL MODELO ---\n");
            println!("{}", response.message.content);
            println!("\n--- FIN DE RESPUESTA ---");
            println!("\n[Finish reason: {:?}]", response.finish_reason);
            if let Some(usage) = response.usage {
                println!(
                    "[Tokens - prompt: {}, completion: {}, total: {}]",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                );
            }
        }
        Err(e) => {
            eprintln!("Error del proveedor: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
