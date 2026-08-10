//! The on-screen twin: a labelled session grid while the agent layer is
//! up, and toasts when a session changes state.
//!
//! One transparent, undecorated, always-on-top window covering the
//! screen, created at startup and shown or hidden with the layer, so
//! showing it costs nothing. It never takes key focus and passes every
//! click through — this display is keyboard-first and read-only. The one
//! exception is switcher mode (Option-Space, for when the board isn't
//! plugged in): the same grid takes key focus like a launcher would,
//! a session's own key label — or arrows and Enter — switches to it,
//! and Escape or clicking away hands focus straight back.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::config::{self, Rgb, State};
use crate::input;

pub struct Row {
    pub slot: u32,
    pub label: String,
    pub state: State,
    pub session: Option<String>,
}

struct Toast {
    label: String,
    state: State,
    raised: Instant,
}

/// What the window should be showing. The daemon writes, the window reads.
#[derive(Default)]
pub struct Overlay {
    ctx: Option<egui::Context>,
    hud: bool,
    switcher: bool,
    rows: Vec<Row>,
    toasts: Vec<Toast>,
    closing: bool,
}

pub type Shared = Arc<Mutex<Overlay>>;

pub fn shared() -> Shared {
    Arc::new(Mutex::new(Overlay::default()))
}

/// Show or hide the HUD, and keep its rows current while it's up.
pub fn set_hud(shared: &Shared, visible: bool, rows: Vec<Row>) {
    if let Ok(mut overlay) = shared.lock() {
        overlay.hud = visible;
        overlay.rows = rows;
        overlay.wake();
    }
}

/// Summon or dismiss the keyboard switcher — the hotkey's landing point,
/// called from the hotkey event handler's thread.
pub fn toggle_switcher(shared: &Shared) {
    if let Ok(mut overlay) = shared.lock() {
        overlay.switcher = !overlay.switcher;
        overlay.wake();
    }
}

pub fn toast(shared: &Shared, label: &str, state: State) {
    if let Ok(mut overlay) = shared.lock() {
        overlay.toasts.push(Toast {
            label: label.to_string(),
            state,
            raised: Instant::now(),
        });
        overlay.wake();
    }
}

/// Bring the window down, which ends `run` and with it the process.
pub fn close(shared: &Shared) {
    if let Ok(mut overlay) = shared.lock() {
        overlay.closing = true;
        overlay.wake();
    }
}

impl Overlay {
    fn wake(&self) {
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }
}

/// Take over the calling thread with the window's event loop; macOS wants
/// that to be the main thread. Returns when the daemon asks it to close.
pub fn run(shared: Shared) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_mouse_passthrough(true)
            .with_taskbar(false)
            .with_active(false)
            .with_visible(false),
        // an accessory app has no Dock icon and never comes forward, so
        // the overlay can't steal focus from what you're typing into
        event_loop_builder: Some(Box::new(|builder| {
            #[cfg(target_os = "macos")]
            {
                use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
                builder.with_activation_policy(ActivationPolicy::Accessory);
                builder.with_activate_ignoring_other_apps(false);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = builder;
        })),
        ..Default::default()
    };
    eframe::run_native(
        "agent-deck",
        options,
        Box::new(|cc| {
            add_keycap_glyphs(&cc.egui_ctx);
            if let Ok(mut overlay) = shared.lock() {
                overlay.ctx = Some(cc.egui_ctx.clone());
            }
            let hotkey = crate::hotkey::register(&shared);
            Ok(Box::new(Window {
                shared,
                visible: false,
                fit: None,
                selected: 0,
                focus: Focus::Off,
                _hotkey: hotkey,
            }))
        }),
    )
}

/// Where the switcher stands with key focus. Grabbing it can take the OS
/// a few frames, and once it's ours, losing it means the user moved on —
/// that dismisses the switcher just like Escape.
#[derive(PartialEq)]
enum Focus {
    Off,
    Wanted(u8),
    Held,
}

