use std::path::PathBuf;

use iced::mouse;
use iced::widget::{
    Space, button, canvas, checkbox, column, container, pick_list, row, scrollable, text,
};
use iced::{Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Theme};
use serde::Deserialize;
use signex_gerber::{
    ApertureShape, Bounds, GerberLoadBatch, GerberPrimitive, LoadedLayer, PrimitivePolarity,
};
use signex_types::theme::ThemeTokens;

mod grid;

use grid::{
    DEFAULT_GRID_INDEX, GridSizePreset, default_grid_catalog, grid_size_choices,
    system_decimal_separator,
};

const MAX_VIEWER_LAYERS: usize = 32;
const CANVAS_MARGIN: f32 = 28.0;
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 30.0;

#[derive(Debug, Clone)]
pub enum GerberViewerMessage
{
    OpenGerberFiles,
    GerberFilesChosen(Option<Vec<PathBuf>>),
    GerberFilesLoaded(GerberLoadBatch),
    OpenExcellonFiles,
    ExcellonFilesChosen(Option<Vec<PathBuf>>),
    ExcellonFilesLoaded(GerberLoadBatch),
    SelectLayer(usize),
    SetLayerVisible(usize, bool),
    ClearCurrentLayer,
    ClearAllLayers,
    RedrawViewport,
    ZoomBy(f32),
    PanBy(iced::Vector),
    FitPage,
    ToggleLayerManager,
    SelectGridSize(usize),
}

#[derive(Debug, Clone)]
pub struct ViewerLayer
{
    pub layer: LoadedLayer,
    pub visible: bool,
    pub color: Color,
}

#[derive(Debug)]
pub struct GerberViewerState
{
    pub layers: Vec<ViewerLayer>,
    pub active_layer: Option<usize>,
    pub loading: bool,
    pub status: String,
    pub redraw_generation: u64,
    pub zoom: f32,
    pub pan: iced::Vector,
    pub layer_manager_visible: bool,
    grid_catalog: Vec<GridSizePreset>,
    active_grid_index: usize,
    decimal_separator: String,
    palette: Vec<Color>,
}

impl Default for GerberViewerState
{
    fn default() -> Self
    {
        Self {
            layers: Vec::new(),
            active_layer: None,
            loading: false,
            status: "Open one or more Gerber files to begin.".into(),
            redraw_generation: 0,
            zoom: 1.0,
            pan: iced::Vector::default(),
            layer_manager_visible: true,
            grid_catalog: default_grid_catalog(),
            active_grid_index: DEFAULT_GRID_INDEX,
            decimal_separator: system_decimal_separator(),
            palette: material_layer_palette(),
        }
    }
}

impl GerberViewerState
{
    pub fn begin_loading(&mut self)
    {
        self.loading = true;
        self.status = "Loading fabrication files…".into();
    }

