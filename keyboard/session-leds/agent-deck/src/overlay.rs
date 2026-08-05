//! The on-screen twin: a labelled session grid while the agent layer is
//! up, and toasts when a session changes state.
//!
//! One transparent, undecorated, always-on-top window covering the
//! screen, created at startup and shown or hidden with the layer, so
//! showing it costs nothing. It never takes key focus and passes every
//! click through — this display is keyboard-first and read-only.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::config::{self, Rgb, State};

pub struct Row {
    pub slot: u32,
    pub label: String,
    pub state: State,
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
            Ok(Box::new(Window {
                shared,
                visible: false,
                sized: false,
            }))
        }),
    )
}

struct Window {
    shared: Shared,
    visible: bool,
    sized: bool,
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
const TEXT: egui::Color32 = egui::Color32::from_rgb(0xF2, 0xF2, 0xF2);
const DIM: egui::Color32 = egui::Color32::from_rgb(0xB0, 0xB0, 0xB0);
const HUD_WIDTH: f32 = 430.0;
const HUD_TOP: f32 = 48.0;
const TOAST_WIDTH: f32 = 260.0;
const MARGIN: f32 = 16.0;
const TEXT_SIZE: f32 = 14.0;
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
        if !self.sized {
            // Cover the screen below the menu bar, once — the panels are
            // then placed within the window and it never moves again. The
            // window can't sit under the menu bar, so its own top offset
            // comes off the height, or the toasts fall off the bottom.
            let (monitor, outer) =
                ctx.input(|i| (i.viewport().monitor_size, i.viewport().outer_rect));
            if let Some(monitor) = monitor {
                let top = outer.map_or(0.0, |rect| rect.min.y);
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(0.0, top)));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    monitor.x,
                    monitor.y - top,
                )));
                self.sized = true;
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

        let showing = overlay.hud || !overlay.toasts.is_empty();
        if showing != self.visible {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(showing));
            self.visible = showing;
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
        if overlay.hud {
            paint_hud(&ctx, &overlay.rows, screen);
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

fn paint_hud(ctx: &egui::Context, rows: &[Row], screen: egui::Vec2) {
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
                for row in rows {
                    ui.horizontal(|ui| {
                        dot(ui, row.state);
                        let key = ui.available_rect_before_wrap();
                        ui.allocate_ui(egui::vec2(KEY_COLUMN, key.height()), |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.label(row_text(config::slot_key(row.slot), false));
                            });
                        });
                        ui.label(row_text(
                            &format!("{} — {}", row.label, row.state.as_str()),
                            row.state == State::Off,
                        ));
                    });
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