struct Window {
    shared: Shared,
    visible: bool,
    /// The (monitor size, top offset) the window last fitted itself to.
    fit: Option<(egui::Vec2, f32)>,
    selected: usize,
    focus: Focus,
    _hotkey: Option<global_hotkey::GlobalHotKeyManager>,
}

/// Hand activation back to whatever had it — an accessory app that took
/// key focus keeps it until it gives it up, and a dismissed switcher
/// swallowing keystrokes would be worse than never focusing at all.
fn deactivate() {
    #[cfg(target_os = "macos")]
    {
        use objc2::MainThreadMarker;
        if let Some(mtm) = MainThreadMarker::new() {
            objc2_app_kit::NSApplication::sharedApplication(mtm).deactivate();
        }
    }
}

/// The base-keycap label a typed key stands for, matching `SLOT_KEYS`.
/// The two modifier-glyph slots (⇧ ⌃) can't be typed; arrows reach them.
fn key_label(key: egui::Key) -> Option<&'static str> {
    use egui::Key::*;
    Some(match key {
        A => "A",
        B => "B",
        C => "C",
        D => "D",
        E => "E",
        F => "F",
        G => "G",
        H => "H",
        I => "I",
        J => "J",
        K => "K",
        L => "L",
        M => "M",
        N => "N",
        O => "O",
        P => "P",
        Q => "Q",
        R => "R",
        S => "S",
        T => "T",
        U => "U",
        V => "V",
        W => "W",
        X => "X",
        Y => "Y",
        Z => "Z",
        Semicolon => ";",
        Quote => "'",
        Comma => ",",
        Period => ".",
        Slash => "/",
        Backslash => "\\",
        Tab => "⇥",
        _ => return None,
    })
}

/// One frame of switcher-mode input. Runs with the overlay lock held;
/// switching itself is spawned off-thread by `input::switch_to`.
fn drive_switcher(
    ctx: &egui::Context,
    overlay: &mut Overlay,
    focus: &mut Focus,
    selected: &mut usize,
) {
    match *focus {
        // grab key focus; the window has to be visible first, and the
        // OS may take a beat, so keep asking for a little while
        Focus::Wanted(tries) if !ctx.input(|i| i.focused) => {
            if tries < 30 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                *focus = Focus::Wanted(tries + 1);
                ctx.request_repaint_after(Duration::from_millis(30));
            }
            return; // no focus, no keystrokes to read
        }
        Focus::Wanted(_) => *focus = Focus::Held,
        // focus went elsewhere: the user clicked away, so fold quietly
        Focus::Held if !ctx.input(|i| i.focused) => {
            overlay.switcher = false;
            ctx.request_repaint(); // the hide lands next frame
            return;
        }
        _ => {}
    }

    let mut chosen = None;
    let mut dismiss = false;
    let last = overlay.rows.len().saturating_sub(1);
    ctx.input(|i| {
        for event in &i.events {
            let egui::Event::Key {
                key, pressed: true, ..
            } = event
            else {
                continue;
            };
            match key {
                egui::Key::Escape => dismiss = true,
                egui::Key::Enter => chosen = Some(*selected),
                egui::Key::ArrowDown => *selected = (*selected + 1).min(last),
                egui::Key::ArrowUp => *selected = selected.saturating_sub(1),
                key => {
                    if let Some(label) = key_label(*key)
                        && let Some(row) = overlay
                            .rows
                            .iter()
                            .position(|r| config::slot_key(r.slot) == label)
                    {
                        chosen = Some(row);
                    }
                }
            }
        }
    });
    if let Some(session) = chosen.and_then(|row| overlay.rows.get(row)?.session.as_deref()) {
        crate::log(&format!("switcher: switching to {session}"));
        input::switch_to(session);
        dismiss = true;
    }
    if dismiss {
        overlay.switcher = false;
        deactivate();
        ctx.request_repaint(); // the hide lands next frame
    }
}

