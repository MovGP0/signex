//! Explanatory cross-section drawn above the results: an outer trace on
//! the board surface and an inner trace buried between two dielectric
//! layers, annotated with the quantities the calculator works with (width
//! W, copper thickness T, area A = W x T, current I, length L, rise dT).
//!
//! The proportions are schematic and never scaled to the entered numbers;
//! the only live content is the computed width next to each W. Colours
//! come from [`ThemeTokens`] only. Own drawing, not a copy of any figure.

use std::cell::RefCell;
use std::fmt;

use iced::widget::canvas::{self, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme, alignment, mouse};
use signex_types::theme::ThemeTokens;

use super::control::format_number;
use super::domain::{LayerResult, TraceCalculation, WidthUnit};
use crate::theme_ext;

/// Height the control reserves for the drawing.
pub const DIAGRAM_HEIGHT: f32 = 190.0;

const LABEL_SIZE: f32 = 10.0;
const LINE_WIDTH: f32 = 1.0;
const HATCH_SPACING: f32 = 5.0;
const TICK: f32 = 3.0;
const FILL_ALPHA: f32 = 0.25;
const MARKER_RADIUS: f32 = 5.0;

/// The text the diagram shows next to W for each layer, `None` when the
/// inputs gave no result.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiagramLabels {
    pub outer_width: Option<String>,
    pub inner_width: Option<String>,
}

/// Build the W labels from a calculation. Pure, so it is tested without
/// rendering.
pub fn width_labels(
    calculation: Option<&TraceCalculation>,
    outer_unit: WidthUnit,
    inner_unit: WidthUnit,
) -> DiagramLabels {
    DiagramLabels {
        outer_width: calculation.map(|c| width_label(c.outer, outer_unit)),
        inner_width: calculation.map(|c| width_label(c.inner, inner_unit)),
    }
}

fn width_label(result: LayerResult, unit: WidthUnit) -> String {
    format!("W = {} {unit}", format_number(result.width(unit)))
}

/// The colours the drawing uses, taken from the theme.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Palette {
    text: Color,
    muted: Color,
    outline: Color,
    copper: Color,
    dielectric: Color,
    background: Color,
}

impl Palette {
    fn from_tokens(tokens: &ThemeTokens) -> Self {
        Self {
            text: theme_ext::text_primary(tokens),
            muted: theme_ext::text_secondary(tokens),
            outline: theme_ext::border_color(tokens),
            copper: theme_ext::accent(tokens),
            dielectric: theme_ext::to_color(&tokens.panel_bg),
            background: theme_ext::to_color(&tokens.bg),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct DiagramKey {
    labels: DiagramLabels,
    palette: Palette,
}

/// Canvas geometry cache, cleared only when the labels or the theme
/// colours change (a size change is handled by the canvas cache itself).
#[derive(Default)]
pub struct DiagramCache {
    cache: canvas::Cache,
    key: RefCell<Option<DiagramKey>>,
}

impl Clone for DiagramCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl fmt::Debug for DiagramCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DiagramCache")
    }
}

impl DiagramCache {
    fn geometry(&self, renderer: &iced::Renderer, size: Size, key: DiagramKey) -> Geometry {
        if self.key.borrow().as_ref() != Some(&key) {
            self.cache.clear();
            *self.key.borrow_mut() = Some(key.clone());
        }
        self.cache.draw(renderer, size, |frame| paint(frame, &key))
    }
}

struct Diagram<'a> {
    cache: &'a DiagramCache,
    key: DiagramKey,
}

/// The diagram as an element, borrowing the control's cache.
pub fn view<'a, M: 'a>(
    cache: &'a DiagramCache,
    labels: DiagramLabels,
    tokens: &ThemeTokens,
) -> Element<'a, M> {
    let key = DiagramKey {
        labels,
        palette: Palette::from_tokens(tokens),
    };
    canvas::Canvas::new(Diagram { cache, key })
        .width(Length::Fill)
        .height(DIAGRAM_HEIGHT)
        .into()
}

impl<M> canvas::Program<M> for Diagram<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        vec![
            self.cache
                .geometry(renderer, bounds.size(), self.key.clone()),
        ]
    }
}

fn stroke(color: Color) -> Stroke<'static> {
    Stroke::default().with_color(color).with_width(LINE_WIDTH)
}

fn label(
    frame: &mut canvas::Frame,
    content: &str,
    at: Point,
    color: Color,
    align_x: alignment::Horizontal,
) {
    frame.fill_text(Text {
        content: content.to_string(),
        position: at,
        size: LABEL_SIZE.into(),
        color,
        align_x: align_x.into(),
        align_y: alignment::Vertical::Center,
        ..Text::default()
    });
}

