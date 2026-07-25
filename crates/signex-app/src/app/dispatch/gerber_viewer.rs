use iced::Task;

use super::super::*;
use signex_widgets::gerber_viewer::{
    GerberShortcutResolver, GerberViewerMessage,
};

impl GerberShortcutResolver for crate::keymap::CompiledKeymap
{
    fn resolve_gerber_shortcut(
        &self,
        key: &iced::keyboard::Key,
        modifiers: iced::keyboard::Modifiers,
    ) -> Option<GerberViewerMessage>
    {
        let stroke = crate::keymap::KeyStroke::from_iced(key, modifiers)?;
        let lookup = self.lookup(
            &[stroke],
            &[
                crate::keymap::ShortcutContext::Global,
                crate::keymap::ShortcutContext::Gerber,
            ],
        );
        match lookup.command?.as_str()
        {
            "gerber_next_layer" => Some(GerberViewerMessage::NextLayer),
            "gerber_previous_layer" => Some(GerberViewerMessage::PreviousLayer),
            "gerber_sketch_flashes" =>
            {
                Some(GerberViewerMessage::ToggleSketchFlashes)
            }
            "gerber_sketch_lines" =>
            {
                Some(GerberViewerMessage::ToggleSketchLines)
            }
            "gerber_sketch_polygons" =>
            {
                Some(GerberViewerMessage::ToggleSketchPolygons)
            }
            "gerber_show_d_codes" =>
            {
                Some(GerberViewerMessage::ToggleDCodeLabels)
            }
            _ => None,
        }
    }
}

impl Signex
{
    pub(super) fn handle_open_gerber_viewer(&mut self) -> Task<Message>
    {
        if let Some(id) = self.ui_state.windows.iter().find_map(|(id, kind)| {
            matches!(kind, crate::app::state::WindowKind::GerberViewer).then_some(*id)
        })
        {
            return iced::window::gain_focus(id);
        }

        let (_id, open_task) = iced::window::open(iced::window::Settings {
            size: iced::Size::new(1280.0, 800.0),
            min_size: Some(iced::Size::new(860.0, 560.0)),
            icon: crate::app::bootstrap::bundled_window_icon(),
            decorations: false,
            ..Default::default()
        });
        open_task.map(Message::GerberViewerOpened)
    }

