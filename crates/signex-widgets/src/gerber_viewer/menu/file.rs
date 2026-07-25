use super::*;

pub(super) fn view(
    state: &GerberViewerState,
    colors: MenuColors,
) -> Item<'static, GerberViewerMessage, Theme, Renderer>
{
    let recent_gerber = recent_files_menu("Open Recent Gerber File", colors);
    let recent_drill = recent_files_menu("Open Recent Drill File", colors);
    let recent_job = recent_files_menu("Open Recent Job File", colors);
    let recent_zip = recent_files_menu("Open Recent Zip File", colors);

    Item::with_menu(
        root_button("File", colors),
        dropdown(vec![
            leaf_if(
                "Open Autodetected File(s) ...",
                None,
                GerberViewerMessage::OpenAutodetectedFiles,
                !state.loading,
                colors,
            ),
            leaf_if(
                "Open Gerber Plot File(s) ...",
                None,
                GerberViewerMessage::OpenGerberFiles,
                !state.loading,
                colors,
            ),
            recent_gerber,
            leaf_if(
                "Open Excellon Drill File(s) ...",
                None,
                GerberViewerMessage::OpenExcellonFiles,
                !state.loading,
                colors,
            ),
            recent_drill,
            leaf_if(
                "Open Gerber Job File ...",
                None,
                GerberViewerMessage::OpenGerberJob,
                !state.loading,
                colors,
            ),
            recent_job,
            leaf_if(
                "Open Zip Archive File ...",
                None,
                GerberViewerMessage::OpenZipArchive,
                !state.loading,
                colors,
            ),
            recent_zip,
            separator(colors),
            leaf_if(
                "Clear All Layers",
                None,
                GerberViewerMessage::ClearAllLayers,
                !state.layers.is_empty(),
                colors,
            ),
            leaf_if(
                "Reload All Layers",
                None,
                GerberViewerMessage::ReloadAllLayers,
                !state.loading && !state.layers.is_empty(),
                colors,
            ),
            separator(colors),
            leaf_if(
                "Export to PCB Editor ...",
                None,
                GerberViewerMessage::ExportNativePcb,
                !state.layers.is_empty(),
                colors,
            ),
            separator(colors),
            leaf_if(
                "Print ...",
                Some("Ctrl+P"),
                GerberViewerMessage::PrintVisibleLayers,
                print::has_visible_layers(state),
                colors,
            ),
            separator(colors),
            leaf(
                "Quit",
                Some("Ctrl+Q"),
                GerberViewerMessage::CloseRequested,
                colors,
            ),
        ]),
    )
}
