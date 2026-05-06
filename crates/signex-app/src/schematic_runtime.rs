//! Local schematic runtime used by `signex-app`.
//!
//! This module keeps schematic rendering, hit-test, and overlay behavior
//! self-contained inside the app runtime contract.

use iced::advanced::text as advanced_text;
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Rectangle, Vector};
use signex_gfx::scene::{DirtyFlags, Scene};
use signex_renderer::schematic::{
    ArcInput, JunctionInput, OverlayCircleInput, OverlayInputs, OverlayLineInput,
    OverlayPolygonInput, PolygonInput, SchematicRenderer,
    SchematicSnapshot as RendererSnapshot, TextInput, ViewRenderer, WireInput,
};
use signex_renderer::theme::ResolvedTheme;
use signex_types::schematic::{
    Aabb, FillType, HAlign, Label, LabelType, Point, SchDrawing, SchematicSheet, SelectedItem,
    SelectedKind, Symbol, TextProp, TextNote, VAlign,
};
use signex_types::theme::{CanvasColors, Color as ThemeColor};
use std::collections::{HashMap, HashSet};

pub type SchematicRenderSnapshot = SchematicSheet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderInvalidation(u32);

impl RenderInvalidation {
    pub const NONE: Self = Self(0);
    pub const SYMBOLS: Self = Self(1 << 0);
    pub const WIRES: Self = Self(1 << 1);
    pub const LABELS: Self = Self(1 << 2);
    pub const TEXT_NOTES: Self = Self(1 << 3);
    pub const BUSES: Self = Self(1 << 4);
    pub const BUS_ENTRIES: Self = Self(1 << 5);
    pub const JUNCTIONS: Self = Self(1 << 6);
    pub const NO_CONNECTS: Self = Self(1 << 7);
    pub const CHILD_SHEETS: Self = Self(1 << 8);
    pub const DRAWINGS: Self = Self(1 << 9);
    pub const LIB_SYMBOLS: Self = Self(1 << 10);
    pub const PAPER: Self = Self(1 << 11);
    pub const FULL: Self = Self(u32::MAX);
}

impl std::ops::BitOr for RenderInvalidation {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for RenderInvalidation {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScreenTransform {
    pub offset_x: f32,
    pub offset_y: f32,
    pub scale: f32,
}

impl ScreenTransform {
    #[inline]
    pub fn world_to_screen(&self, world: (f64, f64)) -> iced::Point {
        iced::Point::new(
            world.0 as f32 * self.scale + self.offset_x,
            world.1 as f32 * self.scale + self.offset_y,
        )
    }
}

pub trait SchematicSheetExt {
    fn symbol_position(&self, uuid: uuid::Uuid) -> Option<(f64, f64)>;
    fn symbol_reference_position(&self, uuid: uuid::Uuid) -> Option<(f64, f64)>;
    fn symbol_value_position(&self, uuid: uuid::Uuid) -> Option<(f64, f64)>;
}

impl SchematicSheetExt for SchematicSheet {
    fn symbol_position(&self, uuid: uuid::Uuid) -> Option<(f64, f64)> {
        self.symbols
            .iter()
            .find(|s| s.uuid == uuid)
            .map(|s| (s.position.x, s.position.y))
    }

    fn symbol_reference_position(&self, uuid: uuid::Uuid) -> Option<(f64, f64)> {
        self.symbols
            .iter()
            .find(|s| s.uuid == uuid)
            .and_then(|s| s.ref_text.as_ref())
            .map(|t| (t.position.x, t.position.y))
    }

    fn symbol_value_position(&self, uuid: uuid::Uuid) -> Option<(f64, f64)> {
        self.symbols
            .iter()
            .find(|s| s.uuid == uuid)
            .and_then(|s| s.val_text.as_ref())
            .map(|t| (t.position.x, t.position.y))
    }
}

#[derive(Debug, Default, Clone)]
pub struct SchematicRenderCache {
    sheet: Option<SchematicSheet>,
    preview: Option<SchematicSheet>,
}

impl SchematicRenderCache {
    pub fn from_sheet(sheet: &SchematicSheet) -> Self {
        Self {
            sheet: Some(sheet.clone()),
            preview: None,
        }
    }

    pub fn update_from_sheet(&mut self, sheet: &SchematicSheet, _invalidation: RenderInvalidation) {
        self.sheet = Some(sheet.clone());
        self.preview = None;
    }

    pub fn snapshot(&self) -> &SchematicSheet {
        self.sheet
            .as_ref()
            .expect("SchematicRenderCache::snapshot called before initialization")
    }