    pub fn apply_load_batch(&mut self, batch: GerberLoadBatch)
    {
        self.loading = false;
        let remaining = MAX_VIEWER_LAYERS.saturating_sub(self.layers.len());
        let loaded_count = batch.layers.len().min(remaining);
        let skipped_count = batch.layers.len().saturating_sub(loaded_count);

        for layer in batch.layers.into_iter().take(loaded_count)
        {
            let color_index = self.layers.len() % self.palette.len();
            self.layers.push(ViewerLayer {
                layer,
                visible: true,
                color: self.palette[color_index],
            });
        }

        if loaded_count > 0
        {
            self.active_layer = Some(self.layers.len() - 1);
        }

        let mut messages = Vec::new();
        if loaded_count > 0
        {
            messages.push(format!("Loaded {loaded_count} fabrication layer(s)."));
        }
        if !batch.failures.is_empty()
        {
            let failures = batch
                .failures
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" | ");
            messages.push(format!(
                "{} file(s) could not be loaded: {failures}",
                batch.failures.len()
            ));
        }
        if skipped_count > 0
        {
            messages.push(format!(
                "{skipped_count} layer(s) were skipped because the prototype supports at most \
                 {MAX_VIEWER_LAYERS} layers."
            ));
        }
        if messages.is_empty()
        {
            messages.push("No files were selected.".into());
        }
        self.status = messages.join(" ");
    }

    pub fn clear_current_layer(&mut self)
    {
        let Some(index) = self.active_layer else
        {
            return;
        };
        self.layers.remove(index);
        self.active_layer = if self.layers.is_empty()
        {
            None
        }
        else
        {
            Some(index.min(self.layers.len() - 1))
        };
        self.status = "Cleared the current layer.".into();
    }

    pub fn clear_all_layers(&mut self)
    {
        self.layers.clear();
        self.active_layer = None;
        self.zoom = 1.0;
        self.pan = iced::Vector::default();
        self.status = "Cleared all Gerber layers.".into();
    }

    pub fn redraw_viewport(&mut self)
    {
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
        self.status = "Gerber viewport redrawn.".into();
    }

    pub fn zoom_by(&mut self, factor: f32)
    {
        if factor.is_finite() && factor > 0.0
        {
            self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
            self.status = format!("Zoom: {:.0}%", self.zoom * 100.0);
        }
    }

    pub fn pan_by(&mut self, delta: iced::Vector)
    {
        if delta.x.is_finite() && delta.y.is_finite()
        {
            self.pan = self.pan + delta;
        }
    }

    pub fn fit_page(&mut self)
    {
        self.zoom = 1.0;
        self.pan = iced::Vector::default();
        self.status = "Fit page to viewport.".into();
    }

    pub fn toggle_layer_manager(&mut self)
    {
        self.layer_manager_visible = !self.layer_manager_visible;
    }

    pub fn set_layer_visible(&mut self, index: usize, visible: bool)
    {
        if let Some(layer) = self.layers.get_mut(index)
        {
            layer.visible = visible;
        }
    }

    pub fn select_layer(&mut self, index: usize)
    {
        if index < self.layers.len()
        {
            self.active_layer = Some(index);
        }
    }

    pub fn select_grid_size(&mut self, index: usize)
    {
        let Some(grid) = self.grid_catalog.get(index) else
        {
            return;
        };

        self.active_grid_index = index;
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
        self.status = format!(
            "Grid: {}",
            grid.display_label(&self.decimal_separator),
        );
    }

    fn active_grid(&self) -> &GridSizePreset
    {
        &self.grid_catalog[self.active_grid_index]
    }
}

