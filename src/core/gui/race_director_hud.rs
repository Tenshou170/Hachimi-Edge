//! Race Director HUD: the broadcast-style timing tower / followed-Uma panel / win
//! probability / predicted-positions overlay, drawn straight into Hachimi's own shared
//! `egui::Context` (see `Gui::run()`, which calls `run(ctx)` here every frame on both
//! platforms) - unlike the race-director-plugin this was ported from, there is no private
//! `egui::Context`/D3D11 painter/present-hook here at all, so this file is pure
//! `egui::Window`/`egui::Painter` drawing, unconditionally cross-platform.
//!
//! Each panel is a real, independently movable/resizable `egui::Window` (unlike
//! `utils::new_window()`'s fixed centered modal windows) whose position/size persists into
//! `config.race_director.window_*` on drag/resize release (see `persist_geometry`).
//!
//! Colors here are the race-director-plugin's own fixed broadcast palette (ported as-is),
//! not Hachimi's Material3 theme - a Material3-themed version was tried and reverted at
//! the user's request, since a dynamically-generated theme's accent roles aren't tuned to
//! sit next to each other as a legend the way this hand-picked palette is. Window chrome
//! (background/border/corner radius) still follows the theme, same as every other Hachimi
//! window - only the panel *content* colors are fixed.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use egui::{Color32, Context, Painter, Pos2, Rect, RichText, Stroke, Vec2, Window};
use egui_material3::theme::{get_global_color, get_global_corner_radius};

use crate::core::hachimi::{self, RaceDirectorWindowState};
use crate::core::race_director::{self, FieldRow};
use crate::core::Hachimi;
use crate::il2cpp::hook::umamusume::RaceHorseManagerBase;

fn style_color(style: i32) -> Color32 {
    match style {
        1 => Color32::from_rgb(226, 75, 74),   // Nige   - front runner
        2 => Color32::from_rgb(239, 159, 39),  // Senko  - stalker
        3 => Color32::from_rgb(55, 138, 221),  // Sashi  - closer
        4 => Color32::from_rgb(176, 123, 224), // Oikomi - deep closer
        _ => Color32::from_rgb(115, 115, 128),
    }
}
fn style_label(style: i32) -> &'static str {
    match style {
        1 => "Nige",
        2 => "Senko",
        3 => "Sashi",
        4 => "Oikomi",
        _ => "?",
    }
}
/// `RaceDefine.DefeatType` - the simulation's own tag for the biggest factor behind a
/// horse's result (1 = Win). Cosmetic flavor for the predicted-positions panel only.
fn defeat_label(defeat: i32) -> Option<&'static str> {
    match defeat {
        2 => Some("outpaced"),
        3 => Some("bad style matchup"),
        4 => Some("temptation"),
        5 => Some("guts order"),
        6 => Some("stamina"),
        7 | 8 => Some("bad spurt timing"),
        9 => Some("lacked skills"),
        10 => Some("boxed in"),
        11 => Some("speed"),
        12 => Some("off-distance"),
        13 => Some("off-surface"),
        14 => Some("low motivation"),
        _ => None,
    }
}

const GOOD: Color32 = Color32::from_rgb(103, 200, 120);
const WARN: Color32 = Color32::from_rgb(230, 179, 60);
const BAD: Color32 = Color32::from_rgb(220, 90, 90);
const GOLD: Color32 = Color32::from_rgb(230, 190, 90);
const DIM: Color32 = Color32::from_rgb(190, 190, 198);

fn stamina_color(frac: f32) -> Color32 {
    if frac > 0.5 {
        GOOD
    } else if frac > 0.25 {
        WARN
    } else {
        BAD
    }
}

fn pbar(painter: &Painter, rect: Rect, frac: f32, color: Color32, label: &str) {
    painter.rect_filled(rect, 3.0, Color32::from_rgb(38, 38, 44));
    let fill_w = rect.width() * frac.clamp(0.0, 1.0);
    if fill_w > 0.5 {
        painter.rect_filled(Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height())), 3.0, color);
    }
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional((rect.height() * 0.72).max(9.0)),
        Color32::WHITE,
    );
}

