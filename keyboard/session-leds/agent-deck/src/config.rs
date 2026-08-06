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

    /// Slot colour, or None for a state that leaves the key dark.
    pub fn color(self) -> Option<Rgb> {
        Some(match self {
            State::Idle => Rgb(0xFF, 0xFF, 0xFF),
            State::Working => Rgb(0x00, 0x66, 0xFF),
            State::Done => Rgb(0x00, 0xCC, 0x33),
            State::NeedsInput => Rgb(0xFF, 0xCC, 0x00),
            State::Error => Rgb(0xFF, 0x00, 0x00),
            State::Off => return None,
        })
    }

    /// Worth a flash and a toast when a session enters it.
    pub fn worth_noticing(self) -> bool {
        matches!(self, State::Done | State::NeedsInput | State::Error)
    }

    /// On screen every state gets a dot, `off` included, where the board
    /// just leaves the key dark.
    pub fn dot(self) -> Rgb {
        self.color().unwrap_or(Rgb(0x59, 0x59, 0x59))
    }
}

pub const BASE_LAYER: u8 = 0;
pub const AGENT_LAYER: u8 = 3;

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

/// The slot a key stands for, if it is one — the inverse of `slot_led`.
pub fn led_slot(led: u8) -> Option<u32> {
    const RIGHT_LAST_LED: u8 = RIGHT_FIRST_LED + RIGHT_SLOTS as u8 - 1;
    const LEFT_LAST_LED: u8 = LEFT_FIRST_LED + (MAX_SLOTS - LEFT_FIRST_SLOT) as u8;
    match led {
        RIGHT_FIRST_LED..=RIGHT_LAST_LED => Some((led - RIGHT_FIRST_LED) as u32 + 1),
        LEFT_FIRST_LED..=LEFT_LAST_LED => Some(LEFT_FIRST_SLOT + (led - LEFT_FIRST_LED) as u32),
        _ => None,
    }
}

/// Physical key positions (matrix row, col) to LED index, generated from
/// `keyboards/zsa/voyager/keyboard.json` in ZSA's QMK fork. The two orders
/// differ — LEDs run the left half's rows then its thumbs and then the
/// right half, while the matrix interleaves halves row by row, and two
/// keys sit on matrix rows of their own.
const KEY_LEDS: [(u8, u8, u8); 52] = [
    (0, 1, 0),
    (0, 2, 1),
    (0, 3, 2),
    (0, 4, 3),
    (0, 5, 4),
    (0, 6, 5),
    (1, 1, 6),
    (1, 2, 7),
    (1, 3, 8),
    (1, 4, 9),
    (1, 5, 10),
    (1, 6, 11),
    (2, 1, 12),
    (2, 2, 13),
    (2, 3, 14),
    (2, 4, 15),
    (2, 5, 16),
    (2, 6, 17),
    (3, 1, 18),
    (3, 2, 19),
    (3, 3, 20),
    (3, 4, 21),
    (3, 5, 22),
    (4, 4, 23),
    (5, 0, 24),
    (5, 1, 25),
    (6, 0, 26),
    (6, 1, 27),
    (6, 2, 28),
    (6, 3, 29),
    (6, 4, 30),
    (6, 5, 31),
    (7, 0, 32),
    (7, 1, 33),
    (7, 2, 34),
    (7, 3, 35),
    (7, 4, 36),
    (7, 5, 37),
    (8, 0, 38),
    (8, 1, 39),
    (8, 2, 40),
    (8, 3, 41),
    (8, 4, 42),
    (8, 5, 43),
    (9, 1, 45),
    (9, 2, 46),
    (9, 3, 47),
    (9, 4, 48),
    (9, 5, 49),
    (10, 2, 44),
    (11, 5, 50),
    (11, 6, 51),
];

pub fn key_led(row: u8, col: u8) -> Option<u8> {
    KEY_LEDS
        .iter()
        .find(|&&(r, c, _)| (r, c) == (row, col))
        .map(|&(_, _, led)| led)
}

/// The bottom-right TG(3) key, lit while the agent layer is on.
pub const TOGGLE_LED: u8 = 49;
pub const TOGGLE_COLOR: Rgb = Rgb(0xFF, 0xFF, 0xFF);

/// Press feedback, agent layer only: the key that was pressed blinks,
/// dimly when there's no session behind it. The base display never blinks.
pub const PULSE: Duration = Duration::from_millis(140);
pub const PULSE_COLOR: Rgb = Rgb(0xFF, 0xFF, 0xFF);
pub const EMPTY_PULSE_COLOR: Rgb = Rgb(0x2A, 0x2A, 0x2A);

/// What to bring forward after a switch. tmux does the session switching,
/// so this only has to be the terminal.
pub const TERMINAL_APP: &str = "Ghostty";

/// The key each slot lives on, as the HUD labels it — base keycaps, so a
/// shifted symbol is never shown for a key you'd press unshifted.
const SLOT_KEYS: [&str; MAX_SLOTS as usize] = [
    "Y", "U", "I", "O", "P", "\\", "H", "J", "K", "L", ";", "'", "N", "M", ",", ".", "/", "⇧", "⇥",
    "Q", "W", "E", "R", "T", "⌃", "A", "S", "D", "F", "G", "⇧", "Z", "X", "C", "V", "B",
];

pub fn slot_key(slot: u32) -> &'static str {
    SLOT_KEYS.get(slot as usize - 1).copied().unwrap_or("?")
}

/// How long a toast stays up.
pub const TOAST: Duration = Duration::from_millis(2500);

/// The base ledmap's green home markers (F, J, both thumbs), repainted so
/// the always-on display doesn't lose them.
pub const HOME_MARKERS: [u8; 4] = [10, 24, 33, 51];
pub const MARKER_COLOR: Rgb = Rgb(0x46, 0x69, 0x14);

/// One flash of a changed slot on a layer that shows nothing.
pub const FLASH: Duration = Duration::from_millis(1200);
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

// Control markers. `agent-leds` uses leds-* for the same purpose, so the
// two daemons can run on one machine while this one is being swapped in.
pub fn pause_file() -> PathBuf {
    state_dir().join("deck-paused")
}

pub fn base_off_file() -> PathBuf {
    state_dir().join("deck-base-off")
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
    fn key_positions_map_to_their_leds() {
        assert_eq!(key_led(6, 0), Some(26), "Y, slot 1");
        assert_eq!(key_led(1, 5), Some(10), "F, a home marker");
        assert_eq!(key_led(9, 5), Some(49), "the TG(3) toggle key");
        assert_eq!(key_led(4, 4), Some(23), "left row 3 sits on its own row");
        assert_eq!(key_led(10, 2), Some(44), "as does right row 3's first key");
        assert_eq!(key_led(9, 0), None, "no key there");
        assert_eq!(key_led(12, 0), None);
    }

    #[test]
    fn slots_and_leds_round_trip() {
        for slot in 1..=MAX_SLOTS {
            assert_eq!(led_slot(slot_led(slot).unwrap()), Some(slot));
        }
        assert_eq!(led_slot(TOGGLE_LED), None, "the toggle key is not a slot");
        assert_eq!(led_slot(24), None, "nor are the thumbs");
        assert_eq!(led_slot(18), None, "nor left row 3");
    }

    #[test]
    fn only_lit_states_have_colours() {
        assert!(State::parse("off").unwrap().color().is_none());
        assert!(State::parse("error").unwrap().color().is_some());
        assert_eq!(State::parse("bogus"), None);
        assert_eq!(State::parse("needs_input").unwrap().as_str(), "needs_input");
    }
}
