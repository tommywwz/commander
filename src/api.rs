use colored::Colorize;
use reqwest::blocking::Client;
use std::collections::HashMap;

use crate::types::{ApiRequest, ApiResponse, Command, Message, parse_commands};

/// Calls the Anthropic API with the given system prompt and user command.
/// Returns parsed commands on success, or an error string on failure.
/// Prints the unparseable response directly and returns `Err("")` if no commands could be parsed.
pub fn fetch_commands(
    api_key: &str,
    system_prompt: String,
    user_cmd: &str,
) -> Result<HashMap<u32, Command>, String> {
    let request_body = ApiRequest {
        model: "claude-haiku-4-5-20251001".to_string(),
        max_tokens: 256,
        system: system_prompt,
        messages: vec![Message {
            role: "user".to_string(),
            content: user_cmd.to_string(),
        }],
    };

    println!("{} {}\n", "Asking Claude for commands to:".cyan(), user_cmd.bold());

    let client = Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .map_err(|e| format!("{} {e}", "Request failed:".red()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("{} {status}: {body}", "API error".red()));
    }

    let api_response = response
        .json::<ApiResponse>()
        .map_err(|e| format!("{} {e}", "Failed to parse response:".red()))?;

    let mut commands = HashMap::new();
    let mut raw_text = String::new();

    for block in &api_response.content {
        if block.block_type == "text" {
            if let Some(text) = &block.text {
                raw_text.push_str(text);
                commands.extend(parse_commands(text));
            }
        }
    }

    if commands.is_empty() {
        println!("{}", "Unparseable response:".yellow().bold());
        println!("{}", raw_text.trim());
        return Err(String::new());
    }

    Ok(commands)
}