/// Pace sparkline: x maps to race PROGRESS (`i / total`), so the line only fills the
/// fraction of the race reached so far.
fn spark(painter: &Painter, rect: Rect, samples: &[f32], total: usize, color: Color32) {
    painter.rect_filled(rect, 3.0, Color32::from_rgb(38, 38, 44));
    if samples.len() < 2 {
        return;
    }
    let mn = samples.iter().cloned().fold(f32::MAX, f32::min);
    let mx = samples.iter().cloned().fold(f32::MIN, f32::max);
    let range = (mx - mn).max(0.8);
    let n = samples.len();
    let pad = 2.5;
    let denom = (total.max(2) - 1) as f32;
    let plot = |i: usize, v: f32| -> Pos2 {
        let t = (i as f32 / denom).min(1.0);
        Pos2::new(
            rect.min.x + pad + t * (rect.width() - 2.0 * pad),
            rect.min.y + pad + (1.0 - (v - mn) / range) * (rect.height() - 2.0 * pad),
        )
    };
    for i in 1..n {
        painter.line_segment([plot(i - 1, samples[i - 1]), plot(i, samples[i])], Stroke::new(1.6_f32, color));
    }
    painter.circle_filled(plot(n - 1, samples[n - 1]), 2.5, color);
}

// ── window geometry: build + persist ──────────────────────────────────────────────────────
// Bumped by the "Reset window positions" settings button - baked into each window's Id so a
// reset gets a brand-new egui::Area with no memorized position/size, instead of needing to
// reach into egui's internal Area memory directly.
static RESET_GEN: AtomicU32 = AtomicU32::new(0);
pub fn reset_window_positions() {
    RESET_GEN.fetch_add(1, Ordering::Relaxed);
    // Spawned, not called inline: this runs synchronously inside the settings button's
    // click handler, itself inside the render thread's egui pass - matching the
    // established convention (see e.g. gameplay.rs's keybind-save flow) of not blocking
    // a frame on config disk I/O.
    std::thread::spawn(|| {
        let hachimi = Hachimi::instance();
        let mut new_config = hachimi.config.load().as_ref().clone();
        new_config.race_director.window_tower = RaceDirectorWindowState::default();
        new_config.race_director.window_followed = RaceDirectorWindowState::default();
        new_config.race_director.window_winprob = RaceDirectorWindowState::default();
        new_config.race_director.window_predicted = RaceDirectorWindowState::default();
        let _ = hachimi.save_and_reload_config(new_config);
    });
}

