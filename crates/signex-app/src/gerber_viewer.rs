use std::{fmt, path::PathBuf};

use iced::mouse;
use iced::widget::{
    Space, button, canvas, checkbox, column, container, pick_list, row, scrollable, text,
    text_input,
};
use iced::{Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Theme};
use serde::Deserialize;
use signex_gerber::{
    ApertureShape, Bounds, GerberLoadBatch, GerberPrimitive, LoadedLayer, PrimitivePolarity,
};
use signex_types::theme::ThemeTokens;

mod grid;

use grid::{
    DEFAULT_GRID_INDEX, GridSizePreset, GridUnit, create_grid_definition,
    format_distance_input, grid_size_choices, load_grid_catalog, persist_grid_catalog,
    system_decimal_separator,
};

const MAX_VIEWER_LAYERS: usize = 32;
const CANVAS_MARGIN: f32 = 28.0;
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GerberDisplayUnit
{
    Inches,
    Mils,
    Millimetres,
}

impl GerberDisplayUnit
{
    const ALL: [Self; 3] = [Self::Inches, Self::Mils, Self::Millimetres];

    fn value_from_millimetres(self, value: f64) -> f64
    {
        match self
        {
            Self::Inches => value / 25.4,
            Self::Mils => value / 0.0254,
            Self::Millimetres => value,
        }
    }

    fn decimal_places(self) -> usize
    {
        match self
        {
            Self::Inches | Self::Millimetres => 4,
            Self::Mils => 2,
        }
    }

    fn suffix(self) -> &'static str
    {
        match self
        {
            Self::Inches => "in",
            Self::Mils => "mils",
            Self::Millimetres => "mm",
        }
    }

    fn format_value(self, millimetres: f64, decimal_separator: &str) -> String
    {
        let value = self.value_from_millimetres(millimetres);
        let decimals = self.decimal_places();
        format!("{value:.decimals$}").replace('.', decimal_separator)
    }
}

impl fmt::Display for GerberDisplayUnit
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        formatter.write_str(match self
        {
            Self::Inches => "Inches",
            Self::Mils => "Mils",
            Self::Millimetres => "Millimetres",
        })
    }
}

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
    ToggleGridEditor,
    NewGridNameChanged(String),
    NewGridXChanged(String),
    NewGridYChanged(String),
    SetNewGridUnitMillimetres(bool),
    CreateGridDefinition,
    DeleteGridDefinition,
    MoveGridUp,
    MoveGridDown,
    EditGridNameChanged(String),
    EditGridXChanged(String),
    EditGridYChanged(String),
    SetEditGridUnitMillimetres(bool),
    UpdateGridDefinition,
    ToggleGridVisibility(bool),
    SetDisplayUnit(GerberDisplayUnit),
    CursorWorldPositionChanged(Option<signex_gerber::Point>),
    TogglePolarCoordinates(bool),
    ToggleFullWindowCrosshair(bool),
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
    grid_visible: bool,
    display_unit: GerberDisplayUnit,
    cursor_world_position: Option<signex_gerber::Point>,
    polar_coordinates: bool,
    full_window_crosshair: bool,
    decimal_separator: String,
    grid_editor_open: bool,
    new_grid_name: String,
    new_grid_x: String,
    new_grid_y: String,
    new_grid_unit: GridUnit,
    edit_grid_name: String,
    edit_grid_x: String,
    edit_grid_y: String,
    edit_grid_unit: GridUnit,
    grid_editor_error: Option<String>,
    palette: Vec<Color>,
}

