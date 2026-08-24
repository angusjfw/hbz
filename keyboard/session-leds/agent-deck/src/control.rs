//! Controls, as a marker file the running daemon honours:
//!
//!     agent-deck pause     # silent, and the HID device is closed
//!                          # (Keymapp needs it to flash firmware)
//!     agent-deck resume
//!     agent-deck status

use std::fs;

use crate::config;

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
        _ => {
            eprintln!("usage: agent-deck [pause|resume|status]");
            return 1;
        }
    }
    0
}
