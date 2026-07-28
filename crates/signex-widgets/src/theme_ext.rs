//! Theme bridge layer — converts `signex_types::theme::ThemeTokens` to Iced styles.
//!
//! All colors in the widget crate flow through this module so that
//! no hardcoded color values leak into widget code.

use iced::widget::container;
use iced::{Border, Color};
use signex_types::theme::{Color as SxColor, ThemeTokens};

// ---------------------------------------------------------------------------
// Core color conversion
// ---------------------------------------------------------------------------

/// Convert a signex `Color` (u8 components) to an Iced `Color` (f32 0..1).
#[must_use]
pub fn to_color(c: &SxColor) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, f32::from(c.a) / 255.0)
}

// ---------------------------------------------------------------------------
// Text color helpers
// ---------------------------------------------------------------------------

/// Primary text color from theme tokens.
#[must_use]
pub fn text_primary(tokens: &ThemeTokens) -> Color {
    to_color(&tokens.text)
}

/// Secondary / muted text color.
#[must_use]
pub fn text_secondary(tokens: &ThemeTokens) -> Color {
    to_color(&tokens.text_secondary)
}

/// Accent color (for highlights, active elements).
#[must_use]
pub fn accent(tokens: &ThemeTokens) -> Color {
    to_color(&tokens.accent)
}

/// Error color.
#[must_use]
pub fn error_color(tokens: &ThemeTokens) -> Color {
    to_color(&tokens.error)
}

/// Warning color.
#[must_use]
pub fn warning_color(tokens: &ThemeTokens) -> Color {
    to_color(&tokens.warning)
}

/// Success / "on" indicator color.
#[must_use]
pub fn success_color(tokens: &ThemeTokens) -> Color {
    to_color(&tokens.success)
}

/// Border color.
#[must_use]
pub fn border_color(tokens: &ThemeTokens) -> Color {
    to_color(&tokens.border)
}

/// Selection highlight background.
#[must_use]
pub fn selection_color(tokens: &ThemeTokens) -> Color {
    to_color(&tokens.selection)
}

/// Hover highlight color.
#[must_use]
pub fn hover_color(tokens: &ThemeTokens) -> Color {
    to_color(&tokens.hover)
}

/// Theme accent color — used for active-project markers and the
/// "open" indicator dot on the tree.
#[must_use]
pub fn accent_color(tokens: &ThemeTokens) -> Color {
    to_color(&tokens.accent)
}

// ---------------------------------------------------------------------------
// Container style factories
// ---------------------------------------------------------------------------

/// Panel background container style (side panels, docks).
#[must_use]
pub fn panel_bg(tokens: &ThemeTokens) -> container::Style {
    container::Style {
        background: Some(to_color(&tokens.panel_bg).into()),
        text_color: Some(to_color(&tokens.text)),
        border: Border {
            width: 1.0,
            radius: 0.0.into(),
            color: to_color(&tokens.border),
        },
        ..container::Style::default()
    }
}

/// Toolbar background container style.
#[must_use]
pub fn toolbar_bg(tokens: &ThemeTokens) -> container::Style {
    container::Style {
        background: Some(to_color(&tokens.toolbar_bg).into()),
        text_color: Some(to_color(&tokens.text)),
        border: Border {
            width: 0.0,
            radius: 0.0.into(),
            color: to_color(&tokens.border),
        },
        ..container::Style::default()
    }
}

/// Status bar background container style (1px top border).
#[must_use]
pub fn status_bar_bg(tokens: &ThemeTokens) -> container::Style {
    container::Style {
        background: Some(to_color(&tokens.statusbar_bg).into()),
        text_color: Some(to_color(&tokens.text)),
        border: Border {
            width: 1.0,
            radius: 0.0.into(),
            color: to_color(&tokens.border),
        },
        ..container::Style::default()
    }
}

/// General application background.
#[must_use]
pub fn app_bg(tokens: &ThemeTokens) -> container::Style {
    container::Style {
        background: Some(to_color(&tokens.bg).into()),
        text_color: Some(to_color(&tokens.text)),
        border: Border::default(),
        ..container::Style::default()
    }
}

/// Paper / content area background.
#[must_use]
pub fn paper_bg(tokens: &ThemeTokens) -> container::Style {
    container::Style {
        background: Some(to_color(&tokens.paper).into()),
        text_color: Some(to_color(&tokens.text)),
        border: Border::default(),
        ..container::Style::default()
    }
}

/// Accent-colored container (for selected / active items).
#[must_use]
pub fn accent_bg(tokens: &ThemeTokens) -> container::Style {
    container::Style {
        background: Some(to_color(&tokens.accent).into()),
        text_color: Some(to_color(&tokens.text)),
        border: Border::default(),
        ..container::Style::default()
    }
}