    pub fn prepared_preview(&self) -> Option<&SchematicSheet> {
        self.preview.as_ref()
    }
}

#[inline]
pub fn instance_transform(symbol: &Symbol, local_point: &Point) -> (f64, f64) {
    let x = local_point.x;
    let y = -local_point.y;
    let rad = -symbol.rotation.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();
    let mut rx = x * cos - y * sin;
    let mut ry = x * sin + y * cos;
    if symbol.mirror_y {
        rx = -rx;
    }
    if symbol.mirror_x {
        ry = -ry;
    }
    (rx + symbol.position.x, ry + symbol.position.y)
}

pub fn draw_power_port_preview(
    frame: &mut canvas::Frame,
    symbol: &Symbol,
    transform: &ScreenTransform,
    color: Color,
) {
    let half_w = screen_px_to_world_mm(7.0, transform.scale) as f32;
    let half_h = screen_px_to_world_mm(4.0, transform.scale) as f32;
    let cx = symbol.position.x as f32;
    let cy = symbol.position.y as f32;

    let mut parameter_texts = Vec::new();
    if !symbol.reference.is_empty() {
        parameter_texts.push(TextInput {
            content: symbol.reference.clone(),
            position: [
                cx + screen_px_to_world_mm(9.0, transform.scale) as f32,
                cy - screen_px_to_world_mm(4.0, transform.scale) as f32,
            ],
            size_mm: (11.0 * 0.72 / transform.scale.max(0.001)).max(0.1),
            color: to_rgba(color),
            bold: false,
            italic: false,
            rotation_rad: 0.0,
            h_align: HAlign::Left,
            v_align: VAlign::Top,
        });
    }

    let snapshot = RendererSnapshot {
        wires: Vec::new(),
        junctions: Vec::new(),
        arcs: Vec::new(),
        polygons: vec![PolygonInput {
            vertices: vec![
                [cx - half_w, cy + half_h],
                [cx - half_w, cy - half_h],
                [cx + half_w, cy],
            ],
            fill_color: to_rgba(Color {
                a: color.a * 0.24,
                ..color
            }),
            stroke_color: Some(to_rgba(color)),
            stroke_width_mm: stroke_world_mm(
                signex_types::schematic::SCHEMATIC_RENDER_POWER_PORT_STROKE_PX,
                transform.scale,
            ),
        }],
        labels: Vec::new(),
        pin_texts: Vec::new(),
        reference_value_texts: Vec::new(),
        parameter_texts,
        overlays: OverlayInputs::default(),
        erc_markers: Vec::new(),
        wire_color_overrides: HashMap::new(),
    };

    draw_renderer_snapshot(
        frame,
        &snapshot,
        &ResolvedTheme::from_canvas_colors(signex_types::theme::canvas_colors(
            signex_types::theme::ThemeId::Signex,
        )),
        DirtyFlags::POLYGONS | DirtyFlags::TEXT,
        transform,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn render_schematic(
    frame: &mut canvas::Frame,
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
    wire_color_overrides: Option<&HashMap<uuid::Uuid, ThemeColor>>,
) {
    render_schematic_with_renderer(
        frame,
        snapshot,
        transform,
        colors,
        bounds,
        focus_set,
        wire_color_overrides,
    );
}

fn render_schematic_with_renderer(
    frame: &mut canvas::Frame,
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
    wire_color_overrides: Option<&HashMap<uuid::Uuid, ThemeColor>>,
) {
    let renderer_snapshot = build_renderer_snapshot(
        snapshot,
        transform,
        colors,
        bounds,
        focus_set,
        wire_color_overrides,
    );
    if renderer_snapshot.wires.is_empty()
        && renderer_snapshot.junctions.is_empty()
        && renderer_snapshot.arcs.is_empty()
        && renderer_snapshot.polygons.is_empty()
        && renderer_snapshot.labels.is_empty()
        && renderer_snapshot.pin_texts.is_empty()
        && renderer_snapshot.reference_value_texts.is_empty()
        && renderer_snapshot.parameter_texts.is_empty()
    {
        return;
    }

    draw_renderer_snapshot(
        frame,
        &renderer_snapshot,
        &ResolvedTheme::from_canvas_colors(*colors),
        DirtyFlags::LINES
            | DirtyFlags::CIRCLES
            | DirtyFlags::ARCS
            | DirtyFlags::POLYGONS
            | DirtyFlags::TEXT,
        transform,
    );
}

fn build_renderer_snapshot(
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
    wire_color_overrides: Option<&HashMap<uuid::Uuid, ThemeColor>>,
) -> RendererSnapshot {
    let mut wires = Vec::new();
    let mut junctions = Vec::with_capacity(snapshot.junctions.len());
    let mut arcs = Vec::new();
    let mut polygons = Vec::new();
    let mut labels = Vec::new();
    let mut reference_value_texts = Vec::new();
    let mut parameter_texts = Vec::new();

    for wire in &snapshot.wires {
        let p0 = transform.world_to_screen((wire.start.x, wire.start.y));
        let p1 = transform.world_to_screen((wire.end.x, wire.end.y));
        if !line_visible(p0, p1, bounds) {
            continue;
        }

        let base_color = wire_color_overrides
            .and_then(|map| map.get(&wire.uuid))
            .map(to_iced)
            .unwrap_or_else(|| to_iced(&colors.wire));
        let color = focus_color(base_color, focus_set, wire.uuid);
        wires.push(WireInput {
            id: renderer_id(wire.uuid),
            p0: [wire.start.x as f32, wire.start.y as f32],
            p1: [wire.end.x as f32, wire.end.y as f32],
            width_mm: wire
                .stroke_width
                .max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM)
                as f32,
            explicit_color: Some(to_rgba(color)),
        });
    }

    for bus in &snapshot.buses {
        let p0 = transform.world_to_screen((bus.start.x, bus.start.y));
        let p1 = transform.world_to_screen((bus.end.x, bus.end.y));
        if !line_visible(p0, p1, bounds) {
            continue;
        }

        wires.push(WireInput {
            id: renderer_id(bus.uuid),
            p0: [bus.start.x as f32, bus.start.y as f32],
            p1: [bus.end.x as f32, bus.end.y as f32],
            width_mm: signex_types::schematic::SCHEMATIC_RENDER_BUS_STROKE_MM as f32,
            explicit_color: Some(to_rgba(focus_color(to_iced(&colors.bus), focus_set, bus.uuid))),
        });
    }

    for no_connect in &snapshot.no_connects {
        let center = transform.world_to_screen((no_connect.position.x, no_connect.position.y));
        if !point_visible(center, bounds, 10.0) {
            continue;
        }

        let color = focus_color(to_iced(&colors.body), focus_set, no_connect.uuid);
        let len_mm = signex_types::schematic::SCHEMATIC_RENDER_NO_CONNECT_HALF_LEN_MM.max(
            screen_px_to_world_mm(
                signex_types::schematic::SCHEMATIC_RENDER_NO_CONNECT_MIN_HALF_LEN_PX,
                transform.scale,
            ),
        );
        let width_mm = stroke_world_mm(
            signex_types::schematic::SCHEMATIC_RENDER_NO_CONNECT_STROKE_PX,
            transform.scale,
        );
        let (cx, cy) = (no_connect.position.x as f32, no_connect.position.y as f32);
        let len = len_mm as f32;
        wires.push(WireInput {
            id: renderer_id(no_connect.uuid),
            p0: [cx - len, cy - len],
            p1: [cx + len, cy + len],
            width_mm,
            explicit_color: Some(to_rgba(color)),
        });
        wires.push(WireInput {
            id: renderer_id(no_connect.uuid).saturating_add(1),
            p0: [cx - len, cy + len],
            p1: [cx + len, cy - len],
            width_mm,
            explicit_color: Some(to_rgba(color)),
        });
    }

    for junction in &snapshot.junctions {
        let center = transform.world_to_screen((junction.position.x, junction.position.y));
        if !point_visible(center, bounds, 6.0) {
            continue;
        }

        junctions.push(JunctionInput {
            center: [junction.position.x as f32, junction.position.y as f32],
            radius_mm: (junction.diameter * 0.5)
                .max(signex_types::schematic::SCHEMATIC_RENDER_JUNCTION_MIN_RADIUS_MM)
                as f32,
            color: to_rgba(focus_color(to_iced(&colors.junction), focus_set, junction.uuid)),
        });
    }

    for symbol in &snapshot.symbols {
        let bbox = symbol_body_aabb(symbol);
        let min = transform.world_to_screen((bbox.min_x, bbox.min_y));
        let max = transform.world_to_screen((bbox.max_x, bbox.max_y));
        let rect_min = iced::Point::new(min.x.min(max.x), min.y.min(max.y));
        let size = iced::Size::new((max.x - min.x).abs(), (max.y - min.y).abs());
        if !rect_visible(rect_min, size, bounds) {
            continue;
        }

        let stroke_color = focus_color(to_iced(&colors.body), focus_set, symbol.uuid);
        let fill_color = focus_color(to_iced(&colors.body_fill), focus_set, symbol.uuid);
        polygons.push(PolygonInput {
            vertices: vec![
                [bbox.min_x as f32, bbox.min_y as f32],
                [bbox.max_x as f32, bbox.min_y as f32],
                [bbox.max_x as f32, bbox.max_y as f32],
                [bbox.min_x as f32, bbox.max_y as f32],
            ],
            fill_color: to_rgba(fill_color),
            stroke_color: Some(to_rgba(stroke_color)),
            stroke_width_mm: stroke_world_mm(
                signex_types::schematic::SCHEMATIC_RENDER_SYMBOL_BODY_STROKE_PX,
                transform.scale,
            ),
        });

        if !symbol.reference.is_empty() {
            reference_value_texts.push(TextInput {
                content: symbol.reference.clone(),
                position: [symbol.position.x as f32, (symbol.position.y - 3.5) as f32],
                size_mm: 1.05,
                color: to_rgba(stroke_color),
                bold: false,
                italic: false,
                rotation_rad: symbol.rotation.to_radians() as f32,
                h_align: HAlign::Center,
                v_align: VAlign::Bottom,
            });
        }
        if !symbol.value.is_empty() {
            reference_value_texts.push(TextInput {
                content: symbol.value.clone(),
                position: [symbol.position.x as f32, (symbol.position.y + 3.6) as f32],
                size_mm: 1.05,
                color: to_rgba(focus_color(to_iced(&colors.value), focus_set, symbol.uuid)),
                bold: false,
                italic: false,
                rotation_rad: symbol.rotation.to_radians() as f32,
                h_align: HAlign::Center,
                v_align: VAlign::Top,
            });
        }
    }

    for sheet in &snapshot.child_sheets {
        let x0 = sheet.position.x;
        let y0 = sheet.position.y;
        let x1 = sheet.position.x + sheet.size.0;
        let y1 = sheet.position.y + sheet.size.1;
        let min = transform.world_to_screen((x0, y0));
        let max = transform.world_to_screen((x1, y1));
        let rect_min = iced::Point::new(min.x.min(max.x), min.y.min(max.y));
        let size = iced::Size::new((max.x - min.x).abs(), (max.y - min.y).abs());
        if !rect_visible(rect_min, size, bounds) {
            continue;
        }

        let color = focus_color(to_iced(&colors.global_label), focus_set, sheet.uuid);
        let min_x = x0.min(x1) as f32;
        let min_y = y0.min(y1) as f32;
        let max_x = x0.max(x1) as f32;
        let max_y = y0.max(y1) as f32;
        polygons.push(PolygonInput {
            vertices: vec![[min_x, min_y], [max_x, min_y], [max_x, max_y], [min_x, max_y]],
            fill_color: [0.0, 0.0, 0.0, 0.0],
            stroke_color: Some(to_rgba(color)),
            stroke_width_mm: stroke_world_mm(
                signex_types::schematic::SCHEMATIC_RENDER_CHILD_SHEET_STROKE_PX,
                transform.scale,
            ),
        });

        parameter_texts.push(TextInput {
            content: sheet.name.clone(),
            position: [
                min_x + screen_px_to_world_mm(6.0, transform.scale) as f32,
                min_y + screen_px_to_world_mm(6.0, transform.scale) as f32,
            ],
            size_mm: 1.05,
            color: to_rgba(color),
            bold: false,
            italic: false,
            rotation_rad: 0.0,
            h_align: HAlign::Left,
            v_align: VAlign::Top,
        });

        for pin in &sheet.pins {
            junctions.push(JunctionInput {
                center: [pin.position.x as f32, pin.position.y as f32],
                radius_mm: screen_px_to_world_mm(
                    signex_types::schematic::SCHEMATIC_RENDER_CHILD_SHEET_PIN_RADIUS_PX,
                    transform.scale,
                ) as f32,
                color: to_rgba(Color { a: 0.3, ..color }),
            });
        }
    }

    for drawing in &snapshot.drawings {
        let uuid = match drawing {
            SchDrawing::Line { uuid, .. }
            | SchDrawing::Rect { uuid, .. }
            | SchDrawing::Circle { uuid, .. }
            | SchDrawing::Arc { uuid, .. }
            | SchDrawing::Polyline { uuid, .. } => *uuid,
        };
        let bbox = drawing_aabb(drawing);
        let min = transform.world_to_screen((bbox.min_x, bbox.min_y));
        let max = transform.world_to_screen((bbox.max_x, bbox.max_y));
        let rect_min = iced::Point::new(min.x.min(max.x), min.y.min(max.y));
        let size = iced::Size::new((max.x - min.x).abs(), (max.y - min.y).abs());
        if !rect_visible(rect_min, size, bounds) {
            continue;
        }

        let base_color = focus_color(to_iced(&colors.body), focus_set, uuid);
        match drawing {
            SchDrawing::Line {
                start,
                end,
                width,
                stroke_color,
                ..
            } => {
                wires.push(WireInput {
                    id: renderer_id(uuid),
                    p0: [start.x as f32, start.y as f32],
                    p1: [end.x as f32, end.y as f32],
                    width_mm: width.max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM)
                        as f32,
                    explicit_color: Some(to_rgba(resolve_stroke_color(stroke_color, base_color))),
                });
            }
            SchDrawing::Rect {
                start,
                end,
                width,
                fill,
                stroke_color,
                ..
            } => {
                polygons.push(PolygonInput {
                    vertices: vec![
                        [start.x as f32, start.y as f32],
                        [end.x as f32, start.y as f32],
                        [end.x as f32, end.y as f32],
                        [start.x as f32, end.y as f32],
                    ],
                    fill_color: fill_color_for(*fill, stroke_color, colors)
                        .map(to_rgba)
                        .unwrap_or([0.0, 0.0, 0.0, 0.0]),
                    stroke_color: Some(to_rgba(resolve_stroke_color(stroke_color, base_color))),
                    stroke_width_mm: width
                        .max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM)
                        as f32,
                });
            }
            SchDrawing::Circle {
                center,
                radius,
                width,
                fill,
                stroke_color,
                ..
            } => {
                polygons.push(PolygonInput {
                    vertices: circle_vertices(
                        [center.x, center.y],
                        radius
                            .max(screen_px_to_world_mm(
                                signex_types::schematic::SCHEMATIC_RENDER_DRAWING_MIN_CIRCLE_RADIUS_PX,
                                transform.scale,
                            )) as f32,
                        40,
                    ),
                    fill_color: fill_color_for(*fill, stroke_color, colors)
                        .map(to_rgba)
                        .unwrap_or([0.0, 0.0, 0.0, 0.0]),
                    stroke_color: Some(to_rgba(resolve_stroke_color(stroke_color, base_color))),
                    stroke_width_mm: width
                        .max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM)
                        as f32,
                });
            }
            SchDrawing::Arc {
                start,
                mid,
                end,
                width,
                stroke_color,
                ..
            } => {
                if let Some((cx, cy, r)) = circumcircle(
                    (start.x, start.y),
                    (mid.x, mid.y),
                    (end.x, end.y),
                ) {
                    let a0 = (start.y - cy).atan2(start.x - cx);
                    let am = (mid.y - cy).atan2(mid.x - cx);
                    let a1 = (end.y - cy).atan2(end.x - cx);
                    let (start_angle, end_angle) = if arc_sweeps_through_mid(a0, am, a1) {
                        (a0, a1)
                    } else {
                        (a1, a0)
                    };
                    arcs.push(ArcInput {
                        center: [cx as f32, cy as f32],
                        radius_mm: r
                            .max(screen_px_to_world_mm(
                                signex_types::schematic::SCHEMATIC_RENDER_DRAWING_MIN_ARC_RADIUS_PX,
                                transform.scale,
                            )) as f32,
                        start_angle_rad: start_angle as f32,
                        end_angle_rad: end_angle as f32,
                        width_mm: width
                            .max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM)
                            as f32,
                        color: to_rgba(resolve_stroke_color(stroke_color, base_color)),
                    });
                } else {
                    let stroke_color = to_rgba(resolve_stroke_color(stroke_color, base_color));
                    let width_mm =
                        width.max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM) as f32;
                    wires.push(WireInput {
                        id: renderer_id(uuid),
                        p0: [start.x as f32, start.y as f32],
                        p1: [mid.x as f32, mid.y as f32],
                        width_mm,
                        explicit_color: Some(stroke_color),
                    });
                    wires.push(WireInput {
                        id: renderer_id(uuid).saturating_add(1),
                        p0: [mid.x as f32, mid.y as f32],
                        p1: [end.x as f32, end.y as f32],
                        width_mm,
                        explicit_color: Some(stroke_color),
                    });
                }
            }
            SchDrawing::Polyline {
                points,
                width,
                fill,
                stroke_color,
                ..
            } => {
                if points.len() < 2 {
                    continue;
                }

                let stroke = to_rgba(resolve_stroke_color(stroke_color, base_color));
                let width_mm =
                    width.max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM) as f32;
                if matches!(fill, FillType::None) {
                    for idx in 1..points.len() {
                        let p0 = points[idx - 1];
                        let p1 = points[idx];
                        wires.push(WireInput {
                            id: renderer_id(uuid).saturating_add(idx as u64),
                            p0: [p0.x as f32, p0.y as f32],
                            p1: [p1.x as f32, p1.y as f32],
                            width_mm,
                            explicit_color: Some(stroke),
                        });
                    }
                } else {
                    polygons.push(PolygonInput {
                        vertices: points
                            .iter()
                            .map(|point| [point.x as f32, point.y as f32])
                            .collect(),
                        fill_color: fill_color_for(*fill, stroke_color, colors)
                            .map(to_rgba)
                            .unwrap_or([0.0, 0.0, 0.0, 0.0]),
                        stroke_color: Some(stroke),
                        stroke_width_mm: width_mm,
                    });
                }
            }
        }
    }

    for label in &snapshot.labels {
        let screen = transform.world_to_screen((label.position.x, label.position.y));
        if !point_visible(screen, bounds, 22.0) {
            continue;
        }

        let color = focus_color(label_color(label, colors), focus_set, label.uuid);
        if matches!(label.label_type, LabelType::Global | LabelType::Hierarchical) {
            polygons.push(label_marker_polygon(
                label,
                color,
                [0.0, 0.0, 0.0, 0.0],
                transform,
            ));
            labels.push(TextInput {
                content: label.text.clone(),
                position: [label.position.x as f32, label.position.y as f32],
                size_mm: label.font_size.max(signex_types::schematic::SCHEMATIC_TEXT_MM) as f32,
                color: to_rgba(color),
                bold: false,
                italic: false,
                rotation_rad: label.rotation.to_radians() as f32,
                h_align: HAlign::Center,
                v_align: VAlign::Center,
            });
        } else {
            labels.push(TextInput {
                content: label.text.clone(),
                position: [label.position.x as f32, label.position.y as f32],
                size_mm: label.font_size.max(signex_types::schematic::SCHEMATIC_TEXT_MM) as f32,
                color: to_rgba(color),
                bold: false,
                italic: false,
                rotation_rad: label.rotation.to_radians() as f32,
                h_align: label.justify,
                v_align: label.justify_v,
            });
        }
    }

    for note in &snapshot.text_notes {
        let pos = transform.world_to_screen((note.position.x, note.position.y));
        if !point_visible(pos, bounds, 28.0) {
            continue;
        }

        parameter_texts.push(TextInput {
            content: note.text.clone(),
            position: [note.position.x as f32, note.position.y as f32],
            size_mm: note.font_size.max(signex_types::schematic::SCHEMATIC_TEXT_MM) as f32,
            color: to_rgba(focus_color(to_iced(&colors.value), focus_set, note.uuid)),
            bold: false,
            italic: false,
            rotation_rad: note.rotation.to_radians() as f32,
            h_align: note.justify_h,
            v_align: note.justify_v,
        });
    }

    RendererSnapshot {
        wires,
        junctions,
        arcs,
        polygons,
        labels,
        pin_texts: Vec::new(),
        reference_value_texts,
        parameter_texts,
        overlays: OverlayInputs::default(),
        erc_markers: Vec::new(),
        wire_color_overrides: HashMap::new(),
    }
}

