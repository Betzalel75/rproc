use egui::{Color32, FontFamily, FontId, Stroke, TextStyle, Visuals};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Theme enum + global
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Theme {
    Dark = 0,
    Light = 1,
    /// Follow the desktop colour scheme. Detected via D-Bus (portal) with a
    /// gsettings fallback for GNOME/Cinnamon. Re-sampled every 10 s so
    /// flipping the system preference takes effect without a restart.
    System = 2,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::System => "system",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "dark" => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            "system" => Some(Theme::System),
            _ => None,
        }
    }
}

/// Lock-free global storing the *user-chosen* theme (0=dark, 1=light,
/// 2=system). The resolved palette (dark vs light) is computed from this
/// plus, when System, the current desktop preference.
static THEME: AtomicU8 = AtomicU8::new(0);

/// Last resolved theme value we actually applied (0=dark, 1=light), so a
/// system-theme change is detected even when the user choice stays System.
static LAST_RESOLVED: AtomicU8 = AtomicU8::new(0xFF);

pub fn init(theme: Theme) {
    THEME.store(theme as u8, Ordering::Relaxed);
}

pub fn set_theme(theme: Theme) {
    THEME.store(theme as u8, Ordering::Relaxed);
    // Force a re-apply on the next frame by invalidating the cached
    // resolved value — otherwise mark_applied() would skip it.
    LAST_RESOLVED.store(0xFF, Ordering::Relaxed);
}

pub fn current_theme() -> Theme {
    match THEME.load(Ordering::Relaxed) {
        0 => Theme::Dark,
        1 => Theme::Light,
        _ => Theme::System,
    }
}

/// Cached result of the last system-theme probe (0=dark, 1=light).
/// Lives at module scope so both branches of `resolved_kind()` read the
/// same value.
static CACHED_DETECTED: AtomicU8 = AtomicU8::new(0);

