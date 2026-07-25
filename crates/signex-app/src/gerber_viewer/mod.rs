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
    ApertureShape, Bounds, GerberLoadBatch, GerberPrimitive, LoadedLayer, PrimitivePolarity,
};
use signex_types::theme::ThemeTokens;

mod display;
mod gerber_viewer_state;
mod grid;
mod message;
pub(crate) mod print;
mod shortcuts;
mod view;

#[path = "canvas.rs"]
mod viewport;

pub use display::{GerberDisplayUnit, GerberPageSize, GerberPrintLayout};
pub use gerber_viewer_state::{GerberViewerState, ViewerLayer};
pub use message::GerberViewerMessage;
pub use view::view;

use shortcuts::{gerber_shortcut_message, next_layer_index, previous_layer_index};

use viewport::{
    material_layer_palette, page_bounds, visible_bounds, zoom_transform_for_selection,
};

#[cfg(test)]
use viewport::*;

use grid::{
    DEFAULT_GRID_INDEX, GridSizePreset, GridUnit, create_grid_definition,
    format_distance_input, grid_size_choices, load_grid_catalog, load_page_size,
    persist_grid_catalog, persist_page_size, system_decimal_separator,
};

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
gerber_canvas_test_definitions::gerber_canvas_tests!();

#[cfg(test)]
gerber_grid_state_test_definitions::gerber_grid_state_tests!();

#[cfg(test)]
gerber_layer_state_test_definitions::gerber_layer_state_tests!();
