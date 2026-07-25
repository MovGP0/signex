use super::*;

const SELECTION_COLOR: Color = Color::from_rgb(
    1.0,
    235.0 / 255.0,
    59.0 / 255.0,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GerberItemSelection
{
    pub layer_index: usize,
    pub primitive_index: usize,
}

impl GerberViewerState
{
    pub fn selected_item(&self) -> Option<GerberItemSelection>
    {
        self.selected_item
    }

    pub fn set_selected_item(&mut self, selection: Option<GerberItemSelection>)
    {
        let selection = selection.filter(|selection|
        {
            self.layers
                .get(selection.layer_index)
                .is_some_and(|layer|
                {
                    layer.visible
                        && selection.primitive_index
                            < layer.layer.geometry.primitives.len()
                })
        });
        self.selected_item = selection;
        self.status = selection.map_or_else(
            || "Rendered item selection cleared.".to_owned(),
            |selection|
            {
                format!(
                    "Selected item {} on {}.",
                    selection.primitive_index + 1,
                    self.layers[selection.layer_index].layer.name,
                )
            },
        );
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
    }

    pub(in crate::gerber_viewer) fn retain_valid_selection(&mut self)
    {
        let Some(selection) = self.selected_item else
        {
            return;
        };
        if self
            .layers
            .get(selection.layer_index)
            .is_none_or(|layer|
            {
                selection.primitive_index
                    >= layer.layer.geometry.primitives.len()
            })
        {
            self.selected_item = None;
        }
    }

    pub(in crate::gerber_viewer) fn remove_layer_from_selection(
        &mut self,
        removed_layer_index: usize,
    )
    {
        self.selected_item = self.selected_item.and_then(|selection|
        {
            if selection.layer_index == removed_layer_index
            {
                None
            }
            else
            {
                Some(GerberItemSelection {
                    layer_index: if selection.layer_index > removed_layer_index
                    {
                        selection.layer_index - 1
                    }
                    else
                    {
                        selection.layer_index
                    },
                    primitive_index: selection.primitive_index,
                })
            }
        });
    }
}

pub(super) fn selected_primitive_color(
    color: Color,
    selection: Option<GerberItemSelection>,
    layer_index: usize,
    primitive_index: usize,
) -> Color
{
    if selection == Some(GerberItemSelection {
        layer_index,
        primitive_index,
    })
    {
        SELECTION_COLOR
    }
    else
    {
        color
    }
}

pub(super) fn hit_test_visible_item(
    layers: &[ViewerLayer],
    point: signex_gerber::Point,
    tolerance: f64,
) -> Option<GerberItemSelection>
{
    if !point.x.is_finite()
        || !point.y.is_finite()
        || !tolerance.is_finite()
        || tolerance < 0.0
    {
        return None;
    }

    layers
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, layer)| layer.visible)
        .find_map(|(layer_index, layer)|
        {
            layer
                .layer
                .geometry
                .primitives
                .iter()
                .enumerate()
                .rev()
                .find_map(|(primitive_index, primitive)|
                {
                    primitive_contains(primitive, point, tolerance).then_some(
                        GerberItemSelection {
                            layer_index,
                            primitive_index,
                        },
                    )
                })
        })
}

fn primitive_contains(
    primitive: &GerberPrimitive,
    point: signex_gerber::Point,
    tolerance: f64,
) -> bool
{
    match primitive
    {
        GerberPrimitive::Stroke {
            start,
            end,
            width,
            ..
        }
        | GerberPrimitive::DrillSlot {
            start,
            end,
            width,
            ..
        } =>
        {
            distance_to_segment(point, *start, *end) <= width / 2.0 + tolerance
        }
        GerberPrimitive::Flash {
            position,
            aperture,
            ..
        } => flash_contains(point, *position, aperture, tolerance),
        GerberPrimitive::Region { points, .. } =>
        {
            point_in_polygon(point, points)
                || polygon_edge_distance(point, points) <= tolerance
        }
        GerberPrimitive::DrillHit {
            position,
            diameter,
            ..
        } =>
        {
            distance(point, *position) <= diameter / 2.0 + tolerance
        }
    }
}

fn flash_contains(
    point: signex_gerber::Point,
    position: signex_gerber::Point,
    aperture: &ApertureShape,
    tolerance: f64,
) -> bool
{
    let dx = (point.x - position.x).abs();
    let dy = (point.y - position.y).abs();
    match aperture
    {
        ApertureShape::Circle { diameter }
        | ApertureShape::Polygon { diameter, .. } =>
        {
            distance(point, position) <= diameter / 2.0 + tolerance
        }
        ApertureShape::Rectangle { width, height }
        | ApertureShape::Obround { width, height } =>
        {
            dx <= width / 2.0 + tolerance
                && dy <= height / 2.0 + tolerance
        }
        ApertureShape::Macro { .. } =>
        {
            distance(point, position)
                <= aperture.maximum_extent() / 2.0 + tolerance
        }
    }
}

fn distance(
    first: signex_gerber::Point,
    second: signex_gerber::Point,
) -> f64
{
    (first.x - second.x).hypot(first.y - second.y)
}

fn distance_to_segment(
    point: signex_gerber::Point,
    start: signex_gerber::Point,
    end: signex_gerber::Point,
) -> f64
{
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f64::EPSILON
    {
        return distance(point, start);
    }
    let projection = (((point.x - start.x) * dx + (point.y - start.y) * dy)
        / length_squared)
        .clamp(0.0, 1.0);
    distance(
        point,
        signex_gerber::Point {
            x: start.x + projection * dx,
            y: start.y + projection * dy,
        },
    )
}

fn point_in_polygon(
    point: signex_gerber::Point,
    polygon: &[signex_gerber::Point],
) -> bool
{
    if polygon.len() < 3
    {
        return false;
    }
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for current in polygon.iter().copied()
    {
        let crosses = (current.y > point.y) != (previous.y > point.y)
            && point.x
                < (previous.x - current.x) * (point.y - current.y)
                    / (previous.y - current.y)
                    + current.x;
        if crosses
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn polygon_edge_distance(
    point: signex_gerber::Point,
    polygon: &[signex_gerber::Point],
) -> f64
{
    if polygon.is_empty()
    {
        return f64::INFINITY;
    }
    polygon
        .iter()
        .copied()
        .zip(polygon.iter().copied().cycle().skip(1))
        .take(polygon.len())
        .map(|(start, end)| distance_to_segment(point, start, end))
        .fold(f64::INFINITY, f64::min)
}

#[cfg(test)]
#[path = "../../tests/gerber_viewer/selection.rs"]
mod gerber_selection_test_definitions;

#[cfg(test)]
gerber_selection_test_definitions::gerber_selection_tests!();
