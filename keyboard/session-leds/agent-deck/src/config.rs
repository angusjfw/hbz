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

/// Slots 1-18 are the three right-hand letter rows, slot 1 = LED 26 (Y).
const RIGHT_FIRST_LED: u8 = 26;
const RIGHT_SLOTS: u32 = 18;
/// Slots 19-36 spill over onto the left half's letter rows (LEDs 0-17) in
/// the same row-major order. Assignment prefers the right half, so the
/// left only lights once the right is full.
const LEFT_FIRST_LED: u8 = 0;
const LEFT_FIRST_SLOT: u32 = RIGHT_SLOTS + 1;
pub const MAX_SLOTS: u32 = 36;

pub fn slot_led(slot: u32) -> Option<u8> {
    match slot {
        1..=RIGHT_SLOTS => Some(RIGHT_FIRST_LED + (slot - 1) as u8),
        LEFT_FIRST_SLOT..=MAX_SLOTS => Some(LEFT_FIRST_LED + (slot - LEFT_FIRST_SLOT) as u8),
        _ => None,
    }
}

/// The key each slot lives on, as the switcher labels it — base keycaps,
/// so a shifted symbol is never shown for a key you'd press unshifted.
const SLOT_KEYS: [&str; MAX_SLOTS as usize] = [
    "Y", "U", "I", "O", "P", "\\", "H", "J", "K", "L", ";", "'", "N", "M", ",", ".", "/", "⇧", "⇥",
    "Q", "W", "E", "R", "T", "⌃", "A", "S", "D", "F", "G", "⇧", "Z", "X", "C", "V", "B",
];

pub fn slot_key(slot: u32) -> &'static str {
    SLOT_KEYS.get(slot as usize - 1).copied().unwrap_or("?")
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

/// The status CLI serialises its read-assign-write on this directory; our
/// writes take the same one. Best-effort on both sides — a lock nobody
/// released within `LOCK_STALE` is taken, and after `LOCK_TIMEOUT` the
/// writer proceeds anyway rather than stalling a hook or a paint.
pub fn lock_dir() -> PathBuf {
    state_dir().join(".lock")
}

pub const LOCK_TIMEOUT: Duration = Duration::from_secs(2);
pub const LOCK_STALE: Duration = Duration::from_secs(10);
pub const LOCK_POLL: Duration = Duration::from_millis(50);

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
        assert_eq!(slot_led(18), Some(43));
        assert_eq!(slot_led(19), Some(0)); // spillover starts on the left
        assert_eq!(slot_led(36), Some(17));
        assert_eq!(slot_led(0), None);
        assert_eq!(slot_led(37), None);
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
