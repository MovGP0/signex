use iced::Task;

use super::super::*;
use crate::gerber_viewer::GerberViewerMessage;

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
            GerberViewerMessage::SelectLayer(index) => {
                self.ui_state.gerber_viewer.select_layer(index);
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
        }
    }
}
