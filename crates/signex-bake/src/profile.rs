//! Closed-profile walker — given a starting Line entity, trace a
//! connected loop of edge entities through shared endpoint Points and
//! emit the boundary as a Polygon (Vec of `[x_mm, y_mm]` vertices).
//!
//! Used by the v0.14 silk / courtyard / mask / pour / keepout /
//! cutout / 3D-extrude bake modules to convert sketch profiles into
//! baked library polygons.
//!
//! v0.14 scope:
//! - Lines only. Arcs in a profile cause [`TraceError::ArcInProfile`]
//!   and the bake module surfaces a "v0.14.1 feature" warning.
//! - Construction entities are skipped silently — they're solver
//!   scaffolding and never participate in the baked geometry.
//! - Branching topology (a vertex with 3+ incident edges) returns
//!   [`TraceError::Branching`]; the bake skips with a warning.
//!
//! Cleanroom: traversal is a textbook depth-first walk over the
//! endpoint-incidence graph. No third-party CAD-tooling source
//! consulted.

use std::collections::{HashMap, HashSet};

use signex_sketch::entity::{Entity, EntityKind};
use signex_sketch::id::SketchEntityId;
use signex_sketch::sketch::SketchData;
use signex_sketch::solver::state::point_xy;
use signex_sketch::solver::FullSolveOutput;

/// Trace failure modes — the bake site decides whether to warn or
/// error per-attr.
#[derive(Clone, Debug, PartialEq)]
pub enum TraceError {
    /// `start` is not a Line / Arc / Circle in the sketch (or doesn't
    /// exist at all).
    NotAnEdge,
    /// One of the trace's endpoints couldn't be resolved to a position
    /// — usually because the endpoint Point isn't in the sketch.
    MissingEndpoint(SketchEntityId),
    /// Trace ran off the open end of a chain (the next endpoint has
    /// no continuing edge).
    OpenChain,
    /// A vertex has 3+ non-construction incident edges — ambiguous
    /// continuation.
    Branching,
    /// Profile contains an Arc — arc tessellation lands in v0.14.1.
    ArcInProfile,
    /// Profile contains a Circle — Circle is a closed primitive on
    /// its own; the bake module should special-case it without going
    /// through the walker.
    CircleInProfile,
    /// Walker exceeded a sanity cap on iterations (broken topology).
    Runaway,
}

/// Result of a trace: either a closed polygon (vertices in mm,
/// CCW or CW depending on starting direction) or a [`TraceError`].
pub type TraceResult = Result<Vec<[f64; 2]>, TraceError>;

/// Trace a closed boundary starting at `start`.
///
/// Algorithm:
/// 1. Build endpoint → list-of-edge adjacency over non-construction
///    Lines (Arcs / Circles are rejected with the matching error).
/// 2. Pick an arbitrary endpoint of `start` as the loop anchor;
///    push its position.
/// 3. Walk: from the current endpoint, find the unique non-visited
///    incident edge (excluding the entity we just came from). If
///    there's exactly one, advance; otherwise return Branching /
///    OpenChain.
/// 4. Loop closes when the next endpoint equals the anchor.
pub fn trace_closed_profile(
    sketch: &SketchData,
    solve: &FullSolveOutput,
    start: SketchEntityId,
) -> TraceResult {
    // Reject Circle / Arc starts up front (they need their own bake).
    let start_entity = sketch
        .entities
        .iter()
        .find(|e| e.id == start)
        .ok_or(TraceError::NotAnEdge)?;
    match start_entity.kind {
        EntityKind::Line { .. } => {}
        EntityKind::Arc { .. } => return Err(TraceError::ArcInProfile),
        EntityKind::Circle { .. } => return Err(TraceError::CircleInProfile),
        EntityKind::Point { .. } => return Err(TraceError::NotAnEdge),
    }

    // Reject if any non-construction edge in the sketch is an Arc.
    // This conservative check catches the case where the loop walks
    // into an Arc partway through.
    for e in &sketch.entities {
        if e.construction {
            continue;
        }
        if matches!(e.kind, EntityKind::Arc { .. }) {
            return Err(TraceError::ArcInProfile);
        }
    }

    let edges = collect_lines(sketch);
    let adj = build_adjacency(&edges);

    let (start_a, start_b) = line_endpoints(start_entity).ok_or(TraceError::NotAnEdge)?;

    let pos_a =
        point_xy(start_a, &solve.result.state, &solve.result.index, sketch)
            .ok_or(TraceError::MissingEndpoint(start_a))?;
    let pos_b =
        point_xy(start_b, &solve.result.state, &solve.result.index, sketch)
            .ok_or(TraceError::MissingEndpoint(start_b))?;

    let mut vertices: Vec<[f64; 2]> = vec![[pos_a.0, pos_a.1], [pos_b.0, pos_b.1]];
    let mut visited: HashSet<SketchEntityId> = HashSet::new();
    visited.insert(start);
    let mut current_endpoint = start_b;
    let max_steps = edges.len() + 2;

    for _ in 0..max_steps {
        let candidates: Vec<SketchEntityId> = adj
            .get(&current_endpoint)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .iter()
            .copied()
            .filter(|eid| !visited.contains(eid))
            .collect();

        match candidates.len() {
            0 => return Err(TraceError::OpenChain),
            1 => {}
            _ => return Err(TraceError::Branching),
        }

        let next_id = candidates[0];
        let next_entity = &edges[&next_id];
        let (next_a, next_b) =
            line_endpoints(next_entity).ok_or(TraceError::NotAnEdge)?;
        let other = if next_a == current_endpoint {
            next_b
        } else {
            next_a
        };

        if other == start_a {
            // Closed loop.
            return Ok(vertices);
        }

        let pos = point_xy(other, &solve.result.state, &solve.result.index, sketch)
            .ok_or(TraceError::MissingEndpoint(other))?;
        vertices.push([pos.0, pos.1]);
        visited.insert(next_id);
        current_endpoint = other;
    }

    Err(TraceError::Runaway)
}

