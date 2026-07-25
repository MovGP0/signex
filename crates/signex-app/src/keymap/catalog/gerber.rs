use super::{CommandGroup, CommandMetadata};

pub(super) const GERBER: &[CommandMetadata] = &[
    CommandMetadata {
        id: "gerber_next_layer",
        category: "layers",
        label: "Next Gerber layer",
        menu_label: Some("Next Layer"),
        group: CommandGroup::Gerber,
        ..CommandMetadata::DEFAULT
    },
    CommandMetadata {
        id: "gerber_previous_layer",
        category: "layers",
        label: "Previous Gerber layer",
        menu_label: Some("Previous Layer"),
        group: CommandGroup::Gerber,
        ..CommandMetadata::DEFAULT
    },
    CommandMetadata {
        id: "gerber_sketch_flashes",
        category: "display",
        label: "Sketch Gerber flashed items",
        menu_label: Some("Sketch Flashed Items"),
        group: CommandGroup::Gerber,
        ..CommandMetadata::DEFAULT
    },
];