/// Applies the Race Director opacity slider directly, independent of Hachimi's global
/// Theme Settings translucency mode - that mode gates the rest of the app's windows, but
/// gating this dedicated per-feature slider behind an unrelated global setting meant it
/// silently did nothing unless the user also went and changed that other setting, which
/// is exactly the confusing behavior this exists to avoid.
fn window_fill(opacity: f32) -> Color32 {
    let surface_container = get_global_color("surfaceContainer");
    Color32::from_rgba_unmultiplied(
        surface_container.r(),
        surface_container.g(),
        surface_container.b(),
        (opacity.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

fn make_window(
    title: &str,
    id: &str,
    default_pos: Pos2,
    default_size: Vec2,
    default_open: bool,
    state: &RaceDirectorWindowState,
    opacity: f32,
) -> Window<'static> {
    let cr = get_global_corner_radius().unwrap_or(8.0).max(8.0) as u8;
    let frame = egui::Frame::NONE
        .fill(window_fill(opacity))
        .stroke(Stroke::new(1.0_f32, get_global_color("outlineVariant")))
        .corner_radius(egui::CornerRadius::same(cr))
        .inner_margin(egui::Margin::same(8));

    // A plain &str title falls back to egui's TextStyle::Heading, which in this app's
    // scaled theme renders far larger than a compact HUD panel calls for - passing an
    // explicitly-sized RichText overrides that fallback. Use the MD3 primary accent color
    // so the title bar visually matches the rest of the Hachimi UI chrome.
    let title_text = RichText::new(title).size(15.0).color(get_global_color("primary"));

    let mut w = Window::new(title_text)
        .id(egui::Id::new((id, RESET_GEN.load(Ordering::Relaxed))))
        .default_open(default_open)
        .resizable(true)
        .movable(true)
        .collapsible(true)
        .default_size(default_size)
        .frame(frame);

    w = w.default_pos(state.pos.map(|[x, y]| Pos2::new(x, y)).unwrap_or(default_pos));
    if let Some([sw, sh]) = state.size {
        w = w.default_size(Vec2::new(sw, sh));
    }
    w
}

static SAVE_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

fn persist_geometry(ctx: &Context, rect: Rect, saved: &RaceDirectorWindowState, apply: fn(&mut hachimi::Config, RaceDirectorWindowState)) {
    let pos = [rect.min.x, rect.min.y];
    let size = [rect.width(), rect.height()];
    if saved.pos == Some(pos) && saved.size == Some(size) {
        return;
    }
    // Wait until the pointer is fully released before persisting, so a drag/resize in
    // progress doesn't trigger a save (and disk write) on every single frame.
    if ctx.input(|i| i.pointer.any_down()) {
        return;
    }
    if SAVE_IN_FLIGHT.swap(true, Ordering::AcqRel) {
        return;
    }
    let new_state = RaceDirectorWindowState { pos: Some(pos), size: Some(size) };
    std::thread::spawn(move || {
        let hachimi = Hachimi::instance();
        let mut new_config = hachimi.config.load().as_ref().clone();
        apply(&mut new_config, new_state);
        let _ = hachimi.save_and_reload_config(new_config);
        SAVE_IN_FLIGHT.store(false, Ordering::Release);
    });
}

// ── visibility gate ──────────────────────────────────────────────────────────────────────
pub fn showing() -> bool {
    // All three checks are atomic reads set on the game thread — safe to call from the
    // render thread with no IL2CPP involvement.
    Hachimi::instance().config.load().race_director.enabled
        && RaceHorseManagerBase::is_race_active()
        && race_director::is_gate_open()
        && !race_director::is_race_finished()
}

// ── panels ───────────────────────────────────────────────────────────────────────────────
fn timing_tower(ctx: &Context, default_pos: Pos2, state: &RaceDirectorWindowState, opacity: f32) {
    let rows = race_director::field_rows();
    // Collapsed by default: with every panel on, this one plus Win Probability are the
    // most likely to overlap the others - starting collapsed keeps the initial layout
    // readable, and either one is still one click away to expand.
    let resp = make_window("Race Director \u{00b7} Timing Tower", "rd_tower", default_pos, Vec2::new(360.0, 420.0), false, state, opacity)
        .show(ctx, |ui| {
            if rows.is_empty() {
                ui.small("Waiting for a race\u{2026}");
                return;
            }
            ui.style_mut().spacing.item_spacing.y = 3.0;
            egui::ScrollArea::vertical().show(ui, |ui| {
                for row in &rows {
                    draw_row(ui, row);
                }
            });
        });
    if let Some(resp) = resp {
        persist_geometry(ctx, resp.response.rect, state, |c, s| c.race_director.window_tower = s);
    }
}

fn draw_row(ui: &mut egui::Ui, row: &FieldRow) {
    let row_h = 24.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), egui::Sense::click());
    if response.clicked() {
        race_director::set_followed_gate(row.gate);
    }
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    if row.followed {
        painter.rect_filled(rect, 3.0, Color32::from_rgb(70, 60, 30));
    } else if response.hovered() {
        painter.rect_filled(rect, 3.0, Color32::from_rgb(50, 50, 56));
    }

    let chip_w = 24.0;
    let chip = Rect::from_min_size(rect.min, Vec2::new(chip_w, row_h));
    let chip_col = if row.trend > 0 { GOOD } else if row.trend < 0 { BAD } else { Color32::from_gray(60) };
    painter.rect_filled(chip, 3.0, chip_col);
    painter.text(chip.center(), egui::Align2::CENTER_CENTER, row.pos.to_string(), egui::FontId::proportional(13.0), Color32::WHITE);

    let mut x = rect.min.x + chip_w + 6.0;
    let bar = Rect::from_min_size(Pos2::new(x, rect.min.y + 2.0), Vec2::new(4.0, row_h - 4.0));
    painter.rect_filled(bar, 1.0, style_color(row.style));
    x += 10.0;

    let name_col = if row.followed { GOLD } else { Color32::from_gray(225) };
    painter.text(Pos2::new(x, rect.center().y), egui::Align2::LEFT_CENTER, &row.name, egui::FontId::proportional(13.0), name_col);

    let win_w = 46.0;
    let stam_w = 58.0;
    let gap_w = 60.0;
    let stam_rect = Rect::from_min_size(Pos2::new(rect.max.x - stam_w, rect.min.y + 3.0), Vec2::new(stam_w, row_h - 6.0));
    pbar(painter, stam_rect, row.sta, stamina_color(row.sta), &format!("{:.0}%", row.sta * 100.0));

    let gap_rect = Rect::from_min_size(Pos2::new(stam_rect.min.x - gap_w, rect.min.y), Vec2::new(gap_w, row_h));
    let gap_txt = if row.pos == 1 { "leader".to_owned() } else { format!("+{:.1}m", row.gap_leader) };
    painter.text(gap_rect.center(), egui::Align2::CENTER_CENTER, gap_txt, egui::FontId::proportional(11.5), DIM);

    let win_rect = Rect::from_min_size(Pos2::new(gap_rect.min.x - win_w, rect.min.y), Vec2::new(win_w, row_h));
    let win_col = if row.win > 0.3 { GOLD } else { DIM };
    painter.text(win_rect.center(), egui::Align2::CENTER_CENTER, format!("{:.0}%", row.win * 100.0), egui::FontId::proportional(12.0), win_col);
}

