//! Mint sketch entities for literal pads — first foundational step
//! toward bidirectional sketch ↔ pads sync.
//!
//! When the user enters Sketch mode for a footprint that has literal
//! pads (created in Pads mode) but no sketch entities yet, this
//! module auto-creates a `Point` + `PadAttr` for every pad. The
//! resulting sketch bakes back into the same pad set, so the
//! round-trip is identity-preserving.
//!
//! Future work (v0.15 bidirectional sync):
//! - Pads-mode edits (move / resize / delete) mirror into the
//!   backing sketch entity.
//! - Drag a sketch Point in Sketch mode → pad position updates.
//! - Editing a pad's `PadAttr` from the Properties panel updates
//!   the matching sketch entity.

use signex_library::primitive::footprint::{Footprint, PadKind as LibPadKind, PadShape as LibPadShape};
use signex_sketch::attr::{
    ChamferedCorners as SkChamferedCorners, CustomPadShape, PadAttr, PadKind as SkPadKind,
    PadShape as SkPadShape, PadSide, PasteAperturePattern,
};
use signex_sketch::entity::{Entity, EntityKind};
use signex_sketch::id::SketchEntityId;
use signex_sketch::plane::{Plane, PlaneId, PlaneKind};
use signex_sketch::sketch::SketchData;

use super::state::EditorPad;

/// When the user transitions into Sketch mode for the first time on
/// a footprint that has literal pads but an empty sketch, mint a
/// `Point` + `PadAttr` for each pad. Writes the minted sketch entity
/// IDs back into each `EditorPad.sketch_entity_id` so subsequent
/// Pads-mode edits can mirror through the link. Returns the number
/// of entities minted (zero if the sketch already had content or no
/// literal pads existed).
///
/// The minted sketch produces the same pad set when re-baked through
/// `signex_bake::bake_pads`, so the bake immediately after this call
/// re-emits the original pads — no visual change for the user, but
/// every pad now has a sketch backing they can edit.
pub fn auto_mint_for_literal_pads(
    pads: &mut [EditorPad],
    footprint: &mut Footprint,
) -> usize {
    if pads.is_empty() {
        return 0;
    }
    // Skip if the sketch already has any non-construction entities —
    // assume the user has already started authoring sketch content.
    if let Some(sketch) = footprint.sketch.as_ref() {
        let has_real_entity = sketch.entities.iter().any(|e| !e.construction);
        if has_real_entity {
            return 0;
        }
    }

    let plane_id = ensure_board_top_plane(footprint);
    let sketch = footprint
        .sketch
        .get_or_insert_with(SketchData::default);

    let mut minted = 0usize;
    for pad in pads.iter_mut() {
        let entity_id = SketchEntityId::new();
        let mut entity = Entity::new(
            entity_id,
            plane_id,
            EntityKind::Point {
                x: pad.position_mm.0,
                y: pad.position_mm.1,
            },
        );
        entity.pad = Some(pad_attr_from_editor_pad(pad));
        sketch.entities.push(entity);
        // v0.15 — link the editor pad to its backing sketch entity.
        pad.sketch_entity_id = Some(entity_id);
        minted += 1;
    }
    minted
}

/// v0.15 — when a pad is added in Pads mode (canvas click, etc.),
/// mirror the new pad into the sketch as a `Point` + `PadAttr`.
/// Stores the minted sketch entity ID back on the editor pad so
/// later moves / deletes can mirror through.
pub fn mirror_add_pad_to_sketch(pad: &mut EditorPad, footprint: &mut Footprint) {
    // No-op when the sketch already has a backing entity for this
    // pad (e.g. caller already wired it up).
    if pad.sketch_entity_id.is_some() {
        return;
    }
    let plane_id = ensure_board_top_plane(footprint);
    let sketch = footprint
        .sketch
        .get_or_insert_with(SketchData::default);
    let entity_id = SketchEntityId::new();
    let mut entity = Entity::new(
        entity_id,
        plane_id,
        EntityKind::Point {
            x: pad.position_mm.0,
            y: pad.position_mm.1,
        },
    );
    entity.pad = Some(pad_attr_from_editor_pad(pad));
    sketch.entities.push(entity);
    pad.sketch_entity_id = Some(entity_id);
}

