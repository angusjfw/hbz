//! Claude session status on the ZSA Voyager, over raw HID.
//!
//! One process: it pairs with the board, follows its layer events, watches
//! the `agent-status` state store and paints one key per session slot.
//! Statuses show on the base layer (home markers repainted alongside) and
//! on the agent layer (toggle key lit); other layers keep their firmware
//! colours and get a brief flash plus a toast when a session changes.
//!
//! Spec: docs/specs/2026-08-04-direct-hid-renderer.md

mod board;
mod config;
mod control;
mod hotkey;
mod input;
mod overlay;
mod render;
mod store;
mod tmux;

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use hidapi::HidApi;

use board::{Board, Event};
use config::State;
use overlay::Shared;
use render::{Frame, Painter};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "preview") {
        return preview();
    }
    if !args.is_empty() {
        std::process::exit(control::run(&args));
    }
    // the window's event loop wants the main thread (macOS insists), so the
    // daemon runs beside it; LEDs keep working even if the window can't
    let shared = overlay::shared();
    let daemon = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || run(shared))
    };
    if let Err(e) = overlay::run(Arc::clone(&shared)) {
        log(&format!("no overlay window ({e}), LEDs only"));
    }
    let _ = daemon.join();
}

/// One line per connection change or real failure — this is a daemon log.
pub fn log(message: &str) {
    println!("agent-deck: {message}");
}

/// Put the overlays on screen from the live store, without a board and
/// without the chord — how the panels get looked at while they're being
/// styled. The switcher is live, so a key press switches for real.
fn preview() {
    let shared = overlay::shared();
    let feed = Arc::clone(&shared);
    thread::spawn(move || {
        let snapshot = store::read(&mut store::Health::new());
        let first = snapshot.tracked.first().map(|t| (t.label.clone(), t.state));
        overlay::set_rows(&feed, rows(&snapshot));
        overlay::toggle_switcher(&feed);
        loop {
            let (label, state) = first
                .clone()
                .unwrap_or_else(|| ("preview".to_string(), State::Done));
            overlay::toast(&feed, &label, state);
            thread::sleep(config::TOAST * 2);
        }
    });
    if let Err(e) = overlay::run(shared) {
        log(&format!("no overlay window ({e})"));
    }
}