fn followed_panel(ctx: &Context, default_pos: Pos2, state: &RaceDirectorWindowState, opacity: f32) {
    let resp = make_window("Race Director \u{00b7} Followed Uma", "rd_followed", default_pos, Vec2::new(320.0, 280.0), true, state, opacity)
        .show(ctx, |ui| {
            let Some(tv) = race_director::telemetry() else {
                ui.small("Waiting for a race\u{2026}");
                return;
            };
            ui.heading(RichText::new(&tv.followed_name).color(GOLD));
            ui.label(RichText::new(format!("Gate {}  \u{00b7}  {}", tv.followed.gate, style_label(tv.followed.running_style))).color(DIM));

            let f = tv.followed;
            let sta = if f.max_hp > 0.0 { (f.hp / f.max_hp).clamp(0.0, 1.0) } else { 0.0 };
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 16.0), egui::Sense::hover());
            pbar(ui.painter(), rect, sta, stamina_color(sta), &format!("Stamina {:.0}%", sta * 100.0));
            ui.label(format!("{:.1} m/s   {:.0} m covered", f.speed, f.distance));

            let follow = race_director::follow_state();
            let mut tags = Vec::new();
            if f.spurt { tags.push(("SPURT", GOLD)); }
            if f.exhausted { tags.push(("EXHAUSTED", BAD)); }
            if f.fight { tags.push(("FIGHT", WARN)); }
            if f.blocked { tags.push(("BOXED IN", BAD)); }
            if follow.kakari { tags.push(("KAKARI", WARN)); }
            if follow.downhill { tags.push(("DOWNHILL", GOOD)); }
            if !tags.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for (label, color) in tags {
                        ui.label(RichText::new(label).color(color).small());
                    }
                });
            }

            if let Some(rival) = tv.rival {
                ui.separator();
                let dir = if tv.rival_ahead { "ahead" } else { "behind" };
                ui.label(format!("Rival ({dir}): {}  \u{00b7}  {:.1}m gap  \u{00b7}  {:.1} m/s", tv.rival_name, tv.gap, rival.speed));
            }

            ui.separator();
            ui.label(RichText::new("Pace").color(DIM).small());
            let trace = race_director::speed_trace();
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 48.0), egui::Sense::hover());
            spark(ui.painter(), rect, &trace, race_director::PACE_BUCKETS, Color32::from_rgb(90, 170, 235));

            let feed = race_director::skill_feed();
            if !feed.is_empty() {
                ui.separator();
                ui.label(RichText::new("Skills").color(DIM).small());
                ui.horizontal_wrapped(|ui| {
                    for (_, name, effect) in feed.iter().rev().take(6) {
                        let text = if effect.is_empty() { name.clone() } else { format!("{name} ({effect})") };
                        ui.label(RichText::new(text).small());
                    }
                });
            }
        });
    if let Some(resp) = resp {
        persist_geometry(ctx, resp.response.rect, state, |c, s| c.race_director.window_followed = s);
    }
}