fn label_marker_polygon(
    label: &Label,
    stroke_color: Color,
    fill_color: [f32; 4],
    transform: &ScreenTransform,
) -> PolygonInput {
    let size_mm = label.font_size.max(signex_types::schematic::SCHEMATIC_TEXT_MM) as f32;
    let em_mm = size_mm / 0.72;
    let glyph_w = (label.text.chars().count().max(1) as f32) * (em_mm * 0.58);
    let half_h = em_mm * 0.62;
    let point = em_mm * 0.52;
    let w = glyph_w + em_mm * 0.65;
    let local = [
        (-point, 0.0_f32),
        (0.0, -half_h),
        (w, -half_h),
        (w, half_h),
        (0.0, half_h),
    ];

    let rad = (label.rotation as f32).to_radians();
    let cos = rad.cos();
    let sin = rad.sin();
    let cx = label.position.x as f32;
    let cy = label.position.y as f32;
    let vertices = local
        .iter()
        .map(|(x, y)| [cx + x * cos - y * sin, cy + x * sin + y * cos])
        .collect();

    PolygonInput {
        vertices,
        fill_color,
        stroke_color: Some(to_rgba(stroke_color)),
        stroke_width_mm: stroke_world_mm(
            signex_types::schematic::SCHEMATIC_RENDER_LABEL_GLYPH_STROKE_PX,
            transform.scale,
        ),
    }
}

fn renderer_id(uuid: uuid::Uuid) -> u64 {
    uuid.as_u128() as u64
}

fn draw_renderer_snapshot(
    frame: &mut canvas::Frame,
    snapshot: &RendererSnapshot,
    theme: &ResolvedTheme,
    dirty: DirtyFlags,
    transform: &ScreenTransform,
) {
    let mut scene = Scene::default();
    SchematicRenderer::build_scene(snapshot, theme, dirty, &mut scene);
    crate::renderer_scene_canvas::draw_scene_with_world_to_screen(
        frame,
        &scene,
        |point| transform.world_to_screen((point[0] as f64, point[1] as f64)),
        crate::renderer_scene_canvas::SceneDrawOptions {
            scale_px_per_mm: transform.scale,
            min_stroke_px: signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_PX,
            text_mm_per_em: 0.72,
            text_min_px: 6.0,
            text_max_px: 64.0,
        },
    );
}

fn to_rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

fn stroke_world_mm(base_width_px_at_100: f32, scale: f32) -> f32 {
    (stroke_px_at_zoom(base_width_px_at_100, scale) / scale.max(0.001))
        .max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM as f32)
}

fn screen_px_to_world_mm(px: f32, scale: f32) -> f64 {
    (px / scale.max(0.001)) as f64
}

fn circle_vertices(center: [f64; 2], radius: f32, segments: usize) -> Vec<[f32; 2]> {
    let segment_count = segments.max(12);
    let cx = center[0] as f32;
    let cy = center[1] as f32;
    let r = radius.max(0.01);

    (0..segment_count)
        .map(|step| {
            let theta = (step as f32 / segment_count as f32) * std::f32::consts::TAU;
            [cx + theta.cos() * r, cy + theta.sin() * r]
        })
        .collect()
}

pub mod text {
    use super::*;

    pub fn expand_char_escapes(text: &str) -> String {
        text.to_string()
    }

    pub fn escape_for_standard(text: &str) -> String {
        text.to_string()
    }

