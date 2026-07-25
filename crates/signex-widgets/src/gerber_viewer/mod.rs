use std::{fmt, path::PathBuf};

use iced::mouse;
use iced::widget::{
    Space, button, canvas, checkbox, container, pick_list, row, scrollable, text,
    text_input,
};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Theme,
    keyboard,
};
use serde::Deserialize;
use signex_gerber::{
    ApertureShape, Bounds, GerberAttributeValue, GerberLoadBatch, GerberPrimitive,
    LoadedLayer, PrimitivePolarity,
};
use signex_types::theme::ThemeTokens;

mod display;
mod gerber_viewer_state;
mod grid;
mod highlight;
mod measurement;
mod message;
pub(crate) mod print;
mod selection;
mod shortcuts;
mod styles;
mod view;

#[path = "canvas.rs"]
mod viewport;

pub use display::{GerberDisplayUnit, GerberPageSize, GerberPrintLayout};
pub use gerber_viewer_state::{GerberViewerState, ViewerLayer};
pub use measurement::GerberMeasurement;
pub use message::GerberViewerMessage;
pub use selection::GerberItemSelection;
pub use shortcuts::GerberShortcutResolver;
pub use view::view;

use shortcuts::{gerber_shortcut_message, next_layer_index, previous_layer_index};

use viewport::{
    material_compare_palette, material_d_code_color, material_layer_palette,
    material_negative_ghost_color, page_bounds, visible_bounds,
    zoom_transform_for_selection,
};

#[cfg(test)]
use viewport::*;

use grid::{
    DEFAULT_GRID_INDEX, GridSizePreset, GridUnit, create_grid_definition,
    default_inactive_layer_opacity, format_distance_input, grid_size_choices,
    load_grid_catalog, load_page_size, persist_grid_catalog, persist_page_size,
    system_decimal_separator,
};
use highlight::{
    attribute_highlight_color, component_highlight_color, d_code_highlight_color,
    net_highlight_color,
};
use measurement::draw_measurement;
use selection::{hit_test_visible_item, selected_primitive_color};

const MAX_VIEWER_LAYERS: usize = 32;
const CANVAS_MARGIN: f32 = 28.0;
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 30.0;

#[cfg(test)]
#[path = "../../tests/gerber_viewer/canvas.rs"]
mod gerber_canvas_test_definitions;

#[cfg(test)]
#[path = "../../tests/gerber_viewer/grid_state.rs"]
mod gerber_grid_state_test_definitions;

#[cfg(test)]
#[path = "../../tests/gerber_viewer/layer_state.rs"]
mod gerber_layer_state_test_definitions;

#[cfg(test)]
#[path = "../../tests/gerber_viewer/flash_outline.rs"]
mod gerber_flash_outline_test_definitions;

#[cfg(test)]
#[path = "../../tests/gerber_viewer/line_outline.rs"]
mod gerber_line_outline_test_definitions;

#[cfg(test)]
#[path = "../../tests/gerber_viewer/polygon_outline.rs"]
mod gerber_polygon_outline_test_definitions;

#[cfg(test)]
#[path = "../../tests/gerber_viewer/ghost_negatives.rs"]
mod gerber_ghost_negatives_test_definitions;

#[cfg(test)]
#[path = "../../tests/gerber_viewer/d_code_labels.rs"]
mod gerber_d_code_labels_test_definitions;

#[cfg(test)]
#[path = "../../tests/gerber_viewer/compare_mode.rs"]
mod gerber_compare_mode_test_definitions;

#[cfg(test)]
#[path = "../../tests/gerber_viewer/dim_inactive_layers.rs"]
mod gerber_dim_inactive_layers_test_definitions;

#[cfg(test)]
gerber_canvas_test_definitions::gerber_canvas_tests!();

#[cfg(test)]
gerber_grid_state_test_definitions::gerber_grid_state_tests!();

#[cfg(test)]
gerber_layer_state_test_definitions::gerber_layer_state_tests!();

#[cfg(test)]
gerber_flash_outline_test_definitions::gerber_flash_outline_tests!();

#[cfg(test)]
gerber_line_outline_test_definitions::gerber_line_outline_tests!();

#[cfg(test)]
gerber_polygon_outline_test_definitions::gerber_polygon_outline_tests!();

#[cfg(test)]
gerber_ghost_negatives_test_definitions::gerber_ghost_negatives_tests!();

#[cfg(test)]
gerber_d_code_labels_test_definitions::gerber_d_code_labels_tests!();


#[cfg(test)]
gerber_compare_mode_test_definitions::gerber_compare_mode_tests!();

#[cfg(test)]
gerber_dim_inactive_layers_test_definitions::gerber_dim_inactive_layers_tests!();
