//! Controls, as marker files the running daemon honours:
//!
//!     agent-deck pause          # silent, and the HID device is closed
//!                               # (Keymapp needs it to flash firmware)
//!     agent-deck pause notify   # only stop transition flashes
//!     agent-deck resume
//!     agent-deck base on|off    # always-on base-layer display (default on)
//!     agent-deck status

use std::fs;

use crate::config;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pause {
    /// No output at all, and the board is left alone entirely.
    All,
    /// Paint as usual, but no transition flashes.
    Notify,
}

/// None while running.
pub fn pause_mode() -> Option<Pause> {
    Some(parse_pause(&fs::read_to_string(config::pause_file()).ok()?))
}

fn parse_pause(marker: &str) -> Pause {
    match marker.trim() {
        "notify" => Pause::Notify,
        _ => Pause::All,
    }
}

pub fn base_display_on() -> bool {
    !config::base_off_file().exists()
}

pub fn run(args: &[String]) -> i32 {
    let arg = |n: usize| args.get(n).map(String::as_str);
    match (arg(0), arg(1)) {
        (Some("pause"), mode) => {
            let mode = mode.unwrap_or("all");
            if mode != "all" && mode != "notify" {
                eprintln!("unknown pause mode {mode:?}, want notify or nothing");
                return 1;
            }
            let _ = fs::create_dir_all(config::state_dir());
            if fs::write(config::pause_file(), mode).is_err() {
                eprintln!("could not write {}", config::pause_file().display());
                return 1;
            }
            println!("paused ({mode})");
        }
        (Some("resume"), _) => {
            let _ = fs::remove_file(config::pause_file());
            println!("running");
        }
        (Some("base"), Some(setting @ ("on" | "off"))) => {
            let _ = fs::create_dir_all(config::state_dir());
            if setting == "off" {
                let _ = fs::write(config::base_off_file(), "");
            } else {
                let _ = fs::remove_file(config::base_off_file());
            }
            println!("base-layer display {setting}");
        }
        (Some("status"), _) => {
            let state = match pause_mode() {
                Some(Pause::All) => "paused (all)".to_string(),
                Some(Pause::Notify) => "paused (notify)".to_string(),
                None => "running".to_string(),
            };
            let base = if base_display_on() { "on" } else { "off" };
            println!("{state} | base-layer display {base}");
        }
        _ => {
            eprintln!("usage: agent-deck [pause [notify]|resume|base on|off|status]");
            return 1;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_marker_defaults_to_a_full_pause() {
        assert_eq!(parse_pause("notify\n"), Pause::Notify);
        assert_eq!(parse_pause("all"), Pause::All);
        assert_eq!(parse_pause(""), Pause::All);
    }
}