pub fn view<'a>(
    state: &'a GerberViewerState,
    tokens: &ThemeTokens,
) -> Element<'a, GerberViewerMessage>
{
    let text_primary = crate::styles::ti(tokens.text);
    let text_muted = crate::styles::ti(tokens.text_secondary);
    let border = crate::styles::ti(tokens.border);
    let panel_bg = crate::styles::ti(tokens.panel_bg);
    let canvas_bg = crate::styles::ti(tokens.bg);

    let open_button = button(text(if state.loading
    {
        "Loading…"
    }
    else
    {
        "Open Gerber Files…"
    }))
    .on_press_maybe((!state.loading).then_some(GerberViewerMessage::OpenGerberFiles));
    let clear_current = button(text("Clear Current")).on_press_maybe(
        state
            .active_layer
            .map(|_| GerberViewerMessage::ClearCurrentLayer),
    );
    let clear_all = button(text("Clear All")).on_press_maybe(
        (!state.layers.is_empty()).then_some(GerberViewerMessage::ClearAllLayers),
    );

    let toolbar = container(
        row![
            open_button,
            button(text("Open Drill Files…"))
                .on_press_maybe((!state.loading).then_some(
                    GerberViewerMessage::OpenExcellonFiles,
                )),
            button(text("Redraw")).on_press(GerberViewerMessage::RedrawViewport),
            button(text("−")).on_press(GerberViewerMessage::ZoomBy(1.0 / 1.2)),
            button(text("+")).on_press(GerberViewerMessage::ZoomBy(1.2)),
            button(text("Fit")).on_press(GerberViewerMessage::FitPage),
            button(text(if state.layer_manager_visible
            {
                "Hide Layers"
            }
            else
            {
                "Show Layers"
            }))
            .on_press(GerberViewerMessage::ToggleLayerManager),
            clear_current,
            clear_all,
            Space::new().width(Length::Fill),
            text(format!("{} / {MAX_VIEWER_LAYERS} layers", state.layers.len()))
                .size(11)
                .color(text_muted),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([6, 10])
    .width(Length::Fill)
    .style(crate::styles::toolbar_strip(tokens));

    let grid_choices = grid_size_choices(
        &state.grid_catalog,
        &state.decimal_separator,
    );
    let selected_grid = grid_choices.get(state.active_grid_index).cloned();
    let grid_picker = pick_list(grid_choices, selected_grid, |choice| {
        GerberViewerMessage::SelectGridSize(choice.index)
    })
    .width(320);
    let grid_toolbar = container(
        row![
            text("Grid").size(11).color(text_muted),
            grid_picker,
            text(format!(
                "X: {:.4} mm  Y: {:.4} mm",
                state.active_grid().x_millimetres(),
                state.active_grid().y_millimetres(),
            ))
            .size(10)
            .color(text_muted),
            Space::new().width(Length::Fill),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([4, 10])
    .width(Length::Fill)
    .style(crate::styles::toolbar_strip(tokens));

    let mut layer_list = column![
        text("Layers").size(13).color(text_primary),
        container(Space::new())
            .width(Length::Fill)
            .height(1)
            .style(crate::styles::chrome_separator(tokens)),
    ]
    .spacing(6);

    if state.layers.is_empty()
    {
        layer_list = layer_list.push(
            text("No loaded layers")
                .size(11)
                .color(text_muted),
        );
    }
    else
    {
        for (index, viewer_layer) in state.layers.iter().enumerate().rev()
        {
            let active = state.active_layer == Some(index);
            let color = viewer_layer.color;
            let color_chip = container(Space::new())
                .width(12)
                .height(12)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(color)),
                    border: Border {
                        width: 1.0,
                        radius: 2.0.into(),
                        color,
                    },
                    ..container::Style::default()
                });
            let visible = checkbox(viewer_layer.visible)
                .size(14)
                .on_toggle(move |visible| {
                    GerberViewerMessage::SetLayerVisible(index, visible)
                });
            let label = button(
                row![
                    color_chip,
                    text(&viewer_layer.layer.name)
                        .size(11)
                        .color(text_primary)
                        .wrapping(text::Wrapping::None),
                ]
                .spacing(7)
                .align_y(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .padding([5, 6])
            .on_press(GerberViewerMessage::SelectLayer(index))
            .style(crate::styles::rail_tab(tokens, active));
            layer_list = layer_list.push(
                row![visible, label]
                    .spacing(5)
                    .align_y(iced::Alignment::Center),
            );
        }
    }

    let sidebar = container(scrollable(layer_list.padding(8)))
        .width(260)
        .height(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(panel_bg)),
            border: Border {
                width: 1.0,
                radius: 0.0.into(),
                color: border,
            },
            text_color: Some(text_primary),
            ..container::Style::default()
        });

    let canvas_widget: Element<'_, GerberViewerMessage> = canvas(GerberCanvas {
        layers: &state.layers,
        background: canvas_bg,
        grid: crate::styles::ti(tokens.text_secondary),
        redraw_generation: state.redraw_generation,
        zoom: state.zoom,
        pan: state.pan,
        grid_size: state.active_grid(),
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into();

    let canvas_panel = container(canvas_widget)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(canvas_bg)),
            ..container::Style::default()
        });
    let content: Element<'_, GerberViewerMessage> = if state.layer_manager_visible
    {
        row![canvas_panel, sidebar]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    else
    {
        canvas_panel.into()
    };

    let status = container(
        text(&state.status)
            .size(10)
            .color(if state.status.contains("could not")
            {
                Color::from_rgb8(239, 83, 80)
            }
            else
            {
                text_muted
            }),
    )
    .padding([4, 10])
    .width(Length::Fill)
    .style(crate::styles::status_bar(tokens));

    column![
        toolbar,
        grid_toolbar,
        content,
        status,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[derive(Debug, Default)]
struct GerberCanvasState
{
    drag_start: Option<Point>,
}

struct GerberCanvas<'a>
{
    layers: &'a [ViewerLayer],
    background: Color,
    grid: Color,
    grid_size: &'a GridSizePreset,
    redraw_generation: u64,
    zoom: f32,
    pan: iced::Vector,
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
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let Some(previous) = state.drag_start else
                {
                    return None;
                };
                let current = Point::new(position.x - bounds.x, position.y - bounds.y);
                state.drag_start = Some(current);
                Some(
                    canvas::Action::publish(GerberViewerMessage::PanBy(current - previous))
                        .and_capture(),
                )
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                state.drag_start = None;
                Some(canvas::Action::capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry>
    {
        let _redraw_generation = self.redraw_generation;
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.background);

        let Some(world_bounds) = visible_bounds(self.layers) else
        {
            draw_grid(
                &mut frame,
                bounds,
                self.grid,
                self.grid_size,
                32.0 / 1.27,
                Point::new(bounds.width / 2.0, bounds.height / 2.0),
            );
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
        draw_grid(
            &mut frame,
            bounds,
            self.grid,
            self.grid_size,
            scale,
            world_to_screen(signex_gerber::Point { x: 0.0, y: 0.0 }),
        );

        for viewer_layer in self.layers.iter().filter(|layer| layer.visible)
        {
            draw_layer(
                &mut frame,
                viewer_layer,
                scale,
                &world_to_screen,
                self.background,
            );
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

fn draw_grid(
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

fn visible_grid_spacing(spacing: f32) -> Option<f32>
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

fn draw_layer(
    frame: &mut canvas::Frame,
    viewer_layer: &ViewerLayer,
    scale: f32,
    world_to_screen: &impl Fn(signex_gerber::Point) -> Point,
    background: Color,
)
{
    for primitive in &viewer_layer.layer.geometry.primitives
    {
        let polarity_color = |polarity: PrimitivePolarity| match polarity
        {
            PrimitivePolarity::Dark => viewer_layer.color,
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
                    viewer_layer.color,
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
                        .with_color(viewer_layer.color)
                        .with_width((*width as f32 * scale).max(1.0)),
                );
            }
        }
    }
}

fn draw_flash(
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

fn visible_bounds(layers: &[ViewerLayer]) -> Option<Bounds>
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

fn fit_transform(
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

#[derive(Debug, Deserialize)]
struct LayerPalette
{
    layer_colors: Vec<String>,
}

fn material_layer_palette() -> Vec<Color>
{
    let palette: LayerPalette = toml::from_str(include_str!(
        "../../../assets/gerber-viewer/material-layer-colors.toml"
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

fn parse_hex_color(value: &str) -> Option<Color>
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

#[cfg(test)]
mod tests
{
    use std::io::Cursor;

    use super::*;

    #[test]
    fn bundled_palette_contains_material_colors_for_every_layer_slot()
    {
        let palette = material_layer_palette();

        assert_eq!(palette.len(), 64);
        assert_eq!(palette[0], Color::from_rgb8(211, 47, 47));
    }

    #[test]
    fn selecting_grid_updates_rectangular_viewport_spacing()
    {
        let mut state = GerberViewerState::default();
        state.decimal_separator = ",".to_owned();
        let initial_generation = state.redraw_generation;

        assert_eq!(state.active_grid_index, DEFAULT_GRID_INDEX);
        assert_eq!(state.active_grid().x_millimetres(), 1.27);
        assert_eq!(state.active_grid().y_millimetres(), 1.27);

        state.select_grid_size(13);

        assert_eq!(state.active_grid_index, 13);
        assert_eq!(state.active_grid().x_millimetres(), 1.5);
        assert_eq!(state.active_grid().y_millimetres(), 2.5);
        assert_eq!(state.redraw_generation, initial_generation + 1);
        assert_eq!(
            state.status,
            "Grid: 1,5000 mm ⨯ 2,5000 mm (59,06 mils ⨯ 98,43 mils)"
        );
    }

    #[test]
    fn zero_grid_axis_is_not_replaced_with_the_other_axis()
    {
        let mut state = GerberViewerState::default();

        state.select_grid_size(19);

        assert_eq!(state.active_grid().x_millimetres(), 0.05);
        assert_eq!(state.active_grid().y_millimetres(), 0.0);
        assert!(visible_grid_spacing(0.05 * 100.0).is_some());
        assert_eq!(visible_grid_spacing(0.0), None);
    }

    #[test]
    fn layer_limit_accepts_32_mixed_layers_and_reports_overflow()
    {
        let mut state = GerberViewerState::default();
        let gerber = signex_gerber::load_gerber_reader(
            "copper.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let drill = signex_gerber::load_excellon_reader(
            "holes.drl",
            Cursor::new(
                b"M48\nMETRIC\nT01C0.8\n%\nG05\nT01\nX1.0Y1.0\nM30\n",
            ),
        )
        .expect("test Excellon must parse");
        let mut mixed = Vec::new();
        for _ in 0..16
        {
            mixed.push(gerber.clone());
            mixed.push(drill.clone());
        }

        state.apply_load_batch(GerberLoadBatch {
            layers: mixed,
            failures: Vec::new(),
        });
        assert_eq!(state.layers.len(), MAX_VIEWER_LAYERS);
        assert_eq!(
            state
                .layers
                .iter()
                .filter(|layer| layer.layer.layer_type == signex_gerber::LayerType::Drill)
                .count(),
            16
        );

        state.apply_load_batch(GerberLoadBatch {
            layers: vec![gerber],
            failures: Vec::new(),
        });
        assert_eq!(state.layers.len(), MAX_VIEWER_LAYERS);
        assert!(state.status.contains("1 layer(s) were skipped"));
    }

    #[test]
    fn redraw_changes_only_the_viewport_generation()
    {
        let mut state = GerberViewerState::default();
        let status_before = state.status.clone();
        let layer_count_before = state.layers.len();

        state.redraw_viewport();

        assert_eq!(state.redraw_generation, 1);
        assert_eq!(state.layers.len(), layer_count_before);
        assert_ne!(state.status, status_before);
    }

    #[test]
    fn zoom_updates_scale_and_clamps_to_safe_limits()
    {
        let mut state = GerberViewerState::default();

        state.zoom_by(2.0);
        assert_eq!(state.zoom, 2.0);

        state.zoom_by(100.0);
        assert_eq!(state.zoom, MAX_ZOOM);

        state.zoom_by(0.0001);
        assert_eq!(state.zoom, MIN_ZOOM);

        state.zoom_by(f32::NAN);
        assert_eq!(state.zoom, MIN_ZOOM);
    }

    #[test]
    fn fit_page_resets_navigation_and_computes_valid_landscape_transform()
    {
        let bounds = Bounds {
            min: signex_gerber::Point { x: 10.0, y: 20.0 },
            max: signex_gerber::Point { x: 110.0, y: 70.0 },
        };
        let viewport = Rectangle::new(Point::ORIGIN, iced::Size::new(1000.0, 700.0));
        let (scale, world_center, screen_center) =
            fit_transform(bounds, viewport, 1.0, iced::Vector::default());

        assert!(scale.is_finite() && scale > 0.0);
        assert_eq!(world_center, Point::new(60.0, 45.0));
        assert_eq!(screen_center, Point::new(500.0, 350.0));
        assert!(bounds.width() as f32 * scale <= viewport.width - CANVAS_MARGIN * 2.0 + 0.01);
        assert!(bounds.height() as f32 * scale <= viewport.height - CANVAS_MARGIN * 2.0 + 0.01);

        let mut state = GerberViewerState::default();
        state.zoom = 4.0;
        state.pan = iced::Vector::new(100.0, -50.0);
        state.fit_page();
        assert_eq!(state.zoom, 1.0);
        assert_eq!(state.pan, iced::Vector::default());
    }

    #[test]
    fn toggling_layer_manager_preserves_viewer_state()
    {
        let mut state = GerberViewerState::default();
        state.zoom = 2.0;
        state.active_layer = None;

        state.toggle_layer_manager();

        assert!(!state.layer_manager_visible);
        assert_eq!(state.zoom, 2.0);
        assert!(state.layers.is_empty());

        state.toggle_layer_manager();
        assert!(state.layer_manager_visible);
    }

    #[test]
    fn hidden_layer_remains_loaded_but_is_excluded_from_visible_bounds()
    {
        let layer = signex_gerber::load_gerber_reader(
            "copper.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![layer],
            failures: Vec::new(),
        });

        state.set_layer_visible(0, false);
        assert_eq!(state.layers.len(), 1);
        assert!(!state.layers[0].visible);
        assert!(visible_bounds(&state.layers).is_none());

        state.set_layer_visible(0, true);
        assert!(visible_bounds(&state.layers).is_some());
    }

    #[test]
    fn selecting_active_layer_does_not_change_visibility()
    {
        let layer = signex_gerber::load_gerber_reader(
            "copper.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![layer.clone(), layer],
            failures: Vec::new(),
        });
        state.set_layer_visible(0, false);

        state.select_layer(0);
        assert_eq!(state.active_layer, Some(0));
        assert!(!state.layers[0].visible);
        assert!(state.layers[1].visible);

        state.select_layer(10);
        assert_eq!(state.active_layer, Some(0));
    }

    #[test]
    fn clearing_current_layer_preserves_siblings_and_selects_next_layer()
    {
        let layer = signex_gerber::load_gerber_reader(
            "layer.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut first = layer.clone();
        first.name = "first.gbr".into();
        let mut second = layer.clone();
        second.name = "second.gbr".into();
        let mut third = layer;
        third.name = "third.gbr".into();
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![first, second, third],
            failures: Vec::new(),
        });
        state.select_layer(1);

        state.clear_current_layer();

        assert_eq!(state.layers.len(), 2);
        assert_eq!(state.layers[0].layer.name, "first.gbr");
        assert_eq!(state.layers[1].layer.name, "third.gbr");
        assert_eq!(state.active_layer, Some(1));
    }

    #[test]
    fn clearing_all_layers_restores_empty_view()
    {
        let layer = signex_gerber::load_gerber_reader(
            "copper.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![layer],
            failures: Vec::new(),
        });
        state.zoom = 3.0;
        state.pan = iced::Vector::new(50.0, 20.0);

        state.clear_all_layers();

        assert!(state.layers.is_empty());
        assert_eq!(state.active_layer, None);
        assert_eq!(state.zoom, 1.0);
        assert_eq!(state.pan, iced::Vector::default());
    }
}