    pub fn draw_text_note_preview(
        frame: &mut canvas::Frame,
        note: &TextNote,
        transform: &ScreenTransform,
        color: Color,
    ) {
        let snapshot = RendererSnapshot {
            wires: Vec::new(),
            junctions: Vec::new(),
            arcs: Vec::new(),
            polygons: Vec::new(),
            labels: Vec::new(),
            pin_texts: Vec::new(),
            reference_value_texts: Vec::new(),
            parameter_texts: vec![TextInput {
                content: note.text.clone(),
                position: [note.position.x as f32, note.position.y as f32],
                size_mm: note.font_size.max(signex_types::schematic::SCHEMATIC_TEXT_MM) as f32,
                color: to_rgba(color),
                bold: false,
                italic: false,
                rotation_rad: note.rotation.to_radians() as f32,
                h_align: note.justify_h,
                v_align: note.justify_v,
            }],
            overlays: OverlayInputs::default(),
            erc_markers: Vec::new(),
            wire_color_overrides: HashMap::new(),
        };

        draw_renderer_snapshot(
            frame,
            &snapshot,
            &ResolvedTheme::from_canvas_colors(signex_types::theme::canvas_colors(
                signex_types::theme::ThemeId::Signex,
            )),
            DirtyFlags::TEXT,
            transform,
        );
    }
}

pub mod label {
    use super::*;

    pub fn draw_label_preview(
        frame: &mut canvas::Frame,
        label: &Label,
        transform: &ScreenTransform,
        stroke_color: Color,
        fill_color: Color,
    ) {
        let mut polygons = Vec::new();
        let mut labels = Vec::new();

        if matches!(label.label_type, LabelType::Global | LabelType::Hierarchical) {
            polygons.push(super::label_marker_polygon(
                label,
                stroke_color,
                to_rgba(fill_color),
                transform,
            ));
            labels.push(TextInput {
                content: label.text.clone(),
                position: [label.position.x as f32, label.position.y as f32],
                size_mm: label.font_size.max(signex_types::schematic::SCHEMATIC_TEXT_MM) as f32,
                color: to_rgba(stroke_color),
                bold: false,
                italic: false,
                rotation_rad: label.rotation.to_radians() as f32,
                h_align: HAlign::Center,
                v_align: VAlign::Center,
            });
        } else {
            labels.push(TextInput {
                content: label.text.clone(),
                position: [label.position.x as f32, label.position.y as f32],
                size_mm: label.font_size.max(signex_types::schematic::SCHEMATIC_TEXT_MM) as f32,
                color: to_rgba(stroke_color),
                bold: false,
                italic: false,
                rotation_rad: label.rotation.to_radians() as f32,
                h_align: label.justify,
                v_align: label.justify_v,
            });
        }

        let snapshot = RendererSnapshot {
            wires: Vec::new(),
            junctions: Vec::new(),
            arcs: Vec::new(),
            polygons,
            labels,
            pin_texts: Vec::new(),
            reference_value_texts: Vec::new(),
            parameter_texts: Vec::new(),
            overlays: OverlayInputs::default(),
            erc_markers: Vec::new(),
            wire_color_overrides: HashMap::new(),
        };

        draw_renderer_snapshot(
            frame,
            &snapshot,
            &ResolvedTheme::from_canvas_colors(signex_types::theme::canvas_colors(
                signex_types::theme::ThemeId::Signex,
            )),
            DirtyFlags::POLYGONS | DirtyFlags::TEXT,
            transform,
        );
    }
}

pub mod selection {
    use super::*;

    pub fn draw_selection_overlay(
        frame: &mut canvas::Frame,
        snapshot: &SchematicRenderSnapshot,
        selected: &[SelectedItem],
        transform: &ScreenTransform,
    ) {
        let stroke = Color::from_rgba(0.95, 0.95, 1.0, 0.95);
        let fill = Color::from_rgba(0.65, 0.72, 1.0, 0.12);
        let mut overlays = OverlayInputs::default();

        for item in selected {
            if let Some(bbox) = item_aabb(snapshot, item) {
                let min = transform.world_to_screen((bbox.min_x, bbox.min_y));
                let max = transform.world_to_screen((bbox.max_x, bbox.max_y));
                let size = iced::Size::new((max.x - min.x).abs(), (max.y - min.y).abs());

                if size.width <= signex_types::schematic::SCHEMATIC_RENDER_SELECTION_MARKER_THRESHOLD_PX
                    && size.height
                        <= signex_types::schematic::SCHEMATIC_RENDER_SELECTION_MARKER_THRESHOLD_PX
                {
                    let center = [
                        ((bbox.min_x + bbox.max_x) * 0.5) as f32,
                        ((bbox.min_y + bbox.max_y) * 0.5) as f32,
                    ];
                    overlays.snap_circles.push(OverlayCircleInput {
                        center,
                        radius_mm: screen_px_to_world_mm(
                            signex_types::schematic::SCHEMATIC_RENDER_SELECTION_MARKER_RADIUS_PX,
                            transform.scale,
                        ) as f32,
                        stroke_width_mm: 0.0,
                        color: to_rgba(fill),
                    });
                    overlays.snap_circles.push(OverlayCircleInput {
                        center,
                        radius_mm: screen_px_to_world_mm(
                            signex_types::schematic::SCHEMATIC_RENDER_SELECTION_MARKER_RADIUS_PX,
                            transform.scale,
                        ) as f32,
                        stroke_width_mm: stroke_world_mm(
                            signex_types::schematic::SCHEMATIC_RENDER_SELECTION_MARKER_STROKE_PX,
                            transform.scale,
                        ),
                        color: to_rgba(stroke),
                    });
                } else {
                    overlays.ghost_polygons.push(OverlayPolygonInput {
                        vertices: vec![
                            [bbox.min_x as f32, bbox.min_y as f32],
                            [bbox.max_x as f32, bbox.min_y as f32],
                            [bbox.max_x as f32, bbox.max_y as f32],
                            [bbox.min_x as f32, bbox.max_y as f32],
                        ],
                        fill_color: to_rgba(fill),
                        stroke_color: Some(to_rgba(stroke)),
                        stroke_width_mm: stroke_world_mm(
                            signex_types::schematic::SCHEMATIC_RENDER_SELECTION_RECT_STROKE_PX,
                            transform.scale,
                        ),
                    });
                }
            }
        }

        if overlays.preview_lines.is_empty()
            && overlays.ghost_polygons.is_empty()
            && overlays.lasso_lines.is_empty()
            && overlays.snap_circles.is_empty()
        {
            return;
        }

        let snapshot = RendererSnapshot {
            wires: Vec::new(),
            junctions: Vec::new(),
            arcs: Vec::new(),
            polygons: Vec::new(),
            labels: Vec::new(),
            pin_texts: Vec::new(),
            reference_value_texts: Vec::new(),
            parameter_texts: Vec::new(),
            overlays,
            erc_markers: Vec::new(),
            wire_color_overrides: HashMap::new(),
        };

        draw_renderer_snapshot(
            frame,
            &snapshot,
            &ResolvedTheme::from_canvas_colors(signex_types::theme::canvas_colors(
                signex_types::theme::ThemeId::Signex,
            )),
            DirtyFlags::OVERLAY,
            transform,
        );
    }
}

pub mod overlay {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ErcSeverity {
        Error,
        Warning,
        Info,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct ErcMarker {
        pub x_mm: f64,
        pub y_mm: f64,
        pub severity: ErcSeverity,
    }

    pub fn draw_erc_markers(
        frame: &mut canvas::Frame,
        markers: &[ErcMarker],
        transform: &ScreenTransform,
    ) {
        if markers.is_empty() {
            return;
        }

        let mut overlays = OverlayInputs::default();
        for marker in markers {
            let (fill, stroke) = match marker.severity {
                ErcSeverity::Error => (
                    Color::from_rgba(0.95, 0.25, 0.25, 0.6),
                    Color::from_rgb(0.95, 0.25, 0.25),
                ),
                ErcSeverity::Warning => (
                    Color::from_rgba(0.98, 0.72, 0.20, 0.6),
                    Color::from_rgb(0.98, 0.72, 0.20),
                ),
                ErcSeverity::Info => (
                    Color::from_rgba(0.30, 0.55, 0.85, 0.55),
                    Color::from_rgb(0.30, 0.55, 0.85),
                ),
            };

            let center = [marker.x_mm as f32, marker.y_mm as f32];
            overlays.snap_circles.push(OverlayCircleInput {
                center,
                radius_mm: screen_px_to_world_mm(16.0, transform.scale) as f32,
                stroke_width_mm: 0.0,
                color: [fill.r, fill.g, fill.b, 0.18],
            });
            overlays.snap_circles.push(OverlayCircleInput {
                center,
                radius_mm: screen_px_to_world_mm(7.0, transform.scale) as f32,
                stroke_width_mm: 0.0,
                color: to_rgba(fill),
            });
            overlays.snap_circles.push(OverlayCircleInput {
                center,
                radius_mm: screen_px_to_world_mm(7.0, transform.scale) as f32,
                stroke_width_mm: screen_px_to_world_mm(2.0, transform.scale) as f32,
                color: to_rgba(stroke),
            });

            let cross_len = screen_px_to_world_mm(4.0, transform.scale) as f32;
            let cx = marker.x_mm as f32;
            let cy = marker.y_mm as f32;
            overlays.preview_lines.push(OverlayLineInput {
                p0: [cx - cross_len, cy - cross_len],
                p1: [cx + cross_len, cy + cross_len],
                width_mm: screen_px_to_world_mm(1.5, transform.scale) as f32,
                color: to_rgba(Color::WHITE),
            });
            overlays.preview_lines.push(OverlayLineInput {
                p0: [cx - cross_len, cy + cross_len],
                p1: [cx + cross_len, cy - cross_len],
                width_mm: screen_px_to_world_mm(1.5, transform.scale) as f32,
                color: to_rgba(Color::WHITE),
            });
        }

        let snapshot = RendererSnapshot {
            wires: Vec::new(),
            junctions: Vec::new(),
            arcs: Vec::new(),
            polygons: Vec::new(),
            labels: Vec::new(),
            pin_texts: Vec::new(),
            reference_value_texts: Vec::new(),
            parameter_texts: Vec::new(),
            overlays,
            erc_markers: Vec::new(),
            wire_color_overrides: HashMap::new(),
        };

        draw_renderer_snapshot(
            frame,
            &snapshot,
            &ResolvedTheme::from_canvas_colors(signex_types::theme::canvas_colors(
                signex_types::theme::ThemeId::Signex,
            )),
            DirtyFlags::OVERLAY,
            transform,
        );
    }
}

pub mod hit_test {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum SelectionMode {
        #[default]
        Inside,
        Touching,
        Single,
    }

