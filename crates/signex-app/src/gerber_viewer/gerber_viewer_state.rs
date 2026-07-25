use super::*;

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
    pub(super) layer_information_visible: bool,
    pub(super) d_code_list_visible: bool,
    pub(super) source_view_visible: bool,
    pub(super) grid_catalog: Vec<GridSizePreset>,
    pub(super) active_grid_index: usize,
    pub(super) grid_visible: bool,
    pub(super) display_unit: GerberDisplayUnit,
    pub(super) cursor_world_position: Option<signex_gerber::Point>,
    pub(super) polar_coordinates: bool,
    pub(super) full_window_crosshair: bool,
    pub(super) page_size: GerberPageSize,
    pub(super) zoom_selection_active: bool,
    pub(super) decimal_separator: String,
    pub(super) grid_editor_open: bool,
    pub(super) new_grid_name: String,
    pub(super) new_grid_x: String,
    pub(super) new_grid_y: String,
    pub(super) new_grid_unit: GridUnit,
    pub(super) edit_grid_name: String,
    pub(super) edit_grid_x: String,
    pub(super) edit_grid_y: String,
    pub(super) edit_grid_unit: GridUnit,
    pub(super) grid_editor_error: Option<String>,
    pub(super) palette: Vec<Color>,
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
            layer_information_visible: false,
            d_code_list_visible: false,
            source_view_visible: false,
            grid_catalog,
            active_grid_index,
            grid_visible: true,
            display_unit: GerberDisplayUnit::Millimetres,
            cursor_world_position: None,
            polar_coordinates: false,
            full_window_crosshair: false,
            page_size: load_page_size(),
            zoom_selection_active: false,
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

    pub fn apply_reload_batch(&mut self, batch: signex_gerber::GerberReloadBatch)
    {
        self.loading = false;
        let reloaded_count = batch.layers.len();
        for (index, layer) in batch.layers
        {
            if let Some(existing) = self.layers.get_mut(index)
            {
                existing.layer = layer;
            }
        }
        if reloaded_count > 0
        {
            self.redraw_generation = self.redraw_generation.wrapping_add(1);
        }

        let mut status = format!("Reloaded {reloaded_count} layer(s).");
        if !batch.failures.is_empty()
        {
            let failures = batch
                .failures
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" | ");
            status.push_str(&format!(
                " {} layer(s) could not be reloaded: {failures}",
                batch.failures.len(),
            ));
        }
        self.status = status;
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

    pub fn toggle_zoom_selection(&mut self)
    {
        self.zoom_selection_active = !self.zoom_selection_active;
        self.status = if self.zoom_selection_active
        {
            "Drag a rectangle with the left mouse button to zoom.".into()
        }
        else
        {
            "Zoom-area selection cancelled.".into()
        };
    }

    pub fn zoom_to_selection(&mut self, selection: Bounds, viewport: Rectangle)
    {
        let Some(base_bounds) = page_bounds(visible_bounds(&self.layers), self.page_size)
        else
        {
            return;
        };
        let Some((zoom, pan)) = zoom_transform_for_selection(
            base_bounds,
            selection,
            viewport,
        )
        else
        {
            self.status = "Zoom area is too small.".into();
            return;
        };
        self.zoom = zoom;
        self.pan = pan;
        self.zoom_selection_active = false;
        self.status = "Zoomed to selected area.".into();
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

    pub fn set_page_size(&mut self, page_size: GerberPageSize)
    {
        let grid_catalog = self.grid_catalog.clone();
        self.set_page_size_with(page_size, move |value| {
            persist_page_size(value, &grid_catalog)
        });
    }

    pub(super) fn set_page_size_with(
        &mut self,
        page_size: GerberPageSize,
        persist: impl FnOnce(GerberPageSize) -> Result<(), String>,
    )
    {
        if self.page_size == page_size
        {
            return;
        }
        if let Err(error) = persist(page_size)
        {
            self.status = format!("Could not save Gerber page size: {error}");
            return;
        }
        self.page_size = page_size;
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
        self.status = format!("Page size: {page_size}.");
    }

    pub fn print_layout(&self) -> Option<GerberPrintLayout>
    {
        page_bounds(visible_bounds(&self.layers), self.page_size).map(|bounds| {
            GerberPrintLayout {
                page_size: self.page_size,
                bounds,
            }
        })
    }

    pub(crate) fn print_pdf(&self) -> Result<Vec<u8>, String>
    {
        print::build_pdf(self)
    }

    pub fn toggle_layer_manager(&mut self)
    {
        self.layer_manager_visible = !self.layer_manager_visible;
    }

    pub fn toggle_layer_information(&mut self)
    {
        self.layer_information_visible = !self.layer_information_visible;
        if self.layer_information_visible
        {
            self.layer_manager_visible = true;
            self.d_code_list_visible = false;
        }
    }

    pub fn toggle_d_code_list(&mut self)
    {
        self.d_code_list_visible = !self.d_code_list_visible;
        if self.d_code_list_visible
        {
            self.layer_manager_visible = true;
            self.layer_information_visible = false;
        }
    }

    pub fn toggle_source_view(&mut self)
    {
        self.source_view_visible = !self.source_view_visible;
    }

    pub(super) fn active_gerber_source(&self) -> Result<(&str, &str), &'static str>
    {
        let index = self.active_layer.ok_or("No active layer.")?;
        let layer = self.layers.get(index).ok_or("No active layer.")?;
        Ok((&layer.layer.name, layer.layer.gerber_source()?))
    }

    pub(super) fn definition_groups(&self) -> Vec<signex_gerber::LayerDefinitionGroup>
    {
        self.layers
            .iter()
            .map(|layer| layer.layer.definition_group())
            .collect()
    }

    pub(super) fn active_layer_metadata(&self) -> Option<signex_gerber::LayerMetadata>
    {
        self.active_layer
            .and_then(|index| self.layers.get(index))
            .map(|layer| layer.layer.metadata())
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

    pub fn select_next_layer(&mut self)
    {
        let Some(index) = next_layer_index(self.active_layer, self.layers.len())
        else
        {
            return;
        };
        self.select_layer_with_status(index);
    }

    pub fn select_previous_layer(&mut self)
    {
        let Some(index) = previous_layer_index(self.active_layer, self.layers.len())
        else
        {
            return;
        };
        self.select_layer_with_status(index);
    }

    pub(super) fn select_layer_with_status(&mut self, index: usize)
    {
        self.active_layer = Some(index);
        self.status = format!("Active layer: {}", self.layers[index].layer.name);
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

    pub(super) fn active_grid(&self) -> &GridSizePreset
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

    pub(super) fn create_grid_with(
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

    pub(super) fn update_grid_with(
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

    pub(super) fn delete_grid_with(
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

    pub(super) fn move_grid_up_with(
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

    pub(super) fn move_grid_down_with(
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

    pub(super) fn move_grid_to_with(
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

    pub(super) fn load_active_grid_into_editor(&mut self)
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
