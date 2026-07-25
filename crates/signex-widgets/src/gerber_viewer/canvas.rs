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
    pub(super) highlighted_component: Option<&'a str>,
    pub(super) highlighted_net: Option<&'a str>,
    pub(super) highlighted_attribute: Option<&'a GerberAttributeValue>,
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
        Some(signex_gerber::Point {
            x: f64::from(world_center.x + (screen.x - screen_center.x) / scale),
            y: f64::from(world_center.y - (screen.y - screen_center.y) / scale),
        })
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
            Point::new(
                screen_center.x + (point.x as f32 - world_center.x) * scale,
                screen_center.y - (point.y as f32 - world_center.y) * scale,
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

        for viewer_layer in self.layers.iter().filter(|layer| layer.visible)
        {
            draw_layer(
                &mut frame,
                viewer_layer,
                scale,
                &world_to_screen,
                self.background,
                self.highlighted_component,
                self.highlighted_net,
                self.highlighted_attribute,
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

pub(super) fn draw_layer(
    frame: &mut canvas::Frame,
    viewer_layer: &ViewerLayer,
    scale: f32,
    world_to_screen: &impl Fn(signex_gerber::Point) -> Point,
    background: Color,
    highlighted_component: Option<&str>,
    highlighted_net: Option<&str>,
    highlighted_attribute: Option<&GerberAttributeValue>,
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
            viewer_layer.color,
            attributes,
            highlighted_component,
        );
        let net_color = net_highlight_color(
            component_color,
            attributes,
            highlighted_net,
        );
        let dark_color = attribute_highlight_color(
            net_color,
            attributes,
            highlighted_attribute,
        );
        let polarity_color = |polarity: PrimitivePolarity| match polarity
        {
            PrimitivePolarity::Dark => dark_color,
            PrimitivePolarity::Clear => background,
        };
        match primitive
        {
            GerberPrimitive::Stroke {
                start,
                end,
                width,
                polarity,
                ..
            } => {
                frame.stroke(
                    &canvas::Path::line(world_to_screen(*start), world_to_screen(*end)),
                    canvas::Stroke::default()
                        .with_color(polarity_color(*polarity))
                        .with_width((*width as f32 * scale).max(0.8)),
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
                );
            }
            GerberPrimitive::Region { points, polarity } => {
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
                frame.fill(&path, polarity_color(*polarity));
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
    }
}

pub(super) fn draw_flash(
    frame: &mut canvas::Frame,
    center: Point,
    aperture: &ApertureShape,
    scale: f32,
    color: Color,
)
{
    match aperture
    {
        ApertureShape::Circle { diameter } => {
            frame.fill(
                &canvas::Path::circle(center, (*diameter as f32 * scale / 2.0).max(0.5)),
                color,
            );
        }
        ApertureShape::Rectangle { width, height }
        | ApertureShape::Obround { width, height } => {
            let size = iced::Size::new(
                (*width as f32 * scale).max(1.0),
                (*height as f32 * scale).max(1.0),
            );
            frame.fill(
                &canvas::Path::rectangle(
                    Point::new(center.x - size.width / 2.0, center.y - size.height / 2.0),
                    size,
                ),
                color,
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
            frame.fill(&path, color);
        }
        ApertureShape::Macro { .. } => {
            frame.fill(&canvas::Path::circle(center, (0.075 * scale).max(1.5)), color);
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
