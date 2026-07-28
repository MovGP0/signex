//! Local render configuration for signex-app.
//!
//! This module replaces the old renderer runtime config surface so app-side
//! preferences stay independent from removed legacy code.

use std::sync::{OnceLock, RwLock};

pub const IOSEVKA: iced::Font = iced::Font::with_name("Iosevka");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerPortStyle {
    Standard,
    #[default]
    Altium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelStyle {
    #[default]
    Standard,
    Altium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MultisheetStyle {
    #[default]
    Standard,
    Altium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridStyle {
    #[default]
    Dots,
    Lines,
    SmallCrosses,
}

impl GridStyle {
    pub const ALL: &'static [Self] = &[Self::Dots, Self::Lines, Self::SmallCrosses];
}

impl std::fmt::Display for PowerPortStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standard => write!(f, "Standard"),
            Self::Altium => write!(f, "Altium"),
        }
    }
}

impl std::fmt::Display for LabelStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standard => write!(f, "Standard"),
            Self::Altium => write!(f, "Altium"),
        }
    }
}

impl std::fmt::Display for MultisheetStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standard => write!(f, "Standard"),
            Self::Altium => write!(f, "Altium"),
        }
    }
}

impl std::fmt::Display for GridStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dots => write!(f, "Dots"),
            Self::Lines => write!(f, "Lines"),
            Self::SmallCrosses => write!(f, "Small crosses"),
        }
    }
}

/// How a click selects/drags a pin in the symbol editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PinSelectionMode {
    /// Altium parity — only the pin body/tip is grabbable.
    #[default]
    PinOnly,
    /// The pin is also grabbable by its name or number label, and a
    /// selected pin's labels glow with it.
    TextAndPin,
}

impl PinSelectionMode {
    pub const ALL: [Self; 2] = [Self::PinOnly, Self::TextAndPin];
    /// True when name/number labels are grabbable + glow.
    #[must_use]
    pub const fn allows_label_grab(self) -> bool {
        matches!(self, Self::TextAndPin)
    }

    /// Stable token used to persist this mode to `prefs.json`.
    #[must_use]
    pub const fn pref_token(self) -> &'static str {
        match self {
            Self::PinOnly => "pin_only",
            Self::TextAndPin => "text_and_pin",
        }
    }

    /// Parse a persisted token back into a mode — unknown/legacy values
    /// fall back to the `PinOnly` default.
    #[must_use]
    pub fn from_pref_token(s: &str) -> Self {
        match s {
            "text_and_pin" => Self::TextAndPin,
            _ => Self::PinOnly,
        }
    }
}

impl std::fmt::Display for PinSelectionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::PinOnly => "Pin only",
            Self::TextAndPin => "Text and pin",
        })
    }
}

#[derive(Clone, Copy)]
struct CanvasTextConfig {
    font_name: &'static str,
    font: iced::Font,
    size_scale: f32,
    bold: bool,
    italic: bool,
    power_port_style: PowerPortStyle,
    label_style: LabelStyle,
    multisheet_style: MultisheetStyle,
    grid_style: GridStyle,
    /// Grid style for the symbol editor (independent of schematic grid style).
    symbol_grid_style: GridStyle,
}

const fn build_font(name: &'static str, bold: bool, italic: bool) -> iced::Font {
    iced::Font {
        family: iced::font::Family::Name(name),
        weight: if bold {
            iced::font::Weight::Bold
        } else {
            iced::font::Weight::Normal
        },
        stretch: iced::font::Stretch::Normal,
        style: if italic {
            iced::font::Style::Italic
        } else {
            iced::font::Style::Normal
        },
    }
}

fn canvas_text_config() -> &'static RwLock<CanvasTextConfig> {
    static CONFIG: OnceLock<RwLock<CanvasTextConfig>> = OnceLock::new();
    CONFIG.get_or_init(|| {
        RwLock::new(CanvasTextConfig {
            font_name: "Iosevka",
            font: IOSEVKA,
            size_scale: 1.0,
            bold: false,
            italic: false,
            power_port_style: PowerPortStyle::Altium,
            label_style: LabelStyle::Standard,
            multisheet_style: MultisheetStyle::Standard,
            grid_style: GridStyle::Dots,
            symbol_grid_style: GridStyle::Dots,
        })
    })
}

pub fn set_canvas_font_name(name: &str) {
    let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
    if let Ok(mut cfg) = canvas_text_config().write() {
        cfg.font_name = leaked;
        cfg.font = build_font(leaked, cfg.bold, cfg.italic);
    }
}

pub fn set_canvas_font_size(size_px: f32) {
    let scale = (size_px / 11.0).clamp(0.5, 3.0);
    if let Ok(mut cfg) = canvas_text_config().write() {
        cfg.size_scale = scale;
    }
}

pub fn set_canvas_font_style(bold: bool, italic: bool) {
    if let Ok(mut cfg) = canvas_text_config().write() {
        cfg.bold = bold;
        cfg.italic = italic;
        cfg.font = build_font(cfg.font_name, cfg.bold, cfg.italic);
    }
}

pub fn set_power_port_style(style: PowerPortStyle) {
    if let Ok(mut cfg) = canvas_text_config().write() {
        cfg.power_port_style = style;
    }
}

#[must_use]
pub fn power_port_style() -> PowerPortStyle {
    canvas_text_config()
        .read()
        .map_or(PowerPortStyle::Altium, |c| c.power_port_style)
}

pub fn set_label_style(style: LabelStyle) {
    if let Ok(mut cfg) = canvas_text_config().write() {
        cfg.label_style = style;
    }
}

#[must_use]
pub fn label_style() -> LabelStyle {
    canvas_text_config()
        .read()
        .map_or(LabelStyle::Standard, |c| c.label_style)
}

pub fn set_multisheet_style(style: MultisheetStyle) {
    if let Ok(mut cfg) = canvas_text_config().write() {
        cfg.multisheet_style = style;
    }
}

#[must_use]
pub fn multisheet_style() -> MultisheetStyle {
    canvas_text_config()
        .read()
        .map_or(MultisheetStyle::Standard, |c| c.multisheet_style)
}

pub fn set_grid_style(style: GridStyle) {
    if let Ok(mut cfg) = canvas_text_config().write() {
        cfg.grid_style = style;
    }
}

#[must_use]
pub fn grid_style() -> GridStyle {
    canvas_text_config()
        .read()
        .map_or(GridStyle::Dots, |c| c.grid_style)
}

pub fn set_symbol_grid_style(style: GridStyle) {
    if let Ok(mut cfg) = canvas_text_config().write() {
        cfg.symbol_grid_style = style;
    }
}

#[must_use]
pub fn symbol_grid_style() -> GridStyle {
    canvas_text_config()
        .read()
        .map_or(GridStyle::Dots, |c| c.symbol_grid_style)
}

#[must_use]
pub fn canvas_font() -> iced::Font {
    canvas_text_config().read().map_or(IOSEVKA, |c| c.font)
}

#[must_use]
pub fn canvas_font_size_scale() -> f32 {
    canvas_text_config().read().map_or(1.0, |c| c.size_scale)
}

#[must_use]
pub fn to_iced(c: &signex_types::theme::Color) -> iced::Color {
    iced::Color::from_rgba8(c.r, c.g, c.b, f32::from(c.a) / 255.0)
}
