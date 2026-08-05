//! tmux is both the switch target and the liveness signal for a session.
//! Short-lived CLI calls, as in the status CLI.

use std::collections::{HashMap, HashSet};
use std::process::Command;

pub enum Panes {
    /// What's running in the session's panes.
    Commands(Vec<String>),
    /// tmux has no such session (or no server at all).
    Missing,
    /// tmux itself wouldn't run.
    Unavailable,
}

pub fn pane_commands(session: &str) -> Panes {
    let out = Command::new("tmux")
        .args([
            "list-panes",
            "-s",
            "-t",
            &format!("={session}"),
            "-F",
            "#{pane_current_command}",
        ])
        .output();
    match out {
        Ok(out) if out.status.success() => Panes::Commands(
            String::from_utf8_lossy(&out.stdout)
                .split_whitespace()
                .map(str::to_string)
                .collect(),
        ),
        Ok(_) => Panes::Missing,
        Err(_) => Panes::Unavailable,
    }
}

/// Session name to creation order. tmux session ids increment, so this is
/// the order the switcher lists them in.
pub fn creation_order() -> HashMap<String, u32> {
    let Ok(out) = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_id} #{session_name}"])
        .output()
    else {
        return HashMap::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let (id, name) = line.split_once(' ')?;
            Some((name.to_string(), id.trim_start_matches('$').parse().ok()?))
        })
        .collect()
}

/// The most recently active attached client, which is the one a switch
/// should move. `switch-client` needs it named explicitly.
pub fn latest_client() -> Option<String> {
    let out = Command::new("tmux")
        .args(["list-clients", "-F", "#{client_activity} #{client_tty}"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_once(' '))
        .max_by_key(|(activity, _)| activity.parse::<u64>().unwrap_or(0))
        .map(|(_, tty)| tty.to_string())
}

/// Sessions an attached client is currently looking at.
pub fn focused_sessions() -> HashSet<String> {
    let Ok(out) = Command::new("tmux")
        .args(["list-clients", "-F", "#{client_session}"])
        .output()
    else {
        return HashSet::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// Claude Code panes report the CLI's version string (e.g. "2.1.220") as
/// pane_current_command; claude and node are fallbacks.
pub fn looks_like_claude(cmd: &str) -> bool {
    if cmd == "claude" || cmd == "node" {
        return true;
    }
    let mut parts = cmd.split('.');
    let version = (0..3).all(|_| {
        parts
            .next()
            .is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    });
    version && parts.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::looks_like_claude;

    #[test]
    fn claude_panes_are_recognised_by_command() {
        assert!(looks_like_claude("2.1.220"));
        assert!(looks_like_claude("claude"));
        assert!(looks_like_claude("node"));
        assert!(!looks_like_claude("zsh"));
        assert!(!looks_like_claude("2.1"));
        assert!(!looks_like_claude("2.1.220.1"));
        assert!(!looks_like_claude("2.1.x"));
        assert!(!looks_like_claude(""));
    }
}
