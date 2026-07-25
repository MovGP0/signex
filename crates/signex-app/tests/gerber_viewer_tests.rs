// Gerber viewer tests are defined as macros so this file can be compiled both
// as Cargo's integration-test target and in the owning modules, where the
// tests retain access to implementation details without widening production APIs.

#![allow(unused_imports, unused_macros)]

macro_rules! gerber_viewer_tests
{
    () => {
        #[cfg(test)]
        mod tests
        {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn bundled_palette_contains_material_colors_for_every_layer_slot()
    {
        let palette = material_layer_palette();

        assert_eq!(palette.len(), 64);
        assert_eq!(palette[0], Color::from_rgb8(211, 47, 47));
    }

    #[test]
    fn selecting_grid_updates_rectangular_viewport_spacing()
    {
        let mut state = GerberViewerState::default();
        state.decimal_separator = ".".to_owned();
        let initial_generation = state.redraw_generation;

        assert_eq!(state.active_grid_index, DEFAULT_GRID_INDEX);
        assert_eq!(state.active_grid().x_millimetres(), 1.27);
        assert_eq!(state.active_grid().y_millimetres(), 1.27);

        state.select_grid_size(13);

        assert_eq!(state.active_grid_index, 13);
        assert_eq!(state.active_grid().x_millimetres(), 1.5);
        assert_eq!(state.active_grid().y_millimetres(), 2.5);
        assert_eq!(state.redraw_generation, initial_generation + 1);
        assert_eq!(
            state.status,
            "Grid: 1.5000 mm ⨯ 2.5000 mm (59.06 mils ⨯ 98.43 mils)"
        );
    }

    #[test]
    fn toggling_grid_visibility_only_requests_viewport_redraw()
    {
        let mut state = GerberViewerState::default();
        let grid_catalog = state.grid_catalog.clone();
        let active_grid_index = state.active_grid_index;
        let layers = state.layers.len();
        let initial_generation = state.redraw_generation;

        state.set_grid_visible(false);

        assert!(!state.grid_visible);
        assert_eq!(state.redraw_generation, initial_generation + 1);
        assert_eq!(state.grid_catalog, grid_catalog);
        assert_eq!(state.active_grid_index, active_grid_index);
        assert_eq!(state.layers.len(), layers);

        state.set_grid_visible(false);
        assert_eq!(state.redraw_generation, initial_generation + 1);

        state.set_grid_visible(true);
        assert!(state.grid_visible);
        assert_eq!(state.redraw_generation, initial_generation + 2);
    }

    #[test]
    fn display_units_convert_coordinates_and_bounds_from_millimetres()
    {
        let point = signex_gerber::Point { x: 25.4, y: 12.7 };
        let bounds = Bounds {
            min: signex_gerber::Point { x: 0.0, y: -12.7 },
            max: point,
        };

        assert_eq!(
            format_coordinate_in_unit(
                point,
                GerberDisplayUnit::Millimetres,
                ".",
                false,
            ),
            "X: 25.4000  Y: 12.7000 mm"
        );
        assert_eq!(
            format_coordinate_in_unit(
                point,
                GerberDisplayUnit::Inches,
                ".",
                false,
            ),
            "X: 1.0000  Y: 0.5000 in"
        );
        assert_eq!(
            format_coordinate_in_unit(
                point,
                GerberDisplayUnit::Mils,
                ".",
                false,
            ),
            "X: 1000.00  Y: 500.00 mils"
        );
        assert_eq!(
            format_bounds_in_unit(bounds, GerberDisplayUnit::Inches, "."),
            "Bounds: X 0.0000…1.0000  Y -0.5000…0.5000 in"
        );
    }

    #[test]
    fn polar_coordinates_use_selected_unit_and_cartesian_origin()
    {
        let point = signex_gerber::Point { x: 25.4, y: 25.4 };

        assert_eq!(
            format_coordinate_in_unit(
                point,
                GerberDisplayUnit::Inches,
                ".",
                true,
            ),
            "R: 1.4142 in  θ: 45.00°"
        );
        assert_eq!(
            format_coordinate_in_unit(
                signex_gerber::Point::default(),
                GerberDisplayUnit::Millimetres,
                ".",
                true,
            ),
            "R: 0.0000 mm  θ: 0.00°"
        );
        assert_eq!(
            format_coordinate_in_unit(
                signex_gerber::Point { x: 0.0, y: -25.4 },
                GerberDisplayUnit::Mils,
                ".",
                true,
            ),
            "R: 1000.00 mils  θ: -90.00°"
        );
    }

    #[test]
    fn toggling_polar_coordinates_preserves_pointer_and_geometry_state()
    {
        let mut state = GerberViewerState::default();
        let position = signex_gerber::Point { x: 3.0, y: 4.0 };
        state.set_cursor_world_position(Some(position));
        let grid_catalog = state.grid_catalog.clone();

        state.set_polar_coordinates(true);

        assert!(state.polar_coordinates);
        assert_eq!(state.cursor_world_position, Some(position));
        assert_eq!(state.grid_catalog, grid_catalog);
    }

    #[test]
    fn full_window_crosshair_spans_viewport_and_toggle_requests_redraw()
    {
        let bounds = Rectangle::new(
            Point::ORIGIN,
            iced::Size::new(640.0, 480.0),
        );
        let segments =
            full_window_crosshair_segments(bounds, Point::new(120.0, 75.0));

        assert_eq!(
            segments,
            [
                (Point::new(0.0, 75.0), Point::new(640.0, 75.0)),
                (Point::new(120.0, 0.0), Point::new(120.0, 480.0)),
            ]
        );

        let mut state = GerberViewerState::default();
        let initial_generation = state.redraw_generation;
        state.set_full_window_crosshair(true);

        assert!(state.full_window_crosshair);
        assert_eq!(state.redraw_generation, initial_generation + 1);

        state.set_full_window_crosshair(true);
        assert_eq!(state.redraw_generation, initial_generation + 1);
    }

    #[test]
    fn changing_display_unit_does_not_modify_source_geometry()
    {
        let layer = signex_gerber::load_gerber_reader(
            "copper.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![layer.clone()],
            failures: Vec::new(),
        });
        let zoom = state.zoom;
        let pan = state.pan;

        state.set_display_unit(GerberDisplayUnit::Inches);
        state.set_cursor_world_position(Some(signex_gerber::Point {
            x: 25.4,
            y: 12.7,
        }));

        assert_eq!(state.display_unit, GerberDisplayUnit::Inches);
        assert_eq!(state.layers[0].layer, layer);
        assert_eq!(state.zoom, zoom);
        assert_eq!(state.pan, pan);
    }

    #[test]
    fn creating_grid_appends_selects_and_persists_definition()
    {
        let mut state = GerberViewerState::default();
        state.decimal_separator = ".".to_owned();
        state.set_new_grid_name(" Fine metric ".to_owned());
        state.set_new_grid_x("0.05".to_owned());
        state.set_new_grid_y("0".to_owned());
        state.set_new_grid_unit_millimetres(true);
        let original_count = state.grid_catalog.len();
        let mut persisted = Vec::new();

        state.create_grid_with(|catalog| {
            persisted = catalog.to_vec();
            Ok(())
        });

        assert_eq!(state.grid_catalog.len(), original_count + 1);
        assert_eq!(persisted, state.grid_catalog);
        assert_eq!(state.active_grid_index, original_count);
        assert_eq!(state.active_grid().name.as_deref(), Some("Fine metric"));
        assert_eq!(state.active_grid().x_millimetres(), 0.05);
        assert_eq!(state.active_grid().y_millimetres(), 0.0);
        assert_eq!(
            state.status,
            "Created grid: Fine metric: 0.0500 mm ⨯ 0.0000 mm (1.97 mils ⨯ 0.00 mils)"
        );
        assert!(state.new_grid_name.is_empty());
        assert!(state.new_grid_x.is_empty());
        assert!(state.new_grid_y.is_empty());
        assert_eq!(state.grid_editor_error, None);
    }

    #[test]
    fn invalid_or_unpersisted_grid_is_not_added()
    {
        let mut state = GerberViewerState::default();
        let original_catalog = state.grid_catalog.clone();
        state.set_new_grid_x("0".to_owned());
        state.set_new_grid_y("1".to_owned());

        state.create_grid_with(|_| panic!("invalid grid must not persist"));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("X must be a finite number greater than zero.")
        );

        state.set_new_grid_x("1".to_owned());
        state.create_grid_with(|_| Err("settings unavailable".to_owned()));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("settings unavailable")
        );
    }

    #[test]
    fn editing_grid_updates_same_entry_and_persists_catalog()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        state.decimal_separator = ".".to_owned();
        state.load_active_grid_into_editor();
        let original_count = state.grid_catalog.len();
        let initial_generation = state.redraw_generation;
        state.set_edit_grid_name(" Fine metric ".to_owned());
        state.set_edit_grid_unit_millimetres(true);
        state.set_edit_grid_x("0.05".to_owned());
        state.set_edit_grid_y("0.10".to_owned());
        let mut persisted = Vec::new();

        state.update_grid_with(|catalog| {
            persisted = catalog.to_vec();
            Ok(())
        });

        assert_eq!(state.grid_catalog, persisted);
        assert_eq!(state.grid_catalog.len(), original_count);
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(state.active_grid().name.as_deref(), Some("Fine metric"));
        assert_eq!(state.active_grid().x_millimetres(), 0.05);
        assert_eq!(state.active_grid().y_millimetres(), 0.10);
        assert_eq!(state.active_grid().unit, GridUnit::Mm);
        assert_eq!(state.redraw_generation, initial_generation + 1);
        assert_eq!(
            state.status,
            "Updated grid: Fine metric: 0.0500 mm ⨯ 0.1000 mm (1.97 mils ⨯ 3.94 mils)"
        );
        assert_eq!(state.edit_grid_name, "Fine metric");
        assert_eq!(state.edit_grid_x, "0.05");
        assert_eq!(state.edit_grid_y, "0.1");
        assert_eq!(state.grid_editor_error, None);
    }

    #[test]
    fn changing_edit_unit_preserves_physical_grid_spacing()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..2].to_vec();
        state.active_grid_index = 0;
        state.decimal_separator = ".".to_owned();
        state.load_active_grid_into_editor();
        let original_x = state.active_grid().x_millimetres();
        let original_y = state.active_grid().y_millimetres();

        state.set_edit_grid_unit_millimetres(true);

        assert_eq!(state.edit_grid_unit, GridUnit::Mm);
        assert_eq!(state.edit_grid_x, "2.54");
        assert_eq!(state.edit_grid_y, "2.54");
        let metric = create_grid_definition(
            "",
            &state.edit_grid_x,
            &state.edit_grid_y,
            state.edit_grid_unit,
            &state.decimal_separator,
        )
        .expect("converted metric draft");
        assert_eq!(metric.x_millimetres(), original_x);
        assert_eq!(metric.y_millimetres(), original_y);

        state.set_edit_grid_unit_millimetres(false);

        assert_eq!(state.edit_grid_unit, GridUnit::Mil);
        assert_eq!(state.edit_grid_x, "100");
        assert_eq!(state.edit_grid_y, "100");
    }

    #[test]
    fn invalid_or_unpersisted_grid_edit_does_not_change_catalog()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        state.load_active_grid_into_editor();
        let original_catalog = state.grid_catalog.clone();
        state.set_edit_grid_x("0".to_owned());

        state.update_grid_with(|_| panic!("invalid edit must not persist"));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("X must be a finite number greater than zero.")
        );

        state.set_edit_grid_x("1".to_owned());
        state.update_grid_with(|_| Err("settings unavailable".to_owned()));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("settings unavailable")
        );
    }

    #[test]
    fn deleting_grid_persists_catalog_and_selects_next_entry()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        let removed = state.grid_catalog[1].clone();
        let next = state.grid_catalog[2].clone();
        let initial_generation = state.redraw_generation;
        let mut persisted = Vec::new();

        state.delete_grid_with(|catalog| {
            persisted = catalog.to_vec();
            Ok(())
        });

        assert_eq!(state.grid_catalog, persisted);
        assert_eq!(state.grid_catalog.len(), 2);
        assert!(!state.grid_catalog.contains(&removed));
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(state.active_grid(), &next);
        assert_eq!(state.redraw_generation, initial_generation + 1);
        assert!(state.status.starts_with("Deleted grid: "));
        assert_eq!(state.grid_editor_error, None);
    }

    #[test]
    fn deleting_final_grid_selects_previous_and_never_empties_catalog()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..2].to_vec();
        state.active_grid_index = 1;
        let previous = state.grid_catalog[0].clone();

        state.delete_grid_with(|_| Ok(()));

        assert_eq!(state.grid_catalog, vec![previous.clone()]);
        assert_eq!(state.active_grid_index, 0);
        assert_eq!(state.active_grid(), &previous);

        state.delete_grid_with(|_| panic!("last grid must not persist"));

        assert_eq!(state.grid_catalog, vec![previous]);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("At least one grid definition must remain.")
        );
    }

    #[test]
    fn failed_grid_deletion_does_not_change_catalog_or_selection()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        let original_catalog = state.grid_catalog.clone();

        state.delete_grid_with(|_| Err("settings unavailable".to_owned()));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("settings unavailable")
        );
    }

    #[test]
    fn moving_grid_up_and_down_preserves_selected_definition()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        let original_catalog = state.grid_catalog.clone();
        let selected = state.active_grid().clone();
        let mut persisted = Vec::new();

        state.move_grid_up_with(|catalog| {
            persisted = catalog.to_vec();
            Ok(())
        });

        assert_eq!(state.grid_catalog, persisted);
        assert_eq!(state.active_grid_index, 0);
        assert_eq!(state.active_grid(), &selected);
        assert_eq!(state.grid_catalog[1], original_catalog[0]);

        state.move_grid_down_with(|catalog| {
            persisted = catalog.to_vec();
            Ok(())
        });

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(state.grid_catalog, persisted);
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(state.active_grid(), &selected);
    }

    #[test]
    fn moving_grid_at_boundary_is_a_no_op()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        let original_catalog = state.grid_catalog.clone();
        let original_generation = state.redraw_generation;

        state.active_grid_index = 0;
        state.move_grid_up_with(|_| panic!("upper boundary must not persist"));
        state.active_grid_index = state.grid_catalog.len() - 1;
        state.move_grid_down_with(|_| panic!("lower boundary must not persist"));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(state.active_grid_index, 2);
        assert_eq!(state.redraw_generation, original_generation);
    }

    #[test]
    fn failed_grid_move_does_not_change_catalog_or_selection()
    {
        let mut state = GerberViewerState::default();
        state.grid_catalog = grid::default_grid_catalog()[0..3].to_vec();
        state.active_grid_index = 1;
        let original_catalog = state.grid_catalog.clone();

        state.move_grid_up_with(|_| Err("settings unavailable".to_owned()));

        assert_eq!(state.grid_catalog, original_catalog);
        assert_eq!(state.active_grid_index, 1);
        assert_eq!(
            state.grid_editor_error.as_deref(),
            Some("settings unavailable")
        );
    }

    #[test]
    fn zero_grid_axis_is_not_replaced_with_the_other_axis()
    {
        let mut state = GerberViewerState::default();

        state.select_grid_size(19);

        assert_eq!(state.active_grid().x_millimetres(), 0.05);
        assert_eq!(state.active_grid().y_millimetres(), 0.0);
        assert!(visible_grid_spacing(0.05 * 100.0).is_some());
        assert_eq!(visible_grid_spacing(0.0), None);
    }

    #[test]
    fn layer_limit_accepts_32_mixed_layers_and_reports_overflow()
    {
        let mut state = GerberViewerState::default();
        let gerber = signex_gerber::load_gerber_reader(
            "copper.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let drill = signex_gerber::load_excellon_reader(
            "holes.drl",
            Cursor::new(
                b"M48\nMETRIC\nT01C0.8\n%\nG05\nT01\nX1.0Y1.0\nM30\n",
            ),
        )
        .expect("test Excellon must parse");
        let mut mixed = Vec::new();
        for _ in 0..16
        {
            mixed.push(gerber.clone());
            mixed.push(drill.clone());
        }

        state.apply_load_batch(GerberLoadBatch {
            layers: mixed,
            failures: Vec::new(),
        });
        assert_eq!(state.layers.len(), MAX_VIEWER_LAYERS);
        assert_eq!(
            state
                .layers
                .iter()
                .filter(|layer| layer.layer.layer_type == signex_gerber::LayerType::Drill)
                .count(),
            16
        );

        state.apply_load_batch(GerberLoadBatch {
            layers: vec![gerber],
            failures: Vec::new(),
        });
        assert_eq!(state.layers.len(), MAX_VIEWER_LAYERS);
        assert!(state.status.contains("1 layer(s) were skipped"));
    }

    #[test]
    fn autodetected_mixed_batch_adds_each_successful_format_as_a_layer()
    {
        let gerber = signex_gerber::load_autodetected_reader(
            "artwork.data",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("Gerber signature");
        let drill = signex_gerber::load_autodetected_reader(
            "drill.data",
            Cursor::new(b"M48\nMETRIC\nT01C0.8\n%\nG05\nT01\nX1.0Y1.0\nM30\n"),
        )
        .expect("Excellon signature");
        let mut state = GerberViewerState::default();

        state.apply_load_batch(GerberLoadBatch {
            layers: vec![gerber, drill],
            failures: vec![signex_gerber::GerberLoadFailure {
                path: PathBuf::from("notes.txt"),
                message: "unsupported fabrication file".to_owned(),
            }],
        });

        assert_eq!(state.layers.len(), 2);
        assert_eq!(state.layers[0].layer.name, "artwork.data");
        assert_eq!(
            state.layers[1].layer.layer_type,
            signex_gerber::LayerType::Drill,
        );
        assert!(state.status.contains("Loaded 2 fabrication layer(s)."));
        assert!(state.status.contains("1 file(s) could not be loaded"));
    }

    #[test]
    fn redraw_changes_only_the_viewport_generation()
    {
        let mut state = GerberViewerState::default();
        let status_before = state.status.clone();
        let layer_count_before = state.layers.len();

        state.redraw_viewport();

        assert_eq!(state.redraw_generation, 1);
        assert_eq!(state.layers.len(), layer_count_before);
        assert_ne!(state.status, status_before);
    }

    #[test]
    fn zoom_updates_scale_and_clamps_to_safe_limits()
    {
        let mut state = GerberViewerState::default();

        state.zoom_by(2.0);
        assert_eq!(state.zoom, 2.0);

        state.zoom_by(100.0);
        assert_eq!(state.zoom, MAX_ZOOM);

        state.zoom_by(0.0001);
        assert_eq!(state.zoom, MIN_ZOOM);

        state.zoom_by(f32::NAN);
        assert_eq!(state.zoom, MIN_ZOOM);
    }

    #[test]
    fn fit_page_resets_navigation_and_computes_valid_landscape_transform()
    {
        let bounds = Bounds {
            min: signex_gerber::Point { x: 10.0, y: 20.0 },
            max: signex_gerber::Point { x: 110.0, y: 70.0 },
        };
        let viewport = Rectangle::new(Point::ORIGIN, iced::Size::new(1000.0, 700.0));
        let (scale, world_center, screen_center) =
            fit_transform(bounds, viewport, 1.0, iced::Vector::default());

        assert!(scale.is_finite() && scale > 0.0);
        assert_eq!(world_center, Point::new(60.0, 45.0));
        assert_eq!(screen_center, Point::new(500.0, 350.0));
        assert!(bounds.width() as f32 * scale <= viewport.width - CANVAS_MARGIN * 2.0 + 0.01);
        assert!(bounds.height() as f32 * scale <= viewport.height - CANVAS_MARGIN * 2.0 + 0.01);

        let mut state = GerberViewerState::default();
        state.zoom = 4.0;
        state.pan = iced::Vector::new(100.0, -50.0);
        state.fit_page();
        assert_eq!(state.zoom, 1.0);
        assert_eq!(state.pan, iced::Vector::default());
    }

    #[test]
    fn page_size_updates_boundary_and_print_layout()
    {
        let artwork = Bounds {
            min: signex_gerber::Point { x: 10.0, y: 20.0 },
            max: signex_gerber::Point { x: 110.0, y: 70.0 },
        };
        assert_eq!(
            page_bounds(Some(artwork), GerberPageSize::FullSize),
            Some(artwork),
        );
        let a4 = page_bounds(Some(artwork), GerberPageSize::A4)
            .expect("fixed page bounds");
        assert!((a4.width() - 297.0).abs() < f64::EPSILON);
        assert!((a4.height() - 210.0).abs() < f64::EPSILON);
        assert!(((a4.min.x + a4.max.x) * 0.5 - 60.0).abs() < f64::EPSILON);
        assert!(((a4.min.y + a4.max.y) * 0.5 - 45.0).abs() < f64::EPSILON);

        let layer = signex_gerber::load_gerber_reader(
            "page.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![layer],
            failures: Vec::new(),
        });
        let initial_generation = state.redraw_generation;
        state.set_page_size_with(GerberPageSize::B, |_| Ok(()));

        let layout = state.print_layout().expect("print layout");
        assert_eq!(layout.page_size, GerberPageSize::B);
        assert!((layout.bounds.width() - 431.8).abs() < 0.000_001);
        assert!((layout.bounds.height() - 279.4).abs() < 0.000_001);
        assert_eq!(state.redraw_generation, initial_generation + 1);
        assert_eq!(state.status, "Page size: ANSI B.");
    }

    #[test]
    fn failed_page_size_persistence_keeps_existing_selection()
    {
        let mut state = GerberViewerState::default();
        state.page_size = GerberPageSize::A4;

        state.set_page_size_with(GerberPageSize::A3, |_| {
            Err("settings unavailable".to_owned())
        });

        assert_eq!(state.page_size, GerberPageSize::A4);
        assert!(state.status.contains("settings unavailable"));
    }

    #[test]
    fn zoom_area_normalizes_reverse_drag_and_rejects_degenerate_drag()
    {
        let forward = normalized_screen_rectangle(
            Point::new(20.0, 30.0),
            Point::new(120.0, 80.0),
        )
        .expect("forward selection");
        let reverse = normalized_screen_rectangle(
            Point::new(120.0, 80.0),
            Point::new(20.0, 30.0),
        )
        .expect("reverse selection");

        assert_eq!(forward, reverse);
        assert_eq!(forward.x, 20.0);
        assert_eq!(forward.y, 30.0);
        assert_eq!(forward.width, 100.0);
        assert_eq!(forward.height, 50.0);
        assert!(normalized_screen_rectangle(
            Point::new(10.0, 10.0),
            Point::new(12.0, 40.0),
        )
        .is_none());
        assert!(normalized_screen_rectangle(
            Point::new(10.0, 10.0),
            Point::new(40.0, 12.0),
        )
        .is_none());
    }

    #[test]
    fn zoom_selection_centres_and_fits_selected_world_bounds()
    {
        let base = Bounds {
            min: signex_gerber::Point { x: 0.0, y: 0.0 },
            max: signex_gerber::Point { x: 200.0, y: 100.0 },
        };
        let selection = Bounds {
            min: signex_gerber::Point { x: 50.0, y: 25.0 },
            max: signex_gerber::Point { x: 100.0, y: 75.0 },
        };
        let viewport = Rectangle::new(
            Point::ORIGIN,
            iced::Size::new(900.0, 600.0),
        );

        let (zoom, pan) = zoom_transform_for_selection(base, selection, viewport)
            .expect("valid zoom transform");
        let (scale, world_center, screen_center) =
            fit_transform(base, viewport, zoom, pan);
        let selection_center = Point::new(75.0, 50.0);
        let selected_screen_center = Point::new(
            screen_center.x + (selection_center.x - world_center.x) * scale,
            screen_center.y - (selection_center.y - world_center.y) * scale,
        );

        assert!((selected_screen_center.x - viewport.width * 0.5).abs() < 0.01);
        assert!((selected_screen_center.y - viewport.height * 0.5).abs() < 0.01);
        assert!(selection.width() as f32 * scale <= viewport.width);
        assert!(selection.height() as f32 * scale <= viewport.height);
        assert!(zoom > 1.0);
        assert!(zoom_transform_for_selection(
            base,
            Bounds {
                min: selection.min,
                max: selection.min,
            },
            viewport,
        )
        .is_none());
    }

    #[test]
    fn zoom_area_mode_deactivates_after_successful_selection()
    {
        let layer = signex_gerber::load_gerber_reader(
            "zoom.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nX100000000Y50000000D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![layer],
            failures: Vec::new(),
        });
        state.toggle_zoom_selection();
        assert!(state.zoom_selection_active);

        state.zoom_to_selection(
            Bounds {
                min: signex_gerber::Point { x: 20.0, y: 10.0 },
                max: signex_gerber::Point { x: 60.0, y: 30.0 },
            },
            Rectangle::new(Point::ORIGIN, iced::Size::new(800.0, 500.0)),
        );

        assert!(!state.zoom_selection_active);
        assert!(state.zoom > 1.0);
        assert_eq!(state.status, "Zoomed to selected area.");
    }

    #[test]
    fn toggling_layer_manager_preserves_viewer_state()
    {
        let mut state = GerberViewerState::default();
        state.zoom = 2.0;
        state.active_layer = None;

        state.toggle_layer_manager();

        assert!(!state.layer_manager_visible);
        assert_eq!(state.zoom, 2.0);
        assert!(state.layers.is_empty());

        state.toggle_layer_manager();
        assert!(state.layer_manager_visible);
    }

    #[test]
    fn layer_information_handles_missing_and_active_layers()
    {
        let mut state = GerberViewerState::default();

        assert_eq!(state.active_layer_metadata(), None);
        state.toggle_layer_information();
        assert!(state.layer_information_visible);
        assert!(state.layer_manager_visible);

        let layer = signex_gerber::load_gerber_reader(
            "copper.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![layer],
            failures: Vec::new(),
        });

        let metadata = state
            .active_layer_metadata()
            .expect("loaded active layer metadata");
        assert_eq!(metadata.file_name, "copper.gbr");
        assert_eq!(metadata.format, "Gerber RS-274X");
        assert_eq!(metadata.definition_label, "Apertures");
    }

    #[test]
    fn d_code_list_groups_all_loaded_layers_and_drill_tools()
    {
        let gerber = signex_gerber::load_gerber_reader(
            "copper.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let drill = signex_gerber::load_excellon_reader(
            "holes.drl",
            Cursor::new(
                b"M48\nMETRIC\nT01C0.8\n%\nG05\nT01\nX1.0Y1.0\nM30\n",
            ),
        )
        .expect("test Excellon must parse");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![gerber, drill],
            failures: Vec::new(),
        });

        state.toggle_d_code_list();
        let groups = state.definition_groups();

        assert!(state.d_code_list_visible);
        assert!(!state.layer_information_visible);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].definition_label, "D-codes");
        assert_eq!(groups[0].definitions[0].code, "D10");
        assert_eq!(groups[0].definitions[0].usage_count, 1);
        assert_eq!(groups[1].definition_label, "Drill tools");
        assert_eq!(groups[1].definitions[0].code, "T1");
        assert_eq!(groups[1].definitions[0].usage_count, 1);

        state.toggle_layer_information();
        assert!(state.layer_information_visible);
        assert!(!state.d_code_list_visible);
    }

    #[test]
    fn source_view_preserves_original_text_and_handles_drill_layers()
    {
        let source = "%FSLAX46Y46*%\r\n%MOMM*%\r\nG04 Keep me *\r\nM02*\r\n";
        let gerber = signex_gerber::load_gerber_reader(
            "source.gbr",
            Cursor::new(source.as_bytes()),
        )
        .expect("test Gerber must parse");
        let drill = signex_gerber::load_excellon_reader(
            "holes.drl",
            Cursor::new(b"M48\nMETRIC\nT01C0.8\n%\nM30\n"),
        )
        .expect("test Excellon must parse");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![gerber, drill],
            failures: Vec::new(),
        });
        state.select_layer(0);

        state.toggle_source_view();
        assert!(state.source_view_visible);
        assert_eq!(state.active_gerber_source(), Ok(("source.gbr", source)));

        state.select_layer(1);
        assert_eq!(
            state.active_gerber_source(),
            Err("Source view is available only for Gerber layers."),
        );

        state.clear_all_layers();
        assert_eq!(state.active_gerber_source(), Err("No active layer."));
    }

    #[test]
    fn reload_replaces_only_successful_layer_data_and_preserves_view_state()
    {
        let first = signex_gerber::load_gerber_reader(
            "first.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("first Gerber");
        let second = signex_gerber::load_gerber_reader(
            "second.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("second Gerber");
        let replacement = signex_gerber::load_gerber_reader(
            "first.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\nX0Y0D03*\nX1000000Y0D03*\nM02*\n",
            ),
        )
        .expect("replacement Gerber");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![first, second.clone()],
            failures: Vec::new(),
        });
        state.select_layer(0);
        state.set_layer_visible(0, false);
        let color = state.layers[0].color;
        let generation = state.redraw_generation;

        state.apply_reload_batch(signex_gerber::GerberReloadBatch {
            layers: vec![(0, replacement)],
            failures: vec![signex_gerber::GerberLoadFailure {
                path: PathBuf::from("second.gbr"),
                message: "could not open file".to_owned(),
            }],
        });

        assert_eq!(state.layers[0].layer.geometry.primitives.len(), 2);
        assert_eq!(state.layers[1].layer, second);
        assert!(!state.layers[0].visible);
        assert_eq!(state.layers[0].color, color);
        assert_eq!(state.active_layer, Some(0));
        assert_eq!(state.redraw_generation, generation + 1);
        assert!(state.status.contains("Reloaded 1 layer(s)."));
        assert!(state.status.contains("1 layer(s) could not be reloaded"));
    }

    #[test]
    fn hidden_layer_remains_loaded_but_is_excluded_from_visible_bounds()
    {
        let layer = signex_gerber::load_gerber_reader(
            "copper.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![layer],
            failures: Vec::new(),
        });

        state.set_layer_visible(0, false);
        assert_eq!(state.layers.len(), 1);
        assert!(!state.layers[0].visible);
        assert!(visible_bounds(&state.layers).is_none());

        state.set_layer_visible(0, true);
        assert!(visible_bounds(&state.layers).is_some());
    }

    #[test]
    fn selecting_active_layer_does_not_change_visibility()
    {
        let layer = signex_gerber::load_gerber_reader(
            "copper.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![layer.clone(), layer],
            failures: Vec::new(),
        });
        state.set_layer_visible(0, false);

        state.select_layer(0);
        assert_eq!(state.active_layer, Some(0));
        assert!(!state.layers[0].visible);
        assert!(state.layers[1].visible);

        state.select_layer(10);
        assert_eq!(state.active_layer, Some(0));
    }

    #[test]
    fn layer_navigation_uses_loaded_order_without_wrapping()
    {
        let layer = signex_gerber::load_gerber_reader(
            "layer.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut first = layer.clone();
        first.name = "first.gbr".into();
        let mut second = layer.clone();
        second.name = "second.gbr".into();
        let mut third = layer;
        third.name = "third.gbr".into();
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![first, second, third],
            failures: Vec::new(),
        });
        state.set_layer_visible(1, false);
        state.select_layer(0);

        state.select_next_layer();
        assert_eq!(state.active_layer, Some(1));
        assert_eq!(state.status, "Active layer: second.gbr");

        state.select_next_layer();
        assert_eq!(state.active_layer, Some(2));
        state.select_next_layer();
        assert_eq!(state.active_layer, Some(2));

        state.select_previous_layer();
        assert_eq!(state.active_layer, Some(1));
        state.select_previous_layer();
        assert_eq!(state.active_layer, Some(0));
        state.select_previous_layer();
        assert_eq!(state.active_layer, Some(0));
    }

    #[test]
    fn layer_navigation_from_no_selection_uses_nearest_boundary()
    {
        assert_eq!(next_layer_index(None, 3), Some(0));
        assert_eq!(previous_layer_index(None, 3), Some(2));
        assert_eq!(next_layer_index(None, 0), None);
        assert_eq!(previous_layer_index(None, 0), None);
    }

    #[test]
    fn built_in_profiles_bind_page_keys_in_gerber_context()
    {
        let mut profiles = crate::keymap::ShortcutProfileSet::built_ins()
            .expect("built-in shortcut profiles must parse");
        let page_up = keyboard::Key::Named(keyboard::key::Named::PageUp);
        let page_down = keyboard::Key::Named(keyboard::key::Named::PageDown);

        for profile in ["altium", "classic"]
        {
            profiles
                .set_active_profile(profile)
                .expect("known built-in profile");
            let keymap = profiles.compile_active();

            assert!(matches!(
                gerber_shortcut_message(
                    &keymap,
                    &page_up,
                    keyboard::Modifiers::default(),
                ),
                Some(GerberViewerMessage::PreviousLayer),
            ));
            assert!(matches!(
                gerber_shortcut_message(
                    &keymap,
                    &page_down,
                    keyboard::Modifiers::default(),
                ),
                Some(GerberViewerMessage::NextLayer),
            ));
        }
    }

    #[test]
    fn clearing_current_layer_preserves_siblings_and_selects_next_layer()
    {
        let layer = signex_gerber::load_gerber_reader(
            "layer.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut first = layer.clone();
        first.name = "first.gbr".into();
        let mut second = layer.clone();
        second.name = "second.gbr".into();
        let mut third = layer;
        third.name = "third.gbr".into();
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![first, second, third],
            failures: Vec::new(),
        });
        state.select_layer(1);

        state.clear_current_layer();

        assert_eq!(state.layers.len(), 2);
        assert_eq!(state.layers[0].layer.name, "first.gbr");
        assert_eq!(state.layers[1].layer.name, "third.gbr");
        assert_eq!(state.active_layer, Some(1));
    }

    #[test]
    fn clearing_all_layers_restores_empty_view()
    {
        let layer = signex_gerber::load_gerber_reader(
            "copper.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![layer],
            failures: Vec::new(),
        });
        state.zoom = 3.0;
        state.pan = iced::Vector::new(50.0, 20.0);

        state.clear_all_layers();

        assert!(state.layers.is_empty());
        assert_eq!(state.active_layer, None);
        assert_eq!(state.zoom, 1.0);
        assert_eq!(state.pan, iced::Vector::default());
    }
        }
    };
}