fn run(shared: Shared) {
    let quit = Arc::new(AtomicBool::new(false));
    for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
        if let Err(e) = signal_hook::flag::register(signal, Arc::clone(&quit)) {
            log(&format!(
                "no signal handler ({e}), LEDs may be left painted"
            ));
        }
    }
    let _ = fs::create_dir_all(config::state_dir());
    let (store_events, _watcher) = store::watch();

    let mut api = match HidApi::new() {
        Ok(api) => api,
        Err(e) => {
            log(&format!("no HID access ({e})"));
            std::process::exit(1);
        }
    };
    let mut board: Option<Board> = None;
    let mut layer: Option<u8> = None;
    let mut reported_down = false;
    let mut painter = Painter::default();
    let mut health = store::Health::new();
    let mut snapshot = store::Snapshot::default();
    let mut lit = BTreeMap::new();
    // None until the first read: starting the daemon is not a state change
    let mut previous: Option<BTreeMap<u32, State>> = None;
    let mut store_dirty = true;
    let (mut last_open, mut last_read) = (None, None);
    let mut last_focus = None;

    while !quit.load(Ordering::Relaxed) {
        let now = Instant::now();
        let paused = control::paused();

        if paused {
            // a full pause frees the device: flashing firmware needs it
            if let Some(open) = &board {
                let _ = painter.release(open);
                log("paused, board released");
            }
            board = None;
            layer = None;
        } else if board.is_none() && due(last_open, now, config::RECONNECT) {
            last_open = Some(now);
            match Board::open(&mut api) {
                Ok(open) => {
                    log(&format!("connected to {}", open.product));
                    board = Some(open);
                    painter.forget();
                    reported_down = false;
                }
                Err(e) => {
                    if !reported_down {
                        log(&format!("waiting for a board: {e}"));
                        reported_down = true;
                    }
                }
            }
        }

        // layer changes are pushed, so there's no poll gap to paint through
        if let Some(open) = &board
            && let Err(e) = read_board(open, &mut layer)
        {
            log(&format!("board gone ({e})"));
            reported_down = true;
            board = None;
            layer = None;
            painter.forget();
        }

        // housekeeping runs whether or not a board is connected
        while store_events.try_recv().is_ok() {
            store_dirty = true;
        }
        let mut changes: Vec<(u32, State)> = Vec::new();
        let mut rows_changed = false;
        if store_dirty || due(last_read, now, config::STORE_REFRESH) {
            store_dirty = false;
            last_read = Some(now);
            snapshot = store::read(&mut health);
            lit = snapshot.lit();
            if let Some(previous) = &previous {
                changes = lit
                    .iter()
                    .filter(|(slot, state)| {
                        state.worth_noticing() && previous.get(slot) != Some(state)
                    })
                    .map(|(&slot, &state)| (slot, state))
                    .collect();
            }
            previous = Some(lit.clone());
            rows_changed = true;
            if due(last_focus, now, config::FOCUS_CHECK) {
                last_focus = Some(now);
                store::demote_done_on_focus(&snapshot.done);
            }
        }
        let frame = if paused {
            Frame::new()
        } else {
            render::frame_for(layer, &lit)
        };

        // a toast is host-side and fires with no board at all
        if !paused {
            for &(slot, state) in &changes {
                let fallback = format!("slot {slot}");
                let label = snapshot.label(slot).unwrap_or(&fallback);
                overlay::toast(&shared, label, state);
            }
        }
        if let Some(open) = &board
            && let Err(e) = painter.show(open, &frame)
        {
            log(&format!("paint failed ({e})"));
            reported_down = true;
            board = None;
            layer = None;
            painter.forget();
        }

        // pushed whether or not the switcher is up, so summoning it shows
        // a current list rather than the one from last time
        if rows_changed {
            overlay::set_rows(&shared, rows(&snapshot));
        }

        if board.is_none() {
            // nothing to read from, so wait on the store instead
            store_dirty |= wait(&store_events);
        }
    }

    if let Some(open) = &board {
        let _ = painter.release(open);
    }
    overlay::close(&shared);
}

/// HUD rows: every tracked session, in the order the store snapshot holds
/// them (tmux creation order, matching the switcher).
fn rows(snapshot: &store::Snapshot) -> Vec<overlay::Row> {
    snapshot
        .tracked
        .iter()
        .map(|t| overlay::Row {
            slot: t.slot,
            label: t.label.clone(),
            state: t.state,
            session: t.session.clone(),
        })
        .collect()
}

/// Drain the board's event stream for the layer it's on — the base
/// display is the only thing that depends on it. Everything else the
/// board pushes, key positions included, is dropped without a look.
fn read_board(board: &Board, layer: &mut Option<u8>) -> Result<(), hidapi::HidError> {
    // the first read paces the loop, the rest drain what's queued
    let mut timeout = config::READ_TIMEOUT_MS;
    while let Some(event) = board.read_event(timeout)? {
        timeout = 0;
        match event {
            Event::Layer(active) => *layer = Some(active),
            Event::Other => {}
        }
    }
    Ok(())
}

/// Wait for a store change, up to the idle interval. True if one landed.
fn wait(store_events: &Receiver<()>) -> bool {
    match store_events.recv_timeout(config::IDLE_WAIT) {
        Ok(()) => true,
        Err(RecvTimeoutError::Timeout) => false,
        // no watcher: pace the loop by hand instead of spinning
        Err(RecvTimeoutError::Disconnected) => {
            thread::sleep(config::IDLE_WAIT);
            false
        }
    }
}

fn due(last: Option<Instant>, now: Instant, every: Duration) -> bool {
    last.is_none_or(|last| now.duration_since(last) >= every)
}
