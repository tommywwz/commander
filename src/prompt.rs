use std::env;
use std::fs;

/// Detects OS, distro, and shell, then prepends that context to the base system prompt.
/// This lets Claude tailor commands to the user's actual environment at runtime.
pub fn build_system_prompt(base: &str) -> String {
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
