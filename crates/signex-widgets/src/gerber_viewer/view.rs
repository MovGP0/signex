use super::*;
use super::viewport::*;
use iced::widget::column;

pub fn view<'a>(
    state: &'a GerberViewerState,
    shortcut_resolver: &'a dyn GerberShortcutResolver,
    tokens: &ThemeTokens,
) -> Element<'a, GerberViewerMessage>
{
    let text_primary = styles::ti(tokens.text);
    let text_muted = styles::ti(tokens.text_secondary);
    let border = styles::ti(tokens.border);
    let panel_bg = styles::ti(tokens.panel_bg);
    let canvas_bg = styles::ti(tokens.bg);

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
    let component_choices = state.component_choices();
    let component_picker = pick_list(
        component_choices,
        state.highlighted_component().map(str::to_owned),
        GerberViewerMessage::SetHighlightedComponent,
    )
    .placeholder("Highlight component")
    .width(180);
    let net_choices = state.net_choices();
    let net_picker = pick_list(
        net_choices,
        state.highlighted_net().map(str::to_owned),
        GerberViewerMessage::SetHighlightedNet,
    )
    .placeholder("Highlight net")
    .width(180);
    let attribute_choices = state.attribute_choices();
    let attribute_picker = pick_list(
        attribute_choices,
        state.highlighted_attribute().cloned(),
        GerberViewerMessage::SetHighlightedAttribute,
    )
    .placeholder("Highlight attribute")
    .width(220);
    let d_code_choices = state.d_code_choices();
    let selected_d_code = state.highlighted_d_code().and_then(|selected|
    {
        d_code_choices
            .iter()
            .find(|choice| choice.code == selected)
            .cloned()
    });
    let d_code_picker = pick_list(
        d_code_choices,
        selected_d_code,
        |choice| GerberViewerMessage::SetHighlightedDCode(choice.code),
    )
    .placeholder("Highlight D-code")
    .width(220);

    let toolbar = container(
        row![
            open_button,
            button(text("Open Autodetected…"))
                .on_press_maybe((!state.loading).then_some(
                    GerberViewerMessage::OpenAutodetectedFiles,
                )),
            button(text("Open ZIP…"))
                .on_press_maybe((!state.loading).then_some(
                    GerberViewerMessage::OpenZipArchive,
                )),
            button(text("Open Job…"))
                .on_press_maybe((!state.loading).then_some(
                    GerberViewerMessage::OpenGerberJob,
                )),
            button(text("Open Drill Files…"))
                .on_press_maybe((!state.loading).then_some(
                    GerberViewerMessage::OpenExcellonFiles,
                )),
            button(text("Redraw")).on_press(GerberViewerMessage::RedrawViewport),
            button(text(if state.sketch_flashes
            {
                "Fill Flashes"
            }
            else
            {
                "Sketch Flashes (F)"
            }))
            .on_press(GerberViewerMessage::ToggleSketchFlashes),
            button(text(if state.sketch_lines
            {
                "Fill Lines"
            }
            else
            {
                "Sketch Lines (L)"
            }))
            .on_press(GerberViewerMessage::ToggleSketchLines),
            button(text(if state.sketch_polygons
            {
                "Fill Polygons"
            }
            else
            {
                "Sketch Polygons (P)"
            }))
            .on_press(GerberViewerMessage::ToggleSketchPolygons),
            button(text(if state.ghost_negative_objects
            {
                "Hide Negative Objects"
            }
            else
            {
                "Ghost Negative Objects"
            }))
            .on_press(GerberViewerMessage::ToggleGhostNegativeObjects),
            button(text(if state.show_d_code_labels
            {
                "Hide D-Codes"
            }
            else
            {
                "Show D-Codes (D)"
            }))
            .on_press(GerberViewerMessage::ToggleDCodeLabels),
            button(text(if state.compare_mode
            {
                "Normal Layer Colors"
            }
            else
            {
                "XOR Compare"
            }))
            .on_press(GerberViewerMessage::ToggleCompareMode),
            button(text(if state.dim_inactive_layers
            {
                "Normal Layer Contrast"
            }
            else
            {
                "Dim Inactive Layers"
            }))
            .on_press(GerberViewerMessage::ToggleDimInactiveLayers),
            button(text(if state.mirrored
            {
                "Normal Orientation"
            }
            else
            {
                "Flip Gerber View"
            }))
            .on_press(GerberViewerMessage::ToggleMirrored),
            button(text("Reload All")).on_press_maybe(
                (!state.loading && !state.layers.is_empty())
                    .then_some(GerberViewerMessage::ReloadAllLayers),
            ),
            button(text("−")).on_press(GerberViewerMessage::ZoomBy(1.0 / 1.2)),
            button(text("+")).on_press(GerberViewerMessage::ZoomBy(1.2)),
            button(text("Fit")).on_press(GerberViewerMessage::FitPage),
            button(text(if state.zoom_selection_active
            {
                "Cancel Zoom Area"
            }
            else
            {
                "Zoom Area"
            }))
            .on_press(GerberViewerMessage::ToggleZoomSelection),
            button(text(if state.measurement_active()
            {
                "Cancel Measure"
            }
            else
            {
                "Measure"
            }))
            .on_press(GerberViewerMessage::ToggleMeasurement),
            button(text("Reset Measure")).on_press_maybe(
                state
                    .measurement()
                    .map(|_| GerberViewerMessage::ResetMeasurement),
            ),
            button(text("Print PDF…")).on_press_maybe(
                self::print::has_visible_layers(state)
                    .then_some(GerberViewerMessage::PrintVisibleLayers),
            ),
            button(text("Previous Layer (PgUp)")).on_press_maybe(
                previous_layer_index(state.active_layer, state.layers.len())
                    .map(|_| GerberViewerMessage::PreviousLayer),
            ),
            button(text("Next Layer (PgDn)")).on_press_maybe(
                next_layer_index(state.active_layer, state.layers.len())
                    .map(|_| GerberViewerMessage::NextLayer),
            ),
            button(text("Move Layer Up (+)")).on_press_maybe(
                state
                    .active_layer
                    .filter(|index| *index + 1 < state.layers.len())
                    .map(|_| GerberViewerMessage::MoveLayerUp),
            ),
            button(text("Move Layer Down (-)")).on_press_maybe(
                state
                    .active_layer
                    .filter(|index| *index > 0)
                    .map(|_| GerberViewerMessage::MoveLayerDown),
            ),
            button(text(if state.layer_manager_visible
            {
                "Hide Layers"
            }
            else
            {
                "Show Layers"
            }))
            .on_press(GerberViewerMessage::ToggleLayerManager),
            button(text(if state.layer_information_visible
            {
                "Hide Layer Info"
            }
            else
            {
                "Layer Info"
            }))
            .on_press_maybe(
                state
                    .active_layer
                    .map(|_| GerberViewerMessage::ToggleLayerInformation),
            ),
            button(text(if state.d_code_list_visible
            {
                "Hide D-Codes"
            }
            else
            {
                "List D-Codes"
            }))
            .on_press_maybe(
                (!state.layers.is_empty())
                    .then_some(GerberViewerMessage::ToggleDCodeList),
            ),
            button(text(if state.source_view_visible
            {
                "Hide Source"
            }
            else
            {
                "Show Source"
            }))
            .on_press_maybe(
                state
                    .active_layer
                    .map(|_| GerberViewerMessage::ToggleSourceView),
            ),
            component_picker,
            button(text("Clear Component")).on_press_maybe(
                state
                    .highlighted_component()
                    .map(|_| GerberViewerMessage::ClearComponentHighlight),
            ),
            net_picker,
            button(text("Clear Net")).on_press_maybe(
                state
                    .highlighted_net()
                    .map(|_| GerberViewerMessage::ClearNetHighlight),
            ),
            attribute_picker,
            button(text("Clear Attribute")).on_press_maybe(
                state
                    .highlighted_attribute()
                    .map(|_| GerberViewerMessage::ClearAttributeHighlight),
            ),
            d_code_picker,
            button(text("Clear D-code")).on_press_maybe(
                state
                    .highlighted_d_code()
                    .map(|_| GerberViewerMessage::ClearDCodeHighlight),
            ),
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
    .style(styles::toolbar_strip(tokens));

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
    let page_size_picker = pick_list(
        GerberPageSize::ALL,
        Some(state.page_size),
        GerberViewerMessage::SetPageSize,
    )
    .width(105);
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
    let measurement_label = state
        .measurement_summary()
        .unwrap_or_else(|| "Measurement: —".to_owned());
    let grid_toolbar = container(
        row![
            text("Grid").size(11).color(text_muted),
            checkbox(state.grid_visible)
                .label("Visible")
                .on_toggle(GerberViewerMessage::ToggleGridVisibility),
            grid_picker,
            display_unit_picker,
            text("Page").size(11).color(text_muted),
            page_size_picker,
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
            text(measurement_label)
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
    .style(styles::toolbar_strip(tokens));
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
            .style(styles::toolbar_strip(tokens))
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
            .style(styles::chrome_separator(tokens)),
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
        let color_choices = state.layer_color_choices();
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
            .style(styles::rail_tab(tokens, active));
            let selected_color = state.selected_layer_color_choice(index);
            let color_picker = pick_list(
                color_choices.clone(),
                selected_color,
                move |choice| {
                    GerberViewerMessage::SetLayerColor(
                        index,
                        choice.palette_index,
                    )
                },
            )
            .placeholder("Layer color")
            .width(128);
            layer_list = layer_list.push(
                row![visible, label, color_picker]
                    .spacing(5)
                    .align_y(iced::Alignment::Center),
            );
        }
    }

    let item_color_choices = state.layer_color_choices();
    let grid_color = state.selected_color_choice(state.grid_color);
    let d_code_color = state.selected_color_choice(state.d_code_color);
    let negative_color =
        state.selected_color_choice(state.negative_ghost_color);
    layer_list = layer_list
        .push(
            container(Space::new())
                .width(Length::Fill)
                .height(1)
                .style(styles::chrome_separator(tokens)),
        )
        .push(text("Item colors · Material Design").size(12).color(text_primary))
        .push(
            row![
                text("Grid").size(11).width(70),
                pick_list(
                    item_color_choices.clone(),
                    grid_color,
                    |choice| GerberViewerMessage::SetGridColor(
                        choice.palette_index,
                    ),
                )
                .width(Length::Fill),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .push(
            row![
                text("D-codes").size(11).width(70),
                pick_list(
                    item_color_choices.clone(),
                    d_code_color,
                    |choice| GerberViewerMessage::SetDCodeColor(
                        choice.palette_index,
                    ),
                )
                .width(Length::Fill),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .push(
            row![
                text("Negative").size(11).width(70),
                pick_list(
                    item_color_choices,
                    negative_color,
                    |choice| GerberViewerMessage::SetNegativeObjectColor(
                        choice.palette_index,
                    ),
                )
                .width(Length::Fill),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        );

    if state.layer_information_visible
    {
        layer_list = layer_list.push(
            container(Space::new())
                .width(Length::Fill)
                .height(1)
                .style(styles::chrome_separator(tokens)),
        );
        if let Some(metadata) = state.active_layer_metadata()
        {
            let bounds = metadata
                .bounds
                .map(|bounds| {
                    format_bounds_in_unit(
                        bounds,
                        state.display_unit,
                        &state.decimal_separator,
                    )
                })
                .unwrap_or_else(|| "Bounds: unavailable".to_owned());
            let coordinate_format = metadata
                .coordinate_format
                .unwrap_or_else(|| "Unavailable".to_owned());
            let definitions = if metadata.definitions.is_empty()
            {
                "None".to_owned()
            }
            else
            {
                metadata.definitions.join("\n")
            };
            let attributes = if metadata.attributes.is_empty()
            {
                "None".to_owned()
            }
            else
            {
                metadata.attributes.join("\n")
            };
            let warnings = if metadata.warnings.is_empty()
            {
                "None".to_owned()
            }
            else
            {
                metadata.warnings.join("\n")
            };
            layer_list = layer_list.push(
                column![
                    text("Active Layer Information")
                        .size(13)
                        .color(text_primary),
                    text(format!("File: {}", metadata.file_name))
                        .size(10)
                        .color(text_muted),
                    text(format!("Source: {}", metadata.source))
                        .size(10)
                        .color(text_muted),
                    text(format!("Format: {}", metadata.format))
                        .size(10)
                        .color(text_muted),
                    text(format!("Role: {}", metadata.layer_role))
                        .size(10)
                        .color(text_muted),
                    text(format!("Units: {}", metadata.units))
                        .size(10)
                        .color(text_muted),
                    text(format!("Coordinate format: {coordinate_format}"))
                        .size(10)
                        .color(text_muted),
                    text(bounds).size(10).color(text_muted),
                    text(format!("Rendered primitives: {}", metadata.primitive_count))
                        .size(10)
                        .color(text_muted),
                    text(format!("{}:\n{definitions}", metadata.definition_label))
                        .size(10)
                        .color(text_muted),
                    text(format!("Attributes:\n{attributes}"))
                        .size(10)
                        .color(text_muted),
                    text(format!("Warnings:\n{warnings}"))
                        .size(10)
                        .color(text_muted),
                ]
                .spacing(4),
            );
        }
        else
        {
            layer_list = layer_list.push(
                text("No active layer")
                    .size(10)
                    .color(text_muted),
            );
        }
    }

    if state.d_code_list_visible
    {
        layer_list = layer_list.push(
            container(Space::new())
                .width(Length::Fill)
                .height(1)
                .style(styles::chrome_separator(tokens)),
        );
        layer_list = layer_list.push(
            text("D-Codes and Drill Tools")
                .size(13)
                .color(text_primary),
        );
        for group in state.definition_groups()
        {
            let definitions = if group.definitions.is_empty()
            {
                format!("No {} defined", group.definition_label.to_lowercase())
            }
            else
            {
                group
                    .definitions
                    .iter()
                    .map(|definition| {
                        format!(
                            "{} — {} — {} use(s)",
                            definition.code,
                            definition.description,
                            definition.usage_count,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            layer_list = layer_list.push(
                column![
                    text(format!("{} · {}", group.layer_name, group.definition_label))
                        .size(11)
                        .color(text_primary),
                    text(definitions).size(10).color(text_muted),
                ]
                .spacing(2),
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
        grid: state.grid_color,
        grid_visible: state.grid_visible,
        full_window_crosshair: state.full_window_crosshair,
        page_size: state.page_size,
        zoom_selection_active: state.zoom_selection_active,
        measurement_active: state.measurement_active(),
        measurement: state.measurement(),
        sketch_flashes: state.sketch_flashes,
        sketch_lines: state.sketch_lines,
        sketch_polygons: state.sketch_polygons,
        ghost_negative_objects: state.ghost_negative_objects,
        negative_ghost_color: state.negative_ghost_color,
        show_d_code_labels: state.show_d_code_labels,
        d_code_color: state.d_code_color,
        compare_mode: state.compare_mode,
        compare_palette: &state.compare_palette,
        dim_inactive_layers: state.dim_inactive_layers,
        inactive_layer_opacity: state.inactive_layer_opacity,
        mirrored: state.mirrored,
        active_layer: state.active_layer,
        highlighted_component: state.highlighted_component(),
        highlighted_net: state.highlighted_net(),
        highlighted_attribute: state.highlighted_attribute(),
        highlighted_d_code: state.highlighted_d_code(),
        selected_item: state.selected_item(),
        redraw_generation: state.redraw_generation,
        zoom: state.zoom,
        pan: state.pan,
        grid_size: state.active_grid(),
        shortcut_resolver,
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
    let content: Element<'_, GerberViewerMessage> = if state.source_view_visible
    {
        let source_content: Element<'_, GerberViewerMessage> =
            match state.active_gerber_source()
            {
                Ok((name, source)) => column![
                    text(format!("Original Gerber source — {name}"))
                        .size(13)
                        .color(text_primary),
                    scrollable(
                        container(
                            text(source)
                                .size(11)
                                .font(iced::Font::MONOSPACE)
                                .color(text_primary),
                        )
                        .padding(12)
                        .width(Length::Fill),
                    )
                    .height(Length::Fill),
                ]
                .spacing(8)
                .padding(10)
                .into(),
                Err(message) => container(
                    text(message)
                        .size(12)
                        .color(text_muted),
                )
                .center(Length::Fill)
                .into(),
            };
        container(source_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(canvas_bg)),
                ..container::Style::default()
            })
            .into()
    }
    else if state.layer_manager_visible
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
    .style(styles::status_bar(tokens));

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
