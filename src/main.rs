use clap::Parser;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Parser)]
#[command(name = "commander", about = "Translate natural language to shell commands using Claude AI")]
struct Args {
    /// Describe what you want to do (e.g. "show files in current directory")
    #[arg(long)]
    cmd: String,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

fn main() {
    let args = Args::parse();

    let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        eprintln!("Error: ANTHROPIC_API_KEY environment variable not set.");
        eprintln!("Set it with: export ANTHROPIC_API_KEY=your_key_here");
        std::process::exit(1);
    });

    let request_body = ApiRequest {
        model: "claude-opus-4-6".to_string(),
        max_tokens: 1024,
        system: String::new(),
        messages: vec![Message {
            role: "user".to_string(),
            content: args.cmd.clone(),
        }],
    };

    let client = Client::new();

    println!("Asking Claude for commands to: {}\n", args.cmd);

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send();

    match response {
        Err(e) => {
            eprintln!("Request failed: {e}");
            std::process::exit(1);
        }
        Ok(resp) => {
            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().unwrap_or_default();
                eprintln!("API error {status}: {body}");
                std::process::exit(1);
            }

            match resp.json::<ApiResponse>() {
                Err(e) => {
                    eprintln!("Failed to parse response: {e}");
                    std::process::exit(1);
                }
                Ok(api_response) => {
                    for block in &api_response.content {
                        if block.block_type == "text" {
                            if let Some(text) = &block.text {
                                println!("{text}");
                            }
                        }
                    }
                }
            }
        }
    }
}
