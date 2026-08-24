//! The board's LED map, state colours, paths and timings.
//!
//! Config is constants in code, as in the scripts this replaces; a file
//! only if it ever hurts.

use std::env;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb(pub u8, pub u8, pub u8);

pub const OFF: Rgb = Rgb(0, 0, 0);

/// White as this board renders it. The red switch housings pass enough of
/// the red channel that a plain #FFFFFF reads pink, so it comes down by a
/// third; picked by eye against candidates lit side by side. On-screen
/// whites are untouched — this is the keyboard's tint, not the state's.
pub const BOARD_WHITE: Rgb = Rgb(0xA0, 0xFF, 0xFF);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Idle,
    Working,
    Done,
    NeedsInput,
    Error,
    Off,
}

impl State {
    pub fn parse(s: &str) -> Option<State> {
        Some(match s {
            "idle" => State::Idle,
            "working" => State::Working,
            "done" => State::Done,
            "needs_input" => State::NeedsInput,
            "error" => State::Error,
            "off" => State::Off,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            State::Idle => "idle",
            State::Working => "working",
            State::Done => "done",
            State::NeedsInput => "needs_input",
            State::Error => "error",
            State::Off => "off",
        }
    }

    /// Slot colour, or None for a state that leaves the key dark. The
    /// firmware scales every channel down by its brightness cap, so each
    /// colour peaks a channel at 0xFF — anything less is left on the
    /// table (done was, at 0xCC; the rest already peaked).
    pub fn color(self) -> Option<Rgb> {
        Some(match self {
            State::Idle => BOARD_WHITE,
            State::Working => Rgb(0x00, 0x66, 0xFF),
            State::Done => Rgb(0x00, 0xFF, 0x40),
            State::NeedsInput => Rgb(0xFF, 0xCC, 0x00),
            State::Error => Rgb(0xFF, 0x00, 0x00),
            State::Off => return None,
        })
    }

    /// Worth a toast when a session enters it.
    pub fn worth_noticing(self) -> bool {
        matches!(self, State::Done | State::NeedsInput | State::Error)
    }

    /// On screen every state gets a dot, `off` included, where the board
    /// just leaves the key dark — and idle is plain white, since a screen
    /// has no switch housing to compensate for.
    pub fn dot(self) -> Rgb {
        match self {
            State::Idle => Rgb(0xFF, 0xFF, 0xFF),
            _ => self.color().unwrap_or(Rgb(0x59, 0x59, 0x59)),
        }
    }
}

pub const BASE_LAYER: u8 = 0;

/// Every slot: the LED it lights, and the base keycap the switcher labels
/// it with — a shifted symbol is never shown for a key you'd press
/// unshifted. The three right-hand letter rows come first (slot 1 = Y),
/// then the left half, which only fills once the right is full. The
/// modifier keys in among them are skipped: the switcher is typed into,
/// so a key you can't type is a slot you can't reach. Keep this in step
/// with `SLOT_KEYS` in `bin/agent-status`.
const SLOTS: [(u8, &str); 33] = [
    // right half, LEDs 26-42, less right shift (43)
    (26, "Y"),
    (27, "U"),
    (28, "I"),
    (29, "O"),
    (30, "P"),
    (31, "\\"),
    (32, "H"),
    (33, "J"),
    (34, "K"),
    (35, "L"),
    (36, ";"),
    (37, "'"),
    (38, "N"),
    (39, "M"),
    (40, ","),
    (41, "."),
    (42, "/"),
    // left half, LEDs 0-17, less left ctrl (6) and left shift (12)
    (0, "⇥"),
    (1, "Q"),
    (2, "W"),
    (3, "E"),
    (4, "R"),
    (5, "T"),
    (7, "A"),
    (8, "S"),
    (9, "D"),
    (10, "F"),
    (11, "G"),
    (13, "Z"),
    (14, "X"),
    (15, "C"),
    (16, "V"),
    (17, "B"),
];

pub const MAX_SLOTS: u32 = SLOTS.len() as u32;

fn slot(slot: u32) -> Option<&'static (u8, &'static str)> {
    SLOTS.get(usize::try_from(slot).ok()?.checked_sub(1)?)
}

