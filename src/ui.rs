use colored::Colorize;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::collections::HashMap;
use std::io::{stdin, stdout, IsTerminal, Write};

use crate::types::{Action, Command, sorted_command_keys};

/// Line-based fallback menu for non-TTY environments (e.g. IDE output panes).
pub fn prompt_menu(commands: &HashMap<u32, Command>) -> Action {
    let keys = sorted_command_keys(commands);

    println!("{}", "Choose an option:".cyan().bold());
    for (idx, key) in keys.iter().enumerate() {
        let entry = &commands[key];
        let comment = entry.comment.as_deref().unwrap_or("");
        if comment.is_empty() {
            println!("  {}. {}", idx + 1, entry.cmd);
        } else {
            println!("  {}. {} {} {}", idx + 1, entry.cmd, "#".dimmed(), comment.dimmed());
        }
    }

    println!("\n{}", "Enter a number to run, 'c <number>' to copy, or 'q' to quit.".dimmed());

    let mut line = String::new();
    loop {
        print!("> ");
        stdout().flush().ok();

        line.clear();
        if stdin().read_line(&mut line).is_err() {
            return Action::Quit;
        }

        let input = line.trim();
        if input.eq_ignore_ascii_case("q") {
            return Action::Quit;
        }

        if let Some(rest) = input.strip_prefix('c').or_else(|| input.strip_prefix('C')) {
            let num = rest.trim().parse::<usize>();
            if let Ok(idx) = num {
                if (1..=keys.len()).contains(&idx) {
                    let cmd = commands[&keys[idx - 1]].cmd.clone();
                    return Action::Copy(cmd);
                }
            }
            println!("{}", "Invalid copy selection. Use: c <number>".yellow());
            continue;
        }

        let num = input.parse::<usize>();
        if let Ok(idx) = num {
            if (1..=keys.len()).contains(&idx) {
                let cmd = commands[&keys[idx - 1]].cmd.clone();
                return Action::Run(cmd);
            }
        }

        println!("{}", "Invalid selection. Enter a listed number, c <number>, or q.".yellow());
    }
}

/// Renders an interactive menu where the user can navigate with arrow keys,
/// press Enter to run the highlighted command, or 'c' to copy it.
/// Falls back to `prompt_menu` when stdout/stdin is not a TTY or raw mode fails.
pub fn interactive_menu(commands: &HashMap<u32, Command>) -> Action {
    let keys = sorted_command_keys(commands);
    let count = keys.len();

    if count == 0 {
        return Action::Quit;
    }

    // Raw key handling requires a real terminal. In some Windows contexts
    // (IDE output panes / non-TTY), fallback to a line-based prompt.
    if !stdin().is_terminal() || !stdout().is_terminal() {
        return prompt_menu(commands);
    }

    let mut selected: usize = 0;
    let mut stdout = stdout();

    if terminal::enable_raw_mode().is_err() {
        return prompt_menu(commands);
    }

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
            let entry = &commands[&keys[i]];
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
            if key.kind != KeyEventKind::Press {
                continue;
            }

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
                    let cmd = commands[&keys[selected]].cmd.clone();
                    break Action::Run(cmd);
                }
                KeyCode::Char('c') => {
                    let cmd = commands[&keys[selected]].cmd.clone();
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
