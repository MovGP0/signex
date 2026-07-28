//! Placed schematic-sheet elements and the sheet container.

use super::{
    Deserialize, FillType, HAlign, HashMap, LabelType, LibSymbol, Point, Serialize, SheetInstance,
    Symbol, Uuid, VAlign,
};

// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wire {
    pub uuid: Uuid,
    pub start: Point,
    pub end: Point,
    /// Stroke width in mm. 0.0 = use schematic default (~0.15mm).
    #[serde(default)]
    pub stroke_width: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Junction {
    pub uuid: Uuid,
    pub position: Point,
    /// 0.0 means use the theme default size.
    #[serde(default)]
    pub diameter: f64,
    /// `true` when this dot was minted by the wire-junction autoplacer
    /// (`reconcile_wire_junctions` / `junctions_for_wire` in
    /// `signex-engine`) rather than placed by the user. Minted dots are
    /// re-validated on every wire-geometry command and removed once no
    /// wire meeting justifies them any more; a user-placed dot — which
    /// includes every junction in a `.snxsch` written before this field
    /// existed, via the `#[serde(default)]` below — is left alone even
    /// if it no longer sits on a wire (issue #422).
    #[serde(default)]
    pub minted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub uuid: Uuid,
    pub text: String,
    pub position: Point,
    #[serde(default)]
    pub rotation: f64,
    pub label_type: LabelType,
    #[serde(default)]
    pub shape: String,
    #[serde(default)]
    pub font_size: f64,
    #[serde(default)]
    pub justify: HAlign,
    #[serde(default = "default_label_v_align")]
    pub justify_v: VAlign,
}

