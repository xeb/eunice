use crate::models::Provider;
use crate::provider::get_available_models;
use colored::*;
use std::env;

/// Print model information
pub fn print_model_info(model: &str, provider: &Provider) {
    println!(
        "{} {} ({})",
        provider.get_icon(),
        model.yellow().bold(),
        provider.to_string().yellow()
    );
}

/// Print error message
pub fn print_error(message: &str) {
    eprintln!("{} {}", "Error:".red(), message.red());
}

/// Print user stopped message (when user cancels with Escape)
pub fn print_user_stopped() {
    eprintln!("{}", "Stopped by user".yellow().bold());
}

/// Print available models in a list format
pub fn print_model_list() {
    println!("\n{}\n", "Available Models".bold().underline());

    let models = get_available_models();

    for (provider, model_list, available) in models {
        let icon = provider.get_icon();
        let status = if available { "available" } else { "unavailable" };
        let key_status = get_key_status(&provider, available);

        println!(
            "{} {} ({}) - {}",
            icon,
            provider.to_string().bold(),
            status,
            key_status.dimmed()
        );

        if model_list.is_empty() {
            println!("   {}", "No models available".dimmed());
        } else {
            for model in &model_list {
                println!("   {} {}", "-".dimmed(), model);
            }
        }
        println!();
    }
}

/// Get the status of the API key for a given provider
fn get_key_status(provider: &Provider, available: bool) -> String {
    match provider {
        Provider::Ollama => {
            if available {
                "running".to_string()
            } else {
                "not running".to_string()
            }
        }
        _ => {
            let key_name = match provider {
                Provider::OpenAI => "OPENAI_API_KEY",
                Provider::Gemini => "GEMINI_API_KEY",
                Provider::Anthropic => "ANTHROPIC_API_KEY",
                _ => unreachable!(),
            };
            if let Ok(key) = env::var(key_name) {
                if key.len() >= 4 {
                    format!("...{}", &key[key.len() - 4..])
                } else {
                    "set".to_string()
                }
            } else {
                "not set".to_string()
            }
        }
    }
}
