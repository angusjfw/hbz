//! Ctrl-Alt-Space (Control-Option-Space on macOS), for when the board
//! isn't around: summons the HUD as a keyboard switcher. Registered with
//! the OS as a single global chord (RegisterEventHotKey on macOS,
//! RegisterHotKey on Windows, via the global-hotkey crate) — no event
//! tap, no accessibility permission, and nothing but this one
//! combination is ever delivered to us.

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::overlay::{self, Shared};

/// Register the chord and route its presses to the switcher. The returned
/// manager owns the registration — dropping it unregisters, so the window
/// holds on to it. None (logged) if the OS refused, which also covers a
/// second copy of the process finding the chord taken.
pub fn register(shared: &Shared) -> Option<GlobalHotKeyManager> {
    let manager = GlobalHotKeyManager::new()
        .and_then(|manager| {
            manager.register(HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space))?;
            Ok(manager)
        })
        .map_err(|e| crate::log(&format!("no global hotkey ({e})")))
        .ok()?;
    let shared = std::sync::Arc::clone(shared);
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state == HotKeyState::Pressed {
            overlay::toggle_switcher(&shared);
        }
    }));
    crate::log("Ctrl-Alt-Space registered");
    Some(manager)
}