const fn default_label_v_align() -> VAlign {
    VAlign::Bottom
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoConnect {
    pub uuid: Uuid,
    pub position: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextNote {
    pub uuid: Uuid,
    pub text: String,
    pub position: Point,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub font_size: f64,
    #[serde(default)]
    pub justify_h: HAlign,
    #[serde(default)]
    pub justify_v: VAlign,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bus {
    pub uuid: Uuid,
    pub start: Point,
    pub end: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusEntry {
    pub uuid: Uuid,
    pub position: Point,
    pub size: (f64, f64),
}

// ---------------------------------------------------------------------------
// Hierarchical sheets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetPin {
    pub uuid: Uuid,
    pub name: String,
    #[serde(default)]
    pub direction: String,
    pub position: Point,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub auto_generated: bool,
    #[serde(default)]
    pub user_moved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildSheet {
    pub uuid: Uuid,
    pub name: String,
    pub filename: String,
    pub position: Point,
    pub size: (f64, f64),
    #[serde(default)]
    pub stroke_width: f64,
    #[serde(default)]
    pub fill: FillType,
    /// Optional outline colour parsed from `(stroke (color r g b a))`.
    /// `None` means "use the renderer's default for the active style".
    #[serde(default)]
    pub stroke_color: Option<StrokeColor>,
    /// Optional body fill colour parsed from `(fill (color r g b a))`.
    /// `None` means "use the renderer's default for the active style".
    #[serde(default)]
    pub fill_color: Option<StrokeColor>,
    #[serde(default)]
    pub fields_autoplaced: bool,
    #[serde(default)]
    pub pins: Vec<SheetPin>,
    #[serde(default)]
    pub instances: Vec<SheetInstance>,
}

// ---------------------------------------------------------------------------
// Schematic drawing primitives
// ---------------------------------------------------------------------------

/// Optional RGBA override for an individual `SchDrawing`. `None` means
///
/// "use the theme's default drawing colour" — the renderer falls back to
/// `CanvasColors.outline`. Stored per-drawing so users can recolour
/// individual shapes without disturbing the sheet theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrokeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SchDrawing {
    Line {
        uuid: Uuid,
        start: Point,
        end: Point,
        #[serde(default)]
        width: f64,
        #[serde(default)]
        stroke_color: Option<StrokeColor>,
    },
    Rect {
        uuid: Uuid,
        start: Point,
        end: Point,
        #[serde(default)]
        width: f64,
        #[serde(default)]
        fill: FillType,
        #[serde(default)]
        stroke_color: Option<StrokeColor>,
    },
    Circle {
        uuid: Uuid,
        center: Point,
        radius: f64,
        #[serde(default)]
        width: f64,
        #[serde(default)]
        fill: FillType,
        #[serde(default)]
        stroke_color: Option<StrokeColor>,
    },
    Arc {
        uuid: Uuid,
        start: Point,
        mid: Point,
        end: Point,
        #[serde(default)]
        width: f64,
        #[serde(default)]
        fill: FillType,
        #[serde(default)]
        stroke_color: Option<StrokeColor>,
    },
    Polyline {
        uuid: Uuid,
        points: Vec<Point>,
        #[serde(default)]
        width: f64,
        #[serde(default)]
        fill: FillType,
        #[serde(default)]
        stroke_color: Option<StrokeColor>,
    },
}

// ---------------------------------------------------------------------------
// Top-level sheet
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicSheet {
    pub uuid: Uuid,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub generator: String,
    #[serde(default)]
    pub generator_version: String,
    #[serde(default)]
    pub paper_size: String,
    #[serde(default = "default_root_sheet_page")]
    pub root_sheet_page: String,
    #[serde(default)]
    pub symbols: Vec<Symbol>,
    #[serde(default)]
    pub wires: Vec<Wire>,
    #[serde(default)]
    pub junctions: Vec<Junction>,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub child_sheets: Vec<ChildSheet>,
    #[serde(default)]
    pub no_connects: Vec<NoConnect>,
    #[serde(default)]
    pub text_notes: Vec<TextNote>,
    #[serde(default)]
    pub buses: Vec<Bus>,
    #[serde(default)]
    pub bus_entries: Vec<BusEntry>,
    #[serde(default)]
    pub drawings: Vec<SchDrawing>,
    #[serde(default)]
    pub no_erc_directives: Vec<NoConnect>,
    #[serde(default)]
    pub title_block: HashMap<String, String>,
    #[serde(default)]
    pub lib_symbols: HashMap<String, LibSymbol>,
}

fn default_root_sheet_page() -> String {
    "1".to_string()
}

// ---------------------------------------------------------------------------
// Selection -- identifies what the user has selected on the canvas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectedKind {
    Symbol,
    Wire,
    Bus,
    BusEntry,
    Junction,
    NoConnect,
    Label,
    /// Hierarchical sheet pin rendered on a child-sheet symbol.
    SheetPin,
    TextNote,
    ChildSheet,
    Drawing,
    /// Symbol reference field ("C39", "R1", …). UUID = symbol UUID.
    SymbolRefField,
    /// Symbol value field ("100n", "10k", …). UUID = symbol UUID.
    SymbolValField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SelectedItem {
    pub uuid: Uuid,
    pub kind: SelectedKind,
}

impl SelectedItem {
    #[must_use]
    pub const fn new(uuid: Uuid, kind: SelectedKind) -> Self {
        Self { uuid, kind }
    }
}

// ---------------------------------------------------------------------------
// Bounding box helpers
// ---------------------------------------------------------------------------

/// Axis-aligned bounding box in world (mm) coordinates.
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Aabb {
    #[must_use]
    pub const fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self {
            min_x: x1.min(x2),
            min_y: y1.min(y2),
            max_x: x1.max(x2),
            max_y: y1.max(y2),
        }
    }

    #[must_use]
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    #[must_use]
    pub fn expand(&self, margin: f64) -> Self {
        Self {
            min_x: self.min_x - margin,
            min_y: self.min_y - margin,
            max_x: self.max_x + margin,
            max_y: self.max_y + margin,
        }
    }

    #[must_use]
    pub const fn union(&self, other: &Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    #[must_use]
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    #[must_use]
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }
}

impl SchematicSheet {
    /// Compute the bounding box of all elements in the sheet.
    #[must_use]
    pub fn content_bounds(&self) -> Option<Aabb> {
        let mut aabb: Option<Aabb> = None;

        let mut extend = |x: f64, y: f64| {
            aabb = Some(aabb.map_or_else(
                || Aabb::new(x, y, x, y),
                |a| Aabb {
                    min_x: a.min_x.min(x),
                    min_y: a.min_y.min(y),
                    max_x: a.max_x.max(x),
                    max_y: a.max_y.max(y),
                },
            ));
        };

        for s in &self.symbols {
            extend(s.position.x, s.position.y);
        }
        for w in &self.wires {
            extend(w.start.x, w.start.y);
            extend(w.end.x, w.end.y);
        }
        for b in &self.buses {
            extend(b.start.x, b.start.y);
            extend(b.end.x, b.end.y);
        }
        for j in &self.junctions {
            extend(j.position.x, j.position.y);
        }
        for l in &self.labels {
            extend(l.position.x, l.position.y);
        }
        for n in &self.no_connects {
            extend(n.position.x, n.position.y);
        }
        for t in &self.text_notes {
            extend(t.position.x, t.position.y);
        }
        for c in &self.child_sheets {
            extend(c.position.x, c.position.y);
            extend(c.position.x + c.size.0, c.position.y + c.size.1);
        }

        // Add margin around content
        aabb.map(|a| a.expand(10.0))
    }
}

/// Distance from a point to a line segment.
#[must_use]
pub fn point_to_segment_dist(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dy.mul_add(dy, dx * dx);
    if len_sq < 1e-12 {
        return (px - ax).hypot(py - ay);
    }
    let t = (py - ay).mul_add(dy, (px - ax) * dx) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = ax + t * dx;
    let proj_y = ay + t * dy;
    (px - proj_x).hypot(py - proj_y)
}