pub fn slot_led(n: u32) -> Option<u8> {
    slot(n).map(|&(led, _)| led)
}

pub fn slot_key(n: u32) -> &'static str {
    slot(n).map_or("?", |&(_, key)| key)
}

/// What to bring forward after a switch. tmux does the session switching,
/// so this only has to be the terminal.
pub const TERMINAL_APP: &str = "Ghostty";

/// How long a toast stays up.
pub const TOAST: Duration = Duration::from_millis(2500);

/// The base ledmap's green home markers (F, J, both thumbs), repainted so
/// the always-on display doesn't lose them.
pub const HOME_MARKERS: [u8; 4] = [10, 24, 33, 51];
pub const MARKER_COLOR: Rgb = Rgb(0x46, 0x69, 0x14);

/// Store re-read cadence. Changes arrive by watcher; this only bounds how
/// stale the tmux-derived bits (error, GC) can get.
pub const STORE_REFRESH: Duration = Duration::from_secs(1);
pub const FOCUS_CHECK: Duration = Duration::from_secs(1);
pub const HEALTH_TTL: Duration = Duration::from_secs(5);
pub const RECONNECT: Duration = Duration::from_secs(2);
/// How long to wait on the board for an event. Layer changes are pushed,
/// so this only bounds how fast a store change reaches the LEDs.
pub const READ_TIMEOUT_MS: i32 = 50;
/// Loop pacing with no board to read from.
pub const IDLE_WAIT: Duration = Duration::from_millis(500);

fn home() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/".into()))
}

pub fn state_dir() -> PathBuf {
    home().join(".local/state/agent-status")
}

/// The pause marker, named for the binary that honours it.
pub fn pause_file() -> PathBuf {
    state_dir().join("deck-paused")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_map_right_half_then_left() {
        assert_eq!(slot_led(1), Some(26));
        assert_eq!(slot_led(8), Some(33)); // J: shares the home marker LED
        assert_eq!(slot_led(17), Some(42));
        assert_eq!(slot_led(18), Some(0)); // spillover starts on the left
        assert_eq!(slot_led(MAX_SLOTS), Some(17));
        assert_eq!(slot_led(0), None);
        assert_eq!(slot_led(MAX_SLOTS + 1), None);
        assert_eq!(slot_key(1), "Y");
        assert_eq!(slot_key(MAX_SLOTS), "B");
        assert_eq!(slot_key(MAX_SLOTS + 1), "?");
    }

    #[test]
    fn no_slot_sits_on_a_modifier_key() {
        // left ctrl and both shifts, which can't be typed at the switcher
        for led in [6, 12, 43] {
            assert!(
                !SLOTS.iter().any(|&(slot_led, _)| slot_led == led),
                "LED {led} is a modifier key"
            );
        }
        let leds: std::collections::BTreeSet<u8> = SLOTS.iter().map(|&(led, _)| led).collect();
        assert_eq!(leds.len(), SLOTS.len(), "no LED serves two slots");
    }

    #[test]
    fn the_board_is_compensated_but_the_screen_is_not() {
        assert_eq!(State::Idle.color(), Some(BOARD_WHITE));
        assert_eq!(State::Idle.dot(), Rgb(0xFF, 0xFF, 0xFF));
        assert_eq!(State::Off.dot(), Rgb(0x59, 0x59, 0x59));
    }

    #[test]
    fn only_lit_states_have_colours() {
        assert!(State::parse("off").unwrap().color().is_none());
        assert!(State::parse("error").unwrap().color().is_some());
        assert_eq!(State::parse("bogus"), None);
        assert_eq!(State::parse("needs_input").unwrap().as_str(), "needs_input");
    }
}