    pub fn hit_test(
        snapshot: &SchematicRenderSnapshot,
        world_x: f64,
        world_y: f64,
    ) -> Option<SelectedItem> {
        let point = Point::new(world_x, world_y);
        hit_test_items(snapshot, point)
            .into_iter()
            .next()
    }

    pub fn hit_test_polygon(
        snapshot: &SchematicRenderSnapshot,
        polygon: &[(f64, f64)],
    ) -> Vec<SelectedItem> {
        if polygon.len() < 3 {
            return Vec::new();
        }

        let mut out = Vec::new();
        for item in collect_item_bounds(snapshot) {
            if point_in_polygon((item.anchor.x, item.anchor.y), polygon) {
                out.push(item.item);
            }
        }
        out
    }

    pub fn hit_test_rect_mode(
        snapshot: &SchematicRenderSnapshot,
        rect: &Aabb,
        mode: SelectionMode,
    ) -> Vec<SelectedItem> {
        let mut out = Vec::new();

        for item in collect_item_bounds(snapshot) {
            let inside = rect.contains(item.bbox.min_x, item.bbox.min_y)
                && rect.contains(item.bbox.max_x, item.bbox.max_y);
            let touching = aabb_overlaps(rect, &item.bbox);

            let keep = match mode {
                SelectionMode::Inside | SelectionMode::Single => inside,
                SelectionMode::Touching => touching,
            };

            if keep {
                out.push(item.item);
            }
        }

        out
    }

    fn hit_test_items(snapshot: &SchematicRenderSnapshot, point: Point) -> Vec<SelectedItem> {
        let mut out = Vec::new();

        for item in collect_item_bounds(snapshot).into_iter().rev() {
            let hit = match item.item.kind {
                SelectedKind::Wire => hit_wire(snapshot, item.item.uuid, point),
                SelectedKind::Bus => hit_bus(snapshot, item.item.uuid, point),
                _ => item.bbox.expand(0.25).contains(point.x, point.y),
            };
            if hit {
                out.push(item.item);
            }
        }

        out
    }

    fn hit_wire(snapshot: &SchematicRenderSnapshot, uuid: uuid::Uuid, point: Point) -> bool {
        snapshot
            .wires
            .iter()
            .find(|wire| wire.uuid == uuid)
            .is_some_and(|wire| {
                let tolerance = wire
                    .stroke_width
                    .max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM)
                    .max(signex_types::schematic::SCHEMATIC_HIT_WIRE_TOLERANCE_MM);
                point_to_segment_distance(point, wire.start, wire.end) <= tolerance
            })
    }

    fn hit_bus(snapshot: &SchematicRenderSnapshot, uuid: uuid::Uuid, point: Point) -> bool {
        snapshot
            .buses
            .iter()
            .find(|bus| bus.uuid == uuid)
            .is_some_and(|bus| {
                point_to_segment_distance(point, bus.start, bus.end)
                    <= signex_types::schematic::SCHEMATIC_HIT_BUS_TOLERANCE_MM
            })
    }
}

#[derive(Debug, Clone, Copy)]
struct ItemBound {
    item: SelectedItem,
    bbox: Aabb,
    anchor: Point,
}

fn collect_item_bounds(snapshot: &SchematicRenderSnapshot) -> Vec<ItemBound> {
    let mut out = Vec::new();

    for symbol in &snapshot.symbols {
        let item = SelectedItem::new(symbol.uuid, SelectedKind::Symbol);
        out.push(ItemBound {
            item,
            bbox: symbol_body_aabb(symbol),
            anchor: symbol.position,
        });

        if let Some(ref_text) = symbol.ref_text.as_ref() {
            out.push(ItemBound {
                item: SelectedItem::new(symbol.uuid, SelectedKind::SymbolRefField),
                bbox: text_prop_aabb(symbol, &symbol.reference, ref_text),
                anchor: ref_text.position,
            });
        }
        if let Some(val_text) = symbol.val_text.as_ref() {
            out.push(ItemBound {
                item: SelectedItem::new(symbol.uuid, SelectedKind::SymbolValField),
                bbox: text_prop_aabb(symbol, &symbol.value, val_text),
                anchor: val_text.position,
            });
        }
    }

    for wire in &snapshot.wires {
        out.push(ItemBound {
            item: SelectedItem::new(wire.uuid, SelectedKind::Wire),
            bbox: Aabb::new(wire.start.x, wire.start.y, wire.end.x, wire.end.y)
                .expand(
                    wire.stroke_width
                        .max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM),
                ),
            anchor: Point::new((wire.start.x + wire.end.x) * 0.5, (wire.start.y + wire.end.y) * 0.5),
        });
    }

    for bus in &snapshot.buses {
        out.push(ItemBound {
            item: SelectedItem::new(bus.uuid, SelectedKind::Bus),
            bbox: Aabb::new(bus.start.x, bus.start.y, bus.end.x, bus.end.y)
                .expand(signex_types::schematic::SCHEMATIC_RENDER_BUS_STROKE_MM),
            anchor: Point::new((bus.start.x + bus.end.x) * 0.5, (bus.start.y + bus.end.y) * 0.5),
        });
    }

    for bus_entry in &snapshot.bus_entries {
        let end = Point::new(
            bus_entry.position.x + bus_entry.size.0,
            bus_entry.position.y + bus_entry.size.1,
        );
        out.push(ItemBound {
            item: SelectedItem::new(bus_entry.uuid, SelectedKind::BusEntry),
            bbox: Aabb::new(bus_entry.position.x, bus_entry.position.y, end.x, end.y),
            anchor: Point::new(
                (bus_entry.position.x + end.x) * 0.5,
                (bus_entry.position.y + end.y) * 0.5,
            ),
        });
    }

    for junction in &snapshot.junctions {
        out.push(ItemBound {
            item: SelectedItem::new(junction.uuid, SelectedKind::Junction),
            bbox: Aabb::new(
                junction.position.x - 0.5,
                junction.position.y - 0.5,
                junction.position.x + 0.5,
                junction.position.y + 0.5,
            ),
            anchor: junction.position,
        });
    }

    for no_connect in &snapshot.no_connects {
        out.push(ItemBound {
            item: SelectedItem::new(no_connect.uuid, SelectedKind::NoConnect),
            bbox: Aabb::new(
                no_connect.position.x - 0.5,
                no_connect.position.y - 0.5,
                no_connect.position.x + 0.5,
                no_connect.position.y + 0.5,
            ),
            anchor: no_connect.position,
        });
    }

    for label in &snapshot.labels {
        out.push(ItemBound {
            item: SelectedItem::new(label.uuid, SelectedKind::Label),
            bbox: label_aabb(label),
            anchor: label.position,
        });
    }

    for note in &snapshot.text_notes {
        out.push(ItemBound {
            item: SelectedItem::new(note.uuid, SelectedKind::TextNote),
            bbox: note_aabb(note),
            anchor: note.position,
        });
    }

    for child in &snapshot.child_sheets {
        out.push(ItemBound {
            item: SelectedItem::new(child.uuid, SelectedKind::ChildSheet),
            bbox: Aabb::new(
                child.position.x,
                child.position.y,
                child.position.x + child.size.0,
                child.position.y + child.size.1,
            ),
            anchor: Point::new(
                child.position.x + child.size.0 * 0.5,
                child.position.y + child.size.1 * 0.5,
            ),
        });

        for pin in &child.pins {
            out.push(ItemBound {
                item: SelectedItem::new(pin.uuid, SelectedKind::SheetPin),
                bbox: Aabb::new(
                    pin.position.x - 0.8,
                    pin.position.y - 0.8,
                    pin.position.x + 0.8,
                    pin.position.y + 0.8,
                ),
                anchor: pin.position,
            });
        }
    }

    for drawing in &snapshot.drawings {
        let uuid = match drawing {
            SchDrawing::Line { uuid, .. }
            | SchDrawing::Rect { uuid, .. }
            | SchDrawing::Circle { uuid, .. }
            | SchDrawing::Arc { uuid, .. }
            | SchDrawing::Polyline { uuid, .. } => *uuid,
        };
        let bbox = drawing_aabb(drawing);
        out.push(ItemBound {
            item: SelectedItem::new(uuid, SelectedKind::Drawing),
            bbox,
            anchor: Point::new((bbox.min_x + bbox.max_x) * 0.5, (bbox.min_y + bbox.max_y) * 0.5),
        });
    }

    out
}

fn item_aabb(snapshot: &SchematicRenderSnapshot, item: &SelectedItem) -> Option<Aabb> {
    collect_item_bounds(snapshot)
        .into_iter()
        .find(|entry| entry.item == *item)
        .map(|entry| entry.bbox)
}

