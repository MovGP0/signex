use super::*;

#[derive(Debug, Default)]
pub(super) struct GerberCanvasState
{
    drag_start: Option<Point>,
    zoom_selection_start: Option<Point>,
    zoom_selection_current: Option<Point>,
}

pub(super) struct GerberCanvas<'a>
{
    pub(super) layers: &'a [ViewerLayer],
    pub(super) background: Color,
    pub(super) grid: Color,
    pub(super) grid_visible: bool,
    pub(super) full_window_crosshair: bool,
    pub(super) page_size: GerberPageSize,
    pub(super) zoom_selection_active: bool,
    pub(super) measurement_active: bool,
    pub(super) measurement: Option<GerberMeasurement>,
    pub(super) sketch_flashes: bool,
    pub(super) sketch_lines: bool,
    pub(super) sketch_polygons: bool,
    pub(super) ghost_negative_objects: bool,
    pub(super) negative_ghost_color: Color,
    pub(super) show_d_code_labels: bool,
    pub(super) d_code_color: Color,
    pub(super) compare_mode: bool,
    pub(super) compare_palette: &'a [Color],
    pub(super) dim_inactive_layers: bool,
    pub(super) inactive_layer_opacity: f32,
    pub(super) mirrored: bool,
    pub(super) active_layer: Option<usize>,
    pub(super) highlighted_component: Option<&'a str>,
    pub(super) highlighted_net: Option<&'a str>,
    pub(super) highlighted_attribute: Option<&'a GerberAttributeValue>,
    pub(super) highlighted_d_code: Option<i32>,
    pub(super) selected_item: Option<GerberItemSelection>,
    pub(super) grid_size: &'a GridSizePreset,
    pub(super) shortcut_resolver: &'a dyn GerberShortcutResolver,
    pub(super) redraw_generation: u64,
    pub(super) zoom: f32,
    pub(super) pan: iced::Vector,
}

impl GerberCanvas<'_>
{
    fn screen_to_world(
        &self,
        bounds: Rectangle,
        screen: Point,
    ) -> Option<signex_gerber::Point>
    {
        let world_bounds = page_bounds(visible_bounds(self.layers), self.page_size)?;
        let (scale, world_center, screen_center) =
            fit_transform(world_bounds, bounds, self.zoom, self.pan);
        Some(screen_to_world_point(
            screen,
            world_center,
            screen_center,
            scale,
            self.mirrored,
        ))
    }

    fn pixels_per_world_unit(&self, bounds: Rectangle) -> Option<f32>
    {
        let world_bounds = page_bounds(visible_bounds(self.layers), self.page_size)?;
        Some(fit_transform(world_bounds, bounds, self.zoom, self.pan).0)
    }
}

