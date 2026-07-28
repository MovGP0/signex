// Shared private unit-test definitions for Gerber layer colors.
#![allow(unused_imports, unused_macros)]

macro_rules! gerber_layer_color_tests
{
    () => {
        #[cfg(test)]
        mod layer_color_tests
        {
            use std::io::Cursor;

            use super::*;

            fn two_layer_state() -> GerberViewerState
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
                let mut second = layer;
                second.name = "second.gbr".into();
                let mut state = GerberViewerState::default();
                state.apply_load_batch(GerberLoadBatch {
                    layers: vec![first, second],
                    failures: Vec::new(),
                });
                state
            }

            #[test]
            fn material_color_choice_updates_rendering_immediately()
            {
                let mut state = two_layer_state();
                let generation = state.redraw_generation;
                let expected = state.palette[5];

                state.set_layer_color(0, 5);

                assert_eq!(state.layers[0].color, expected);
                assert_eq!(state.redraw_generation, generation + 1);
                assert!(state.status.contains("#B0BEC5"));
                assert_eq!(
                    state
                        .selected_layer_color_choice(0)
                        .expect("selected color")
                        .palette_index,
                    5,
                );
            }

            #[test]
            fn customized_color_stays_with_layer_after_reordering()
            {
                let mut state = two_layer_state();
                let expected = state.palette[5];
                state.set_layer_color(0, 5);
                state.select_layer(0);

                state.move_active_layer_up();

                assert_eq!(state.layers[1].layer.name, "first.gbr");
                assert_eq!(state.layers[1].color, expected);
            }

            #[test]
            fn material_color_choices_do_not_repeat_palette_colors()
            {
                let state = two_layer_state();
                let choices = state.layer_color_choices();

                assert!(choices.len() > 1);
                assert!(choices.len() < state.palette.len());
                for (index, choice) in choices.iter().enumerate()
                {
                    assert_eq!(
                        choice.color,
                        state.palette[choice.palette_index],
                    );
                    assert!(!choices[..index].iter().any(|previous|
                    {
                        state.palette[previous.palette_index]
                            == state.palette[choice.palette_index]
                    }));
                }
            }

            #[test]
            fn invalid_or_unchanged_color_choice_is_a_no_op()
            {
                let mut state = two_layer_state();
                let generation = state.redraw_generation;
                let original = state.layers[0].color;

                state.set_layer_color(0, 0);
                state.set_layer_color(0, usize::MAX);
                state.set_layer_color(usize::MAX, 1);

                assert_eq!(state.layers[0].color, original);
                assert_eq!(state.redraw_generation, generation);
            }

            #[test]
            fn color_picker_tracks_one_target_and_closes_after_selection()
            {
                let mut state = two_layer_state();

                state.toggle_color_picker(GerberColorTarget::Layer(0));
                assert!(state.color_picker_open(GerberColorTarget::Layer(0)));

                state.toggle_color_picker(GerberColorTarget::Grid);
                assert!(!state.color_picker_open(GerberColorTarget::Layer(0)));
                assert!(state.color_picker_open(GerberColorTarget::Grid));

                state.set_grid_color(1);
                assert!(!state.color_picker_open(GerberColorTarget::Grid));
            }
        }
    };
}

pub(crate) use gerber_layer_color_tests;
