use signex_sketch::entity::{Entity, EntityKind};
use signex_sketch::id::{ConstraintId, SketchEntityId};
use signex_sketch::plane::{Plane, PlaneId, PlaneKind};
use uuid::Uuid;

#[test]
fn entity_id_round_trip() {
    let id = SketchEntityId(Uuid::new_v4());
    let s = serde_json::to_string(&id).unwrap();
    let back: SketchEntityId = serde_json::from_str(&s).unwrap();
    assert_eq!(id, back);
}

#[test]
fn constraint_id_round_trip() {
    let id = ConstraintId(Uuid::new_v4());
    let s = serde_json::to_string(&id).unwrap();
    let back: ConstraintId = serde_json::from_str(&s).unwrap();
    assert_eq!(id, back);
}

#[test]
fn plane_board_top_round_trip() {
    let p = Plane {
        id: PlaneId::new(),
        kind: PlaneKind::BoardTop,
    };
    let s = toml::to_string(&p).unwrap();
    let back: Plane = toml::from_str(&s).unwrap();
    assert_eq!(p.kind, back.kind);
}

#[test]
fn plane_body_top_round_trip() {
    let p = Plane {
        id: PlaneId::new(),
        kind: PlaneKind::BodyTop {
            offset_z_expr: "= body_h".to_string(),
        },
    };
    let s = toml::to_string(&p).unwrap();
    let back: Plane = toml::from_str(&s).unwrap();
    assert_eq!(p.kind, back.kind);
}

#[test]
fn point_entity_round_trip() {
    let pt_id = SketchEntityId::new();
    let plane_id = PlaneId::new();
    let e = Entity {
        id: pt_id,
        plane: plane_id,
        construction: false,
        kind: EntityKind::Point { x: 1.5, y: 2.5 },
    };
    let s = toml::to_string(&e).unwrap();
    let back: Entity = toml::from_str(&s).unwrap();
    assert_eq!(e, back);
}

#[test]
fn line_entity_round_trip() {
    let plane_id = PlaneId::new();
    let p1 = SketchEntityId::new();
    let p2 = SketchEntityId::new();
    let e = Entity {
        id: SketchEntityId::new(),
        plane: plane_id,
        construction: true,
        kind: EntityKind::Line { start: p1, end: p2 },
    };
    let s = toml::to_string(&e).unwrap();
    let back: Entity = toml::from_str(&s).unwrap();
    assert_eq!(e, back);
}

#[test]
fn arc_entity_round_trip() {
    let plane_id = PlaneId::new();
    let e = Entity {
        id: SketchEntityId::new(),
        plane: plane_id,
        construction: false,
        kind: EntityKind::Arc {
            center: SketchEntityId::new(),
            start: SketchEntityId::new(),
            end: SketchEntityId::new(),
            sweep_ccw: true,
        },
    };
    let s = toml::to_string(&e).unwrap();
    let back: Entity = toml::from_str(&s).unwrap();
    assert_eq!(e, back);
}

#[test]
fn circle_entity_round_trip() {
    let plane_id = PlaneId::new();
    let e = Entity {
        id: SketchEntityId::new(),
        plane: plane_id,
        construction: false,
        kind: EntityKind::Circle {
            center: SketchEntityId::new(),
            radius: 0.75,
        },
    };
    let s = toml::to_string(&e).unwrap();
    let back: Entity = toml::from_str(&s).unwrap();
    assert_eq!(e, back);
}