macro_rules! gerber_grid_tests
{
    () => {
        #[cfg(test)]
        mod tests
        {
    use super::*;

    #[test]
    fn bundled_catalog_has_the_required_order_and_labels()
    {
        let catalog = default_grid_catalog();
        let labels = catalog
            .iter()
            .map(|grid| grid.display_label("."))
            .collect::<Vec<_>>();

        assert_eq!(catalog.len(), 22);
        assert_eq!(
            labels,
            [
                "100.00 mils (2.5400 mm)",
                "50.00 mils (1.2700 mm)",
                "25.00 mils (0.6350 mm)",
                "20.00 mils (0.5080 mm)",
                "10.00 mils (0.2540 mm)",
                "5.00 mils (0.1270 mm)",
                "2.50 mils (0.0635 mm)",
                "2.00 mils (0.0508 mm)",
                "1.00 mils (0.0254 mm)",
                "0.50 mils (0.0127 mm)",
                "0.20 mils (0.0051 mm)",
                "0.10 mils (0.0025 mm)",
                "5.0000 mm (196.85 mils)",
                "1.5000 mm ⨯ 2.5000 mm (59.06 mils ⨯ 98.43 mils)",
                "1.0000 mm (39.37 mils)",
                "0.5000 mm (19.69 mils)",
                "0.2500 mm (9.84 mils)",
                "0.2000 mm (7.87 mils)",
                "0.1000 mm (3.94 mils)",
                "0.0500 mm ⨯ 0.0000 mm (1.97 mils ⨯ 0.00 mils)",
                "0.0250 mm ⨯ 0.0000 mm (0.98 mils ⨯ 0.00 mils)",
                "0.0100 mm ⨯ 0.0000 mm (0.39 mils ⨯ 0.00 mils)",
            ]
        );
    }

    #[test]
    fn metric_presets_keep_exact_metric_spacing()
    {
        let catalog = default_grid_catalog();
        let rectangular = &catalog[13];
        let x_only = &catalog[19];

        assert_eq!(rectangular.x_millimetres(), 1.5);
        assert_eq!(rectangular.y_millimetres(), 2.5);
        assert_eq!(x_only.x_millimetres(), 0.05);
        assert_eq!(x_only.y_millimetres(), 0.0);
    }

    #[test]
    fn labels_use_the_supplied_decimal_separator()
    {
        let catalog = default_grid_catalog();

        assert_eq!(
            catalog[0].display_label("."),
            "100.00 mils (2.5400 mm)"
        );
        assert_eq!(
            catalog[21].display_label("."),
            "0.0100 mm ⨯ 0.0000 mm (0.39 mils ⨯ 0.00 mils)"
        );
        assert_eq!(
            catalog[21].display_label(","),
            "0,0100 mm ⨯ 0,0000 mm (0,39 mils ⨯ 0,00 mils)"
        );
    }

    #[test]
    fn operating_system_decimal_separator_is_available()
    {
        assert_eq!(DEFAULT_DECIMAL_SEPARATOR, ".");
        assert!(!system_decimal_separator().is_empty());
    }

    #[test]
    fn creates_named_and_unnamed_grid_definitions()
    {
        let named = create_grid_definition(
            " Fine metric ",
            "0.05",
            "0",
            GridUnit::Mm,
            ".",
        )
        .expect("valid named metric grid");
        let unnamed = create_grid_definition(
            "  ",
            "2.5",
            "2.5",
            GridUnit::Mil,
            ".",
        )
        .expect("valid unnamed mil grid");

        assert_eq!(named.name.as_deref(), Some("Fine metric"));
        assert_eq!(named.x, 0.05);
        assert_eq!(named.y, 0.0);
        assert_eq!(named.unit, GridUnit::Mm);
        assert_eq!(unnamed.name, None);
    }

    #[test]
    fn rejects_invalid_grid_distances()
    {
        for x in ["", "0", "-1", "NaN", "inf"]
        {
            assert!(
                create_grid_definition("", x, "1", GridUnit::Mm, ".")
                    .is_err()
            );
        }
        for y in ["", "-1", "NaN", "inf"]
        {
            assert!(
                create_grid_definition("", "1", y, GridUnit::Mm, ".")
                    .is_err()
            );
        }
    }

    #[test]
    fn persists_and_loads_the_ordered_grid_catalog()
    {
        let directory = tempfile::tempdir().expect("temporary settings directory");
        let path = directory.path().join("gerber_viewer.toml");
        let mut catalog = default_grid_catalog();
        catalog.push(
            create_grid_definition(
                "Assembly",
                "0.25",
                "0.5",
                GridUnit::Mm,
                ".",
            )
            .expect("valid custom grid"),
        );

        persist_grid_catalog_to(&path, &catalog)
            .expect("grid catalog must persist");
        let loaded = load_grid_catalog_from(&path)
            .expect("persisted grid catalog must load");

        assert_eq!(loaded, catalog);
        assert_eq!(loaded.last().and_then(|grid| grid.name.as_deref()), Some("Assembly"));
    }

    #[test]
    fn persists_page_size_in_the_shared_gerber_settings_file()
    {
        let directory = tempfile::tempdir().expect("temporary settings directory");
        let path = directory.path().join("gerber_viewer.toml");
        let catalog = default_grid_catalog();

        persist_settings_to(&path, &catalog, GerberPageSize::A3)
            .expect("Gerber settings must persist");
        let loaded = load_settings_from(&path)
            .expect("persisted Gerber settings must load");

        assert_eq!(loaded.page_size.size, GerberPageSize::A3);
        assert_eq!(loaded.grid_sizes, catalog);
    }
        }
    };
}

