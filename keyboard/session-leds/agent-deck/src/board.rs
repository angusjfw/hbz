//! The Voyager over raw HID, speaking the Oryx protocol (ZSA's QMK fork,
//! `modules/zsa/oryx/`). Pair once, then the board pushes layer changes
//! and keypresses while we write LEDs.

use std::fmt;

use hidapi::{HidApi, HidDevice, HidError};

use crate::config::Rgb;

/// ZSA Technology Labs. The LED map below is the Voyager's, but any ZSA
/// board exposing the raw HID interface will open — keyboard swaps and
/// re-enumeration just work.
const ZSA_VID: u16 = 0x3297;
/// QMK's raw HID endpoint.
const RAW_USAGE_PAGE: u16 = 0xFF60;
const RAW_USAGE: u16 = 0x61;
/// RAW_EPSIZE: every frame in or out is this long.
const EPSIZE: usize = 32;

// Oryx_Command_Code
const CMD_PAIRING_INIT: u8 = 1;
const CMD_SET_LAYER: u8 = 4;
const CMD_RGB_CONTROL: u8 = 5;
const CMD_SET_RGB_LED: u8 = 6;

// Oryx_Event_Code
const EVT_LAYER: u8 = 5;
const EVT_KEYDOWN: u8 = 6;

pub enum Event {
    Layer(u8),
    /// A key going down, by position. Positions arrive for every keypress
    /// on every layer — effectively keystroke telemetry — so they are only
    /// ever acted on while the agent layer is up, and never logged or
    /// persisted.
    KeyDown {
        row: u8,
        col: u8,
    },
    /// Key releases, pairing and RGB-control acks, firmware version.
    Other,
}

pub enum OpenError {
    NotFound,
    Hid(HidError),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OpenError::NotFound => write!(f, "no ZSA board on the raw HID interface"),
            OpenError::Hid(e) => write!(f, "{e}"),
        }
    }
}

impl From<HidError> for OpenError {
    fn from(e: HidError) -> OpenError {
        OpenError::Hid(e)
    }
}

pub struct Board {
    dev: HidDevice,
    pub product: String,
}

impl Board {
    /// Open the board and pair with it. Pairing is unconditional in the
    /// firmware: it acks, starts the event stream and pushes the current
    /// layer, so no polling is needed to learn where we are.
    pub fn open(api: &mut HidApi) -> Result<Board, OpenError> {
        api.refresh_devices()?;
        let info = api
            .device_list()
            .find(|d| {
                d.vendor_id() == ZSA_VID
                    && d.usage_page() == RAW_USAGE_PAGE
                    && d.usage() == RAW_USAGE
            })
            .ok_or(OpenError::NotFound)?;
        let product = info.product_string().unwrap_or("keyboard").to_string();
        let board = Board {
            dev: api.open_path(info.path())?,
            product,
        };
        board.send(CMD_PAIRING_INIT, &[])?;
        // an earlier run may have died mid-paint, leaving the board frozen
        // under host control: start from the firmware's own colours
        board.release_rgb()?;
        Ok(board)
    }

    fn send(&self, cmd: u8, params: &[u8]) -> Result<(), HidError> {
        // hidapi wants a leading report id; QMK's raw HID has no numbered
        // reports, so it's 0 and the frame itself starts at byte 1
        let mut frame = [0u8; EPSIZE + 1];
        frame[1] = cmd;
        frame[2..2 + params.len()].copy_from_slice(params);
        self.dev.write(&frame)?;
        Ok(())
    }

    /// Write one LED into the firmware's webhid buffer. The first write
    /// also engages host control, which stops firmware layer colours.
    pub fn set_led(&self, led: u8, c: Rgb) -> Result<(), HidError> {
        self.send(CMD_SET_RGB_LED, &[led, c.0, c.1, c.2])
    }

    /// Hand the board back to the firmware (reloads its layer colours).
    pub fn release_rgb(&self) -> Result<(), HidError> {
        self.send(CMD_RGB_CONTROL, &[0])
    }

    /// Move the board to `layer` — how the agent layer dismisses itself
    /// after a switch.
    pub fn set_layer(&self, layer: u8) -> Result<(), HidError> {
        self.send(CMD_SET_LAYER, &[1, layer])
    }

    /// Wait up to `timeout_ms` for one event; None if nothing arrived.
    pub fn read_event(&self, timeout_ms: i32) -> Result<Option<Event>, HidError> {
        let mut frame = [0u8; EPSIZE];
        if self.dev.read_timeout(&mut frame, timeout_ms)? == 0 {
            return Ok(None);
        }
        Ok(Some(match frame[0] {
            EVT_LAYER => Event::Layer(frame[1]),
            EVT_KEYDOWN => Event::KeyDown {
                col: frame[1],
                row: frame[2],
            },
            _ => Event::Other,
        }))
    }
}
