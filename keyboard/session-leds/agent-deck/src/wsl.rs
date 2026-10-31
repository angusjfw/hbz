//! The wsl.exe seam: on Windows every process the deck reaches into —
//! tmux, the status CLI, ps and kill for the suspended-client check —
//! lives inside WSL, not on the Windows filesystem. On macOS and Linux
//! these are just the commands themselves, so the two worlds meet here.

use std::process::Command;
#[cfg(target_os = "windows")]
use std::sync::OnceLock;

/// The distro the store and tmux live in. Config is constants in code;
/// this is the one Windows-specific knob.
#[cfg(target_os = "windows")]
pub const DISTRO: &str = "Debian";

/// Run `bin` — bare, on the distro's PATH — with `args` following.
pub fn command(bin: &str) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("wsl.exe");
        // `--shell-type none` execs the command directly, no shell: the
        // default shell (zsh) would expand `=name` and mangle the format
        // arguments tmux relies on.
        cmd.arg("-d")
            .arg(DISTRO)
            .arg("--shell-type")
            .arg("none")
            .arg("--")
            .arg(bin);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new(bin)
    }
}

/// The WSL user's home as a Linux path (`/home/<user>`), asked once.
/// The store and the status CLI both live under it.
#[cfg(target_os = "windows")]
pub fn home() -> &'static str {
    static HOME: OnceLock<String> = OnceLock::new();
    HOME.get_or_init(|| {
        command("printenv")
            .arg("HOME")
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .filter(|home| !home.is_empty())
            .unwrap_or_else(|| {
                crate::log("could not resolve the WSL home directory");
                "/home/unknown".to_string()
            })
    })
}

/// The status CLI, by absolute path: `~/.local/bin` isn't on the PATH
/// that `wsl.exe -- command` gets, so a bare `agent-status` wouldn't
/// resolve there.
pub fn agent_status() -> Command {
    #[cfg(target_os = "windows")]
    {
        command(&format!("{}/.local/bin/agent-status", home()))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("agent-status")
    }
}
