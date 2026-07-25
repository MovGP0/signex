// Shared private unit-test definitions for Gerber measurements.
#![allow(unused_imports, unused_macros)]

macro_rules! gerber_measurement_tests
{
    () => {
        #[cfg(test)]
        mod tests
        {
            use super::*;

            #[test]
            fn measurement_reports_delta_and_distance()
            {
                let measurement = GerberMeasurement {
                    start: signex_gerber::Point { x: 1.0, y: 2.0 },
                    end: Some(signex_gerber::Point { x: 4.0, y: 6.0 }),
                };

                assert_eq!(
                    measurement.delta(),
                    Some(signex_gerber::Point { x: 3.0, y: 4.0 })
                );
                assert_eq!(measurement.distance(), Some(5.0));
            }

            #[test]
            fn captures_two_points_and_formats_selected_units()
            {
                let mut state = GerberViewerState::default();
                state.decimal_separator = ".".into();
                state.set_display_unit(GerberDisplayUnit::Millimetres);

                state.toggle_measurement();
                state.capture_measurement_point(signex_gerber::Point {
                    x: 1.0,
                    y: 2.0,
                });
                state.capture_measurement_point(signex_gerber::Point {
                    x: 4.0,
                    y: 6.0,
                });

                assert!(!state.measurement_active());
                assert_eq!(
                    state.measurement_summary().as_deref(),
                    Some("Distance: 5.0000 mm  ΔX: 3.0000  ΔY: 4.0000")
                );

                state.set_display_unit(GerberDisplayUnit::Mils);

                assert_eq!(
                    state.measurement_summary().as_deref(),
                    Some(
                        "Distance: 196.85 mils  ΔX: 118.11  ΔY: 157.48"
                    )
                );
            }

            #[test]
            fn measurement_format_uses_supplied_decimal_separator()
            {
                let measurement = GerberMeasurement {
                    start: signex_gerber::Point::default(),
                    end: Some(signex_gerber::Point { x: 25.4, y: 0.0 }),
                };

                assert_eq!(
                    format_measurement(
                        measurement,
                        GerberDisplayUnit::Inches,
                        ",",
                    )
                    .as_deref(),
                    Some("Distance: 1,0000 in  ΔX: 1,0000  ΔY: 0,0000")
                );
            }

            #[test]
            fn cancel_and_reset_clear_the_expected_measurement_state()
            {
                let mut state = GerberViewerState::default();
                state.toggle_measurement();
                state.capture_measurement_point(signex_gerber::Point {
                    x: 1.0,
                    y: 1.0,
                });

                state.toggle_measurement();

                assert!(!state.measurement_active());
                assert_eq!(state.measurement(), None);

                state.toggle_measurement();
                state.capture_measurement_point(signex_gerber::Point {
                    x: 0.0,
                    y: 0.0,
                });
                state.capture_measurement_point(signex_gerber::Point {
                    x: 1.0,
                    y: 0.0,
                });
                state.reset_measurement();

                assert_eq!(state.measurement(), None);
                assert!(!state.measurement_active());
            }

            #[test]
            fn measurement_and_zoom_area_modes_are_mutually_exclusive()
            {
                let mut state = GerberViewerState::default();
                state.toggle_zoom_selection();

                state.toggle_measurement();

                assert!(state.measurement_active());
                assert!(!state.zoom_selection_active);

                state.toggle_zoom_selection();

                assert!(!state.measurement_active());
                assert!(state.zoom_selection_active);
            }
        }
    };
}

pub(crate) use gerber_measurement_tests;
