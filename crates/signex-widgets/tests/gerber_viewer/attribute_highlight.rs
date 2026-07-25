// Shared private unit-test definitions for Gerber X2 attribute highlighting.
#![allow(unused_imports, unused_macros)]

macro_rules! gerber_attribute_highlight_tests
{
    () => {
        #[cfg(test)]
        mod attribute_tests
        {
            use std::io::Cursor;

            use super::*;

            fn attribute_layer() -> LoadedLayer
            {
                signex_gerber::load_gerber_reader(
                    "attributes.gbr",
                    Cursor::new(
                        b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.000*%\nD10*\n%TO.CVal,10k*%\nX0Y0D03*\n%TO.CVal,22k*%\n%TO.MyAttribute,Alpha*%\nX1000000Y0D03*\nM02*\n",
                    ),
                )
                .expect("X2 attribute layer must load")
            }

            fn attribute(name: &str, values: &[&str]) -> GerberAttributeValue
            {
                GerberAttributeValue {
                    name: name.into(),
                    values: values.iter().map(|value| (*value).into()).collect(),
                }
            }

            #[test]
            fn attribute_choices_are_sorted_unique_and_include_values()
            {
                let layer = attribute_layer();
                let mut state = GerberViewerState::default();
                state.apply_load_batch(GerberLoadBatch {
                    layers: vec![layer.clone(), layer],
                    failures: Vec::new(),
                });

                assert_eq!(
                    state.attribute_choices(),
                    [
                        attribute(".CVal", &["10k"]),
                        attribute(".CVal", &["22k"]),
                        attribute(".MyAttribute", &["Alpha"]),
                    ]
                );
                assert_eq!(
                    state.attribute_choices()[2].to_string(),
                    ".MyAttribute: Alpha"
                );
            }

            #[test]
            fn selecting_attribute_is_exclusive_and_clears_cleanly()
            {
                let mut state = GerberViewerState::default();
                state.apply_load_batch(GerberLoadBatch {
                    layers: vec![attribute_layer()],
                    failures: Vec::new(),
                });
                state.highlighted_component = Some("R1".into());
                state.highlighted_net = Some("GND".into());
                let selected = attribute(".CVal", &["22k"]);

                state.set_highlighted_attribute(selected.clone());

                assert_eq!(state.highlighted_attribute(), Some(&selected));
                assert_eq!(state.highlighted_component(), None);
                assert_eq!(state.highlighted_net(), None);

                state.clear_attribute_highlight();

                assert_eq!(state.highlighted_attribute(), None);
            }

            #[test]
            fn unavailable_attribute_selection_is_ignored()
            {
                let mut state = GerberViewerState::default();
                state.apply_load_batch(GerberLoadBatch {
                    layers: vec![attribute_layer()],
                    failures: Vec::new(),
                });

                state.set_highlighted_attribute(attribute(".Missing", &["value"]));

                assert_eq!(state.highlighted_attribute(), None);
            }

            #[test]
            fn attribute_color_highlights_exact_match_and_dims_absent_values()
            {
                let layer_color = Color::from_rgb8(156, 39, 176);
                let selected = attribute(".CVal", &["10k"]);
                let matching = signex_gerber::GerberObjectAttributes {
                    attributes: vec![selected.clone()],
                    ..Default::default()
                };

                assert_eq!(
                    attribute_highlight_color(
                        layer_color,
                        Some(&matching),
                        Some(&selected),
                    ),
                    Color::from_rgb8(255, 193, 7)
                );
                assert_eq!(
                    attribute_highlight_color(
                        layer_color,
                        None,
                        Some(&selected),
                    ),
                    Color { a: 0.18, ..layer_color }
                );
                assert_eq!(
                    attribute_highlight_color(layer_color, None, None),
                    layer_color
                );
            }
        }
    };
}

pub(crate) use gerber_attribute_highlight_tests;