fn win_probability(ctx: &Context, default_pos: Pos2, state: &RaceDirectorWindowState, opacity: f32) {
    // Collapsed by default - see timing_tower's comment.
    let resp = make_window("Race Director \u{00b7} Win Probability", "rd_winprob", default_pos, Vec2::new(240.0, 240.0), false, state, opacity)
        .show(ctx, |ui| {
            let mut rows = race_director::field_rows();
            if rows.is_empty() {
                ui.small("Waiting for a race\u{2026}");
                return;
            }
            rows.sort_by(|a, b| b.win.partial_cmp(&a.win).unwrap_or(std::cmp::Ordering::Equal));
            for row in rows.iter().take(8) {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width() * 0.55, 16.0), egui::Sense::hover());
                    let col = if row.followed { GOLD } else { style_color(row.style) };
                    pbar(ui.painter(), rect, row.win, col, &format!("{:.0}%", row.win * 100.0));
                    ui.label(&row.name);
                });
            }
        });
    if let Some(resp) = resp {
        persist_geometry(ctx, resp.response.rect, state, |c, s| c.race_director.window_winprob = s);
    }
}

fn predicted_positions(ctx: &Context, default_pos: Pos2, state: &RaceDirectorWindowState, opacity: f32) {
    let resp = make_window("Race Director \u{00b7} Server Expected Positions", "rd_predicted", default_pos, Vec2::new(300.0, 320.0), true, state, opacity)
        .show(ctx, |ui| {
            let rows = race_director::predicted_rows();
            if rows.is_empty() {
                ui.small("Waiting for a race\u{2026}");
                return;
            }
            ui.label(RichText::new("Precomputed by the server - not a guess.").color(DIM).small());
            let min_t = rows.first().map(|r| r.time).unwrap_or(0.0);
            ui.style_mut().spacing.item_spacing.y = 3.0;
            egui::ScrollArea::vertical().show(ui, |ui| {
                for row in &rows {
                    ui.horizontal(|ui| {
                        // FinishOrder is 0-based (0 = winner) in the raw data - +1 for display.
                        ui.label(RichText::new(format!("{:>2}", row.order + 1)).color(GOLD).strong());
                        ui.label(race_director::gate_name(row.gate));
                        let gap = row.time - min_t;
                        let gap_txt = if gap < 0.005 { "leader".to_owned() } else { format!("+{gap:.2}s") };
                        ui.label(RichText::new(gap_txt).color(DIM).small());
                        if let Some(reason) = defeat_label(row.defeat) {
                            ui.label(RichText::new(reason).color(BAD).small());
                        }
                    });
                }
            });
        });
    if let Some(resp) = resp {
        persist_geometry(ctx, resp.response.rect, state, |c, s| c.race_director.window_predicted = s);
    }
}

pub fn run(ctx: &Context) {
    if !showing() {
        return;
    }
    let cfg = Hachimi::instance().config.load();
    let rd = &cfg.race_director;
    let vp = ctx.viewport_rect();
    let margin = 16.0;

    if rd.show_tower {
        timing_tower(ctx, Pos2::new(vp.max.x - 376.0 - margin, vp.min.y + margin), &rd.window_tower, rd.opacity);
    }
    if rd.show_followed {
        followed_panel(ctx, Pos2::new(vp.min.x + margin, vp.min.y + margin), &rd.window_followed, rd.opacity);
    }
    if rd.show_winprob {
        win_probability(ctx, Pos2::new(vp.max.x - 256.0 - margin, vp.max.y - 256.0 - margin), &rd.window_winprob, rd.opacity);
    }
    if rd.show_predicted {
        predicted_positions(ctx, Pos2::new(vp.min.x + margin, vp.max.y - 336.0 - margin), &rd.window_predicted, rd.opacity);
    }
}
