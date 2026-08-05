//! On-screen twin: the labelled session grid while the agent layer is on,
//! plus toasts on state changes worth noticing.
//!
//! Both still go through Hammerspoon — a marker file it watches for the
//! HUD, a URL event per toast — which the in-process egui panels replace
//! next (see the direct-HID spec).

use std::fs::{self, File};
use std::process::Command;
use std::thread;

use crate::config;

pub fn set_visible(visible: bool) {
    let _ = if visible {
        File::create(config::hud_file()).map(|_| ())
    } else {
        fs::remove_file(config::hud_file())
    };
}

pub fn toast(label: &str, state: &str) {
    if !cfg!(target_os = "macos") {
        return;
    }
    let url = format!(
        "hammerspoon://agent-toast?label={}&state={}",
        escape(label),
        escape(state)
    );
    // off the loop's thread, and waited on so no zombie is left behind
    thread::spawn(move || {
        let _ = Command::new("open").args(["-g", &url]).status();
    });
}

fn escape(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn labels_are_url_escaped() {
        assert_eq!(escape("asy-1121"), "asy-1121");
        assert_eq!(escape("a b&c=d"), "a%20b%26c%3Dd");
        assert_eq!(escape("ünïcode"), "%C3%BCn%C3%AFcode");
    }
}
