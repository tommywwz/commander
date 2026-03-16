use arboard::Clipboard;
use clap::Parser;
use colored::Colorize;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::io::{stdout, Write};
use std::fs;

/// CLI arguments parsed by clap
#[derive(Parser)]
#[command(name = "commander", about = "Translate natural language to shell commands using Claude AI")]
struct Args {
    /// Describe what you want to do (e.g. "show files in current directory")
    #[arg(short, long, required_unless_present = "print_prompt")]
    cmd: Option<String>,

    /// Print the resolved system prompt and exit (for debugging)
    #[arg(long)]
    print_prompt: bool,
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

/// A parsed command with its optional inline comment
struct Command {
    cmd: String,
    comment: Option<String>,
}

/// What the user chose to do with the selected command
enum Action {
    Run(String),
    Copy(String),
    Quit,
}

/// Renders an interactive menu where the user can navigate with arrow keys,
/// press Enter to run the highlighted command, or 'c' to copy it.
fn interactive_menu(commands: &HashMap<u32, Command>) -> Action {
    let count = commands.len();
    let mut selected: usize = 0;
    let mut stdout = stdout();

    terminal::enable_raw_mode().unwrap();

    // count lines = one per command + blank line + hint line
    let menu_height = (count + 2) as u16;

    // Reserve space upfront so any terminal scrolling happens now, not during redraws
    for _ in 0..menu_height {
        execute!(stdout, Print("\r\n")).unwrap();
    }
    execute!(stdout, cursor::MoveUp(menu_height)).unwrap();

    // Capture the absolute row of the anchor after scrolling is done.
    // All redraws use MoveTo(0, anchor_row) — immune to scroll drift.
    let (_, anchor_row) = cursor::position().unwrap();

    let action = loop {
        // Jump to absolute anchor position and clear everything below
        execute!(
            stdout,
            cursor::MoveTo(0, anchor_row),
            terminal::Clear(ClearType::FromCursorDown)
        )
        .unwrap();

        // Draw each command, highlighting the selected one
        for i in 0..count {
            let entry = commands.get(&((i + 1) as u32)).unwrap();
            let comment_str = entry.comment.as_deref().unwrap_or("");
            if i == selected {
                execute!(
                    stdout,
                    SetBackgroundColor(Color::Cyan),
                    SetForegroundColor(Color::Black),
                    Print(format!(" > {}. {} ", i + 1, entry.cmd)),
                    ResetColor,
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!(" # {}\r\n", comment_str)),
                    ResetColor
                )
                .unwrap();
            } else {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Reset),
                    Print(format!("   {}. {} ", i + 1, entry.cmd)),
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!(" # {}\r\n", comment_str)),
                    ResetColor
                )
                .unwrap();
            }
        }

        // Key hint bar
        execute!(
            stdout,
            Print("\r\n"),
            SetForegroundColor(Color::DarkGrey),
            Print(" [↑↓] Navigate   [Enter] Run   [c] Copy   [q] Quit"),
            ResetColor
        )
        .unwrap();

        stdout.flush().unwrap();

        // Block until a key event arrives
        if let Event::Key(key) = event::read().unwrap() {
            match key.code {
                KeyCode::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if selected < count - 1 {
                        selected += 1;
                    }
                }
                KeyCode::Enter => {
                    let cmd = commands.get(&((selected + 1) as u32)).unwrap().cmd.clone();
                    break Action::Run(cmd);
                }
                KeyCode::Char('c') => {
                    let cmd = commands.get(&((selected + 1) as u32)).unwrap().cmd.clone();
                    break Action::Copy(cmd);
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    break Action::Quit;
                }
                _ => {}
            }
        }
    };

    // Restore normal terminal mode and clear the menu
    terminal::disable_raw_mode().unwrap();
    execute!(
        stdout,
        cursor::MoveTo(0, anchor_row),
        terminal::Clear(ClearType::FromCursorDown)
    )
    .unwrap();

    action
}