    pub(super) fn dispatch_gerber_viewer_message(
        &mut self,
        message: GerberViewerMessage,
    ) -> Task<Message>
    {
        match message
        {
            GerberViewerMessage::OpenAutodetectedFiles => {
                self.ui_state.gerber_viewer.begin_loading();
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Open Fabrication Files")
                            .pick_files()
                            .await
                            .map(|files| {
                                files
                                    .into_iter()
                                    .map(|file| file.path().to_path_buf())
                                    .collect::<Vec<_>>()
                            })
                    },
                    |paths| {
                        Message::GerberViewer(
                            GerberViewerMessage::AutodetectedFilesChosen(paths),
                        )
                    },
                )
            }
            GerberViewerMessage::AutodetectedFilesChosen(Some(paths)) => Task::perform(
                async move { signex_gerber::load_autodetected_files(paths) },
                |batch| {
                    Message::GerberViewer(
                        GerberViewerMessage::AutodetectedFilesLoaded(batch),
                    )
                },
            ),
            GerberViewerMessage::AutodetectedFilesChosen(None) => {
                self.ui_state.gerber_viewer.loading = false;
                self.ui_state.gerber_viewer.status =
                    "Open fabrication files cancelled.".into();
                Task::none()
            }
            GerberViewerMessage::AutodetectedFilesLoaded(batch) => {
                self.ui_state.gerber_viewer.apply_load_batch(batch);
                Task::none()
            }
            GerberViewerMessage::OpenZipArchive => {
                self.ui_state.gerber_viewer.begin_loading();
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Open Gerber and Drill ZIP Archive")
                            .add_filter("ZIP archive", &["zip"])
                            .pick_file()
                            .await
                            .map(|file| file.path().to_path_buf())
                    },
                    |path| {
                        Message::GerberViewer(
                            GerberViewerMessage::ZipArchiveChosen(path),
                        )
                    },
                )
            }
            GerberViewerMessage::ZipArchiveChosen(Some(path)) => Task::perform(
                async move { signex_gerber::load_zip_archive(path) },
                |batch| {
                    Message::GerberViewer(
                        GerberViewerMessage::ZipArchiveLoaded(batch),
                    )
                },
            ),
            GerberViewerMessage::ZipArchiveChosen(None) => {
                self.ui_state.gerber_viewer.loading = false;
                self.ui_state.gerber_viewer.status =
                    "Open ZIP archive cancelled.".into();
                Task::none()
            }
            GerberViewerMessage::ZipArchiveLoaded(batch) => {
                self.ui_state.gerber_viewer.apply_load_batch(batch);
                Task::none()
            }
            GerberViewerMessage::OpenGerberJob => {
                self.ui_state.gerber_viewer.begin_loading();
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Open Gerber Job File")
                            .add_filter("Gerber job", &["gbrjob"])
                            .pick_file()
                            .await
                            .map(|file| file.path().to_path_buf())
                    },
                    |path| {
                        Message::GerberViewer(
                            GerberViewerMessage::GerberJobChosen(path),
                        )
                    },
                )
            }
            GerberViewerMessage::GerberJobChosen(Some(path)) => Task::perform(
                async move { signex_gerber::load_gerber_job_file(path) },
                |batch| {
                    Message::GerberViewer(
                        GerberViewerMessage::GerberJobLoaded(batch),
                    )
                },
            ),
            GerberViewerMessage::GerberJobChosen(None) => {
                self.ui_state.gerber_viewer.loading = false;
                self.ui_state.gerber_viewer.status =
                    "Open Gerber job cancelled.".into();
                Task::none()
            }
            GerberViewerMessage::GerberJobLoaded(batch) => {
                self.ui_state.gerber_viewer.apply_load_batch(batch);
                Task::none()
            }
            GerberViewerMessage::OpenGerberFiles => {
                self.ui_state.gerber_viewer.begin_loading();
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Open Gerber Files")
                            .add_filter(
                                "Gerber RS-274X",
                                &[
                                    "gbr", "ger", "pho", "art", "gbx", "gtl", "gbl", "gto", "gbo",
                                    "gts", "gbs", "gtp", "gbp", "gm1", "gm2", "gko", "gvc",
                                    "gsp",
                                ],
                            )
                            .pick_files()
                            .await
                            .map(|files| {
                                files
                                    .into_iter()
                                    .map(|file| file.path().to_path_buf())
                                    .collect::<Vec<_>>()
                            })
                    },
                    |paths| {
                        Message::GerberViewer(GerberViewerMessage::GerberFilesChosen(paths))
                    },
                )
            }
            GerberViewerMessage::GerberFilesChosen(Some(paths)) => Task::perform(
                async move { signex_gerber::load_gerber_files(paths) },
                |batch| Message::GerberViewer(GerberViewerMessage::GerberFilesLoaded(batch)),
            ),
            GerberViewerMessage::GerberFilesChosen(None) => {
                self.ui_state.gerber_viewer.loading = false;
                self.ui_state.gerber_viewer.status = "Open Gerber files cancelled.".into();
                Task::none()
            }
            GerberViewerMessage::GerberFilesLoaded(batch) => {
                self.ui_state.gerber_viewer.apply_load_batch(batch);
                Task::none()
            }
            GerberViewerMessage::OpenExcellonFiles => {
                self.ui_state.gerber_viewer.begin_loading();
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Open Excellon Drill Files")
                            .add_filter("Excellon Drill", &["drl", "drd"])
                            .pick_files()
                            .await
                            .map(|files| {
                                files
                                    .into_iter()
                                    .map(|file| file.path().to_path_buf())
                                    .collect::<Vec<_>>()
                            })
                    },
                    |paths| {
                        Message::GerberViewer(GerberViewerMessage::ExcellonFilesChosen(paths))
                    },
                )
            }
            GerberViewerMessage::ExcellonFilesChosen(Some(paths)) => Task::perform(
                async move { signex_gerber::load_excellon_files(paths) },
                |batch| Message::GerberViewer(GerberViewerMessage::ExcellonFilesLoaded(batch)),
            ),
            GerberViewerMessage::ExcellonFilesChosen(None) => {
                self.ui_state.gerber_viewer.loading = false;
                self.ui_state.gerber_viewer.status = "Open Excellon files cancelled.".into();
                Task::none()
            }
            GerberViewerMessage::ExcellonFilesLoaded(batch) => {
                self.ui_state.gerber_viewer.apply_load_batch(batch);
                Task::none()
            }
            GerberViewerMessage::ReloadAllLayers => {
                self.ui_state.gerber_viewer.begin_loading();
                let layers = self
                    .ui_state
                    .gerber_viewer
                    .layers
                    .iter()
                    .map(|layer| layer.layer.clone())
                    .collect::<Vec<_>>();
                Task::perform(
                    async move { signex_gerber::reload_layers(layers) },
                    |batch| Message::GerberViewer(GerberViewerMessage::LayersReloaded(batch)),
                )
            }
            GerberViewerMessage::LayersReloaded(batch) => {
                self.ui_state.gerber_viewer.apply_reload_batch(batch);
                Task::none()
            }
            GerberViewerMessage::SelectLayer(index) => {
                self.ui_state.gerber_viewer.select_layer(index);
                Task::none()
            }
            GerberViewerMessage::NextLayer => {
                self.ui_state.gerber_viewer.select_next_layer();
                Task::none()
            }
            GerberViewerMessage::PreviousLayer => {
                self.ui_state.gerber_viewer.select_previous_layer();
                Task::none()
            }
            GerberViewerMessage::SetLayerVisible(index, visible) => {
                self.ui_state
                    .gerber_viewer
                    .set_layer_visible(index, visible);
                Task::none()
            }
            GerberViewerMessage::ClearCurrentLayer => {
                self.ui_state.gerber_viewer.clear_current_layer();
                Task::none()
            }
            GerberViewerMessage::ClearAllLayers => {
                self.ui_state.gerber_viewer.clear_all_layers();
                Task::none()
            }
            GerberViewerMessage::RedrawViewport => {
                self.ui_state.gerber_viewer.redraw_viewport();
                Task::none()
            }
            GerberViewerMessage::ToggleSketchFlashes =>
            {
                self.ui_state.gerber_viewer.toggle_sketch_flashes();
                Task::none()
            }
            GerberViewerMessage::ToggleSketchLines =>
            {
                self.ui_state.gerber_viewer.toggle_sketch_lines();
                Task::none()
            }
            GerberViewerMessage::ToggleSketchPolygons =>
            {
                self.ui_state.gerber_viewer.toggle_sketch_polygons();
                Task::none()
            }
            GerberViewerMessage::ToggleGhostNegativeObjects =>
            {
                self.ui_state
                    .gerber_viewer
                    .toggle_ghost_negative_objects();
                Task::none()
            }
            GerberViewerMessage::ToggleDCodeLabels =>
            {
                self.ui_state.gerber_viewer.toggle_d_code_labels();
                Task::none()
            }
            GerberViewerMessage::ZoomBy(factor) => {
                self.ui_state.gerber_viewer.zoom_by(factor);
                Task::none()
            }
            GerberViewerMessage::PanBy(delta) => {
                self.ui_state.gerber_viewer.pan_by(delta);
                Task::none()
            }
            GerberViewerMessage::FitPage => {
                self.ui_state.gerber_viewer.fit_page();
                Task::none()
            }
            GerberViewerMessage::ToggleLayerManager => {
                self.ui_state.gerber_viewer.toggle_layer_manager();
                Task::none()
            }
            GerberViewerMessage::ToggleLayerInformation => {
                self.ui_state.gerber_viewer.toggle_layer_information();
                Task::none()
            }
            GerberViewerMessage::ToggleDCodeList => {
                self.ui_state.gerber_viewer.toggle_d_code_list();
                Task::none()
            }
            GerberViewerMessage::ToggleSourceView => {
                self.ui_state.gerber_viewer.toggle_source_view();
                Task::none()
            }
            GerberViewerMessage::SelectGridSize(index) => {
                self.ui_state.gerber_viewer.select_grid_size(index);
                Task::none()
            }
            GerberViewerMessage::ToggleGridEditor => {
                self.ui_state.gerber_viewer.toggle_grid_editor();
                Task::none()
            }
            GerberViewerMessage::NewGridNameChanged(value) => {
                self.ui_state.gerber_viewer.set_new_grid_name(value);
                Task::none()
            }
            GerberViewerMessage::NewGridXChanged(value) => {
                self.ui_state.gerber_viewer.set_new_grid_x(value);
                Task::none()
            }
            GerberViewerMessage::NewGridYChanged(value) => {
                self.ui_state.gerber_viewer.set_new_grid_y(value);
                Task::none()
            }
            GerberViewerMessage::SetNewGridUnitMillimetres(millimetres) => {
                self.ui_state
                    .gerber_viewer
                    .set_new_grid_unit_millimetres(millimetres);
                Task::none()
            }
            GerberViewerMessage::CreateGridDefinition => {
                self.ui_state.gerber_viewer.create_grid();
                Task::none()
            }
            GerberViewerMessage::DeleteGridDefinition => {
                self.ui_state.gerber_viewer.delete_grid();
                Task::none()
            }
            GerberViewerMessage::MoveGridUp => {
                self.ui_state.gerber_viewer.move_grid_up();
                Task::none()
            }
            GerberViewerMessage::MoveGridDown => {
                self.ui_state.gerber_viewer.move_grid_down();
                Task::none()
            }
            GerberViewerMessage::EditGridNameChanged(value) => {
                self.ui_state.gerber_viewer.set_edit_grid_name(value);
                Task::none()
            }
            GerberViewerMessage::EditGridXChanged(value) => {
                self.ui_state.gerber_viewer.set_edit_grid_x(value);
                Task::none()
            }
            GerberViewerMessage::EditGridYChanged(value) => {
                self.ui_state.gerber_viewer.set_edit_grid_y(value);
                Task::none()
            }
            GerberViewerMessage::SetEditGridUnitMillimetres(millimetres) => {
                self.ui_state
                    .gerber_viewer
                    .set_edit_grid_unit_millimetres(millimetres);
                Task::none()
            }
            GerberViewerMessage::UpdateGridDefinition => {
                self.ui_state.gerber_viewer.update_grid();
                Task::none()
            }
            GerberViewerMessage::ToggleGridVisibility(visible) => {
                self.ui_state.gerber_viewer.set_grid_visible(visible);
                Task::none()
            }
            GerberViewerMessage::SetDisplayUnit(unit) => {
                self.ui_state.gerber_viewer.set_display_unit(unit);
                Task::none()
            }
            GerberViewerMessage::CursorWorldPositionChanged(position) => {
                self.ui_state
                    .gerber_viewer
                    .set_cursor_world_position(position);
                Task::none()
            }
            GerberViewerMessage::ToggleMeasurement =>
            {
                self.ui_state.gerber_viewer.toggle_measurement();
                Task::none()
            }
            GerberViewerMessage::CaptureMeasurementPoint(point) =>
            {
                self.ui_state
                    .gerber_viewer
                    .capture_measurement_point(point);
                Task::none()
            }
            GerberViewerMessage::ResetMeasurement =>
            {
                self.ui_state.gerber_viewer.reset_measurement();
                Task::none()
            }
            GerberViewerMessage::TogglePolarCoordinates(polar) => {
                self.ui_state
                    .gerber_viewer
                    .set_polar_coordinates(polar);
                Task::none()
            }
            GerberViewerMessage::ToggleFullWindowCrosshair(full_window) => {
                self.ui_state
                    .gerber_viewer
                    .set_full_window_crosshair(full_window);
                Task::none()
            }
            GerberViewerMessage::SetPageSize(page_size) => {
                self.ui_state.gerber_viewer.set_page_size(page_size);
                Task::none()
            }
            GerberViewerMessage::PrintVisibleLayers => {
                let bytes = match self.ui_state.gerber_viewer.print_pdf()
                {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        self.ui_state.gerber_viewer.status = error;
                        return Task::none();
                    }
                };
                self.ui_state.gerber_viewer.status =
                    "Choose where to save the Gerber print PDF.".into();
                Task::perform(
                    async move {
                        let Some(file) = rfd::AsyncFileDialog::new()
                            .set_title("Print Visible Gerber Layers to PDF")
                            .add_filter("PDF document", &["pdf"])
                            .set_file_name("gerber-view.pdf")
                            .save_file()
                            .await
                        else
                        {
                            return Err("Gerber PDF print cancelled.".to_owned());
                        };
                        let mut path = file.path().to_path_buf();
                        if path.extension().is_none()
                        {
                            path.set_extension("pdf");
                        }
                        std::fs::write(&path, bytes).map_err(|error| {
                            format!("Could not write {}: {error}", path.display())
                        })?;
                        Ok(path)
                    },
                    |result| {
                        Message::GerberViewer(
                            GerberViewerMessage::GerberPrintFinished(result),
                        )
                    },
                )
            }
            GerberViewerMessage::GerberPrintFinished(result) => {
                self.ui_state.gerber_viewer.status = match result
                {
                    Ok(path) => {
                        format!("Printed visible Gerber layers to {}.", path.display())
                    }
                    Err(error) => error,
                };
                Task::none()
            }
            GerberViewerMessage::ToggleZoomSelection => {
                self.ui_state.gerber_viewer.toggle_zoom_selection();
                Task::none()
            }
            GerberViewerMessage::SetHighlightedComponent(component) => {
                self.ui_state
                    .gerber_viewer
                    .set_highlighted_component(component);
                Task::none()
            }
            GerberViewerMessage::ClearComponentHighlight => {
                self.ui_state
                    .gerber_viewer
                    .clear_component_highlight();
                Task::none()
            }
            GerberViewerMessage::SetHighlightedNet(net) => {
                self.ui_state.gerber_viewer.set_highlighted_net(net);
                Task::none()
            }
            GerberViewerMessage::ClearNetHighlight => {
                self.ui_state.gerber_viewer.clear_net_highlight();
                Task::none()
            }
            GerberViewerMessage::SetHighlightedAttribute(attribute) =>
            {
                self.ui_state
                    .gerber_viewer
                    .set_highlighted_attribute(attribute);
                Task::none()
            }
            GerberViewerMessage::ClearAttributeHighlight =>
            {
                self.ui_state
                    .gerber_viewer
                    .clear_attribute_highlight();
                Task::none()
            }
            GerberViewerMessage::SetHighlightedDCode(d_code) =>
            {
                self.ui_state
                    .gerber_viewer
                    .set_highlighted_d_code(d_code);
                Task::none()
            }
            GerberViewerMessage::ClearDCodeHighlight =>
            {
                self.ui_state
                    .gerber_viewer
                    .clear_d_code_highlight();
                Task::none()
            }
            GerberViewerMessage::SetSelectedItem(selection) =>
            {
                self.ui_state
                    .gerber_viewer
                    .set_selected_item(selection);
                Task::none()
            }
            GerberViewerMessage::ZoomToSelection { bounds, viewport } => {
                self.ui_state
                    .gerber_viewer
                    .zoom_to_selection(bounds, viewport);
                Task::none()
            }
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn built_in_profiles_bind_gerber_keys_through_widget_boundary()
    {
        let mut profiles = crate::keymap::ShortcutProfileSet::built_ins()
            .expect("built-in shortcut profiles must parse");
        let page_up = iced::keyboard::Key::Named(
            iced::keyboard::key::Named::PageUp,
        );
        let page_down = iced::keyboard::Key::Named(
            iced::keyboard::key::Named::PageDown,
        );
        let f = iced::keyboard::Key::Character("f".into());
        let l = iced::keyboard::Key::Character("l".into());
        let p = iced::keyboard::Key::Character("p".into());
        let d = iced::keyboard::Key::Character("d".into());

        for profile in ["altium", "classic"]
        {
            profiles
                .set_active_profile(profile)
                .expect("known built-in profile");
            let keymap = profiles.compile_active();

            assert!(matches!(
                keymap.resolve_gerber_shortcut(
                    &page_up,
                    iced::keyboard::Modifiers::default(),
                ),
                Some(GerberViewerMessage::PreviousLayer),
            ));
            assert!(matches!(
                keymap.resolve_gerber_shortcut(
                    &page_down,
                    iced::keyboard::Modifiers::default(),
                ),
                Some(GerberViewerMessage::NextLayer),
            ));
            assert!(matches!(
                keymap.resolve_gerber_shortcut(
                    &f,
                    iced::keyboard::Modifiers::default(),
                ),
                Some(GerberViewerMessage::ToggleSketchFlashes),
            ));
            assert!(matches!(
                keymap.resolve_gerber_shortcut(
                    &l,
                    iced::keyboard::Modifiers::default(),
                ),
                Some(GerberViewerMessage::ToggleSketchLines),
            ));
            assert!(matches!(
                keymap.resolve_gerber_shortcut(
                    &p,
                    iced::keyboard::Modifiers::default(),
                ),
                Some(GerberViewerMessage::ToggleSketchPolygons),
            ));
            assert!(matches!(
                keymap.resolve_gerber_shortcut(
                    &d,
                    iced::keyboard::Modifiers::default(),
                ),
                Some(GerberViewerMessage::ToggleDCodeLabels),
            ));
        }
    }
}