impl Default for GerberViewerState
{
    fn default() -> Self
    {
        let grid_catalog = load_grid_catalog();
        let active_grid_index = DEFAULT_GRID_INDEX.min(grid_catalog.len() - 1);
        let decimal_separator = system_decimal_separator();
        let active_grid = &grid_catalog[active_grid_index];
        let edit_grid_name = active_grid.name.clone().unwrap_or_default();
        let edit_grid_x =
            format_distance_input(active_grid.x, &decimal_separator);
        let edit_grid_y =
            format_distance_input(active_grid.y, &decimal_separator);
        let edit_grid_unit = active_grid.unit;
        Self {
            layers: Vec::new(),
            active_layer: None,
            loading: false,
            status: "Open one or more Gerber files to begin.".into(),
            redraw_generation: 0,
            zoom: 1.0,
            pan: iced::Vector::default(),
            layer_manager_visible: true,
            grid_catalog,
            active_grid_index,
            grid_visible: true,
            display_unit: GerberDisplayUnit::Millimetres,
            cursor_world_position: None,
            polar_coordinates: false,
            full_window_crosshair: false,
            decimal_separator: decimal_separator.clone(),
            grid_editor_open: false,
            new_grid_name: String::new(),
            new_grid_x: String::new(),
            new_grid_y: String::new(),
            new_grid_unit: GridUnit::Mil,
            edit_grid_name,
            edit_grid_x,
            edit_grid_y,
            edit_grid_unit,
            grid_editor_error: None,
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
        let Some(label) = self
            .grid_catalog
            .get(index)
            .map(|grid| grid.display_label(&self.decimal_separator))
        else
        {
            return;
        };

        self.active_grid_index = index;
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
        self.status = format!("Grid: {label}");
        self.load_active_grid_into_editor();
    }

    pub fn set_grid_visible(&mut self, visible: bool)
    {
        if self.grid_visible != visible
        {
            self.grid_visible = visible;
            self.redraw_generation = self.redraw_generation.wrapping_add(1);
        }
    }

    pub fn set_display_unit(&mut self, unit: GerberDisplayUnit)
    {
        self.display_unit = unit;
    }

    pub fn set_cursor_world_position(
        &mut self,
        position: Option<signex_gerber::Point>,
    )
    {
        self.cursor_world_position = position;
    }

    pub fn set_polar_coordinates(&mut self, polar: bool)
    {
        self.polar_coordinates = polar;
    }

    pub fn set_full_window_crosshair(&mut self, full_window: bool)
    {
        if self.full_window_crosshair != full_window
        {
            self.full_window_crosshair = full_window;
            self.redraw_generation = self.redraw_generation.wrapping_add(1);
        }
    }

    fn active_grid(&self) -> &GridSizePreset
    {
        &self.grid_catalog[self.active_grid_index]
    }

    pub fn toggle_grid_editor(&mut self)
    {
        self.grid_editor_open = !self.grid_editor_open;
        if self.grid_editor_open
        {
            self.load_active_grid_into_editor();
        }
        self.grid_editor_error = None;
    }

    pub fn set_new_grid_name(&mut self, value: String)
    {
        self.new_grid_name = value;
        self.grid_editor_error = None;
    }

    pub fn set_new_grid_x(&mut self, value: String)
    {
        self.new_grid_x = value;
        self.grid_editor_error = None;
    }

    pub fn set_new_grid_y(&mut self, value: String)
    {
        self.new_grid_y = value;
        self.grid_editor_error = None;
    }

    pub fn set_new_grid_unit_millimetres(&mut self, millimetres: bool)
    {
        self.new_grid_unit = if millimetres
        {
            GridUnit::Mm
        }
        else
        {
            GridUnit::Mil
        };
        self.grid_editor_error = None;
    }

    pub fn set_edit_grid_name(&mut self, value: String)
    {
        self.edit_grid_name = value;
        self.grid_editor_error = None;
    }

    pub fn set_edit_grid_x(&mut self, value: String)
    {
        self.edit_grid_x = value;
        self.grid_editor_error = None;
    }

    pub fn set_edit_grid_y(&mut self, value: String)
    {
        self.edit_grid_y = value;
        self.grid_editor_error = None;
    }

    pub fn set_edit_grid_unit_millimetres(&mut self, millimetres: bool)
    {
        let target_unit = if millimetres
        {
            GridUnit::Mm
        }
        else
        {
            GridUnit::Mil
        };
        if target_unit == self.edit_grid_unit
        {
            return;
        }

        let draft = match create_grid_definition(
            &self.edit_grid_name,
            &self.edit_grid_x,
            &self.edit_grid_y,
            self.edit_grid_unit,
            &self.decimal_separator,
        )
        {
            Ok(draft) => draft.converted_to(target_unit),
            Err(error) => {
                self.grid_editor_error = Some(error);
                return;
            }
        };
        self.edit_grid_x =
            format_distance_input(draft.x, &self.decimal_separator);
        self.edit_grid_y =
            format_distance_input(draft.y, &self.decimal_separator);
        self.edit_grid_unit = target_unit;
        self.grid_editor_error = None;
    }

    pub fn create_grid(&mut self)
    {
        self.create_grid_with(persist_grid_catalog);
    }

    pub fn update_grid(&mut self)
    {
        self.update_grid_with(persist_grid_catalog);
    }

    pub fn delete_grid(&mut self)
    {
        self.delete_grid_with(persist_grid_catalog);
    }

    pub fn move_grid_up(&mut self)
    {
        self.move_grid_up_with(persist_grid_catalog);
    }

    pub fn move_grid_down(&mut self)
    {
        self.move_grid_down_with(persist_grid_catalog);
    }

    fn create_grid_with(
        &mut self,
        persist: impl FnOnce(&[GridSizePreset]) -> Result<(), String>,
    )
    {
        let grid = match create_grid_definition(
            &self.new_grid_name,
            &self.new_grid_x,
            &self.new_grid_y,
            self.new_grid_unit,
            &self.decimal_separator,
        )
        {
            Ok(grid) => grid,
            Err(error) => {
                self.grid_editor_error = Some(error);
                return;
            }
        };
        let mut catalog = self.grid_catalog.clone();
        catalog.push(grid);
        if let Err(error) = persist(&catalog)
        {
            self.grid_editor_error = Some(error);
            return;
        }

        self.grid_catalog = catalog;
        self.active_grid_index = self.grid_catalog.len() - 1;
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
        self.status = format!(
            "Created grid: {}",
            self.active_grid()
                .display_label(&self.decimal_separator),
        );
        self.new_grid_name.clear();
        self.new_grid_x.clear();
        self.new_grid_y.clear();
        self.load_active_grid_into_editor();
        self.grid_editor_error = None;
    }

    fn update_grid_with(
        &mut self,
        persist: impl FnOnce(&[GridSizePreset]) -> Result<(), String>,
    )
    {
        let grid = match create_grid_definition(
            &self.edit_grid_name,
            &self.edit_grid_x,
            &self.edit_grid_y,
            self.edit_grid_unit,
            &self.decimal_separator,
        )
        {
            Ok(grid) => grid,
            Err(error) => {
                self.grid_editor_error = Some(error);
                return;
            }
        };
        let mut catalog = self.grid_catalog.clone();
        catalog[self.active_grid_index] = grid;
        if let Err(error) = persist(&catalog)
        {
            self.grid_editor_error = Some(error);
            return;
        }

        self.grid_catalog = catalog;
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
        self.status = format!(
            "Updated grid: {}",
            self.active_grid()
                .display_label(&self.decimal_separator),
        );
        self.load_active_grid_into_editor();
        self.grid_editor_error = None;
    }

    fn delete_grid_with(
        &mut self,
        persist: impl FnOnce(&[GridSizePreset]) -> Result<(), String>,
    )
    {
        if self.grid_catalog.len() <= 1
        {
            self.grid_editor_error =
                Some("At least one grid definition must remain.".to_owned());
            return;
        }

        let removed_label = self.active_grid().display_label(&self.decimal_separator);
        let mut catalog = self.grid_catalog.clone();
        catalog.remove(self.active_grid_index);
        if let Err(error) = persist(&catalog)
        {
            self.grid_editor_error = Some(error);
            return;
        }

        self.grid_catalog = catalog;
        self.active_grid_index =
            self.active_grid_index.min(self.grid_catalog.len() - 1);
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
        self.status = format!("Deleted grid: {removed_label}");
        self.load_active_grid_into_editor();
        self.grid_editor_error = None;
    }

    fn move_grid_up_with(
        &mut self,
        persist: impl FnOnce(&[GridSizePreset]) -> Result<(), String>,
    )
    {
        let Some(target_index) = self.active_grid_index.checked_sub(1) else
        {
            return;
        };
        self.move_grid_to_with(target_index, persist);
    }

    fn move_grid_down_with(
        &mut self,
        persist: impl FnOnce(&[GridSizePreset]) -> Result<(), String>,
    )
    {
        let target_index = self.active_grid_index + 1;
        if target_index >= self.grid_catalog.len()
        {
            return;
        }
        self.move_grid_to_with(target_index, persist);
    }

    fn move_grid_to_with(
        &mut self,
        target_index: usize,
        persist: impl FnOnce(&[GridSizePreset]) -> Result<(), String>,
    )
    {
        let moved_label = self.active_grid().display_label(&self.decimal_separator);
        let mut catalog = self.grid_catalog.clone();
        catalog.swap(self.active_grid_index, target_index);
        if let Err(error) = persist(&catalog)
        {
            self.grid_editor_error = Some(error);
            return;
        }

        self.grid_catalog = catalog;
        self.active_grid_index = target_index;
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
        self.status = format!("Moved grid: {moved_label}");
        self.load_active_grid_into_editor();
        self.grid_editor_error = None;
    }

    fn load_active_grid_into_editor(&mut self)
    {
        let grid = self.active_grid().clone();
        self.edit_grid_name = grid.name.unwrap_or_default();
        self.edit_grid_x =
            format_distance_input(grid.x, &self.decimal_separator);
        self.edit_grid_y =
            format_distance_input(grid.y, &self.decimal_separator);
        self.edit_grid_unit = grid.unit;
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
    let display_unit_picker = pick_list(
        GerberDisplayUnit::ALL,
        Some(state.display_unit),
        GerberViewerMessage::SetDisplayUnit,
    )
    .width(125);
    let cursor_label = state
        .cursor_world_position
        .map(|position| {
            format_coordinate_in_unit(
                position,
                state.display_unit,
                &state.decimal_separator,
                state.polar_coordinates,
            )
        })
        .unwrap_or_else(|| "X: —  Y: —".to_owned());
    let bounds_label = visible_bounds(&state.layers)
        .map(|bounds| {
            format_bounds_in_unit(
                bounds,
                state.display_unit,
                &state.decimal_separator,
            )
        })
        .unwrap_or_else(|| "Bounds: —".to_owned());
    let grid_toolbar = container(
        row![
            text("Grid").size(11).color(text_muted),
            checkbox(state.grid_visible)
                .label("Visible")
                .on_toggle(GerberViewerMessage::ToggleGridVisibility),
            grid_picker,
            display_unit_picker,
            checkbox(state.polar_coordinates)
                .label("Polar")
                .on_toggle(GerberViewerMessage::TogglePolarCoordinates),
            checkbox(state.full_window_crosshair)
                .label("Full crosshair")
                .on_toggle(GerberViewerMessage::ToggleFullWindowCrosshair),
            text(cursor_label)
                .size(10)
                .color(text_muted),
            text(bounds_label)
            .size(10)
            .color(text_muted),
            Space::new().width(Length::Fill),
            button(text(if state.grid_editor_open
            {
                "Close Grid Editor"
            }
            else
            {
                "Edit Grids…"
            }))
            .on_press(GerberViewerMessage::ToggleGridEditor),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([4, 10])
    .width(Length::Fill)
    .style(crate::styles::toolbar_strip(tokens));
    let grid_editor: Element<'_, GerberViewerMessage> = if state.grid_editor_open
    {
        let unit_picker = pick_list(
            GridUnit::ALL,
            Some(state.new_grid_unit),
            |unit| {
                GerberViewerMessage::SetNewGridUnitMillimetres(
                    unit == GridUnit::Mm,
                )
            },
        )
        .width(90);
        let add_form = row![
            text("Add grid").size(12).color(text_primary),
            text_input("Optional name", &state.new_grid_name)
                .on_input(GerberViewerMessage::NewGridNameChanged)
                .width(180),
            text_input("X distance", &state.new_grid_x)
                .on_input(GerberViewerMessage::NewGridXChanged)
                .width(120),
            text("⨯").size(13).color(text_muted),
            text_input("Y distance", &state.new_grid_y)
                .on_input(GerberViewerMessage::NewGridYChanged)
                .width(120),
            unit_picker,
            button(text("Create"))
                .on_press(GerberViewerMessage::CreateGridDefinition),
            button(text("Delete selected"))
                .on_press_maybe(
                    (state.grid_catalog.len() > 1)
                        .then_some(GerberViewerMessage::DeleteGridDefinition),
                ),
            button(text("Move up"))
                .on_press_maybe(
                    (state.active_grid_index > 0)
                        .then_some(GerberViewerMessage::MoveGridUp),
                ),
            button(text("Move down"))
                .on_press_maybe(
                    (state.active_grid_index + 1 < state.grid_catalog.len())
                        .then_some(GerberViewerMessage::MoveGridDown),
                ),
            Space::new().width(Length::Fill),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        let edit_unit_picker = pick_list(
            GridUnit::ALL,
            Some(state.edit_grid_unit),
            |unit| {
                GerberViewerMessage::SetEditGridUnitMillimetres(
                    unit == GridUnit::Mm,
                )
            },
        )
        .width(90);
        let edit_form = row![
            text("Edit selected").size(12).color(text_primary),
            text_input("Optional name", &state.edit_grid_name)
                .on_input(GerberViewerMessage::EditGridNameChanged)
                .width(180),
            text_input("X distance", &state.edit_grid_x)
                .on_input(GerberViewerMessage::EditGridXChanged)
                .width(120),
            text("⨯").size(13).color(text_muted),
            text_input("Y distance", &state.edit_grid_y)
                .on_input(GerberViewerMessage::EditGridYChanged)
                .width(120),
            edit_unit_picker,
            button(text("Update"))
                .on_press(GerberViewerMessage::UpdateGridDefinition),
            Space::new().width(Length::Fill),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        let editor_content = if let Some(error) = state.grid_editor_error.as_deref()
        {
            column![
                add_form,
                edit_form,
                text(error)
                    .size(10)
                    .color(Color::from_rgb8(239, 83, 80)),
            ]
            .spacing(4)
        }
        else
        {
            column![add_form, edit_form]
        };

        container(editor_content)
            .padding([6, 10])
            .width(Length::Fill)
            .style(crate::styles::toolbar_strip(tokens))
            .into()
    }
    else
    {
        Space::new().height(0).into()
    };

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
        grid_visible: state.grid_visible,
        full_window_crosshair: state.full_window_crosshair,
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
        grid_editor,
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
    grid_visible: bool,
    full_window_crosshair: bool,
    grid_size: &'a GridSizePreset,
    redraw_generation: u64,
    zoom: f32,
    pan: iced::Vector,
}

impl GerberCanvas<'_>
{
    fn screen_to_world(
        &self,
        bounds: Rectangle,
        screen: Point,
    ) -> Option<signex_gerber::Point>
    {
        let world_bounds = visible_bounds(self.layers)?;
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
                if let Some(previous) = state.drag_start
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
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry>
    {
        let _redraw_generation = self.redraw_generation;
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.background);

        let Some(world_bounds) = visible_bounds(self.layers) else
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

fn full_window_crosshair_segments(
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

fn draw_full_window_crosshair(
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

fn format_coordinate_in_unit(
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

fn format_bounds_in_unit(
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
        state.decimal_separator = ".".to_owned();
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
            "Grid: 1.5000 mm ⨯ 2.5000 mm (59.06 mils ⨯ 98.43 mils)"
        );
    }

    #[test]
    fn toggling_grid_visibility_only_requests_viewport_redraw()
    {
        let mut state = GerberViewerState::default();
        let grid_catalog = state.grid_catalog.clone();
        let active_grid_index = state.active_grid_index;
        let layers = state.layers.len();
        let initial_generation = state.redraw_generation;

        state.set_grid_visible(false);

        assert!(!state.grid_visible);
        assert_eq!(state.redraw_generation, initial_generation + 1);
        assert_eq!(state.grid_catalog, grid_catalog);
        assert_eq!(state.active_grid_index, active_grid_index);
        assert_eq!(state.layers.len(), layers);

        state.set_grid_visible(false);
        assert_eq!(state.redraw_generation, initial_generation + 1);

        state.set_grid_visible(true);
        assert!(state.grid_visible);
        assert_eq!(state.redraw_generation, initial_generation + 2);
    }

    #[test]
    fn display_units_convert_coordinates_and_bounds_from_millimetres()
    {
        let point = signex_gerber::Point { x: 25.4, y: 12.7 };
        let bounds = Bounds {
            min: signex_gerber::Point { x: 0.0, y: -12.7 },
            max: point,
        };

        assert_eq!(
            format_coordinate_in_unit(
                point,
                GerberDisplayUnit::Millimetres,
                ".",
                false,
            ),
            "X: 25.4000  Y: 12.7000 mm"
        );
        assert_eq!(
            format_coordinate_in_unit(
                point,
                GerberDisplayUnit::Inches,
                ".",
                false,
            ),
            "X: 1.0000  Y: 0.5000 in"
        );
        assert_eq!(
            format_coordinate_in_unit(
                point,
                GerberDisplayUnit::Mils,
                ".",
                false,
            ),
            "X: 1000.00  Y: 500.00 mils"
        );
        assert_eq!(
            format_bounds_in_unit(bounds, GerberDisplayUnit::Inches, "."),
            "Bounds: X 0.0000…1.0000  Y -0.5000…0.5000 in"
        );
    }

    #[test]
    fn polar_coordinates_use_selected_unit_and_cartesian_origin()
    {
        let point = signex_gerber::Point { x: 25.4, y: 25.4 };

        assert_eq!(
            format_coordinate_in_unit(
                point,
                GerberDisplayUnit::Inches,
                ".",
                true,
            ),
            "R: 1.4142 in  θ: 45.00°"
        );
        assert_eq!(
            format_coordinate_in_unit(
                signex_gerber::Point::default(),
                GerberDisplayUnit::Millimetres,
                ".",
                true,
            ),
            "R: 0.0000 mm  θ: 0.00°"
        );
        assert_eq!(
            format_coordinate_in_unit(
                signex_gerber::Point { x: 0.0, y: -25.4 },
                GerberDisplayUnit::Mils,
                ".",
                true,
            ),
            "R: 1000.00 mils  θ: -90.00°"
        );
    }

    #[test]
    fn toggling_polar_coordinates_preserves_pointer_and_geometry_state()
    {
        let mut state = GerberViewerState::default();
        let position = signex_gerber::Point { x: 3.0, y: 4.0 };
        state.set_cursor_world_position(Some(position));
        let grid_catalog = state.grid_catalog.clone();

        state.set_polar_coordinates(true);

        assert!(state.polar_coordinates);
        assert_eq!(state.cursor_world_position, Some(position));
        assert_eq!(state.grid_catalog, grid_catalog);
    }

    #[test]
    fn full_window_crosshair_spans_viewport_and_toggle_requests_redraw()
    {
        let bounds = Rectangle::new(
            Point::ORIGIN,
            iced::Size::new(640.0, 480.0),
        );
        let segments =
            full_window_crosshair_segments(bounds, Point::new(120.0, 75.0));

        assert_eq!(
            segments,
            [
                (Point::new(0.0, 75.0), Point::new(640.0, 75.0)),
                (Point::new(120.0, 0.0), Point::new(120.0, 480.0)),
            ]
        );

        let mut state = GerberViewerState::default();
        let initial_generation = state.redraw_generation;
        state.set_full_window_crosshair(true);

        assert!(state.full_window_crosshair);
        assert_eq!(state.redraw_generation, initial_generation + 1);

        state.set_full_window_crosshair(true);
        assert_eq!(state.redraw_generation, initial_generation + 1);
    }

    #[test]
    fn changing_display_unit_does_not_modify_source_geometry()
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
            layers: vec![layer.clone()],
            failures: Vec::new(),
        });
        let zoom = state.zoom;
        let pan = state.pan;

        state.set_display_unit(GerberDisplayUnit::Inches);
        state.set_cursor_world_position(Some(signex_gerber::Point {
            x: 25.4,
            y: 12.7,
        }));

        assert_eq!(state.display_unit, GerberDisplayUnit::Inches);
        assert_eq!(state.layers[0].layer, layer);
        assert_eq!(state.zoom, zoom);
        assert_eq!(state.pan, pan);
    }

    #[test]
    fn creating_grid_appends_selects_and_persists_definition()
    {
        let mut state = GerberViewerState::default();
        state.decimal_separator = ".".to_owned();
        state.set_new_grid_name(" Fine metric ".to_owned());
        state.set_new_grid_x("0.05".to_owned());
        state.set_new_grid_y("0".to_owned());
        state.set_new_grid_unit_millimetres(true);
        let original_count = state.grid_catalog.len();
        let mut persisted = Vec::new();

        state.create_grid_with(|catalog| {
            persisted = catalog.to_vec();
            Ok(())
        });

        assert_eq!(state.grid_catalog.len(), original_count + 1);
        assert_eq!(persisted, state.grid_catalog);
        assert_eq!(state.active_grid_index, original_count);
        assert_eq!(state.active_grid().name.as_deref(), Some("Fine metric"));
        assert_eq!(state.active_grid().x_millimetres(), 0.05);
        assert_eq!(state.active_grid().y_millimetres(), 0.0);
        assert_eq!(
            state.status,
            "Created grid: Fine metric: 0.0500 mm ⨯ 0.0000 mm (1.97 mils ⨯ 0.00 mils)"
        );
        assert!(state.new_grid_name.is_empty());
        assert!(state.new_grid_x.is_empty());
        assert!(state.new_grid_y.is_empty());
        assert_eq!(state.grid_editor_error, None);
    }

    #[test]
    fn invalid_or_unpersisted_grid_is_not_added()
    {
        let mut state = GerberViewerState::default();
        let original_catalog = state.grid_catalog.clone();
        state.set_new_grid_x("0".to_owned());
        state.set_new_grid_y("1".to_owned());

        state.create_grid_with(|_| panic!("invalid grid must not persist"));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("X must be a finite number greater than zero.")
        );

        state.set_new_grid_x("1".to_owned());
        state.create_grid_with(|_| Err("settings unavailable".to_owned()));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("settings unavailable")
        );
    }

    #[test]
    fn editing_grid_updates_same_entry_and_persists_catalog()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        state.decimal_separator = ".".to_owned();
        state.load_active_grid_into_editor();
        let original_count = state.grid_catalog.len();
        let initial_generation = state.redraw_generation;
        state.set_edit_grid_name(" Fine metric ".to_owned());
        state.set_edit_grid_unit_millimetres(true);
        state.set_edit_grid_x("0.05".to_owned());
        state.set_edit_grid_y("0.10".to_owned());
        let mut persisted = Vec::new();

        state.update_grid_with(|catalog| {
            persisted = catalog.to_vec();
            Ok(())
        });

        assert_eq!(state.grid_catalog, persisted);
        assert_eq!(state.grid_catalog.len(), original_count);
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(state.active_grid().name.as_deref(), Some("Fine metric"));
        assert_eq!(state.active_grid().x_millimetres(), 0.05);
        assert_eq!(state.active_grid().y_millimetres(), 0.10);
        assert_eq!(state.active_grid().unit, GridUnit::Mm);
        assert_eq!(state.redraw_generation, initial_generation + 1);
        assert_eq!(
            state.status,
            "Updated grid: Fine metric: 0.0500 mm ⨯ 0.1000 mm (1.97 mils ⨯ 3.94 mils)"
        );
        assert_eq!(state.edit_grid_name, "Fine metric");
        assert_eq!(state.edit_grid_x, "0.05");
        assert_eq!(state.edit_grid_y, "0.1");
        assert_eq!(state.grid_editor_error, None);
    }

    #[test]
    fn changing_edit_unit_preserves_physical_grid_spacing()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..2].to_vec();
        state.active_grid_index = 0;
        state.decimal_separator = ".".to_owned();
        state.load_active_grid_into_editor();
        let original_x = state.active_grid().x_millimetres();
        let original_y = state.active_grid().y_millimetres();

        state.set_edit_grid_unit_millimetres(true);

        assert_eq!(state.edit_grid_unit, GridUnit::Mm);
        assert_eq!(state.edit_grid_x, "2.54");
        assert_eq!(state.edit_grid_y, "2.54");
        let metric = create_grid_definition(
            "",
            &state.edit_grid_x,
            &state.edit_grid_y,
            state.edit_grid_unit,
            &state.decimal_separator,
        )
        .expect("converted metric draft");
        assert_eq!(metric.x_millimetres(), original_x);
        assert_eq!(metric.y_millimetres(), original_y);

        state.set_edit_grid_unit_millimetres(false);

        assert_eq!(state.edit_grid_unit, GridUnit::Mil);
        assert_eq!(state.edit_grid_x, "100");
        assert_eq!(state.edit_grid_y, "100");
    }

    #[test]
    fn invalid_or_unpersisted_grid_edit_does_not_change_catalog()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        state.load_active_grid_into_editor();
        let original_catalog = state.grid_catalog.clone();
        state.set_edit_grid_x("0".to_owned());

        state.update_grid_with(|_| panic!("invalid edit must not persist"));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("X must be a finite number greater than zero.")
        );