/// The actual palette-kind in use right now. When the user chose System,
/// this queries the desktop preference (throttled to every 10 s).
fn resolved_kind() -> u8 {
    let raw = THEME.load(Ordering::Relaxed);
    match raw {
        0 => 0, // Dark
        1 => 1, // Light
        _ => {
            // Throttle system detection: D-Bus calls are cheap but not free.
            // Re-sample at most once per 10 s.
            static LAST_CHECK: std::sync::Mutex<Option<Instant>> =
                std::sync::Mutex::new(None);
            let now = Instant::now();
            let should_check = LAST_CHECK
                .lock()
                .ok()
                .map_or(true, |last| last.map_or(true, |t| now.duration_since(t).as_secs() >= 10));

            if should_check {
                let detected = detect_system_theme();
                if let Ok(mut guard) = LAST_CHECK.lock() {
                    *guard = Some(now);
                }
                let v: u8 = match detected {
                    Some(Theme::Light) => 1,
                    _ => 0,
                };
                CACHED_DETECTED.store(v, Ordering::Relaxed);
                v
            } else {
                CACHED_DETECTED.load(Ordering::Relaxed)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// System theme detection
// ---------------------------------------------------------------------------

/// Probe the desktop colour-scheme preference. Returns `Some(Theme::Light)`
/// for a light desktop, `Some(Theme::Dark)` for dark, and `None` if
/// detection failed (falls back to dark).
///
/// Tries in order:
/// 1. D-Bus freedesktop portal (`org.freedesktop.portal.Settings.Read`)
///    — the cross-desktop standard (GNOME, KDE, wlroots, etc.).
/// 2. `gsettings` — GNOME/Cinnamon fallback when D-Bus isn't available.
fn detect_system_theme() -> Option<Theme> {
    detect_via_dbus().or_else(detect_via_gsettings)
}

/// D-Bus portal: `color-scheme` returns `1` = prefer-dark, `2` = prefer-light.
fn detect_via_dbus() -> Option<Theme> {
    let out = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--dest=org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Settings.Read",
            "string:org.freedesktop.appearance",
            "string:color-scheme",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);

    for line in text.lines() {
        let line = line.trim();
        if let Some(num) = line.strip_prefix("uint32 ") {
            return match num.trim().parse::<u32>().ok()? {
                1 => Some(Theme::Dark),
                _ => Some(Theme::Light),
            };
        }
    }
    None
}

/// gsettings: `color-scheme` returns `'prefer-dark'` or `'default'` (light).
fn detect_via_gsettings() -> Option<Theme> {
    let out = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    if text.contains("prefer-dark") {
        Some(Theme::Dark)
    } else {
        Some(Theme::Light)
    }
}

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub struct Palette {
    pub accent: Color32,
    pub bg: Color32,
    pub sidebar_bg: Color32,
    pub panel_bg: Color32,
    pub card_bg: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub graph_cpu: Color32,
    pub graph_ram: Color32,
    pub graph_disk: Color32,
    pub graph_net: Color32,
    pub graph_gpu: Color32,
    pub ok: Color32,
    pub warn: Color32,
    pub err: Color32,
}

pub const DARK: Palette = Palette {
    accent: Color32::from_rgb(0x60, 0xCD, 0xFF),
    bg: Color32::from_rgb(0x1F, 0x1F, 0x1F),
    sidebar_bg: Color32::from_rgb(0x2A, 0x2A, 0x2A),
    panel_bg: Color32::from_rgb(0x26, 0x26, 0x26),
    card_bg: Color32::from_rgb(0x2E, 0x2E, 0x2E),
    text: Color32::from_rgb(0xE6, 0xE6, 0xE6),
    text_dim: Color32::from_rgb(0x9A, 0x9A, 0x9A),
    graph_cpu: Color32::from_rgb(0x39, 0xA7, 0xFF),
    graph_ram: Color32::from_rgb(0xB4, 0x6A, 0xFF),
    graph_disk: Color32::from_rgb(0x4E, 0xE0, 0xB3),
    graph_net: Color32::from_rgb(0xFF, 0xB0, 0x4E),
    graph_gpu: Color32::from_rgb(0xFF, 0x5C, 0x8A),
    ok: Color32::from_rgb(0x55, 0xD1, 0x7C),
    warn: Color32::from_rgb(0xFF, 0xC4, 0x4D),
    err: Color32::from_rgb(0xFF, 0x6B, 0x6B),
};

pub const LIGHT: Palette = Palette {
    accent: Color32::from_rgb(0x00, 0x78, 0xD4),
    bg: Color32::from_rgb(0xF5, 0xF5, 0xF5),
    sidebar_bg: Color32::from_rgb(0xE8, 0xE8, 0xE8),
    panel_bg: Color32::from_rgb(0xEE, 0xEE, 0xEE),
    card_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    text: Color32::from_rgb(0x1A, 0x1A, 0x1A),
    text_dim: Color32::from_rgb(0x70, 0x70, 0x70),
    graph_cpu: Color32::from_rgb(0x00, 0x78, 0xD4),
    graph_ram: Color32::from_rgb(0x88, 0x6C, 0xE4),
    graph_disk: Color32::from_rgb(0x0D, 0x94, 0x88),
    graph_net: Color32::from_rgb(0xD9, 0x77, 0x06),
    graph_gpu: Color32::from_rgb(0xE1, 0x1D, 0x48),
    ok: Color32::from_rgb(0x16, 0xA3, 0x4A),
    warn: Color32::from_rgb(0xCA, 0x8A, 0x04),
    err: Color32::from_rgb(0xDC, 0x26, 0x26),
};

fn palette() -> &'static Palette {
    match resolved_kind() {
        0 => &DARK,
        _ => &LIGHT,
    }
}

// ---------------------------------------------------------------------------
// Apply tracking — detects when the effective (resolved) theme changes so
// we only rebuild the full egui style on actual transitions.
// ---------------------------------------------------------------------------

fn mark_applied(resolved: u8) -> bool {
    LAST_RESOLVED.swap(resolved, Ordering::Relaxed) != resolved
}

// ---------------------------------------------------------------------------
// Accessor functions
// ---------------------------------------------------------------------------

pub fn accent() -> Color32 { palette().accent }
pub fn bg() -> Color32 { palette().bg }
pub fn sidebar_bg() -> Color32 { palette().sidebar_bg }
pub fn panel_bg() -> Color32 { palette().panel_bg }
pub fn card_bg() -> Color32 { palette().card_bg }
pub fn text() -> Color32 { palette().text }
pub fn text_dim() -> Color32 { palette().text_dim }
pub fn graph_cpu() -> Color32 { palette().graph_cpu }
pub fn graph_ram() -> Color32 { palette().graph_ram }
pub fn graph_disk() -> Color32 { palette().graph_disk }
pub fn graph_net() -> Color32 { palette().graph_net }
pub fn graph_gpu() -> Color32 { palette().graph_gpu }
pub fn ok() -> Color32 { palette().ok }
pub fn warn() -> Color32 { palette().warn }
pub fn err() -> Color32 { palette().err }

// ---------------------------------------------------------------------------
// Apply the active theme to an egui context.
// ---------------------------------------------------------------------------

pub fn apply(ctx: &egui::Context) {
    // Check the *effective* (resolved) theme — not the raw user choice — so
    // that a system-theme flip triggers a rebuild even though the user-chosen
    // Theme::System→u8 hasn't changed.
    let resolved = resolved_kind();
    if !mark_applied(resolved) {
        return;
    }

    // Drop the bundled emoji fonts. They're the heaviest blobs in the default
    // set (~1.5 MB of raw glyph data the font system would otherwise keep
    // parsed in memory) and rproc renders no emoji. Removing by name is a
    // no-op if egui ever renames them, so it can't accidentally blank the
    // Latin text fonts (Hack / Ubuntu-Light), which stay untouched.
    let mut fonts = egui::FontDefinitions::default();
    for emoji in ["NotoEmoji-Regular", "emoji-icon-font"] {
        fonts.font_data.remove(emoji);
        for family in fonts.families.values_mut() {
            family.retain(|name| name != emoji);
        }
    }
    ctx.set_fonts(fonts);

    let p = palette();
    let is_dark = resolved == 0;

    let mut visuals = if is_dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.panel_fill = p.bg;
    visuals.window_fill = p.bg;
    visuals.extreme_bg_color = p.panel_bg;
    visuals.widgets.noninteractive.bg_fill = p.sidebar_bg;
    visuals.widgets.inactive.bg_fill = p.panel_bg;
    visuals.widgets.inactive.weak_bg_fill = p.panel_bg;

    if is_dark {
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x38, 0x38, 0x38);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x38, 0x38, 0x38);
        visuals.widgets.active.bg_fill = Color32::from_rgb(0x44, 0x44, 0x44);
        visuals.widgets.active.weak_bg_fill = Color32::from_rgb(0x44, 0x44, 0x44);
        visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0x60, 0xCD, 0xFF, 60);
    } else {
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(0xDD, 0xDD, 0xDD);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xDD, 0xDD, 0xDD);
        visuals.widgets.active.bg_fill = Color32::from_rgb(0xCC, 0xCC, 0xCC);
        visuals.widgets.active.weak_bg_fill = Color32::from_rgb(0xCC, 0xCC, 0xCC);
        visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0x00, 0x78, 0xD4, 50);
    }

    visuals.selection.stroke = Stroke::new(1.0, p.accent);
    visuals.override_text_color = Some(p.text);
    visuals.hyperlink_color = p.accent;

    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(8);
    style.text_styles = std::collections::BTreeMap::from([
        (
            TextStyle::Heading,
            FontId::new(20.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(13.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        ),
    ]);
    ctx.set_style(style);
}
