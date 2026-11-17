//! tmux is both the switch target and the liveness signal for a session.
//! Short-lived CLI calls, as in the status CLI.

use std::collections::{HashMap, HashSet};

pub struct Pane {
    pub pid: u32,
    pub command: String,
}

pub enum Panes {
    /// What's running in the session's panes.
    Commands(Vec<Pane>),
    /// tmux has no such session (or no server at all).
    Missing,
    /// tmux itself wouldn't run.
    Unavailable,
}

pub fn pane_commands(session: &str) -> Panes {
    let out = crate::wsl::command("tmux")
        .args([
            "list-panes",
            "-s",
            "-t",
            &format!("={session}"),
            "-F",
            "#{pane_pid} #{pane_current_command}",
        ])
        .output();
    match out {
        Ok(out) if out.status.success() => Panes::Commands(
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| {
                    let (pid, command) = line.split_once(' ')?;
                    Some(Pane {
                        pid: pid.parse().ok()?,
                        command: command.trim().to_string(),
                    })
                })
                .collect(),
        ),
        Ok(_) => Panes::Missing,
        Err(_) => Panes::Unavailable,
    }
}

/// Whether an agent runs as a direct child of one of these panes. A
/// wrapper shell keeps the pane's foreground process group, so tmux
/// names the shell (amx runs agents under `/bin/sh`, which is bash on
/// macOS) and the agent only shows up as its child.
pub fn agent_child(panes: &[Pane]) -> bool {
    let Ok(out) = crate::wsl::command("ps")
        .args(["-A", "-o", "ppid=,comm="])
        .output()
    else {
        return false;
    };
    let pids: HashSet<u32> = panes.iter().map(|p| p.pid).collect();
    child_commands(&String::from_utf8_lossy(&out.stdout), &pids)
        .any(looks_like_claude)
}

/// Commands of the children of `pids` in a `ps -o ppid=,comm=` listing,
/// as bare names: macOS gives `comm` as a path when the process ran by one.
fn child_commands<'a>(listing: &'a str, pids: &'a HashSet<u32>) -> impl Iterator<Item = &'a str> {
    listing.lines().filter_map(move |line| {
        let (ppid, comm) = line.trim().split_once(char::is_whitespace)?;
        pids.contains(&ppid.parse().ok()?)
            .then(|| comm.trim().rsplit('/').next().unwrap_or(""))
    })
}

/// Session name to creation order. tmux session ids increment, so this is
/// the order the switcher lists them in.
pub fn creation_order() -> HashMap<String, u32> {
    let Ok(out) = crate::wsl::command("tmux")
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
    let out = crate::wsl::command("tmux")
        .args(["list-clients", "-F", "#{client_activity} #{client_tty}"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_once(' '))
        .max_by_key(|(activity, _)| activity.parse::<u64>().unwrap_or(0))
        .map(|(_, tty)| tty.to_string())
}

/// Point `client` at `session`. Best-effort and unchecked: a switch can
/// be accepted and still not land, so the caller confirms with
/// `client_session` (see `input`).
pub fn switch_client(client: &str, session: &str) {
    let _ = crate::wsl::command("tmux")
        .args(["switch-client", "-c", client, "-t", &format!("={session}")])
        .status();
}

/// The session the client on `tty` is showing, as tmux lists it. None
/// when it lists none there — a suspended client never appears, which is
/// exactly what makes it worth asking.
pub fn client_session(tty: &str) -> Option<String> {
    let out = crate::wsl::command("tmux")
        .args(["list-clients", "-F", "#{client_tty} #{client_session}"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_once(' '))
        .find(|&(listed, _)| listed == tty)
        .map(|(_, session)| session.to_string())
}

/// Sessions an attached client is currently looking at.
pub fn focused_sessions() -> HashSet<String> {
    let Ok(out) = crate::wsl::command("tmux")
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

/// Agent panes report the CLI's version string (e.g. "2.1.220") as
/// pane_current_command; claude, node and pi are the named fallbacks.
pub fn looks_like_claude(cmd: &str) -> bool {
    if cmd == "claude" || cmd == "node" || cmd == "pi" {
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
    use super::{child_commands, looks_like_claude};
    use std::collections::HashSet;

    #[test]
    fn agents_are_found_under_a_wrapper_shell() {
        let listing =
            "    1 /sbin/launchd\n62349 claude\n62349 /opt/homebrew/bin/node\n  999 2.1.220\n";
        let pids = HashSet::from([62349]);
        let found: Vec<&str> = child_commands(listing, &pids).collect();
        assert_eq!(found, ["claude", "node"]);
        assert_eq!(child_commands(listing, &HashSet::from([7])).count(), 0);
    }

    #[test]
    fn claude_panes_are_recognised_by_command() {
        assert!(looks_like_claude("2.1.220"));
        assert!(looks_like_claude("claude"));
        assert!(looks_like_claude("node"));
        assert!(looks_like_claude("pi"));
        assert!(!looks_like_claude("zsh"));
        assert!(!looks_like_claude("pico"));
        assert!(!looks_like_claude("2.1"));
        assert!(!looks_like_claude("2.1.220.1"));
        assert!(!looks_like_claude("2.1.x"));
        assert!(!looks_like_claude(""));
    }
}
