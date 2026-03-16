# commander

A terminal tool that translates natural language into shell commands using the Claude AI API.

## Usage

```bash
commander --cmd "Show me the files under this directory"
```

**Example output:**
```
Asking Claude for commands to: Show me the files under this directory

1. ls
   # List files in the current directory (simple view)

2. ls -la
   # List all files including hidden ones, with permissions and sizes

3. find . -maxdepth 1
   # List files using find, one entry per line
```

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (1.70 or later)
- An [Anthropic API key](https://console.anthropic.com/)

### Install

```bash
git clone <repo-url>
cd commander
cargo install --path .
```

This builds a release binary and installs it to `~/.cargo/bin/commander`. Make sure `~/.cargo/bin` is in your `$PATH`.

On Windows, the binary is installed to `%USERPROFILE%\\.cargo\\bin\\commander.exe`. Make sure `%USERPROFILE%\\.cargo\\bin` is in your `PATH`.

## Setup

Export your Anthropic API key before running:

```bash
export ANTHROPIC_API_KEY=your_key_here
```

To make it permanent, add the line above to your `~/.bashrc` or `~/.zshrc`.

### Windows Setup

Set your Anthropic API key before running:

**PowerShell (current session):**

```powershell
$env:ANTHROPIC_API_KEY="your_key_here"
```

**PowerShell (persist for future sessions):**

```powershell
[Environment]::SetEnvironmentVariable("ANTHROPIC_API_KEY", "your_key_here", "User")
```

Restart PowerShell after setting a persistent variable.

**Command Prompt (current session):**

```cmd
set ANTHROPIC_API_KEY=your_key_here
```

Run example on Windows:

```powershell
commander.exe --cmd "List files in this folder"
```

## Examples

```bash
commander --cmd "Find all .log files modified in the last 7 days"
commander --cmd "Kill a process running on port 8080"
commander --cmd "Compress a folder into a tar.gz archive"
commander --cmd "Check how much disk space is being used"
commander --cmd "Download a file from a URL"
```

## How It Works

`commander` sends your natural language request to the Claude API (`claude-haiku-4-5`), which responds with practical shell commands and brief explanations. When multiple approaches exist, Claude returns a numbered list of options you can navigate with arrow keys — press Enter to run, `c` to copy, or `q` to quit.

## License

MIT