/// egui's bundled font has no keycap symbols (⇧ ⇥ ⌃), which the key column
/// needs, so fall back to a platform font that does.
fn add_keycap_glyphs(ctx: &egui::Context) {
    const CANDIDATES: [&str; 3] = [
        "/System/Library/Fonts/Apple Symbols.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
    ];
    let Some(bytes) = CANDIDATES.iter().find_map(|path| std::fs::read(path).ok()) else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "keycaps".to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(bytes)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("keycaps".to_owned());
    }
    ctx.set_fonts(fonts);
}

// Panel styling: the dark translucent rounded slab the Hammerspoon
// canvases used, in egui's units.
const PANEL: egui::Color32 = egui::Color32::from_rgba_premultiplied(18, 18, 18, 225);
/// The switcher's arrow-key highlight.
const SELECTED: egui::Color32 = egui::Color32::from_rgba_premultiplied(70, 70, 70, 200);
const TEXT: egui::Color32 = egui::Color32::from_rgb(0xF2, 0xF2, 0xF2);
const DIM: egui::Color32 = egui::Color32::from_rgb(0xB0, 0xB0, 0xB0);
const HUD_WIDTH: f32 = 430.0;
const HUD_TOP: f32 = 48.0;
const TOAST_WIDTH: f32 = 260.0;
const MARGIN: f32 = 16.0;
const TEXT_SIZE: f32 = 15.0;
const DOT: f32 = 5.5;
/// Keycaps vary in width, so they get a column of their own and the labels
/// line up under each other.
const KEY_COLUMN: f32 = 16.0;

impl eframe::App for Window {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // the window itself is just glass
    }

    /// Runs while the window is hidden too, which is what lets a hidden
    /// window bring itself back.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Cover the screen below the menu bar — re-fitted whenever the
        // display arrangement changes (laptop screen to external monitor),
        // since the OS drags the window along but keeps its old size. The
        // window can't sit under the menu bar, so its own top offset comes
        // off the height, or the toasts fall off the bottom; the clamp
        // stops wherever the OS parked the window from becoming that
        // offset, and the loop settles once position and size match.
        let (monitor, outer) = ctx.input(|i| (i.viewport().monitor_size, i.viewport().outer_rect));
        if let Some(monitor) = monitor {
            let top = outer.map_or(0.0, |rect| rect.min.y).clamp(0.0, 64.0);
            if self.fit != Some((monitor, top)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(0.0, top)));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    monitor.x,
                    monitor.y - top,
                )));
                self.fit = Some((monitor, top));
                ctx.request_repaint(); // read back where the OS put it
            }
        }

        let now = Instant::now();
        let Ok(mut overlay) = self.shared.lock() else {
            return;
        };
        if overlay.closing {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        overlay.toasts.retain(|t| now - t.raised < config::TOAST);

        if overlay.switcher && self.focus == Focus::Off {
            self.selected = 0;
            self.focus = Focus::Wanted(0);
        }

        let showing = overlay.hud || overlay.switcher || !overlay.toasts.is_empty();
        if showing != self.visible {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(showing));
            self.visible = showing;
        }

        if overlay.switcher {
            drive_switcher(ctx, &mut overlay, &mut self.focus, &mut self.selected);
        }
        if !overlay.switcher {
            self.focus = Focus::Off;
        }
        if !overlay.toasts.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(100)); // toasts age out
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // the window's own rect, so panels sit right even if the OS gave us
        // something other than the size we asked for
        let screen = ctx.content_rect().size();
        let now = Instant::now();
        let Ok(overlay) = self.shared.lock() else {
            return;
        };
        if overlay.hud || overlay.switcher {
            let selected = overlay.switcher.then_some(self.selected);
            paint_hud(&ctx, &overlay.rows, screen, selected);
        }
        paint_toasts(&ctx, &overlay.toasts, screen, now);
    }
}

fn panel() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .corner_radius(10.0)
        .inner_margin(10.0)
}

fn color(rgb: Rgb) -> egui::Color32 {
    egui::Color32::from_rgb(rgb.0, rgb.1, rgb.2)
}

