use super::*;

#[test]
fn file_menu_contains_requested_commands_in_order()
{
    assert_eq!(
        menu::FILE_MENU_LABELS,
        [
            "Open Autodetected File(s) ...",
            "Open Gerber Plot File(s) ...",
            "Open Recent Gerber File",
            "Open Excellon Drill File(s) ...",
            "Open Recent Drill File",
            "Open Gerber Job File ...",
            "Open Recent Job File",
            "Open Zip Archive File ...",
            "Open Recent Zip File",
            "Clear All Layers",
            "Reload All Layers",
            "Export to PCB Editor ...",
            "Print ...",
            "Quit",
        ],
    );
}

#[test]
fn view_menu_contains_requested_commands_in_order()
{
    assert_eq!(
        menu::VIEW_MENU_LABELS,
        [
            "Zoom In",
            "Zoom Out",
            "Zoom to Fit",
            "Zoom to Selection Area",
            "Refresh",
            "Show Grid",
            "Polar Coordinates",
            "Units",
            "Sketch Flashed Items",
            "Sketch Lines",
            "Sketch Polygons",
            "Show DCodes",
            "Ghost Negative Objects",
            "Show with Forced Opacity Mode",
            "Show in XOR Mode",
            "Inactive Layer View Mode",
            "Flip Gerber View",
            "Show Layers Manager",
        ],
    );
}

#[test]
fn tools_menu_contains_requested_commands_in_order()
{
    assert_eq!(
        menu::TOOLS_MENU_LABELS,
        [
            "List DCodes ...",
            "Show Source ...",
            "Measure Tool",
            "Clear Current Layer ...",
        ],
    );
}
