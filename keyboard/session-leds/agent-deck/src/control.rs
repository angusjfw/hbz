//! Controls. The pause ones are a marker file the running daemon
//! honours; `switch` runs here and then, borrowing the switcher's own
//! confirm-and-clear so nothing has to reimplement it:
//!
//!     agent-deck pause     # silent, and the HID device is closed
//!                          # (Keymapp needs it to flash firmware)
//!     agent-deck resume
//!     agent-deck status
//!     agent-deck switch <session> [client-tty]

use std::fs;

use crate::config;
use crate::input::{self, Switched};

/// Paused means paint nothing and leave the board alone entirely.
pub fn paused() -> bool {
    config::pause_file().exists()
}

pub fn run(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("pause") => {
            let _ = fs::create_dir_all(config::state_dir());
            if fs::write(config::pause_file(), "").is_err() {
                eprintln!("could not write {}", config::pause_file().display());
                return 1;
            }
            println!("paused");
        }
        Some("resume") => {
            let _ = fs::remove_file(config::pause_file());
            println!("running");
        }
        Some("status") => println!("{}", if paused() { "paused" } else { "running" }),
        // the client defaults to whichever tmux lists as most recently
        // active, which is what a switch from the keyboard moves
        Some("switch") if args.len() == 2 || args.len() == 3 => {
            let session = &args[1];
            return match input::switch(args.get(2).map(String::as_str), session) {
                Switched::Landed => 0,
                Switched::NoClient => {
                    eprintln!("no tmux client to switch");
                    1
                }
                Switched::Stuck { client } => {
                    eprintln!("{client} would not switch to {session}");
                    1
                }
            };
        }
        _ => {
            eprintln!("usage: agent-deck [pause|resume|status|switch <session> [client-tty]]");
            return 1;
        }
    }
    0
}
