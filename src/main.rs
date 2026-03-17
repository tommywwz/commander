mod api;
mod prompt;
mod types;
mod ui;

use arboard::Clipboard;
use clap::Parser;
use colored::Colorize;
use std::env;

use types::Action;

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

fn main() {
    let args = Args::parse();

    let system_prompt = prompt::build_system_prompt(include_str!("../config/system_prompt.md"));

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

    let commands = match api::fetch_commands(&api_key, system_prompt, &cmd) {
        Ok(cmds) => cmds,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("{msg}");
            }
            std::process::exit(1);
        }
    };

    match ui::interactive_menu(&commands) {
        Action::Run(cmd) => {
            println!("{} {}\n", "Running:".bright_green().bold(), cmd.bold());
            let (shell, flag) = if cfg!(target_os = "windows") {
                ("powershell", "-Command")
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