/// Detects OS, distro, and shell, then prepends that context to the base system prompt.
/// This lets Claude tailor commands to the user's actual environment at runtime.
fn build_system_prompt(base: &str) -> String {
    let os = std::env::consts::OS; // "linux", "macos", "windows"

    // On Linux, read /etc/os-release to get the distro name (e.g. "Ubuntu 24.04")
    let distro = if os == "linux" {
        fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|contents| {
                contents
                    .lines()
                    .find(|l| l.starts_with("PRETTY_NAME="))
                    .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
            })
            .unwrap_or_else(|| "Linux".to_string())
    } else {
        os.to_string()
    };

    // Read the current shell from $SHELL (e.g. /bin/zsh → zsh) on Unix,
    // or default to "powershell" on Windows.
    let shell = if cfg!(target_os = "windows") {
        "powershell".to_string()
    } else {
        env::var("SHELL")
            .ok()
            .and_then(|s| s.split('/').last().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown shell".to_string())
    };

    format!(
        "The user's environment:\n- OS: {distro}\n- Shell: {shell}\n\nTailor all commands to this environment.\n\n{base}"
    )
}

fn main() {
    let args = Args::parse();

    let system_prompt = build_system_prompt(include_str!("../config/system_prompt.md"));

    // Debug flag: print the resolved system prompt and exit without calling the API
    if args.print_prompt {
        println!("{}\n{}", "--- System Prompt ---".yellow().bold(), system_prompt);
        return;
    }

    // Read API key from environment — exits early with a helpful message if missing
    let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        eprintln!("{}", "Error: ANTHROPIC_API_KEY environment variable not set.".red());
        eprintln!("{}", "Set it with: export ANTHROPIC_API_KEY=your_key_here".yellow());
        std::process::exit(1);
    });

    let cmd = args.cmd.unwrap();

    // Build the API request: model, token limit, system prompt (embedded at compile time), and user message
    let request_body = ApiRequest {
        model: "claude-haiku-4-5-20251001".to_string(),
        max_tokens: 256,
        system: system_prompt,
        messages: vec![Message {
            role: "user".to_string(),
            content: cmd.clone(),
        }],
    };

    let client = Client::new();

    println!("{} {}\n", "Asking Claude for commands to:".cyan(), cmd.bold());

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
                    let mut commands: HashMap<u32, Command> = HashMap::new();
                    for block in &api_response.content {
                        if block.block_type == "text" {
                            if let Some(text) = &block.text {
                                for line in text.lines() {
                                    // Find the ". " separator between the number and command
                                    if let Some(dot_pos) = line.find(". ") {
                                        let num_str = &line[..dot_pos];
                                        if let Ok(num) = num_str.trim().parse::<u32>() {
                                            let rest = &line[dot_pos + 2..];
                                            // Split command and inline comment on " #"
                                            let (cmd_part, comment_part) = match rest.split_once(" #") {
                                                Some((c, comment)) => (c.trim(), Some(comment.trim().to_string())),
                                                None => (rest.trim(), None),
                                            };
                                            commands.insert(num, Command {
                                                cmd: cmd_part.to_string(),
                                                comment: comment_part,
                                            });
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

                    // Show interactive menu and handle the user's choice
                    match interactive_menu(&commands) {
                        Action::Run(cmd) => {
                            println!("{} {}\n", "Running:".bright_green().bold(), cmd.bold());
                            let (shell, flag) = if cfg!(target_os = "windows") {
                                ("cmd", "/C")
                            } else {
                                ("sh", "-c")
                            };
                            std::process::Command::new(shell)
                                .args([flag, &cmd])
                                .status()
                                .unwrap_or_else(|e| {
                                    eprintln!("{} {e}", "Failed to run command:".red());
                                    std::process::exit(1);
                                });
                        }
                        Action::Copy(cmd) => {
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
                        }
                        Action::Quit => {
                            println!("{}", "Cancelled.".dimmed());
                        }
                    }
                }
            }
        }
    }
}