fn collect_lines(sketch: &SketchData) -> HashMap<SketchEntityId, &Entity> {
    let mut out = HashMap::new();
    for e in &sketch.entities {
        if e.construction {
            continue;
        }
        if matches!(e.kind, EntityKind::Line { .. }) {
            out.insert(e.id, e);
        }
    }
    out
}

fn build_adjacency(
    edges: &HashMap<SketchEntityId, &Entity>,
) -> HashMap<SketchEntityId, Vec<SketchEntityId>> {
    let mut adj: HashMap<SketchEntityId, Vec<SketchEntityId>> = HashMap::new();
    for (eid, ent) in edges {
        if let Some((a, b)) = line_endpoints(ent) {
            adj.entry(a).or_default().push(*eid);
            adj.entry(b).or_default().push(*eid);
        }
    }
    adj
}

fn line_endpoints(entity: &Entity) -> Option<(SketchEntityId, SketchEntityId)> {
    match entity.kind {
        EntityKind::Line { start, end } => Some((start, end)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use signex_sketch::entity::{Entity, EntityKind};
    use signex_sketch::id::SketchEntityId;
    use signex_sketch::plane::{Plane, PlaneId, PlaneKind};
    use signex_sketch::sketch::SketchData;
    use signex_sketch::solver::residual::ResolvedParams;
    use signex_sketch::solver::Solver;

    /// Build a sketch with one rectangle (4 Points + 4 Lines), solve,
    /// trace from the first Line, expect a 4-vertex polygon.
    fn rectangle_sketch() -> (SketchData, SketchEntityId) {
        let plane = PlaneId::new();
        let mut data = SketchData::default();
        data.planes.push(Plane {
            id: plane,
            kind: PlaneKind::BoardTop,
        });

        let p1 = SketchEntityId::new();
        let p2 = SketchEntityId::new();
        let p3 = SketchEntityId::new();
        let p4 = SketchEntityId::new();
        data.entities
            .push(Entity::new(p1, plane, EntityKind::Point { x: 0.0, y: 0.0 }));
        data.entities
            .push(Entity::new(p2, plane, EntityKind::Point { x: 1.0, y: 0.0 }));
        data.entities
            .push(Entity::new(p3, plane, EntityKind::Point { x: 1.0, y: 1.0 }));
        data.entities
            .push(Entity::new(p4, plane, EntityKind::Point { x: 0.0, y: 1.0 }));

        let l1 = SketchEntityId::new();
        let l2 = SketchEntityId::new();
        let l3 = SketchEntityId::new();
        let l4 = SketchEntityId::new();
        data.entities.push(Entity::new(
            l1,
            plane,
            EntityKind::Line { start: p1, end: p2 },
        ));
        data.entities.push(Entity::new(
            l2,
            plane,
            EntityKind::Line { start: p2, end: p3 },
        ));
        data.entities.push(Entity::new(
            l3,
            plane,
            EntityKind::Line { start: p3, end: p4 },
        ));
        data.entities.push(Entity::new(
            l4,
            plane,
            EntityKind::Line { start: p4, end: p1 },
        ));

        (data, l1)
    }

    fn solve(sketch: &SketchData) -> FullSolveOutput {
        Solver::default().solve(sketch, &ResolvedParams::new()).unwrap()
    }

    #[test]
    fn trace_rectangle_closes() {
        let (sketch, l1) = rectangle_sketch();
        let solved = solve(&sketch);
        let polygon = trace_closed_profile(&sketch, &solved, l1).expect("rectangle should close");
        assert_eq!(polygon.len(), 4);
    }

    #[test]
    fn trace_open_chain_returns_open_error() {
        // Three points, two lines, no closure.
        let plane = PlaneId::new();
        let mut data = SketchData::default();
        data.planes.push(Plane {
            id: plane,
            kind: PlaneKind::BoardTop,
        });
        let p1 = SketchEntityId::new();
        let p2 = SketchEntityId::new();
        let p3 = SketchEntityId::new();
        data.entities
            .push(Entity::new(p1, plane, EntityKind::Point { x: 0.0, y: 0.0 }));
        data.entities
            .push(Entity::new(p2, plane, EntityKind::Point { x: 1.0, y: 0.0 }));
        data.entities
            .push(Entity::new(p3, plane, EntityKind::Point { x: 2.0, y: 0.0 }));
        let l1 = SketchEntityId::new();
        let l2 = SketchEntityId::new();
        data.entities.push(Entity::new(
            l1,
            plane,
            EntityKind::Line { start: p1, end: p2 },
        ));
        data.entities.push(Entity::new(
            l2,
            plane,
            EntityKind::Line { start: p2, end: p3 },
        ));

        let solved = solve(&data);
        assert_eq!(
            trace_closed_profile(&data, &solved, l1),
            Err(TraceError::OpenChain)
        );
    }

    #[test]
    fn trace_arc_in_profile_errors() {
        // A Line + an Arc → ArcInProfile.
        let plane = PlaneId::new();
        let mut data = SketchData::default();
        data.planes.push(Plane {
            id: plane,
            kind: PlaneKind::BoardTop,
        });
        let p1 = SketchEntityId::new();
        let p2 = SketchEntityId::new();
        let pc = SketchEntityId::new();
        data.entities
            .push(Entity::new(p1, plane, EntityKind::Point { x: 0.0, y: 0.0 }));
        data.entities
            .push(Entity::new(p2, plane, EntityKind::Point { x: 1.0, y: 0.0 }));
        data.entities
            .push(Entity::new(pc, plane, EntityKind::Point { x: 0.5, y: 0.0 }));
        let l1 = SketchEntityId::new();
        let arc = SketchEntityId::new();
        data.entities.push(Entity::new(
            l1,
            plane,
            EntityKind::Line { start: p1, end: p2 },
        ));
        data.entities.push(Entity::new(
            arc,
            plane,
            EntityKind::Arc {
                center: pc,
                start: p1,
                end: p2,
                sweep_ccw: true,
            },
        ));
        let solved = solve(&data);
        assert_eq!(
            trace_closed_profile(&data, &solved, l1),
            Err(TraceError::ArcInProfile)
        );
    }

    #[test]
    fn trace_branching_topology_errors() {
        // 3 lines all sharing the centre point pc — walker walks
        // outward from p1 toward pc, finds 2 candidates (l2, l3),
        // returns Branching.
        let plane = PlaneId::new();
        let mut data = SketchData::default();
        data.planes.push(Plane {
            id: plane,
            kind: PlaneKind::BoardTop,
        });
        let pc = SketchEntityId::new();
        let p1 = SketchEntityId::new();
        let p2 = SketchEntityId::new();
        let p3 = SketchEntityId::new();
        data.entities
            .push(Entity::new(pc, plane, EntityKind::Point { x: 0.0, y: 0.0 }));
        data.entities
            .push(Entity::new(p1, plane, EntityKind::Point { x: 1.0, y: 0.0 }));
        data.entities
            .push(Entity::new(p2, plane, EntityKind::Point { x: -1.0, y: 0.0 }));
        data.entities
            .push(Entity::new(p3, plane, EntityKind::Point { x: 0.0, y: 1.0 }));
        let l1 = SketchEntityId::new();
        let l2 = SketchEntityId::new();
        let l3 = SketchEntityId::new();
        // l1 oriented so the walker exits via pc (start_b = pc).
        data.entities.push(Entity::new(
            l1,
            plane,
            EntityKind::Line { start: p1, end: pc },
        ));
        data.entities.push(Entity::new(
            l2,
            plane,
            EntityKind::Line { start: pc, end: p2 },
        ));
        data.entities.push(Entity::new(
            l3,
            plane,
            EntityKind::Line { start: pc, end: p3 },
        ));
        let solved = solve(&data);
        assert_eq!(
            trace_closed_profile(&data, &solved, l1),
            Err(TraceError::Branching)
        );
    }

    #[test]
    fn trace_construction_lines_skipped() {
        // Rectangle with one extra construction line — walker ignores
        // the construction line and still closes the rectangle.
        let (mut sketch, l1) = rectangle_sketch();
        // Find the first Point entity.
        let p1_id = sketch
            .entities
            .iter()
            .find(|e| matches!(e.kind, EntityKind::Point { .. }))
            .unwrap()
            .id;
        // Add a Point + construction Line that touches p1 — this would
        // create branching if not skipped.
        let pc = SketchEntityId::new();
        let plane = sketch.planes[0].id;
        sketch
            .entities
            .push(Entity::new(pc, plane, EntityKind::Point { x: -1.0, y: -1.0 }));
        let mut construction_line = Entity::new(
            SketchEntityId::new(),
            plane,
            EntityKind::Line {
                start: p1_id,
                end: pc,
            },
        );
        construction_line.construction = true;
        sketch.entities.push(construction_line);
        let solved = solve(&sketch);
        let polygon =
            trace_closed_profile(&sketch, &solved, l1).expect("rectangle still closes");
        assert_eq!(polygon.len(), 4);
    }
}