macro_rules! gerber_print_tests
{
    () => {
        #[cfg(test)]
        mod tests
        {
    use std::io::Cursor;

    use super::*;
    use crate::gerber_viewer::GerberViewerState;
    use iced::Color;
    use signex_gerber::GerberLoadBatch;

    fn two_layer_state() -> GerberViewerState
    {
        let first = signex_gerber::load_gerber_reader(
            "visible.gbr",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("visible Gerber");
        let second = signex_gerber::load_excellon_reader(
            "hidden.drl",
            Cursor::new(b"M48\nMETRIC\nT01C0.8\n%\nG05\nT01\nX1.0Y1.0\nM30\n"),
        )
        .expect("hidden drill");
        let mut state = GerberViewerState::default();
        state.apply_load_batch(GerberLoadBatch {
            layers: vec![first, second],
            failures: Vec::new(),
        });
        state.layers[0].color = Color::from_rgb8(244, 67, 54);
        state.layers[1].visible = false;
        state
    }

    #[test]
    fn print_plan_contains_only_visible_layers_and_preserves_colors()
    {
        let state = two_layer_state();

        let plan = build_plan(&state).expect("print plan");

        assert_eq!(plan.layers.len(), 1);
        assert_eq!(plan.layers[0].name, "visible.gbr");
        assert_eq!(plan.layers[0].color, [244.0 / 255.0, 67.0 / 255.0, 54.0 / 255.0]);
        assert_eq!(plan.page_size, GerberPageSize::FullSize);
        assert!(plan.content_scale > 0.0);
    }

    #[test]
    fn fixed_page_plan_respects_page_size_and_printable_margin()
    {
        let mut state = two_layer_state();
        state.page_size = GerberPageSize::A4;

        let plan = build_plan(&state).expect("print plan");

        assert_eq!((plan.page_width_mm, plan.page_height_mm), (297.0, 210.0));
        assert_eq!(plan.printable_bounds_mm.min, Point { x: 10.0, y: 10.0 });
        assert_eq!(plan.printable_bounds_mm.max, Point { x: 287.0, y: 200.0 });
        assert!(plan.content_scale < 1.0);
    }

    #[test]
    fn pdf_is_vector_page_with_clipping_and_no_hidden_layer_color()
    {
        let state = two_layer_state();

        let bytes = build_pdf(&state).expect("Gerber PDF");
        let text = String::from_utf8_lossy(&bytes);

        assert!(bytes.starts_with(b"%PDF-"));
        assert!(text.contains("/MediaBox"));
        assert!(text.contains("W n"));
        assert!(text.contains("0.95686275 0.2627451 0.21176471 rg"));
        assert!(!text.contains("hidden.drl"));
    }

    #[test]
    fn empty_viewer_cannot_build_a_print_plan()
    {
        let state = GerberViewerState::default();

        assert!(build_plan(&state).is_err());
    }
        }
    };
}

pub(crate) use gerber_grid_tests;
pub(crate) use gerber_print_tests;
pub(crate) use gerber_viewer_tests;