fn draw_wires(
    frame: &mut canvas::Frame,
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
    wire_color_overrides: Option<&HashMap<uuid::Uuid, ThemeColor>>,
) {
    for wire in &snapshot.wires {
        let p0 = transform.world_to_screen((wire.start.x, wire.start.y));
        let p1 = transform.world_to_screen((wire.end.x, wire.end.y));
        if !line_visible(p0, p1, bounds) {
            continue;
        }

        let base_color = wire_color_overrides
            .and_then(|map| map.get(&wire.uuid))
            .map(to_iced)
            .unwrap_or_else(|| to_iced(&colors.wire));
        let color = focus_color(base_color, focus_set, wire.uuid);
        let width = mm_to_px(
            wire.stroke_width
                .max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM),
            transform.scale,
        );
        let path = canvas::Path::line(p0, p1);
        frame.stroke(
            &path,
            canvas::Stroke::default().with_width(width).with_color(color),
        );
    }
}

fn draw_buses(
    frame: &mut canvas::Frame,
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
) {
    for bus in &snapshot.buses {
        let p0 = transform.world_to_screen((bus.start.x, bus.start.y));
        let p1 = transform.world_to_screen((bus.end.x, bus.end.y));
        if !line_visible(p0, p1, bounds) {
            continue;
        }
        let color = focus_color(to_iced(&colors.bus), focus_set, bus.uuid);
        let path = canvas::Path::line(p0, p1);
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_width(mm_to_px(
                    signex_types::schematic::SCHEMATIC_RENDER_BUS_STROKE_MM,
                    transform.scale,
                ))
                .with_color(color),
        );
    }
}

fn draw_junctions(
    frame: &mut canvas::Frame,
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
) {
    for junction in &snapshot.junctions {
        let center = transform.world_to_screen((junction.position.x, junction.position.y));
        if !point_visible(center, bounds, 6.0) {
            continue;
        }
        let color = focus_color(to_iced(&colors.junction), focus_set, junction.uuid);
        let radius_mm = (junction.diameter * 0.5)
            .max(signex_types::schematic::SCHEMATIC_RENDER_JUNCTION_MIN_RADIUS_MM);
        let circle = canvas::Path::circle(center, mm_to_px(radius_mm, transform.scale));
        frame.fill(&circle, color);
    }
}

fn draw_no_connects(
    frame: &mut canvas::Frame,
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
) {
    for item in &snapshot.no_connects {
        let center = transform.world_to_screen((item.position.x, item.position.y));
        if !point_visible(center, bounds, 10.0) {
            continue;
        }
        let color = focus_color(to_iced(&colors.body), focus_set, item.uuid);
        let len = mm_to_px(
            signex_types::schematic::SCHEMATIC_RENDER_NO_CONNECT_HALF_LEN_MM,
            transform.scale,
        )
        .max(signex_types::schematic::SCHEMATIC_RENDER_NO_CONNECT_MIN_HALF_LEN_PX);
        let a = canvas::Path::line(
            iced::Point::new(center.x - len, center.y - len),
            iced::Point::new(center.x + len, center.y + len),
        );
        let b = canvas::Path::line(
            iced::Point::new(center.x - len, center.y + len),
            iced::Point::new(center.x + len, center.y - len),
        );
        let stroke = canvas::Stroke::default()
            .with_width(stroke_px_at_zoom(
                signex_types::schematic::SCHEMATIC_RENDER_NO_CONNECT_STROKE_PX,
                transform.scale,
            ))
            .with_color(color);
        frame.stroke(&a, stroke);
        frame.stroke(&b, stroke);
    }
}

fn draw_symbols(
    frame: &mut canvas::Frame,
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
) {
    for symbol in &snapshot.symbols {
        let bbox = symbol_body_aabb(symbol);
        let min = transform.world_to_screen((bbox.min_x, bbox.min_y));
        let max = transform.world_to_screen((bbox.max_x, bbox.max_y));
        let rect_min = iced::Point::new(min.x.min(max.x), min.y.min(max.y));
        let size = iced::Size::new((max.x - min.x).abs(), (max.y - min.y).abs());
        if !rect_visible(rect_min, size, bounds) {
            continue;
        }

        let stroke_color = focus_color(to_iced(&colors.body), focus_set, symbol.uuid);
        let fill_color = focus_color(to_iced(&colors.body_fill), focus_set, symbol.uuid);
        let rect = canvas::Path::rectangle(rect_min, size);
        frame.fill(&rect, fill_color);
        frame.stroke(
            &rect,
            canvas::Stroke::default()
                .with_width(stroke_px_at_zoom(
                    signex_types::schematic::SCHEMATIC_RENDER_SYMBOL_BODY_STROKE_PX,
                    transform.scale,
                ))
                .with_color(stroke_color),
        );

        if !symbol.reference.is_empty() {
            let p = transform.world_to_screen((symbol.position.x, symbol.position.y - 3.5));
            draw_rotated_text(
                frame,
                &symbol.reference,
                p,
                symbol.rotation,
                text_size_px(1.05, transform.scale),
                stroke_color,
                HAlign::Center,
                VAlign::Bottom,
            );
        }
        if !symbol.value.is_empty() {
            let p = transform.world_to_screen((symbol.position.x, symbol.position.y + 3.6));
            draw_rotated_text(
                frame,
                &symbol.value,
                p,
                symbol.rotation,
                text_size_px(1.05, transform.scale),
                focus_color(to_iced(&colors.value), focus_set, symbol.uuid),
                HAlign::Center,
                VAlign::Top,
            );
        }
    }
}

fn draw_child_sheets(
    frame: &mut canvas::Frame,
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
) {
    for sheet in &snapshot.child_sheets {
        let min = transform.world_to_screen((sheet.position.x, sheet.position.y));
        let max = transform.world_to_screen((sheet.position.x + sheet.size.0, sheet.position.y + sheet.size.1));
        let rect_min = iced::Point::new(min.x.min(max.x), min.y.min(max.y));
        let size = iced::Size::new((max.x - min.x).abs(), (max.y - min.y).abs());
        if !rect_visible(rect_min, size, bounds) {
            continue;
        }
        let color = focus_color(to_iced(&colors.global_label), focus_set, sheet.uuid);
        let rect = canvas::Path::rectangle(rect_min, size);
        frame.stroke(
            &rect,
            canvas::Stroke::default()
                .with_width(stroke_px_at_zoom(
                    signex_types::schematic::SCHEMATIC_RENDER_CHILD_SHEET_STROKE_PX,
                    transform.scale,
                ))
                .with_color(color),
        );
        draw_rotated_text(
            frame,
            &sheet.name,
            iced::Point::new(rect_min.x + 6.0, rect_min.y + 6.0),
            0.0,
            text_size_px(1.05, transform.scale),
            color,
            HAlign::Left,
            VAlign::Top,
        );

        for pin in &sheet.pins {
            let center = transform.world_to_screen((pin.position.x, pin.position.y));
            let mark = canvas::Path::circle(
                center,
                signex_types::schematic::SCHEMATIC_RENDER_CHILD_SHEET_PIN_RADIUS_PX,
            );
            frame.fill(&mark, Color { a: 0.3, ..color });
        }
    }
}