/// v0.15 — when a pad moves in Pads mode (drag), update its backing
/// sketch `Point`'s coordinates so the sketch stays in sync. No-op
/// when the pad has no backing sketch entity yet.
pub fn mirror_move_pad_in_sketch(pad: &EditorPad, footprint: &mut Footprint) {
    let Some(entity_id) = pad.sketch_entity_id else {
        return;
    };
    let Some(sketch) = footprint.sketch.as_mut() else {
        return;
    };
    if let Some(entity) = sketch.entities.iter_mut().find(|e| e.id == entity_id) {
        if let EntityKind::Point { x, y } = &mut entity.kind {
            *x = pad.position_mm.0;
            *y = pad.position_mm.1;
        }
    }
}

/// v0.15 — when a pad is deleted in Pads mode, also drop its
/// backing sketch entity (and any constraints that referenced it).
/// No-op when the pad has no backing sketch entity yet.
pub fn mirror_delete_pad_from_sketch(pad: &EditorPad, footprint: &mut Footprint) {
    let Some(entity_id) = pad.sketch_entity_id else {
        return;
    };
    let Some(sketch) = footprint.sketch.as_mut() else {
        return;
    };
    sketch.entities.retain(|e| e.id != entity_id);
    // Drop dangling constraint refs — coarse rule via Debug
    // stringification (mirrors the SketchEdit::DeleteEntity path in
    // sketch_dispatch.rs).
    let id_str = entity_id.to_string();
    sketch
        .constraints
        .retain(|c| !format!("{:?}", c.kind).contains(&id_str));
}

fn ensure_board_top_plane(footprint: &mut Footprint) -> PlaneId {
    let sketch = footprint
        .sketch
        .get_or_insert_with(SketchData::default);
    if let Some(p) = sketch.planes.iter().find(|p| matches!(p.kind, PlaneKind::BoardTop)) {
        return p.id;
    }
    let p = Plane {
        id: PlaneId::new(),
        kind: PlaneKind::BoardTop,
    };
    let id = p.id;
    sketch.planes.push(p);
    id
}

fn pad_attr_from_editor_pad(pad: &EditorPad) -> PadAttr {
    PadAttr {
        number: pad.number.clone(),
        kind: map_kind(pad.kind),
        side: map_side(pad),
        shape: map_shape(&pad.shape),
        size_x_expr: format!("{}mm", format_f64(pad.size_mm.0)),
        size_y_expr: format!("{}mm", format_f64(pad.size_mm.1)),
        rotation_expr: None,
        offset_x_expr: None,
        offset_y_expr: None,
        drill: None,
        mask_margin_expr: None,
        paste_margin_expr: None,
        paste_apertures: PasteAperturePattern::Single,
    }
}

fn map_kind(k: LibPadKind) -> SkPadKind {
    match k {
        LibPadKind::Smd => SkPadKind::Smd,
        LibPadKind::Tht => SkPadKind::Tht,
        LibPadKind::NptHole => SkPadKind::NptHole,
        LibPadKind::ConnectorPad => SkPadKind::ConnectorPad,
        LibPadKind::Castellated => SkPadKind::Castellated,
        LibPadKind::Fiducial => SkPadKind::Fiducial,
        // Future-proof the non_exhaustive lib enum.
        _ => SkPadKind::Smd,
    }
}

fn map_side(pad: &EditorPad) -> PadSide {
    use crate::library::editor::footprint::layers::FpLayer;
    let primary = pad.primary_layer();
    match primary {
        FpLayer::FCu | FpLayer::FFab | FpLayer::FSilks => PadSide::Top,
        FpLayer::BCu | FpLayer::BFab | FpLayer::BSilks => PadSide::Bottom,
        _ => PadSide::All,
    }
}