impl canvas::Program<GerberViewerMessage> for GerberCanvas<'_>
{
    type State = GerberCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<GerberViewerMessage>>
    {
        match event
        {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                ..
            }) => gerber_shortcut_message(self.shortcut_resolver, key, *modifiers)
                .map(|message| canvas::Action::publish(message).and_capture()),
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(bounds)
                {
                    return None;
                }
                let lines = match delta
                {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 30.0,
                };
                Some(
                    canvas::Action::publish(GerberViewerMessage::ZoomBy(
                        1.12_f32.powf(lines),
                    ))
                    .and_capture(),
                )
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                state.drag_start = cursor.position_in(bounds);
                Some(canvas::Action::capture())
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if self.zoom_selection_active =>
            {
                let position = cursor.position_in(bounds)?;
                state.zoom_selection_start = Some(position);
                state.zoom_selection_current = Some(position);
                Some(canvas::Action::capture())
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if self.measurement_active =>
            {
                let position = cursor.position_in(bounds)?;
                let world = self.screen_to_world(bounds, position)?;
                Some(
                    canvas::Action::publish(
                        GerberViewerMessage::CaptureMeasurementPoint(world),
                    )
                    .and_capture(),
                )
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) =>
            {
                let position = cursor.position_in(bounds)?;
                let world = self.screen_to_world(bounds, position)?;
                let scale = self.pixels_per_world_unit(bounds)?;
                let selection = hit_test_visible_item(
                    self.layers,
                    world,
                    6.0 / f64::from(scale),
                );
                Some(
                    canvas::Action::publish(
                        GerberViewerMessage::SetSelectedItem(selection),
                    )
                    .and_capture(),
                )
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if state.zoom_selection_start.is_some()
                {
                    state.zoom_selection_current =
                        Some(Point::new(position.x - bounds.x, position.y - bounds.y));
                    Some(canvas::Action::capture())
                }
                else if let Some(previous) = state.drag_start
                {
                    let current = Point::new(position.x - bounds.x, position.y - bounds.y);
                    state.drag_start = Some(current);
                    Some(
                        canvas::Action::publish(GerberViewerMessage::PanBy(current - previous))
                            .and_capture(),
                    )
                }
                else
                {
                    let position = cursor.position_in(bounds)?;
                    Some(canvas::Action::publish(
                        GerberViewerMessage::CursorWorldPositionChanged(
                            self.screen_to_world(bounds, position),
                        ),
                    ))
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                state.drag_start = None;
                Some(canvas::Action::capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.zoom_selection_start.is_some() =>
            {
                let start = state.zoom_selection_start.take()?;
                let end = state.zoom_selection_current.take().unwrap_or(start);
                let Some(selection) = normalized_screen_rectangle(start, end)
                else
                {
                    return Some(canvas::Action::capture());
                };
                let Some(world_start) = self.screen_to_world(
                    bounds,
                    Point::new(selection.x, selection.y + selection.height),
                )
                else
                {
                    return Some(canvas::Action::capture());
                };
                let Some(world_end) = self.screen_to_world(
                    bounds,
                    Point::new(selection.x + selection.width, selection.y),
                )
                else
                {
                    return Some(canvas::Action::capture());
                };
                Some(
                    canvas::Action::publish(GerberViewerMessage::ZoomToSelection {
                        bounds: Bounds {
                            min: signex_gerber::Point {
                                x: world_start.x.min(world_end.x),
                                y: world_start.y.min(world_end.y),
                            },
                            max: signex_gerber::Point {
                                x: world_start.x.max(world_end.x),
                                y: world_start.y.max(world_end.y),
                            },
                        },
                        viewport: bounds,
                    })
                    .and_capture(),
                )
            }
            Event::Mouse(mouse::Event::CursorLeft) => Some(
                canvas::Action::publish(
                    GerberViewerMessage::CursorWorldPositionChanged(None),
                ),
            ),
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry>
    {
        let _redraw_generation = self.redraw_generation;
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.background);

        let artwork_bounds = visible_bounds(self.layers);
        let Some(world_bounds) = page_bounds(artwork_bounds, self.page_size) else
        {
            if self.grid_visible
            {
                draw_grid(
                    &mut frame,
                    bounds,
                    self.grid,
                    self.grid_size,
                    32.0 / 1.27,
                    Point::new(bounds.width / 2.0, bounds.height / 2.0),
                );
            }
            frame.fill_text(canvas::Text {
                content: "Open Gerber files to inspect fabrication layers".into(),
                position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
                color: Color {
                    a: 0.65,
                    ..self.grid
                },
                size: iced::Pixels(14.0),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center,
                ..canvas::Text::default()
            });
            if self.full_window_crosshair
            {
                if let Some(position) = cursor.position_in(bounds)
                {
                    draw_full_window_crosshair(
                        &mut frame,
                        bounds,
                        position,
                        self.grid,
                    );
                }
            }
            return vec![frame.into_geometry()];
        };

        let (scale, world_center, screen_center) =
            fit_transform(world_bounds, bounds, self.zoom, self.pan);
        let world_to_screen = |point: signex_gerber::Point| -> Point {
            world_to_screen_point(
                point,
                world_center,
                screen_center,
                scale,
                self.mirrored,
            )
        };
        if self.grid_visible
        {
            draw_grid(
                &mut frame,
                bounds,
                self.grid,
                self.grid_size,
                scale,
                world_to_screen(signex_gerber::Point { x: 0.0, y: 0.0 }),
            );
        }
        draw_page_boundary(
            &mut frame,
            world_bounds,
            scale,
            &world_to_screen,
            self.grid,
        );

        let visible_layer_count =
            self.layers.iter().filter(|layer| layer.visible).count();
        for (visible_ordinal, (layer_index, viewer_layer)) in self
            .layers
            .iter()
            .enumerate()
            .filter(|(_, layer)| layer.visible)
            .enumerate()
        {
            let highlighted_d_code = if self.active_layer == Some(layer_index)
            {
                self.highlighted_d_code
            }
            else
            {
                None
            };
            draw_layer(
                &mut frame,
                viewer_layer,
                compare_layer_color(
                    viewer_layer.color,
                    visible_ordinal,
                    visible_layer_count,
                    self.compare_mode,
                    self.compare_palette,
                ),
                self.active_layer == Some(layer_index),
                self.dim_inactive_layers,
                self.inactive_layer_opacity,
                scale,
                &world_to_screen,
                self.background,
                self.highlighted_component,
                self.highlighted_net,
                self.highlighted_attribute,
                highlighted_d_code,
                self.selected_item,
                layer_index,
                self.sketch_flashes,
                self.sketch_lines,
                self.sketch_polygons,
                self.ghost_negative_objects,
                self.negative_ghost_color,
                self.show_d_code_labels,
                self.d_code_color,
                self.zoom,
            );
        }
        if let Some(measurement) = self.measurement
        {
            draw_measurement(
                &mut frame,
                measurement,
                &world_to_screen,
                self.grid,
            );
        }
        if let (Some(start), Some(end)) = (
            state.zoom_selection_start,
            state.zoom_selection_current,
        )
        {
            if let Some(selection) = normalized_screen_rectangle(start, end)
            {
                let path = canvas::Path::rectangle(
                    Point::new(selection.x, selection.y),
                    selection.size(),
                );
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_color(self.grid)
                        .with_width(1.0),
                );
            }
        }
        if self.full_window_crosshair
        {
            if let Some(position) = cursor.position_in(bounds)
            {
                draw_full_window_crosshair(
                    &mut frame,
                    bounds,
                    position,
                    self.grid,
                );
            }
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction
    {
        if state.drag_start.is_some()
        {
            mouse::Interaction::Grabbing
        }
        else if cursor.is_over(bounds)
        {
            mouse::Interaction::Crosshair
        }
        else
        {
            mouse::Interaction::default()
        }
    }
}

pub(super) fn full_window_crosshair_segments(
    bounds: Rectangle,
    position: Point,
) -> [(Point, Point); 2]
{
    [
        (
            Point::new(0.0, position.y),
            Point::new(bounds.width, position.y),
        ),
        (
            Point::new(position.x, 0.0),
            Point::new(position.x, bounds.height),
        ),
    ]
}

pub(super) fn draw_full_window_crosshair(
    frame: &mut canvas::Frame,
    bounds: Rectangle,
    position: Point,
    color: Color,
)
{
    let color = Color { a: 0.72, ..color };
    for (start, end) in full_window_crosshair_segments(bounds, position)
    {
        frame.stroke(
            &canvas::Path::line(start, end),
            canvas::Stroke::default()
                .with_color(color)
                .with_width(1.0),
        );
    }
}

pub(super) fn draw_grid(
    frame: &mut canvas::Frame,
    bounds: Rectangle,
    color: Color,
    grid_size: &GridSizePreset,
    pixels_per_millimetre: f32,
    origin: Point,
)
{
    let dot_color = Color { a: 0.16, ..color };
    let x_spacing = visible_grid_spacing(
        grid_size.x_millimetres() as f32 * pixels_per_millimetre,
    );
    let y_spacing = visible_grid_spacing(
        grid_size.y_millimetres() as f32 * pixels_per_millimetre,
    );

    match (x_spacing, y_spacing)
    {
        (Some(x_spacing), Some(y_spacing)) => {
            let mut x = origin.x.rem_euclid(x_spacing);
            while x <= bounds.width
            {
                let mut y = origin.y.rem_euclid(y_spacing);
                while y <= bounds.height
                {
                    frame.fill(&canvas::Path::circle(Point::new(x, y), 0.75), dot_color);
                    y += y_spacing;
                }
                x += x_spacing;
            }
        }
        (Some(x_spacing), None) => {
            let mut x = origin.x.rem_euclid(x_spacing);
            while x <= bounds.width
            {
                frame.stroke(
                    &canvas::Path::line(
                        Point::new(x, 0.0),
                        Point::new(x, bounds.height),
                    ),
                    canvas::Stroke::default()
                        .with_color(dot_color)
                        .with_width(1.0),
                );
                x += x_spacing;
            }
        }
        (None, Some(y_spacing)) => {
            let mut y = origin.y.rem_euclid(y_spacing);
            while y <= bounds.height
            {
                frame.stroke(
                    &canvas::Path::line(
                        Point::new(0.0, y),
                        Point::new(bounds.width, y),
                    ),
                    canvas::Stroke::default()
                        .with_color(dot_color)
                        .with_width(1.0),
                );
                y += y_spacing;
            }
        }
        (None, None) => {}
    }
}

pub(super) fn visible_grid_spacing(spacing: f32) -> Option<f32>
{
    const MINIMUM_GRID_SPACING_PIXELS: f32 = 10.0;

    if !spacing.is_finite() || spacing <= 0.0
    {
        return None;
    }
    if spacing >= MINIMUM_GRID_SPACING_PIXELS
    {
        Some(spacing)
    }
    else
    {
        Some(
            spacing
                * (MINIMUM_GRID_SPACING_PIXELS / spacing)
                    .ceil()
                    .max(1.0),
        )
    }
}

pub(super) fn inactive_layer_color(
    color: Color,
    active: bool,
    dim_inactive_layers: bool,
    inactive_layer_opacity: f32,
) -> Color
{
    if !dim_inactive_layers || active
    {
        return color;
    }

    Color {
        a: color.a * inactive_layer_opacity.clamp(0.0, 1.0),
        ..color
    }
}

pub(super) fn compare_layer_color(
    original: Color,
    visible_ordinal: usize,
    visible_layer_count: usize,
    compare_mode: bool,
    compare_palette: &[Color],
) -> Color
{
    if !compare_mode || visible_layer_count < 2 || compare_palette.is_empty()
    {
        return original;
    }

    Color {
        a: 0.68,
        ..compare_palette[visible_ordinal % compare_palette.len()]
    }
}

#[cfg(test)]
pub(super) fn composite_compare_colors(top: Color, bottom: Color) -> Color
{
    let alpha = top.a + bottom.a * (1.0 - top.a);
    Color {
        r: (top.r * top.a + bottom.r * bottom.a * (1.0 - top.a)) / alpha,
        g: (top.g * top.a + bottom.g * bottom.a * (1.0 - top.a)) / alpha,
        b: (top.b * top.a + bottom.b * bottom.a * (1.0 - top.a)) / alpha,
        a: alpha,
    }
}

pub(super) fn primitive_polarity_color(
    polarity: PrimitivePolarity,
    dark_color: Color,
    background: Color,
    negative_ghost_color: Color,
    ghost_negative_objects: bool,
) -> Color
{
    match polarity
    {
        PrimitivePolarity::Dark => dark_color,
        PrimitivePolarity::Clear if ghost_negative_objects =>
        {
            negative_ghost_color
        }
        PrimitivePolarity::Clear => background,
    }
}

pub(super) fn draw_layer(
    frame: &mut canvas::Frame,
    viewer_layer: &ViewerLayer,
    layer_color: Color,
    active: bool,
    dim_inactive_layers: bool,
    inactive_layer_opacity: f32,
    scale: f32,
    world_to_screen: &impl Fn(signex_gerber::Point) -> Point,
    background: Color,
    highlighted_component: Option<&str>,
    highlighted_net: Option<&str>,
    highlighted_attribute: Option<&GerberAttributeValue>,
    highlighted_d_code: Option<i32>,
    selected_item: Option<GerberItemSelection>,
    layer_index: usize,
    sketch_flashes: bool,
    sketch_lines: bool,
    sketch_polygons: bool,
    ghost_negative_objects: bool,
    negative_ghost_color: Color,
    show_d_code_labels: bool,
    d_code_color: Color,
    zoom: f32,
)
{
    for (primitive_index, primitive) in viewer_layer
        .layer
        .geometry
        .primitives
        .iter()
        .enumerate()
    {
        let attributes = viewer_layer
            .layer
            .geometry
            .primitive_attributes
            .get(primitive_index);
        let component_color = component_highlight_color(
            inactive_layer_color(
                layer_color,
                active,
                dim_inactive_layers,
                inactive_layer_opacity,
            ),
            attributes,
            highlighted_component,
        );
        let net_color = net_highlight_color(
            component_color,
            attributes,
            highlighted_net,
        );
        let attribute_color = attribute_highlight_color(
            net_color,
            attributes,
            highlighted_attribute,
        );
        let dark_color = d_code_highlight_color(
            attribute_color,
            primitive,
            highlighted_d_code,
        );
        let selected = selected_item == Some(GerberItemSelection {
            layer_index,
            primitive_index,
        });
        let dark_color = selected_primitive_color(
            dark_color,
            selected_item,
            layer_index,
            primitive_index,
        );
        let polarity_color = |polarity: PrimitivePolarity| if selected
        {
            dark_color
        }
        else
        {
            primitive_polarity_color(
                polarity,
                dark_color,
                background,
                negative_ghost_color,
                ghost_negative_objects,
            )
        };
        match primitive
        {
            GerberPrimitive::Stroke {
                start,
                end,
                width,
                polarity,
                ..
            } =>
            {
                paint_line_path(
                    frame,
                    &canvas::Path::line(world_to_screen(*start), world_to_screen(*end)),
                    polarity_color(*polarity),
                    background,
                    (*width as f32 * scale).max(0.8),
                    line_render_mode(sketch_lines),
                );
            }
            GerberPrimitive::Flash {
                position,
                aperture,
                polarity,
                ..
            } => {
                draw_flash(
                    frame,
                    world_to_screen(*position),
                    aperture,
                    scale,
                    polarity_color(*polarity),
                    flash_render_mode(sketch_flashes),
                );
            }
            GerberPrimitive::Region { points, polarity } =>
            {
                if points.len() < 3
                {
                    continue;
                }
                let path = canvas::Path::new(|builder| {
                    builder.move_to(world_to_screen(points[0]));
                    for point in &points[1..]
                    {
                        builder.line_to(world_to_screen(*point));
                    }
                    builder.close();
                });
                paint_polygon_path(
                    frame,
                    &path,
                    polarity_color(*polarity),
                    polygon_render_mode(sketch_polygons),
                );
            }
            GerberPrimitive::DrillHit {
                position,
                diameter,
                ..
            } => {
                frame.fill(
                    &canvas::Path::circle(
                        world_to_screen(*position),
                        (*diameter as f32 * scale / 2.0).max(0.75),
                    ),
                    dark_color,
                );
            }
            GerberPrimitive::DrillSlot {
                start,
                end,
                width,
                ..
            } => {
                frame.stroke(
                    &canvas::Path::line(world_to_screen(*start), world_to_screen(*end)),
                    canvas::Stroke::default()
                        .with_color(dark_color)
                        .with_width((*width as f32 * scale).max(1.0)),
                );
            }
        }
        if d_code_labels_visible(show_d_code_labels, zoom)
        {
            if let Some(label) = d_code_label(primitive)
            {
                let anchor = world_to_screen(label.anchor);
                frame.fill_text(canvas::Text {
                    content: label.content,
                    position: Point::new(anchor.x + 4.0, anchor.y - 4.0),
                    color: d_code_color,
                    size: iced::Pixels(11.0),
                    align_x: iced::alignment::Horizontal::Left.into(),
                    align_y: iced::alignment::Vertical::Bottom,
                    ..canvas::Text::default()
                });
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct DCodeLabel
{
    pub(super) content: String,
    pub(super) anchor: signex_gerber::Point,
}

pub(super) fn d_code_labels_visible(
    show_d_code_labels: bool,
    zoom: f32,
) -> bool
{
    show_d_code_labels && zoom >= 0.75
}

pub(super) fn d_code_label(
    primitive: &GerberPrimitive,
) -> Option<DCodeLabel>
{
    match primitive
    {
        GerberPrimitive::Stroke {
            start,
            end,
            d_code: Some(d_code),
            ..
        } => Some(DCodeLabel {
            content: format!("D{d_code}"),
            anchor: signex_gerber::Point {
                x: (start.x + end.x) / 2.0,
                y: (start.y + end.y) / 2.0,
            },
        }),
        GerberPrimitive::Flash {
            position,
            d_code: Some(d_code),
            ..
        } => Some(DCodeLabel {
            content: format!("D{d_code}"),
            anchor: *position,
        }),
        _ => None,
    }
}

pub(super) fn draw_flash(
    frame: &mut canvas::Frame,
    center: Point,
    aperture: &ApertureShape,
    scale: f32,
    color: Color,
    render_mode: FlashRenderMode,
)
{
    match aperture
    {
        ApertureShape::Circle { diameter } => {
            paint_flash_path(
                frame,
                &canvas::Path::circle(
                    center,
                    (*diameter as f32 * scale / 2.0).max(0.5),
                ),
                color,
                render_mode,
            );
        }
        ApertureShape::Rectangle { width, height }
        | ApertureShape::Obround { width, height } => {
            let size = iced::Size::new(
                (*width as f32 * scale).max(1.0),
                (*height as f32 * scale).max(1.0),
            );
            paint_flash_path(
                frame,
                &canvas::Path::rectangle(
                    Point::new(center.x - size.width / 2.0, center.y - size.height / 2.0),
                    size,
                ),
                color,
                render_mode,
            );
        }
        ApertureShape::Polygon {
            diameter,
            vertices,
            rotation_degrees,
        } => {
            let count = usize::from((*vertices).max(3));
            let radius = *diameter as f32 * scale / 2.0;
            let rotation = (*rotation_degrees as f32).to_radians();
            let path = canvas::Path::new(|builder| {
                for index in 0..count
                {
                    let angle =
                        rotation + std::f32::consts::TAU * index as f32 / count as f32;
                    let point = Point::new(
                        center.x + radius * angle.cos(),
                        center.y - radius * angle.sin(),
                    );
                    if index == 0
                    {
                        builder.move_to(point);
                    }
                    else
                    {
                        builder.line_to(point);
                    }
                }
                builder.close();
            });
            paint_flash_path(frame, &path, color, render_mode);
        }
        ApertureShape::Macro { .. } => {
            paint_flash_path(
                frame,
                &canvas::Path::circle(center, (0.075 * scale).max(1.5)),
                color,
                render_mode,
            );
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FlashRenderMode
{
    Filled,
    Outline,
}

pub(super) fn flash_render_mode(sketch_flashes: bool) -> FlashRenderMode
{
    if sketch_flashes
    {
        FlashRenderMode::Outline
    }
    else
    {
        FlashRenderMode::Filled
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LineRenderMode
{
    Filled,
    Outline,
}

pub(super) fn line_render_mode(sketch_lines: bool) -> LineRenderMode
{
    if sketch_lines
    {
        LineRenderMode::Outline
    }
    else
    {
        LineRenderMode::Filled
    }
}

pub(super) fn line_stroke_widths(
    aperture_width: f32,
    render_mode: LineRenderMode,
) -> (f32, Option<f32>)
{
    let aperture_width = aperture_width.max(0.8);
    let inner_width = match render_mode
    {
        LineRenderMode::Filled => None,
        LineRenderMode::Outline =>
        {
            let inner_width = aperture_width - 2.0;
            (inner_width > 0.0).then_some(inner_width)
        }
    };
    (aperture_width, inner_width)
}

fn paint_line_path(
    frame: &mut canvas::Frame,
    path: &canvas::Path,
    color: Color,
    background: Color,
    aperture_width: f32,
    render_mode: LineRenderMode,
)
{
    let (outer_width, inner_width) =
        line_stroke_widths(aperture_width, render_mode);
    frame.stroke(
        path,
        canvas::Stroke::default()
            .with_color(color)
            .with_width(outer_width)
            .with_line_cap(canvas::LineCap::Round),
    );
    if let Some(inner_width) = inner_width
    {
        frame.stroke(
            path,
            canvas::Stroke::default()
                .with_color(background)
                .with_width(inner_width)
                .with_line_cap(canvas::LineCap::Round),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PolygonRenderMode
{
    Filled,
    Outline,
}

pub(super) fn polygon_render_mode(
    sketch_polygons: bool,
) -> PolygonRenderMode
{
    if sketch_polygons
    {
        PolygonRenderMode::Outline
    }
    else
    {
        PolygonRenderMode::Filled
    }
}

fn paint_polygon_path(
    frame: &mut canvas::Frame,
    path: &canvas::Path,
    color: Color,
    render_mode: PolygonRenderMode,
)
{
    match render_mode
    {
        PolygonRenderMode::Filled => frame.fill(path, color),
        PolygonRenderMode::Outline =>
        {
            frame.stroke(
                path,
                canvas::Stroke::default()
                    .with_color(color)
                    .with_width(1.0),
            );
        }
    }
}

fn paint_flash_path(
    frame: &mut canvas::Frame,
    path: &canvas::Path,
    color: Color,
    render_mode: FlashRenderMode,
)
{
    match render_mode
    {
        FlashRenderMode::Filled => frame.fill(path, color),
        FlashRenderMode::Outline =>
        {
            frame.stroke(
                path,
                canvas::Stroke::default()
                    .with_color(color)
                    .with_width(1.0),
            );
        }
    }
}

pub(super) fn format_coordinate_in_unit(
    point: signex_gerber::Point,
    unit: GerberDisplayUnit,
    decimal_separator: &str,
    polar: bool,
) -> String
{
    if polar
    {
        let radius = point.x.hypot(point.y);
        let angle = point.y.atan2(point.x).to_degrees();
        return format!(
            "R: {} {}  θ: {}°",
            unit.format_value(radius, decimal_separator),
            unit.suffix(),
            format!("{angle:.2}").replace('.', decimal_separator),
        );
    }

    format!(
        "X: {}  Y: {} {}",
        unit.format_value(point.x, decimal_separator),
        unit.format_value(point.y, decimal_separator),
        unit.suffix(),
    )
}

pub(super) fn format_bounds_in_unit(
    bounds: Bounds,
    unit: GerberDisplayUnit,
    decimal_separator: &str,
) -> String
{
    format!(
        "Bounds: X {}…{}  Y {}…{} {}",
        unit.format_value(bounds.min.x, decimal_separator),
        unit.format_value(bounds.max.x, decimal_separator),
        unit.format_value(bounds.min.y, decimal_separator),
        unit.format_value(bounds.max.y, decimal_separator),
        unit.suffix(),
    )
}

pub(super) fn visible_bounds(layers: &[ViewerLayer]) -> Option<Bounds>
{
    layers
        .iter()
        .filter(|layer| layer.visible)
        .filter_map(|layer| layer.layer.geometry.bounds)
        .reduce(|left, right| Bounds {
            min: signex_gerber::Point {
                x: left.min.x.min(right.min.x),
                y: left.min.y.min(right.min.y),
            },
            max: signex_gerber::Point {
                x: left.max.x.max(right.max.x),
                y: left.max.y.max(right.max.y),
            },
        })
}

pub(super) fn page_bounds(
    artwork_bounds: Option<Bounds>,
    page_size: GerberPageSize,
) -> Option<Bounds>
{
    let artwork = artwork_bounds?;
    let Some((width, height)) = page_size.dimensions_millimetres()
    else
    {
        return Some(artwork);
    };
    let center_x = (artwork.min.x + artwork.max.x) * 0.5;
    let center_y = (artwork.min.y + artwork.max.y) * 0.5;
    Some(Bounds {
        min: signex_gerber::Point {
            x: center_x - width * 0.5,
            y: center_y - height * 0.5,
        },
        max: signex_gerber::Point {
            x: center_x + width * 0.5,
            y: center_y + height * 0.5,
        },
    })
}

pub(super) fn draw_page_boundary(
    frame: &mut canvas::Frame,
    page: Bounds,
    scale: f32,
    world_to_screen: &impl Fn(signex_gerber::Point) -> Point,
    color: Color,
)
{
    let top_left = world_to_screen(signex_gerber::Point {
        x: page.min.x,
        y: page.max.y,
    });
    let path = canvas::Path::rectangle(
        top_left,
        iced::Size::new(
            page.width() as f32 * scale,
            page.height() as f32 * scale,
        ),
    );
    frame.stroke(
        &path,
        canvas::Stroke::default()
            .with_width(1.0)
            .with_color(Color { a: 0.7, ..color }),
    );
}

pub(super) fn world_to_screen_point(
    point: signex_gerber::Point,
    world_center: Point,
    screen_center: Point,
    scale: f32,
    mirrored: bool,
) -> Point
{
    let horizontal_direction = if mirrored { -1.0 } else { 1.0 };
    Point::new(
        screen_center.x
            + (point.x as f32 - world_center.x)
                * scale
                * horizontal_direction,
        screen_center.y - (point.y as f32 - world_center.y) * scale,
    )
}

pub(super) fn screen_to_world_point(
    point: Point,
    world_center: Point,
    screen_center: Point,
    scale: f32,
    mirrored: bool,
) -> signex_gerber::Point
{
    let horizontal_direction = if mirrored { -1.0 } else { 1.0 };
    signex_gerber::Point {
        x: f64::from(
            world_center.x
                + (point.x - screen_center.x) / scale
                    * horizontal_direction,
        ),
        y: f64::from(
            world_center.y - (point.y - screen_center.y) / scale,
        ),
    }
}

pub(super) fn fit_transform(
    world_bounds: Bounds,
    viewport: Rectangle,
    zoom: f32,
    pan: iced::Vector,
) -> (f32, Point, Point)
{
    let view_width = (viewport.width - CANVAS_MARGIN * 2.0).max(1.0);
    let view_height = (viewport.height - CANVAS_MARGIN * 2.0).max(1.0);
    let world_width = world_bounds.width().max(0.001) as f32;
    let world_height = world_bounds.height().max(0.001) as f32;
    let fitted_scale = (view_width / world_width).min(view_height / world_height);
    let scale = fitted_scale * zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    let world_center = Point::new(
        ((world_bounds.min.x + world_bounds.max.x) * 0.5) as f32,
        ((world_bounds.min.y + world_bounds.max.y) * 0.5) as f32,
    );
    let screen_center = Point::new(
        viewport.width / 2.0 + pan.x,
        viewport.height / 2.0 + pan.y,
    );
    (scale, world_center, screen_center)
}

pub(super) fn normalized_screen_rectangle(start: Point, end: Point) -> Option<Rectangle>
{
    const MINIMUM_SELECTION_PIXELS: f32 = 4.0;

    let width = (end.x - start.x).abs();
    let height = (end.y - start.y).abs();
    if !width.is_finite()
        || !height.is_finite()
        || width < MINIMUM_SELECTION_PIXELS
        || height < MINIMUM_SELECTION_PIXELS
    {
        return None;
    }
    Some(Rectangle::new(
        Point::new(start.x.min(end.x), start.y.min(end.y)),
        iced::Size::new(width, height),
    ))
}

pub(super) fn zoom_transform_for_selection(
    base_bounds: Bounds,
    selection: Bounds,
    viewport: Rectangle,
) -> Option<(f32, iced::Vector)>
{
    if !selection.is_finite()
        || selection.width() <= 0.0
        || selection.height() <= 0.0
    {
        return None;
    }
    let view_width = (viewport.width - CANVAS_MARGIN * 2.0).max(1.0);
    let view_height = (viewport.height - CANVAS_MARGIN * 2.0).max(1.0);
    let base_scale = (view_width / base_bounds.width().max(0.001) as f32)
        .min(view_height / base_bounds.height().max(0.001) as f32);
    let selection_scale = (view_width / selection.width() as f32)
        .min(view_height / selection.height() as f32);
    let zoom = (selection_scale / base_scale).clamp(MIN_ZOOM, MAX_ZOOM);
    let scale = base_scale * zoom;
    let base_center = Point::new(
        ((base_bounds.min.x + base_bounds.max.x) * 0.5) as f32,
        ((base_bounds.min.y + base_bounds.max.y) * 0.5) as f32,
    );
    let selection_center = Point::new(
        ((selection.min.x + selection.max.x) * 0.5) as f32,
        ((selection.min.y + selection.max.y) * 0.5) as f32,
    );
    Some((
        zoom,
        iced::Vector::new(
            (base_center.x - selection_center.x) * scale,
            (selection_center.y - base_center.y) * scale,
        ),
    ))
}

#[derive(Debug, Deserialize)]
struct LayerPalette
{
    negative_ghost_color: String,
    grid_color: String,
    d_code_color: String,
    compare_colors: Vec<String>,
    layer_colors: Vec<String>,
}

pub(super) fn material_layer_palette() -> Vec<Color>
{
    let palette: LayerPalette = toml::from_str(include_str!(
        "../../../../assets/gerber-viewer/material-layer-colors.toml"
    ))
    .expect("bundled Material Design Gerber layer palette must parse");
    let colors = palette
        .layer_colors
        .iter()
        .filter_map(|value| parse_hex_color(value))
        .collect::<Vec<_>>();
    if colors.is_empty()
    {
        vec![Color::from_rgb8(211, 47, 47)]
    }
    else
    {
        colors
    }
}

pub(super) fn material_negative_ghost_color() -> Color
{
    let palette: LayerPalette = toml::from_str(include_str!(
        "../../../../assets/gerber-viewer/material-layer-colors.toml"
    ))
    .expect("bundled Material Design Gerber layer palette must parse");
    parse_hex_color(&palette.negative_ghost_color)
        .unwrap_or_else(|| Color::from_rgb8(117, 117, 117))
}

pub(super) fn material_grid_color() -> Color
{
    let palette: LayerPalette = toml::from_str(include_str!(
        "../../../../assets/gerber-viewer/material-layer-colors.toml"
    ))
    .expect("bundled Material Design Gerber layer palette must parse");
    parse_hex_color(&palette.grid_color)
        .unwrap_or_else(|| Color::from_rgb8(117, 117, 117))
}

pub(super) fn material_d_code_color() -> Color
{
    let palette: LayerPalette = toml::from_str(include_str!(
        "../../../../assets/gerber-viewer/material-layer-colors.toml"
    ))
    .expect("bundled Material Design Gerber layer palette must parse");
    parse_hex_color(&palette.d_code_color)
        .unwrap_or_else(|| Color::from_rgb8(250, 250, 250))
}

pub(super) fn material_compare_palette() -> Vec<Color>
{
    let palette: LayerPalette = toml::from_str(include_str!(
        "../../../../assets/gerber-viewer/material-layer-colors.toml"
    ))
    .expect("bundled Material Design Gerber layer palette must parse");
    let colors = palette
        .compare_colors
        .iter()
        .filter_map(|value| parse_hex_color(value))
        .collect::<Vec<_>>();
    if colors.is_empty()
    {
        vec![
            Color::from_rgb8(211, 47, 47),
            Color::from_rgb8(0, 188, 212),
        ]
    }
    else
    {
        colors
    }
}

pub(super) fn parse_hex_color(value: &str) -> Option<Color>
{
    let value = value.strip_prefix('#')?;
    if value.len() != 6
    {
        return None;
    }
    let red = u8::from_str_radix(&value[0..2], 16).ok()?;
    let green = u8::from_str_radix(&value[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some(Color::from_rgb8(red, green, blue))
}