fn draw_drawings(
    frame: &mut canvas::Frame,
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
) {
    for drawing in &snapshot.drawings {
        let uuid = match drawing {
            SchDrawing::Line { uuid, .. }
            | SchDrawing::Rect { uuid, .. }
            | SchDrawing::Circle { uuid, .. }
            | SchDrawing::Arc { uuid, .. }
            | SchDrawing::Polyline { uuid, .. } => *uuid,
        };
        let bbox = drawing_aabb(drawing);
        let min = transform.world_to_screen((bbox.min_x, bbox.min_y));
        let max = transform.world_to_screen((bbox.max_x, bbox.max_y));
        let rect_min = iced::Point::new(min.x.min(max.x), min.y.min(max.y));
        let size = iced::Size::new((max.x - min.x).abs(), (max.y - min.y).abs());
        if !rect_visible(rect_min, size, bounds) {
            continue;
        }

        let base_color = focus_color(to_iced(&colors.body), focus_set, uuid);

        match drawing {
            SchDrawing::Line {
                start,
                end,
                width,
                stroke_color,
                ..
            } => {
                let stroke = resolve_stroke_color(stroke_color, base_color);
                let path = canvas::Path::line(
                    transform.world_to_screen((start.x, start.y)),
                    transform.world_to_screen((end.x, end.y)),
                );
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_width(mm_to_px(
                            width.max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM),
                            transform.scale,
                        ))
                        .with_color(stroke),
                );
            }
            SchDrawing::Rect {
                start,
                end,
                width,
                fill,
                stroke_color,
                ..
            } => {
                let p0 = transform.world_to_screen((start.x, start.y));
                let p1 = transform.world_to_screen((end.x, end.y));
                let min = iced::Point::new(p0.x.min(p1.x), p0.y.min(p1.y));
                let size = iced::Size::new((p1.x - p0.x).abs(), (p1.y - p0.y).abs());
                let path = canvas::Path::rectangle(min, size);
                if let Some(fill_color) = fill_color_for(*fill, stroke_color, colors) {
                    frame.fill(&path, fill_color);
                }
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_width(mm_to_px(
                            width.max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM),
                            transform.scale,
                        ))
                        .with_color(resolve_stroke_color(stroke_color, base_color)),
                );
            }
            SchDrawing::Circle {
                center,
                radius,
                width,
                fill,
                stroke_color,
                ..
            } => {
                let c = transform.world_to_screen((center.x, center.y));
                let path = canvas::Path::circle(
                    c,
                    mm_to_px(*radius, transform.scale)
                        .max(signex_types::schematic::SCHEMATIC_RENDER_DRAWING_MIN_CIRCLE_RADIUS_PX),
                );
                if let Some(fill_color) = fill_color_for(*fill, stroke_color, colors) {
                    frame.fill(&path, fill_color);
                }
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_width(mm_to_px(
                            width.max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM),
                            transform.scale,
                        ))
                        .with_color(resolve_stroke_color(stroke_color, base_color)),
                );
            }
            SchDrawing::Arc {
                start,
                mid,
                end,
                width,
                stroke_color,
                ..
            } => {
                if let Some((cx, cy, r)) = circumcircle(
                    (start.x, start.y),
                    (mid.x, mid.y),
                    (end.x, end.y),
                ) {
                    let center = transform.world_to_screen((cx, cy));
                    let a0 = (start.y - cy).atan2(start.x - cx);
                    let am = (mid.y - cy).atan2(mid.x - cx);
                    let a1 = (end.y - cy).atan2(end.x - cx);
                    let (start_angle, end_angle) = if arc_sweeps_through_mid(a0, am, a1) {
                        (a0, a1)
                    } else {
                        (a1, a0)
                    };
                    let path = canvas::Path::new(|builder| {
                        builder.arc(canvas::path::Arc {
                            center,
                            radius: mm_to_px(r, transform.scale)
                                .max(signex_types::schematic::SCHEMATIC_RENDER_DRAWING_MIN_ARC_RADIUS_PX),
                            start_angle: iced::Radians(start_angle as f32),
                            end_angle: iced::Radians(end_angle as f32),
                        });
                    });
                    frame.stroke(
                        &path,
                        canvas::Stroke::default()
                            .with_width(mm_to_px(
                                width.max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM),
                                transform.scale,
                            ))
                            .with_color(resolve_stroke_color(stroke_color, base_color)),
                    );
                } else {
                    let p0 = transform.world_to_screen((start.x, start.y));
                    let p1 = transform.world_to_screen((mid.x, mid.y));
                    let p2 = transform.world_to_screen((end.x, end.y));
                    let path = canvas::Path::new(|builder| {
                        builder.move_to(p0);
                        builder.line_to(p1);
                        builder.line_to(p2);
                    });
                    frame.stroke(
                        &path,
                        canvas::Stroke::default()
                            .with_width(mm_to_px(
                                width.max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM),
                                transform.scale,
                            ))
                            .with_color(resolve_stroke_color(stroke_color, base_color)),
                    );
                }
            }
            SchDrawing::Polyline {
                points,
                width,
                fill,
                stroke_color,
                ..
            } => {
                if points.len() < 2 {
                    continue;
                }
                let path = canvas::Path::new(|builder| {
                    let mut it = points.iter();
                    if let Some(first) = it.next() {
                        builder.move_to(transform.world_to_screen((first.x, first.y)));
                    }
                    for point in it {
                        builder.line_to(transform.world_to_screen((point.x, point.y)));
                    }
                    if !matches!(fill, FillType::None) {
                        builder.close();
                    }
                });
                if let Some(fill_color) = fill_color_for(*fill, stroke_color, colors) {
                    frame.fill(&path, fill_color);
                }
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_width(mm_to_px(
                            width.max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_MM),
                            transform.scale,
                        ))
                        .with_color(resolve_stroke_color(stroke_color, base_color)),
                );
            }
        }
    }
}

fn draw_labels(
    frame: &mut canvas::Frame,
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
) {
    for label in &snapshot.labels {
        let screen = transform.world_to_screen((label.position.x, label.position.y));
        if !point_visible(screen, bounds, 22.0) {
            continue;
        }
        let color = focus_color(label_color(label, colors), focus_set, label.uuid);
        draw_label_impl(frame, label, transform, color, None);
    }
}

fn draw_text_notes(
    frame: &mut canvas::Frame,
    snapshot: &SchematicRenderSnapshot,
    transform: &ScreenTransform,
    colors: &CanvasColors,
    bounds: Rectangle,
    focus_set: Option<&HashSet<uuid::Uuid>>,
) {
    for note in &snapshot.text_notes {
        let pos = transform.world_to_screen((note.position.x, note.position.y));
        if !point_visible(pos, bounds, 28.0) {
            continue;
        }
        let color = focus_color(to_iced(&colors.value), focus_set, note.uuid);
        let size_px = text_size_px(note.font_size, transform.scale);
        draw_rotated_text(
            frame,
            &note.text,
            pos,
            note.rotation,
            size_px,
            color,
            note.justify_h,
            note.justify_v,
        );
    }
}

fn draw_label_impl(
    frame: &mut canvas::Frame,
    label: &Label,
    transform: &ScreenTransform,
    stroke_color: Color,
    fill_override: Option<Color>,
) {
    let pos = transform.world_to_screen((label.position.x, label.position.y));
    let size_px = text_size_px(label.font_size, transform.scale);

    if matches!(label.label_type, LabelType::Global | LabelType::Hierarchical) {
        let glyph_w = (label.text.chars().count().max(1) as f32) * (size_px * 0.58);
        let half_h = size_px * 0.62;
        let point = size_px * 0.52;
        let w = glyph_w + size_px * 0.65;
        let local = [
            (-point, 0.0_f32),
            (0.0, -half_h),
            (w, -half_h),
            (w, half_h),
            (0.0, half_h),
        ];

        let rad = (label.rotation as f32).to_radians();
        let cos = rad.cos();
        let sin = rad.sin();
        let verts: Vec<iced::Point> = local
            .iter()
            .map(|(x, y)| iced::Point::new(pos.x + x * cos - y * sin, pos.y + x * sin + y * cos))
            .collect();

        let path = canvas::Path::new(|builder| {
            builder.move_to(verts[0]);
            for vertex in &verts[1..] {
                builder.line_to(*vertex);
            }
            builder.close();
        });
        if let Some(fill_color) = fill_override {
            frame.fill(&path, fill_color);
        }
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_width(stroke_px_at_zoom(
                    signex_types::schematic::SCHEMATIC_RENDER_LABEL_GLYPH_STROKE_PX,
                    transform.scale,
                ))
                .with_color(stroke_color),
        );

        draw_rotated_text(
            frame,
            &label.text,
            pos,
            label.rotation,
            size_px,
            stroke_color,
            HAlign::Center,
            VAlign::Center,
        );
    } else {
        draw_rotated_text(
            frame,
            &label.text,
            pos,
            label.rotation,
            size_px,
            stroke_color,
            label.justify,
            label.justify_v,
        );
    }
}

fn label_color(label: &Label, colors: &CanvasColors) -> Color {
    match label.label_type {
        LabelType::Net => to_iced(&colors.net_label),
        LabelType::Global => to_iced(&colors.global_label),
        LabelType::Hierarchical => to_iced(&colors.hier_label),
        LabelType::Power => to_iced(&colors.power),
    }
}

fn symbol_body_aabb(symbol: &Symbol) -> Aabb {
    let half_w = 4.0;
    let half_h = 2.8;
    Aabb::new(
        symbol.position.x - half_w,
        symbol.position.y - half_h,
        symbol.position.x + half_w,
        symbol.position.y + half_h,
    )
}

fn text_prop_aabb(symbol: &Symbol, text: &str, prop: &TextProp) -> Aabb {
    let chars = text.chars().count().max(1) as f64;
    let h = prop.font_size.max(signex_types::schematic::SCHEMATIC_TEXT_MM);
    let w = h * 0.6 * chars;
    let (x, y) = instance_transform(symbol, &prop.position);
    Aabb::new(x - w * 0.5, y - h * 0.5, x + w * 0.5, y + h * 0.5)
}

fn note_aabb(note: &TextNote) -> Aabb {
    let h = note.font_size.max(signex_types::schematic::SCHEMATIC_TEXT_MM);
    let w = h * 0.6 * note.text.chars().count().max(1) as f64;
    Aabb::new(
        note.position.x - w * 0.5,
        note.position.y - h * 0.5,
        note.position.x + w * 0.5,
        note.position.y + h * 0.5,
    )
}

fn label_aabb(label: &Label) -> Aabb {
    let h = label.font_size.max(signex_types::schematic::SCHEMATIC_TEXT_MM);
    let mut w = h * 0.6 * label.text.chars().count().max(1) as f64;
    if matches!(label.label_type, LabelType::Global | LabelType::Hierarchical) {
        w += h * 1.2;
    }
    Aabb::new(
        label.position.x - w * 0.5,
        label.position.y - h,
        label.position.x + w * 0.5,
        label.position.y + h,
    )
}

fn drawing_aabb(drawing: &SchDrawing) -> Aabb {
    match drawing {
        SchDrawing::Line { start, end, .. } | SchDrawing::Rect { start, end, .. } => {
            Aabb::new(start.x, start.y, end.x, end.y)
        }
        SchDrawing::Circle { center, radius, .. } => Aabb::new(
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        ),
        SchDrawing::Arc { start, mid, end, .. } => {
            if let Some((cx, cy, r)) = circumcircle((start.x, start.y), (mid.x, mid.y), (end.x, end.y)) {
                Aabb::new(cx - r, cy - r, cx + r, cy + r)
            } else {
                Aabb::new(start.x, start.y, end.x, end.y).union(&Aabb::new(mid.x, mid.y, mid.x, mid.y))
            }
        }
        SchDrawing::Polyline { points, .. } => {
            if let Some(first) = points.first() {
                let mut bbox = Aabb::new(first.x, first.y, first.x, first.y);
                for point in points.iter().skip(1) {
                    bbox = bbox.union(&Aabb::new(point.x, point.y, point.x, point.y));
                }
                bbox
            } else {
                Aabb::new(0.0, 0.0, 0.0, 0.0)
            }
        }
    }
}

fn point_to_segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len2 = dx * dx + dy * dy;
    if len2 <= f64::EPSILON {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0);
    let px = a.x + t * dx;
    let py = a.y + t * dy;
    ((p.x - px).powi(2) + (p.y - py).powi(2)).sqrt()
}

