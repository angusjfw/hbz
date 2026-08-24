//! Switching to a session, and making sure the switch landed. The one
//! place that does it — the switcher goes through `switch_to`, and
//! anything else through `agent-deck switch` (see `control`).
//!
//! `switch-client` names a client by its tty path and nothing else —
//! tmux has no client id. A suspended client is hidden from
//! `list-clients` but still wins that lookup, so a switch aimed at the
//! tty can be accepted, exit 0, and move a client nobody can see. That
//! reads as "the terminal came forward but tmux didn't move", so every
//! switch is confirmed, and a phantom found in the way is cleared.

use std::process::Command;
use std::thread;

use crate::config;
use crate::overlay::{self, Shared};
use crate::tmux;

/// How a switch went, for whoever has to report it.
pub enum Switched {
    Landed,
    NoClient,
    Stuck { client: String },
}

/// Switch to `session` and bring the terminal forward. Off the loop's
/// thread — the tmux and launch calls cost more than a frame.
pub fn switch_to(shared: &Shared, session: &str) {
    let session = session.to_string();
    let shared = shared.clone();
    thread::spawn(move || {
        match switch(None, &session) {
            Switched::Landed => {}
            Switched::NoClient => overlay::notice(&shared, "no tmux client to switch"),
            Switched::Stuck { client } => {
                overlay::notice(&shared, &format!("tmux didn't switch ({client})"));
            }
        }
        if cfg!(target_os = "macos") {
            let _ = Command::new("open")
                .args(["-a", config::TERMINAL_APP])
                .status();
        }
    });
}

/// Switch `client` — or whichever tmux lists as most recently active —
/// to `session`, confirm it landed, and clear a phantom out of the way
/// if that's what swallowed it. Synchronous, and says nothing itself:
/// the caller knows whether it has a toast or a stderr to report on.
pub fn switch(client: Option<&str>, session: &str) -> Switched {
    let client = match client {
        Some(client) => client.to_string(),
        None => match tmux::latest_client() {
            Some(client) => client,
            None => return Switched::NoClient,
        },
    };
    if switch_and_confirm(&client, session) {
        return Switched::Landed;
    }
    // nothing else can name the client we meant, so clear what outranks
    // it and ask again
    if clear_suspended(&client) > 0 && switch_and_confirm(&client, session) {
        return Switched::Landed;
    }
    crate::log(&format!("{client} did not switch to {session}"));
    Switched::Stuck { client }
}

fn switch_and_confirm(client: &str, session: &str) -> bool {
    tmux::switch_client(client, session);
    tmux::client_session(client).as_deref() == Some(session)
}

/// Kill the suspended tmux clients sitting on `tty`. A stopped client
/// never processes a detach, so killing it is the only way to take it
/// out of the name lookup. Returns how many went.
fn clear_suspended(tty: &str) -> usize {
    let device = tty.trim_start_matches("/dev/");
    let Ok(out) = Command::new("ps")
        .args(["-o", "pid=,stat=,comm=", "-t", device])
        .output()
    else {
        return 0;
    };
    let pids = suspended_pids(&String::from_utf8_lossy(&out.stdout));
    for pid in &pids {
        crate::log(&format!("clearing suspended tmux client {pid} on {tty}"));
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
    }
    pids.len()
}

/// Stopped tmux processes in a `ps -o pid=,stat=,comm=` listing. State
/// `T` is the whole test for suspended, and the command has to be tmux
/// itself — nothing else on the tty is ours to kill.
fn suspended_pids(listing: &str) -> Vec<u32> {
    listing
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let stopped = fields.next()?.starts_with('T');
            let command = fields.next()?;
            let name = command.rsplit('/').next()?;
            (stopped && name == "tmux").then_some(pid)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::suspended_pids;

    #[test]
    fn only_stopped_tmux_clients_are_cleared() {
        let listing = "\
 5282 Ss   /usr/bin/login
 5284 S    -/bin/zsh
 5515 T    tmux
81000 S+   tmux
 9001 T    /opt/homebrew/bin/tmux
 9002 T    vim
";
        assert_eq!(suspended_pids(listing), vec![5515, 9001]);
        assert!(suspended_pids("").is_empty());
        assert!(suspended_pids("garbage\n").is_empty());
    }
}
