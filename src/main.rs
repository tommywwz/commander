use arboard::Clipboard;
use clap::Parser;
use colored::Colorize;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::io::{self, Write};

/// CLI arguments parsed by clap
#[derive(Parser)]
#[command(name = "commander", about = "Translate natural language to shell commands using Claude AI")]
struct Args {
    /// Describe what you want to do (e.g. "show files in current directory")
    #[arg(short, long)]
    cmd: String,
}

/// A single message in the conversation (role + content)
#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

/// The full request body sent to the Anthropic API
#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

/// A single content block in the API response (type + optional text)
#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")] // "type" is a reserved keyword in Rust, so rename it
    block_type: String,
    text: Option<String>, // only present for "text" blocks
}

/// The top-level API response containing a list of content blocks
#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

fn main() {
    let args = Args::parse();

    // Read API key from environment — exits early with a helpful message if missing
    let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        eprintln!("{}", "Error: ANTHROPIC_API_KEY environment variable not set.".red());
        eprintln!("{}", "Set it with: export ANTHROPIC_API_KEY=your_key_here".yellow());
        std::process::exit(1);
    });

    // Build the API request: model, token limit, system prompt (embedded at compile time), and user message
    let request_body = ApiRequest {
        model: "claude-haiku-4-5-20251001".to_string(),
        max_tokens: 256,
        system: include_str!("../config/system_prompt.md").to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: args.cmd.clone(),
        }],
    };

    let client = Client::new();

    println!("{} {}\n", "Asking Claude for commands to:".cyan(), args.cmd.bold());

    // Send the request to the Anthropic API
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send();

    match response {
        Err(e) => {
            eprintln!("{} {e}", "Request failed:".red());
            std::process::exit(1);
        }
        Ok(resp) => {
            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().unwrap_or_default();
                eprintln!("{} {status}: {body}", "API error".red());
                std::process::exit(1);
            }

            match resp.json::<ApiResponse>() {
                Err(e) => {
                    eprintln!("{} {e}", "Failed to parse response:".red());
                    std::process::exit(1);
                }
                Ok(api_response) => {
                    // Parse numbered commands from the response text into a HashMap
                    // Expected format: "1. some-command   # description"
                    let mut commands: HashMap<u32, String> = HashMap::new();
                    for block in &api_response.content {
                        if block.block_type == "text" {
                            if let Some(text) = &block.text {
                                for line in text.lines() {
                                    // Find the ". " separator between the number and command
                                    if let Some(dot_pos) = line.find(". ") {
                                        let num_str = &line[..dot_pos];
                                        if let Ok(num) = num_str.trim().parse::<u32>() {
                                            let rest = &line[dot_pos + 2..];
                                            // Strip the inline comment (everything after " #")
                                            let command = rest.split(" #").next().unwrap_or(rest).trim();
                                            commands.insert(num, command.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if commands.is_empty() {
                        eprintln!("{}", "Could not parse any commands from response.".red());
                        std::process::exit(1);
                    }

                    // Print commands in order
                    for i in 1..=commands.len() as u32 {
                        if let Some(cmd) = commands.get(&i) {
                            println!("{}  {}", format!("{i}.").bright_blue().bold(), cmd.green());
                        }
                    }

                    // Prompt the user to pick a command to copy
                    print!("\n{}", "Enter number to copy to clipboard: ".yellow());
                    io::stdout().flush().unwrap();

                    let mut input = String::new();
                    io::stdin().read_line(&mut input).unwrap();

                    if let Ok(choice) = input.trim().parse::<u32>() {
                        if let Some(cmd) = commands.get(&choice) {
                            match Clipboard::new() {
                                Ok(mut cb) => {
                                    if let Err(e) = cb.set_text(cmd.clone()) {
                                        eprintln!("{} {e}", "Failed to copy to clipboard:".red());
                                    } else {
                                        println!("{} {}", "Copied:".bright_green().bold(), cmd.bold());
                                        // Keep clipboard alive briefly so clipboard managers can read it
                                        std::thread::sleep(std::time::Duration::from_millis(100));
                                    }
                                }
                                Err(e) => eprintln!("{} {e}", "Failed to open clipboard:".red()),
                            }
                        } else {
                            eprintln!("{}", "Invalid selection.".red());
                        }
                    } else {
                        eprintln!("{}", "Invalid input.".red());
                    }
                }
            }
        }
    }
}