fn point_in_polygon(point: (f64, f64), polygon: &[(f64, f64)]) -> bool {
    let (x, y) = point;
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let (xi, yi) = polygon[i];
        let (xj, yj) = polygon[j];
        let intersects = ((yi > y) != (yj > y))
            && (x < (xj - xi) * (y - yi) / ((yj - yi).abs().max(1e-9) * (if yj >= yi { 1.0 } else { -1.0 })) + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn text_size_px(font_size_mm: f64, zoom: f32) -> f32 {
    let size_mm = font_size_mm.max(signex_types::schematic::SCHEMATIC_TEXT_MM);
    let em_mm = size_mm / 0.72;
    (em_mm * zoom as f64).clamp(6.0, 64.0) as f32
}

fn mm_to_px(mm: f64, scale: f32) -> f32 {
    (mm.max(0.0) as f32 * scale).max(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_PX)
}

fn stroke_px_at_zoom(base_width_px_at_100: f32, scale: f32) -> f32 {
    let zoom_factor = (scale / signex_types::schematic::SCHEMATIC_ZOOM_100_SCALE).max(0.0);
    let scaled = base_width_px_at_100 * zoom_factor;
    let max_stroke =
        base_width_px_at_100 * signex_types::schematic::SCHEMATIC_RENDER_STROKE_MAX_SCALE_MULTIPLIER;
    scaled.clamp(signex_types::schematic::SCHEMATIC_RENDER_MIN_STROKE_PX, max_stroke)
}

fn to_iced(color: &ThemeColor) -> Color {
    Color::from_rgba8(color.r, color.g, color.b, color.a as f32 / 255.0)
}

fn focus_color(base: Color, focus_set: Option<&HashSet<uuid::Uuid>>, uuid: uuid::Uuid) -> Color {
    if let Some(set) = focus_set
        && !set.contains(&uuid)
    {
        return Color {
            a: (base.a * 0.26).clamp(0.0, 1.0),
            ..base
        };
    }
    base
}

fn aabb_overlaps(a: &Aabb, b: &Aabb) -> bool {
    !(a.max_x < b.min_x || a.min_x > b.max_x || a.max_y < b.min_y || a.min_y > b.max_y)
}

fn line_visible(p0: iced::Point, p1: iced::Point, bounds: Rectangle) -> bool {
    let min_x = p0.x.min(p1.x);
    let max_x = p0.x.max(p1.x);
    let min_y = p0.y.min(p1.y);
    let max_y = p0.y.max(p1.y);
    !(max_x < -8.0
        || max_y < -8.0
        || min_x > bounds.width + 8.0
        || min_y > bounds.height + 8.0)
}

fn rect_visible(min: iced::Point, size: iced::Size, bounds: Rectangle) -> bool {
    !(min.x + size.width < -8.0
        || min.y + size.height < -8.0
        || min.x > bounds.width + 8.0
        || min.y > bounds.height + 8.0)
}

fn point_visible(p: iced::Point, bounds: Rectangle, pad: f32) -> bool {
    p.x >= -pad && p.y >= -pad && p.x <= bounds.width + pad && p.y <= bounds.height + pad
}

fn resolve_stroke_color(stroke_color: &Option<signex_types::schematic::StrokeColor>, fallback: Color) -> Color {
    stroke_color
        .map(|color| Color::from_rgba8(color.r, color.g, color.b, color.a as f32 / 255.0))
        .unwrap_or(fallback)
}

fn fill_color_for(
    fill: FillType,
    stroke_color: &Option<signex_types::schematic::StrokeColor>,
    colors: &CanvasColors,
) -> Option<Color> {
    match fill {
        FillType::None => None,
        FillType::Outline => Some(resolve_stroke_color(stroke_color, to_iced(&colors.body))),
        FillType::Background => Some(to_iced(&colors.body_fill)),
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_rotated_text(
    frame: &mut canvas::Frame,
    text: &str,
    position: iced::Point,
    rotation_deg: f64,
    size_px: f32,
    color: Color,
    h_align: HAlign,
    v_align: VAlign,
) {
    if text.is_empty() {
        return;
    }

    let align_x = match h_align {
        HAlign::Left => advanced_text::Alignment::Left,
        HAlign::Center => advanced_text::Alignment::Center,
        HAlign::Right => advanced_text::Alignment::Right,
    };
    let align_y = match v_align {
        VAlign::Top => alignment::Vertical::Top,
        VAlign::Center => alignment::Vertical::Center,
        VAlign::Bottom => alignment::Vertical::Bottom,
    };

    let base = canvas::Text {
        content: text.to_string(),
        position: iced::Point::ORIGIN,
        color,
        size: iced::Pixels(size_px),
        align_x,
        align_y,
        ..canvas::Text::default()
    };

    let rad = rotation_deg.to_radians() as f32;
    if rad.abs() < f32::EPSILON {
        let mut placed = base;
        placed.position = position;
        frame.fill_text(placed);
        return;
    }

    frame.with_save(|inner| {
        inner.translate(Vector::new(position.x, position.y));
        inner.rotate(iced::Radians(rad));
        inner.fill_text(base);
    });
}

fn circumcircle(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> Option<(f64, f64, f64)> {
    let (ax, ay) = a;
    let (bx, by) = b;
    let (cx, cy) = c;
    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() < 1e-12 {
        return None;
    }
    let ux = ((ax * ax + ay * ay) * (by - cy)
        + (bx * bx + by * by) * (cy - ay)
        + (cx * cx + cy * cy) * (ay - by))
        / d;
    let uy = ((ax * ax + ay * ay) * (cx - bx)
        + (bx * bx + by * by) * (ax - cx)
        + (cx * cx + cy * cy) * (bx - ax))
        / d;
    let radius = ((ax - ux).powi(2) + (ay - uy).powi(2)).sqrt();
    Some((ux, uy, radius))
}

fn arc_sweeps_through_mid(a0: f64, am: f64, a1: f64) -> bool {
    let two_pi = 2.0 * std::f64::consts::PI;
    let normalize = |a: f64| (a - a0).rem_euclid(two_pi);
    normalize(am) < normalize(a1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_snapshot() -> SchematicRenderSnapshot {
        SchematicRenderSnapshot {
            uuid: uuid::Uuid::nil(),
            version: 1,
            generator: "signex-test".into(),
            generator_version: "0.0.0".into(),
            paper_size: "A4".into(),
            root_sheet_page: "1".into(),
            symbols: Vec::new(),
            wires: Vec::new(),
            junctions: Vec::new(),
            labels: Vec::new(),
            child_sheets: Vec::new(),
            no_connects: Vec::new(),
            text_notes: Vec::new(),
            buses: Vec::new(),
            bus_entries: Vec::new(),
            drawings: Vec::new(),
            no_erc_directives: Vec::new(),
            title_block: HashMap::new(),
            lib_symbols: HashMap::new(),
        }
    }

    #[test]
    fn hit_test_wire_uses_segment_tolerance() {
        let mut snapshot = empty_snapshot();
        let wire_uuid = uuid::Uuid::new_v4();
        snapshot.wires.push(signex_types::schematic::Wire {
            uuid: wire_uuid,
            start: Point::new(0.0, 0.0),
            end: Point::new(10.0, 0.0),
            stroke_width: 0.15,
        });

        let hit = hit_test::hit_test(&snapshot, 5.0, 0.08);
        assert_eq!(hit, Some(SelectedItem::new(wire_uuid, SelectedKind::Wire)));
    }

    #[test]
    fn hit_test_rect_mode_distinguishes_inside_and_touching() {
        let mut snapshot = empty_snapshot();
        let wire_uuid = uuid::Uuid::new_v4();
        snapshot.wires.push(signex_types::schematic::Wire {
            uuid: wire_uuid,
            start: Point::new(-4.0, 0.0),
            end: Point::new(4.0, 0.0),
            stroke_width: 0.15,
        });

        let rect = Aabb::new(0.0, -0.2, 2.0, 0.2);
        let inside = hit_test::hit_test_rect_mode(&snapshot, &rect, hit_test::SelectionMode::Inside);
        let touching =
            hit_test::hit_test_rect_mode(&snapshot, &rect, hit_test::SelectionMode::Touching);

        assert!(!inside.contains(&SelectedItem::new(wire_uuid, SelectedKind::Wire)));
        assert!(touching.contains(&SelectedItem::new(wire_uuid, SelectedKind::Wire)));
    }

    #[test]
    fn hit_test_polygon_selects_wire_and_label_by_anchor() {
        let mut snapshot = empty_snapshot();
        let wire_uuid = uuid::Uuid::new_v4();
        let label_uuid = uuid::Uuid::new_v4();

        snapshot.wires.push(signex_types::schematic::Wire {
            uuid: wire_uuid,
            start: Point::new(1.0, 1.0),
            end: Point::new(9.0, 1.0),
            stroke_width: 0.15,
        });
        snapshot.labels.push(Label {
            uuid: label_uuid,
            text: "NET_MAIN".into(),
            position: Point::new(4.0, 4.0),
            rotation: 0.0,
            label_type: LabelType::Net,
            shape: String::new(),
            font_size: signex_types::schematic::SCHEMATIC_TEXT_MM,
            justify: HAlign::Left,
            justify_v: VAlign::Bottom,
        });

        let polygon = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 8.0), (0.0, 8.0)];
        let hits = hit_test::hit_test_polygon(&snapshot, &polygon);

        assert!(hits.contains(&SelectedItem::new(wire_uuid, SelectedKind::Wire)));
        assert!(hits.contains(&SelectedItem::new(label_uuid, SelectedKind::Label)));
    }
}
