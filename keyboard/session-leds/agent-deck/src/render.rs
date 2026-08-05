//! Frames and painting. Host RGB control is all-or-nothing — while it's
//! engaged the firmware paints nothing — so the daemon owns the board only
//! on the layers it displays statuses on, and hands it straight back
//! everywhere else.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use hidapi::HidError;

use crate::board::Board;
use crate::config::{self, Rgb, State};

/// What every LED should be showing. LEDs absent from a frame are dark.
pub type Frame = BTreeMap<u8, Rgb>;

/// The frame for `layer`: statuses on the base layer (home markers
/// repainted alongside) and on the agent layer (toggle key lit); any other
/// layer, or no board, keeps the firmware's colours.
pub fn frame_for(layer: Option<u8>, slots: &BTreeMap<u32, State>, base_display: bool) -> Frame {
    let mut frame = Frame::new();
    match layer {
        Some(config::AGENT_LAYER) => {
            frame.insert(config::TOGGLE_LED, config::TOGGLE_COLOR);
        }
        Some(config::BASE_LAYER) if base_display => {
            // markers first: an occupied slot outranks its marker
            for led in config::HOME_MARKERS {
                frame.insert(led, config::MARKER_COLOR);
            }
        }
        _ => return frame,
    }
    frame.extend(slot_leds(slots.iter().map(|(&slot, &state)| (slot, state))));
    frame
}

fn slot_leds(slots: impl IntoIterator<Item = (u32, State)>) -> Frame {
    slots
        .into_iter()
        .filter_map(|(slot, state)| Some((config::slot_led(slot)?, state.color()?)))
        .collect()
}

/// The LED writes that turn `prev` into `next`. Taking host control blanks
/// every unpainted LED anyway, so a first paint only sends the lit ones.
fn diff(prev: Option<&Frame>, next: &Frame) -> Vec<(u8, Rgb)> {
    let Some(prev) = prev else {
        return next
            .iter()
            .filter(|&(_, &color)| color != config::OFF)
            .map(|(&led, &color)| (led, color))
            .collect();
    };
    let leds: BTreeSet<u8> = prev.keys().chain(next.keys()).copied().collect();
    leds.into_iter()
        .filter_map(|led| {
            let want = next.get(&led).copied().unwrap_or(config::OFF);
            (prev.get(&led).copied().unwrap_or(config::OFF) != want).then_some((led, want))
        })
        .collect()
}

/// Diff-paints the board, tracking whether the host holds RGB control.
#[derive(Default)]
pub struct Painter {
    painted: Option<Frame>,
}

impl Painter {
    /// Show `frame`, or hand the board back when it's empty.
    pub fn show(&mut self, board: &Board, frame: &Frame) -> Result<(), HidError> {
        if frame.is_empty() {
            return self.release(board);
        }
        for (led, color) in diff(self.painted.as_ref(), frame) {
            board.set_led(led, color)?;
        }
        self.painted = Some(frame.clone());
        Ok(())
    }

    pub fn release(&mut self, board: &Board) -> Result<(), HidError> {
        if self.painted.take().is_some() {
            board.release_rgb()?;
        }
        Ok(())
    }

    /// The board went away mid-paint: there's nothing to hand back.
    pub fn forget(&mut self) {
        self.painted = None;
    }
}

/// A single brief flash of changed slots, for layers that show nothing.
#[derive(Default)]
pub struct Flash {
    frame: Frame,
    until: Option<Instant>,
}

impl Flash {
    pub fn start(&mut self, changes: &[(u32, State)], now: Instant) {
        self.frame.extend(slot_leds(changes.iter().copied()));
        self.until = Some(now + config::FLASH);
    }

    /// The flash frame while it lasts, empty once it's over.
    pub fn frame(&mut self, now: Instant) -> Frame {
        match self.until {
            Some(until) if now < until => self.frame.clone(),
            _ => {
                self.frame.clear();
                self.until = None;
                Frame::new()
            }
        }
    }
}

/// Press feedback: the pressed key blinks for a moment, dimly when no
/// session sits behind it. Only ever overlaid on the agent layer — the
/// base display is pure status and never blinks.
#[derive(Default)]
pub struct Pulse {
    key: Option<(u8, Rgb)>,
    until: Option<Instant>,
}