        state.set_edit_grid_x("1".to_owned());
        state.update_grid_with(|_| Err("settings unavailable".to_owned()));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("settings unavailable")
        );
    }

    #[test]
    fn deleting_grid_persists_catalog_and_selects_next_entry()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        let removed = state.grid_catalog[1].clone();
        let next = state.grid_catalog[2].clone();
        let initial_generation = state.redraw_generation;
        let mut persisted = Vec::new();

        state.delete_grid_with(|catalog| {
            persisted = catalog.to_vec();
            Ok(())
        });

        assert_eq!(state.grid_catalog, persisted);
        assert_eq!(state.grid_catalog.len(), 2);
        assert!(!state.grid_catalog.contains(&removed));
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(state.active_grid(), &next);
        assert_eq!(state.redraw_generation, initial_generation + 1);
        assert!(state.status.starts_with("Deleted grid: "));
        assert_eq!(state.grid_editor_error, None);
    }

    #[test]
    fn deleting_final_grid_selects_previous_and_never_empties_catalog()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..2].to_vec();
        state.active_grid_index = 1;
        let previous = state.grid_catalog[0].clone();

        state.delete_grid_with(|_| Ok(()));

        assert_eq!(state.grid_catalog, vec![previous.clone()]);
        assert_eq!(state.active_grid_index, 0);
        assert_eq!(state.active_grid(), &previous);

        state.delete_grid_with(|_| panic!("last grid must not persist"));

        assert_eq!(state.grid_catalog, vec![previous]);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("At least one grid definition must remain.")
        );
    }

    #[test]
    fn failed_grid_deletion_does_not_change_catalog_or_selection()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        let original_catalog = state.grid_catalog.clone();

        state.delete_grid_with(|_| Err("settings unavailable".to_owned()));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("settings unavailable")
        );
    }

    #[test]
    fn moving_grid_up_and_down_preserves_selected_definition()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        let original_catalog = state.grid_catalog.clone();
        let selected = state.active_grid().clone();
        let mut persisted = Vec::new();

        state.move_grid_up_with(|catalog| {
            persisted = catalog.to_vec();
            Ok(())
        });

        assert_eq!(state.grid_catalog, persisted);
        assert_eq!(state.active_grid_index, 0);
        assert_eq!(state.active_grid(), &selected);
        assert_eq!(state.grid_catalog[1], original_catalog[0]);

        state.move_grid_down_with(|catalog| {
            persisted = catalog.to_vec();
            Ok(())
        });

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(state.grid_catalog, persisted);
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(state.active_grid(), &selected);
    }

    #[test]
    fn moving_grid_at_boundary_is_a_no_op()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        let original_catalog = state.grid_catalog.clone();
        let original_generation = state.redraw_generation;

        state.active_grid_index = 0;
        state.move_grid_up_with(|_| panic!("upper boundary must not persist"));
        state.active_grid_index = state.grid_catalog.len() - 1;
        state.move_grid_down_with(|_| panic!("lower boundary must not persist"));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(state.active_grid_index, 2);
        assert_eq!(state.redraw_generation, original_generation);
    }

    #[test]
    fn failed_grid_move_does_not_change_catalog_or_selection()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        let original_catalog = state.grid_catalog.clone();

        state.move_grid_up_with(|_| Err("settings unavailable".to_owned()));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("settings unavailable")
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