fn paint(frame: &mut canvas::Frame, key: &DiagramKey) {
    let palette = key.palette;
    let (w, h) = (frame.width(), frame.height());
    let slab_left = 0.05 * w;
    let slab_right = 0.95 * w;
    let slab_top = 0.42 * h;
    let slab_mid = 0.64 * h;
    let slab_bottom = 0.82 * h;
    let copper_t = 0.10 * h;
    let trace_w = 0.22 * w;

    // Two dielectric layers, the inner trace sits on their joint.
    for (top, bottom) in [(slab_top, slab_mid), (slab_mid, slab_bottom)] {
        let origin = Point::new(slab_left, top);
        let size = Size::new(slab_right - slab_left, bottom - top);
        frame.fill_rectangle(origin, size, palette.dielectric);
        frame.stroke(&Path::rectangle(origin, size), stroke(palette.outline));
    }

    let outer = Rectangle::new(
        Point::new(0.12 * w, slab_top - copper_t),
        Size::new(trace_w, copper_t),
    );
    let inner = Rectangle::new(
        Point::new(0.62 * w, slab_mid - copper_t / 2.0),
        Size::new(trace_w, copper_t),
    );
    for trace in [outer, inner] {
        draw_trace(frame, trace, &palette);
    }

    annotate_outer(frame, outer, key);
    annotate_inner(frame, inner, key);
    layer_names(frame, outer, inner, slab_bottom, &palette);
    legend(frame, w, h, &palette);
}

fn draw_trace(frame: &mut canvas::Frame, trace: Rectangle, palette: &Palette) {
    let fill = Color {
        a: FILL_ALPHA,
        ..palette.copper
    };
    frame.fill_rectangle(trace.position(), trace.size(), fill);
    hatch(frame, trace, stroke(palette.copper));
    frame.stroke(
        &Path::rectangle(trace.position(), trace.size()),
        stroke(palette.copper),
    );
    marker(frame, trace.center(), palette);
}

/// 45 degree hatching clipped to `rect`.
fn hatch(frame: &mut canvas::Frame, rect: Rectangle, line: Stroke<'static>) {
    let (x0, y0) = (rect.x, rect.y);
    let (x1, y1) = (rect.x + rect.width, rect.y + rect.height);
    let mut c = x0 - y1;
    while c < x1 - y0 {
        // The line y = x - c, clipped to the rectangle.
        let xa = x0.max(c + y0);
        let xb = x1.min(c + y1);
        if xa < xb {
            frame.stroke(
                &Path::line(Point::new(xa, xa - c), Point::new(xb, xb - c)),
                line,
            );
        }
        c += HATCH_SPACING;
    }
}

/// Circle with a dot: a vector pointing out of the page, which is the
/// direction of both the current I and the length L.
fn marker(frame: &mut canvas::Frame, at: Point, palette: &Palette) {
    frame.fill(&Path::circle(at, MARKER_RADIUS), palette.background);
    frame.stroke(&Path::circle(at, MARKER_RADIUS), stroke(palette.text));
    frame.fill(&Path::circle(at, 1.5), palette.text);
}

fn horizontal_dimension(frame: &mut canvas::Frame, x0: f32, x1: f32, y: f32, color: Color) {
    frame.stroke(
        &Path::line(Point::new(x0, y), Point::new(x1, y)),
        stroke(color),
    );
    for x in [x0, x1] {
        frame.stroke(
            &Path::line(Point::new(x, y - TICK), Point::new(x, y + TICK)),
            stroke(color),
        );
    }
}

fn vertical_dimension(frame: &mut canvas::Frame, x: f32, y0: f32, y1: f32, color: Color) {
    frame.stroke(
        &Path::line(Point::new(x, y0), Point::new(x, y1)),
        stroke(color),
    );
    for y in [y0, y1] {
        frame.stroke(
            &Path::line(Point::new(x - TICK, y), Point::new(x + TICK, y)),
            stroke(color),
        );
    }
}

fn width_text(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("W")
}

fn annotate_outer(frame: &mut canvas::Frame, trace: Rectangle, key: &DiagramKey) {
    let p = &key.palette;
    let y = trace.y - 12.0;
    horizontal_dimension(frame, trace.x, trace.x + trace.width, y, p.muted);
    let center = Point::new(trace.center_x(), y - 9.0);
    label(
        frame,
        width_text(&key.labels.outer_width),
        center,
        p.text,
        alignment::Horizontal::Center,
    );
    let t_x = trace.x - 10.0;
    vertical_dimension(frame, t_x, trace.y, trace.y + trace.height, p.muted);
    label(
        frame,
        "T",
        Point::new(t_x - 6.0, trace.center_y()),
        p.text,
        alignment::Horizontal::Right,
    );
    label(
        frame,
        "A = W \u{d7} T",
        Point::new(trace.x + trace.width + 10.0, trace.center_y()),
        p.text,
        alignment::Horizontal::Left,
    );
}