impl Pulse {
    pub fn start(&mut self, led: u8, occupied: bool, now: Instant) {
        let color = if occupied {
            config::PULSE_COLOR
        } else {
            config::EMPTY_PULSE_COLOR
        };
        self.key = Some((led, color));
        self.until = Some(now + config::PULSE);
    }

    pub fn overlay(&mut self, frame: &mut Frame, now: Instant) {
        match (self.until, self.key) {
            (Some(until), Some((led, color))) if now < until => {
                frame.insert(led, color);
            }
            _ => {
                self.key = None;
                self.until = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn slots(pairs: &[(u32, State)]) -> BTreeMap<u32, State> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn base_layer_shows_statuses_over_home_markers() {
        // slot 8 is the J key, which is also a home marker
        let frame = frame_for(
            Some(config::BASE_LAYER),
            &slots(&[(8, State::Working), (1, State::Off)]),
            true,
        );
        assert_eq!(frame.get(&33), Some(&State::Working.color().unwrap()));
        assert_eq!(frame.get(&10), Some(&config::MARKER_COLOR));
        assert_eq!(frame.get(&26), None, "an off session leaves its key dark");
    }

    #[test]
    fn base_display_off_and_other_layers_keep_firmware_colours() {
        let busy = slots(&[(1, State::Working)]);
        assert!(frame_for(Some(config::BASE_LAYER), &busy, false).is_empty());
        assert!(frame_for(Some(1), &busy, true).is_empty());
        assert!(frame_for(None, &busy, true).is_empty());
    }

    #[test]
    fn agent_layer_lights_the_toggle_key_with_statuses() {
        let frame = frame_for(
            Some(config::AGENT_LAYER),
            &slots(&[(19, State::Error)]),
            true,
        );
        assert_eq!(frame.get(&config::TOGGLE_LED), Some(&config::TOGGLE_COLOR));
        assert_eq!(frame.get(&0), Some(&State::Error.color().unwrap()));
        assert_eq!(frame.get(&10), None, "no home markers on the agent layer");
    }

    #[test]
    fn first_paint_sends_only_lit_leds_then_diffs() {
        let mut first = Frame::new();
        first.insert(26, State::Working.color().unwrap());
        first.insert(27, config::OFF);
        assert_eq!(
            diff(None, &first),
            vec![(26, State::Working.color().unwrap())]
        );

        let mut next = Frame::new();
        next.insert(26, State::Working.color().unwrap());
        next.insert(33, State::Done.color().unwrap());
        assert_eq!(
            diff(Some(&first), &next),
            vec![(33, State::Done.color().unwrap())],
            "unchanged LEDs aren't rewritten"
        );
        assert_eq!(
            diff(Some(&next), &first),
            vec![(33, config::OFF)],
            "a cleared LED goes black"
        );
    }

    #[test]
    fn pulse_marks_the_pressed_key_then_clears() {
        let now = Instant::now();
        let mut pulse = Pulse::default();
        let mut frame = frame_for(
            Some(config::AGENT_LAYER),
            &slots(&[(1, State::Working)]),
            true,
        );

        pulse.start(26, true, now);
        pulse.overlay(&mut frame, now);
        assert_eq!(
            frame.get(&26),
            Some(&config::PULSE_COLOR),
            "over its status"
        );

        pulse.start(30, false, now);
        pulse.overlay(&mut frame, now);
        assert_eq!(frame.get(&30), Some(&config::EMPTY_PULSE_COLOR), "dim");

        let mut later = frame_for(
            Some(config::AGENT_LAYER),
            &slots(&[(1, State::Working)]),
            true,
        );
        pulse.overlay(&mut later, now + config::PULSE + Duration::from_millis(1));
        assert_eq!(later.get(&30), None);
        assert_eq!(later.get(&26), Some(&State::Working.color().unwrap()));
    }

    #[test]
    fn flash_expires() {
        let now = Instant::now();
        let mut flash = Flash::default();
        assert!(flash.frame(now).is_empty());
        flash.start(&[(2, State::NeedsInput)], now);
        assert_eq!(
            flash.frame(now).get(&27),
            Some(&State::NeedsInput.color().unwrap())
        );
        assert!(
            flash
                .frame(now + config::FLASH + Duration::from_millis(1))
                .is_empty()
        );
    }
}