fn dot(ui: &mut egui::Ui, state: State) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(DOT * 2.0 + 6.0, DOT * 2.0), egui::Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), DOT, color(state.dot()));
}

fn row_text(text: &str, faded: bool) -> egui::RichText {
    egui::RichText::new(text)
        .size(TEXT_SIZE)
        .color(if faded { DIM } else { TEXT })
}

/// `selected` is Some only in switcher mode: it highlights the arrow-key
/// choice and adds the hint footer.
fn paint_hud(ctx: &egui::Context, rows: &[Row], screen: egui::Vec2, selected: Option<usize>) {
    let left = ((screen.x - HUD_WIDTH) / 2.0).max(0.0);
    egui::Area::new(egui::Id::new("hud"))
        .fixed_pos(egui::pos2(left, HUD_TOP))
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_width(HUD_WIDTH);
            panel().show(ui, |ui| {
                ui.set_width(HUD_WIDTH - 20.0);
                ui.spacing_mut().item_spacing.y = 6.0;
                if rows.is_empty() {
                    ui.label(row_text("no sessions", true));
                    return;
                }
                for (i, row) in rows.iter().enumerate() {
                    let fill = if selected == Some(i) {
                        SELECTED
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    egui::Frame::new()
                        .fill(fill)
                        .corner_radius(5.0)
                        .inner_margin(egui::Margin::symmetric(4, 2))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                dot(ui, row.state);
                                let key = ui.available_rect_before_wrap();
                                ui.allocate_ui(egui::vec2(KEY_COLUMN, key.height()), |ui| {
                                    ui.centered_and_justified(|ui| {
                                        // monospace: the sans face draws I as a
                                        // bare stroke, unreadable as a keycap (|)
                                        ui.label(
                                            row_text(config::slot_key(row.slot), false).monospace(),
                                        );
                                    });
                                });
                                ui.label(row_text(
                                    &format!("{} — {}", row.label, row.state.as_str()),
                                    row.state == State::Off,
                                ));
                            });
                        });
                }
                if selected.is_some() {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("type a session's key · ↑↓ ↵ · esc")
                            .size(TEXT_SIZE - 3.0)
                            .color(DIM),
                    );
                }
            });
        });
}

fn paint_toasts(ctx: &egui::Context, toasts: &[Toast], screen: egui::Vec2, now: Instant) {
    for (i, toast) in toasts.iter().enumerate() {
        let height = 34.0;
        let bottom = screen.y - MARGIN - (i as f32 + 1.0) * (height + 8.0);
        // fade out over the last third rather than vanishing
        let left = config::TOAST.saturating_sub(now - toast.raised);
        let fade = (left.as_secs_f32() / 0.6).min(1.0);
        egui::Area::new(egui::Id::new(("toast", i)))
            .fixed_pos(egui::pos2(screen.x - TOAST_WIDTH - MARGIN, bottom))
            .interactable(false)
            .show(ctx, |ui| {
                ui.set_opacity(fade);
                ui.set_width(TOAST_WIDTH);
                panel().corner_radius(8.0).show(ui, |ui| {
                    ui.set_width(TOAST_WIDTH - 20.0);
                    ui.horizontal(|ui| {
                        dot(ui, toast.state);
                        ui.label(row_text(
                            &format!("{} — {}", toast.label, toast.state.as_str()),
                            false,
                        ));
                    });
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_slot_key_is_typeable_or_a_modifier() {
        let typeable: Vec<&str> = egui::Key::ALL
            .iter()
            .filter_map(|&k| key_label(k))
            .collect();
        for slot in 1..=config::MAX_SLOTS {
            let label = config::slot_key(slot);
            assert!(
                typeable.contains(&label) || matches!(label, "⇧" | "⌃"),
                "slot {slot} ({label}) is unreachable from the switcher"
            );
        }
        // and nothing maps to a label that isn't a slot key
        for label in &typeable {
            assert!(
                (1..=config::MAX_SLOTS).any(|s| config::slot_key(s) == *label),
                "{label} matches no slot"
            );
        }
    }
}
