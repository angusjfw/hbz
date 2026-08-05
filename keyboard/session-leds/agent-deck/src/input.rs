//! Agent-layer presses. The board is the input device for its own
//! feature: a press arrives as a key position, so switching needs no
//! global hotkey and no keycode reaches the OS.

use std::process::Command;
use std::thread;

use crate::config;
use crate::tmux;

/// Switch to `session` and bring the terminal forward. Off the loop's
/// thread — the tmux and launch calls cost more than a frame.
pub fn switch_to(session: &str) {
    let session = session.to_string();
    thread::spawn(move || {
        if let Some(client) = tmux::latest_client() {
            let _ = Command::new("tmux")
                .args(["switch-client", "-c", &client, "-t", &session])
                .status();
        }
        if cfg!(target_os = "macos") {
            let _ = Command::new("open")
                .args(["-a", config::TERMINAL_APP])
                .status();
        }
    });
}