fn annotate_inner(frame: &mut canvas::Frame, trace: Rectangle, key: &DiagramKey) {
    let p = &key.palette;
    let y = trace.y + trace.height + 12.0;
    horizontal_dimension(frame, trace.x, trace.x + trace.width, y, p.muted);
    label(
        frame,
        width_text(&key.labels.inner_width),
        Point::new(trace.center_x(), y + 9.0),
        p.text,
        alignment::Horizontal::Center,
    );
    let t_x = trace.x + trace.width + 10.0;
    vertical_dimension(frame, t_x, trace.y, trace.y + trace.height, p.muted);
    label(
        frame,
        "T",
        Point::new(t_x + 6.0, trace.center_y()),
        p.text,
        alignment::Horizontal::Left,
    );
    label(
        frame,
        "A = W \u{d7} T",
        Point::new(trace.x - 10.0, trace.center_y()),
        p.text,
        alignment::Horizontal::Right,
    );
}

fn layer_names(
    frame: &mut canvas::Frame,
    outer: Rectangle,
    inner: Rectangle,
    slab_bottom: f32,
    palette: &Palette,
) {
    for (name, trace) in [("Outer layer", outer), ("Inner layer", inner)] {
        label(
            frame,
            name,
            Point::new(trace.center_x(), slab_bottom + 12.0),
            palette.text,
            alignment::Horizontal::Center,
        );
    }
}

/// What the symbols mean, in the free space above the slab.
fn legend(frame: &mut canvas::Frame, w: f32, h: f32, palette: &Palette) {
    let x = 0.40 * w;
    let y = 0.14 * h;
    marker(frame, Point::new(x, y), palette);
    label(
        frame,
        "I and L run along the trace",
        Point::new(x + 12.0, y),
        palette.muted,
        alignment::Horizontal::Left,
    );
    label(
        frame,
        "\u{394}T = rise above ambient",
        Point::new(x, y + 18.0),
        palette.muted,
        alignment::Horizontal::Left,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace_calculator::control::{TraceCalculatorControl, TraceCalculatorMessage};

    #[test]
    fn labels_come_from_the_computed_widths_in_the_selected_units() {
        let control = TraceCalculatorControl::default();
        let calculation = control.calculation().expect("defaults are valid");

        let labels = width_labels(Some(&calculation), WidthUnit::Millimeter, WidthUnit::Mil);

        let outer_mm = calculation.outer.width(WidthUnit::Millimeter);
        let inner_mil = calculation.inner.width(WidthUnit::Mil);
        assert_eq!(
            labels.outer_width,
            Some(format!("W = {} mm", format_number(outer_mm)))
        );
        assert_eq!(
            labels.inner_width,
            Some(format!("W = {} mil", format_number(inner_mil)))
        );
    }

    #[test]
    fn outer_and_inner_labels_differ_because_k_differs() {
        let control = TraceCalculatorControl::default();
        let calculation = control.calculation().expect("defaults are valid");
        let labels = width_labels(Some(&calculation), WidthUnit::Mil, WidthUnit::Mil);
        assert_ne!(labels.outer_width, labels.inner_width);
    }

    #[test]
    fn invalid_input_yields_no_value_labels() {
        let mut control = TraceCalculatorControl::default();
        control.update(TraceCalculatorMessage::CurrentChanged("x".into()));
        assert!(control.calculation().is_err());

        let labels = width_labels(
            control.calculation().ok().as_ref(),
            WidthUnit::Millimeter,
            WidthUnit::Millimeter,
        );

        assert_eq!(labels, DiagramLabels::default());
        assert_eq!(width_text(&labels.outer_width), "W");
    }

    #[test]
    fn the_palette_follows_the_theme_tokens() {
        use signex_types::theme::{ThemeId, theme_tokens};
        let dark = Palette::from_tokens(&theme_tokens(ThemeId::Signex));
        let light = Palette::from_tokens(&theme_tokens(ThemeId::SolarizedLight));
        assert_ne!(dark, light);
        assert_eq!(
            dark.copper,
            theme_ext::accent(&theme_tokens(ThemeId::Signex))
        );
    }
}
