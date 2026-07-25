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
    CommandMetadata {
        id: "gerber_sketch_lines",
        category: "display",
        label: "Sketch Gerber line items",
        menu_label: Some("Sketch Lines"),
        group: CommandGroup::Gerber,
        ..CommandMetadata::DEFAULT
    },
    CommandMetadata {
        id: "gerber_sketch_polygons",
        category: "display",
        label: "Sketch Gerber polygon items",
        menu_label: Some("Sketch Polygons"),
        group: CommandGroup::Gerber,
        ..CommandMetadata::DEFAULT
    },
    CommandMetadata {
        id: "gerber_show_d_codes",
        category: "display",
        label: "Show Gerber D-code labels",
        menu_label: Some("Show D-Codes"),
        group: CommandGroup::Gerber,
        ..CommandMetadata::DEFAULT
    },
    CommandMetadata {
        id: "gerber_compare_layers",
        category: "display",
        label: "Compare visible Gerber layers",
        menu_label: Some("Show in XOR Mode"),
        group: CommandGroup::Gerber,
        ..CommandMetadata::DEFAULT
    },
];