fn map_shape(s: &LibPadShape) -> SkPadShape {
    match s {
        LibPadShape::Round => SkPadShape::Round,
        LibPadShape::Rect => SkPadShape::Rect,
        LibPadShape::Oval => SkPadShape::Oval,
        LibPadShape::RoundRect { radius_ratio } => SkPadShape::RoundRect {
            radius_ratio_expr: format_f64(*radius_ratio),
        },
        LibPadShape::Chamfered {
            chamfer_ratio,
            corners,
        } => SkPadShape::Chamfered {
            chamfer_ratio_expr: format_f64(*chamfer_ratio),
            corners: SkChamferedCorners {
                top_left: corners.top_left,
                top_right: corners.top_right,
                bottom_left: corners.bottom_left,
                bottom_right: corners.bottom_right,
            },
        },
        LibPadShape::Custom(poly) => {
            // Convert lib's free-form polygon into a sketch
            // CustomPadShape::StaticPoints — sketch-profile bake
            // (closed-loop walker) is not used here since literal
            // pads don't have a sketch profile to walk.
            SkPadShape::Custom(CustomPadShape::StaticPoints {
                points: poly.points.clone(),
            })
        }
    }
}

/// Format a float with up to 4 fractional digits, trimming trailing
/// zeros. Keeps the generated expression strings readable
/// (e.g. `1.5` rather than `1.5000000000000`).
fn format_f64(v: f64) -> String {
    let s = format!("{v:.4}");
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor_pad(number: &str, x: f64, y: f64) -> EditorPad {
        let mut p = EditorPad::new_default(number.into(), (x, y));
        p.size_mm = (1.0, 0.5);
        p
    }

    #[test]
    fn empty_pads_mint_nothing() {
        let mut fp = Footprint::empty("test");
        let mut pads: Vec<EditorPad> = Vec::new();
        let n = auto_mint_for_literal_pads(&mut pads, &mut fp);
        assert_eq!(n, 0);
        assert!(fp.sketch.is_none() || fp.sketch.as_ref().unwrap().entities.is_empty());
    }

    #[test]
    fn three_pads_mint_three_points_with_pad_attrs() {
        let mut fp = Footprint::empty("test");
        let mut pads = vec![
            editor_pad("1", 0.0, 0.0),
            editor_pad("2", 1.27, 0.0),
            editor_pad("3", 2.54, 0.0),
        ];
        let n = auto_mint_for_literal_pads(&mut pads, &mut fp);
        assert_eq!(n, 3);
        let sketch = fp.sketch.as_ref().unwrap();
        // 1 plane + 3 entities (one Point each).
        assert_eq!(sketch.planes.len(), 1);
        assert_eq!(sketch.entities.len(), 3);
        for entity in &sketch.entities {
            assert!(matches!(entity.kind, EntityKind::Point { .. }));
            let attr = entity.pad.as_ref().expect("Point should carry PadAttr");
            assert!(!attr.number.is_empty());
            assert_eq!(attr.size_x_expr, "1mm");
            assert_eq!(attr.size_y_expr, "0.5mm");
        }
        // v0.15: every pad should now carry the minted entity ID.
        for pad in &pads {
            assert!(pad.sketch_entity_id.is_some());
        }
    }

    #[test]
    fn skip_when_sketch_already_has_entities() {
        let mut fp = Footprint::empty("test");
        // Pre-populate sketch with one non-construction entity.
        let mut sketch = SketchData::default();
        let plane = Plane {
            id: PlaneId::new(),
            kind: PlaneKind::BoardTop,
        };
        sketch.planes.push(plane.clone());
        sketch.entities.push(Entity::new(
            SketchEntityId::new(),
            plane.id,
            EntityKind::Point { x: 0.0, y: 0.0 },
        ));
        fp.sketch = Some(sketch);

        let mut pads = vec![editor_pad("1", 0.0, 0.0)];
        let n = auto_mint_for_literal_pads(&mut pads, &mut fp);
        assert_eq!(n, 0, "auto-mint must skip when sketch is already populated");
        assert_eq!(fp.sketch.as_ref().unwrap().entities.len(), 1);
        assert!(pads[0].sketch_entity_id.is_none(), "skip leaves the link unset");
    }

    #[test]
    fn skip_when_sketch_only_has_construction_entities() {
        // Construction-only sketches are still treated as "no real
        // user authoring", so auto-mint should fire.
        let mut fp = Footprint::empty("test");
        let mut sketch = SketchData::default();
        let plane = Plane {
            id: PlaneId::new(),
            kind: PlaneKind::BoardTop,
        };
        let plane_id = plane.id;
        sketch.planes.push(plane);
        let mut construction = Entity::new(
            SketchEntityId::new(),
            plane_id,
            EntityKind::Point { x: 0.0, y: 0.0 },
        );
        construction.construction = true;
        sketch.entities.push(construction);
        fp.sketch = Some(sketch);

        let mut pads = vec![editor_pad("1", 0.0, 0.0)];
        let n = auto_mint_for_literal_pads(&mut pads, &mut fp);
        assert_eq!(n, 1);
        // Construction entity preserved + 1 minted pad point = 2 total.
        assert_eq!(fp.sketch.as_ref().unwrap().entities.len(), 2);
        assert!(pads[0].sketch_entity_id.is_some());
    }

    #[test]
    fn mirror_add_pad_links_to_new_sketch_entity() {
        let mut fp = Footprint::empty("test");
        let mut pad = editor_pad("X", 5.0, 5.0);
        assert!(pad.sketch_entity_id.is_none());
        mirror_add_pad_to_sketch(&mut pad, &mut fp);
        let id = pad.sketch_entity_id.expect("mirror should mint id");
        let sketch = fp.sketch.as_ref().unwrap();
        let entity = sketch.entities.iter().find(|e| e.id == id).expect("entity exists");
        match entity.kind {
            EntityKind::Point { x, y } => {
                assert_eq!((x, y), (5.0, 5.0));
            }
            _ => panic!("minted entity must be a Point"),
        }
        assert!(entity.pad.is_some(), "Point should carry PadAttr");
    }

    #[test]
    fn mirror_add_pad_with_existing_link_is_noop() {
        let mut fp = Footprint::empty("test");
        let mut pad = editor_pad("X", 0.0, 0.0);
        pad.sketch_entity_id = Some(SketchEntityId::new());
        mirror_add_pad_to_sketch(&mut pad, &mut fp);
        // Sketch should not have been touched.
        assert!(fp.sketch.is_none() || fp.sketch.as_ref().unwrap().entities.is_empty());
    }

    #[test]
    fn mirror_move_pad_updates_sketch_point() {
        let mut fp = Footprint::empty("test");
        let mut pad = editor_pad("X", 0.0, 0.0);
        mirror_add_pad_to_sketch(&mut pad, &mut fp);
        // Now move the pad.
        pad.position_mm = (3.5, 7.25);
        mirror_move_pad_in_sketch(&pad, &mut fp);
        let id = pad.sketch_entity_id.unwrap();
        let entity = fp
            .sketch
            .as_ref()
            .unwrap()
            .entities
            .iter()
            .find(|e| e.id == id)
            .unwrap();
        match entity.kind {
            EntityKind::Point { x, y } => assert_eq!((x, y), (3.5, 7.25)),
            _ => panic!("entity must still be a Point"),
        }
    }

    #[test]
    fn mirror_delete_pad_drops_sketch_entity() {
        let mut fp = Footprint::empty("test");
        let mut pad = editor_pad("X", 0.0, 0.0);
        mirror_add_pad_to_sketch(&mut pad, &mut fp);
        assert_eq!(fp.sketch.as_ref().unwrap().entities.len(), 1);
        mirror_delete_pad_from_sketch(&pad, &mut fp);
        assert_eq!(fp.sketch.as_ref().unwrap().entities.len(), 0);
    }

    #[test]
    fn format_f64_trims_trailing_zeros() {
        assert_eq!(format_f64(1.0), "1");
        assert_eq!(format_f64(1.5), "1.5");
        assert_eq!(format_f64(0.25), "0.25");
        assert_eq!(format_f64(1.27), "1.27");
        assert_eq!(format_f64(0.0), "0");
    }
}
