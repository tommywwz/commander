use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single message in the conversation (role + content)
#[derive(Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// The full request body sent to the Anthropic API
#[derive(Serialize)]
pub struct ApiRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<Message>,
}

/// A single content block in the API response (type + optional text)
#[derive(Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")] // "type" is a reserved keyword in Rust, so rename it
    pub block_type: String,
    pub text: Option<String>, // only present for "text" blocks
}

/// The top-level API response containing a list of content blocks
#[derive(Deserialize)]
pub struct ApiResponse {
    pub content: Vec<ContentBlock>,
}

/// A parsed command with its optional inline comment
pub struct Command {
    pub cmd: String,
    pub comment: Option<String>,
}

/// What the user chose to do with the selected command
pub enum Action {
    Run(String),
    Copy(String),
    Quit,
}

/// Returns the keys of `commands` in ascending sorted order.
pub fn sorted_command_keys(commands: &HashMap<u32, Command>) -> Vec<u32> {
    let mut keys: Vec<u32> = commands.keys().copied().collect();
    keys.sort_unstable();
    keys
}

/// Parse numbered commands from API response text into a HashMap.
/// Expected format: "1. some-command   # description"
pub fn parse_commands(text: &str) -> HashMap<u32, Command> {
    let mut commands = HashMap::new();
    for line in text.lines() {
        if let Some(dot_pos) = line.find(". ") {
            let num_str = &line[..dot_pos];
            if let Ok(num) = num_str.trim().parse::<u32>() {
                let rest = &line[dot_pos + 2..];
                let (cmd_part, comment_part) = match rest.split_once(" #") {
                    Some((c, comment)) => (c.trim(), Some(comment.trim().to_string())),
                    None => (rest.trim(), None),
                };
                commands.insert(
                    num,
                    Command {
                        cmd: cmd_part.to_string(),
                        comment: comment_part,
                    },
                );
            }
        }
    }
    commands
}
