use serde::{Deserialize, Serialize};

use crate::id::SketchEntityId;
use crate::plane::PlaneId;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub id: SketchEntityId,
    pub plane: PlaneId,
    #[serde(default)]
    pub construction: bool,
    #[serde(flatten)]
    pub kind: EntityKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "PascalCase")]
pub enum EntityKind {
    /// Point in plane-local coordinates (mm).
    Point { x: f64, y: f64 },

    /// Line — both endpoints reference Point entities by ID.
    Line {
        start: SketchEntityId,
        end: SketchEntityId,
    },

    /// Arc — center is a Point, start/end are Points.
    /// `sweep_ccw = true` means CCW from start to end.
    Arc {
        center: SketchEntityId,
        start: SketchEntityId,
        end: SketchEntityId,
        #[serde(default = "default_sweep_ccw")]
        sweep_ccw: bool,
    },

    /// Circle — center is a Point, radius is a literal.
    /// (Parametric radius is expressed via a Distance constraint
    /// to a construction Point — see SKETCH_MODE_PLAN.md §
    /// Constraints.)
    Circle {
        center: SketchEntityId,
        radius: f64,
    },
}

fn default_sweep_ccw() -> bool {
    true
}

impl Entity {
    /// Point endpoints reachable from this entity. Used by the
    /// solver to discover entity → state-vector mapping.
    pub fn point_refs(&self) -> Vec<SketchEntityId> {
        match self.kind {
            EntityKind::Point { .. } => vec![self.id],
            EntityKind::Line { start, end } => vec![start, end],
            EntityKind::Arc {
                center, start, end, ..
            } => vec![center, start, end],
            EntityKind::Circle { center, .. } => vec![center],
        }
    }
}
