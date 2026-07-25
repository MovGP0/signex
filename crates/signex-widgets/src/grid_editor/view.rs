use super::{GerberGridEditorMessage, GerberGridEditorState};
use crate::gerber_viewer::{GridUnit, styles};
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, text,
    text_input,
};
use iced::{
    Background, Border, Color, Element, Length, Theme,
};
use signex_types::theme::ThemeTokens;

pub fn view<'a>(
    state: &'a GerberGridEditorState,
    tokens: &'a ThemeTokens,
) -> Element<'a, GerberGridEditorMessage>
{
    let text_primary = styles::ti(tokens.text);
    let text_muted = styles::ti(tokens.text_secondary);
    let panel_bg = styles::ti(tokens.panel_bg);
    let border = styles::ti(tokens.border);
    let choices = state.catalog_choices();

    let mut grid_rows = column![].spacing(3);
    for choice in &choices
    {
        let index = choice.index;
        grid_rows = grid_rows.push(
            button(
                text(choice.to_string())
                    .size(12)
                    .color(text_primary)
                    .wrapping(iced::widget::text::Wrapping::None),
            )
            .padding([7, 10])
            .width(Length::Fill)
            .on_press(GerberGridEditorMessage::SelectGrid(index))
            .style(styles::rail_tab(
                tokens,
                index == state.selected_index(),
            )),
        );
    }

    let selected = state.selected_index();
    let list_controls = row![
        compact_button("+", Some(GerberGridEditorMessage::AddGrid)),
        compact_button(
            "↑",
            (selected > 0).then_some(GerberGridEditorMessage::MoveGridUp),
        ),
        compact_button(
            "↓",
            (selected + 1 < choices.len())
                .then_some(GerberGridEditorMessage::MoveGridDown),
        ),
        Space::new().width(20),
        compact_button(
            "⌫",
            (choices.len() > 1).then_some(
                GerberGridEditorMessage::DeleteGrid,
            ),
        ),
    ]
    .spacing(8);

    let grids_panel = column![
        text("Grids").size(13).color(text_primary),
        container(scrollable(grid_rows).height(Length::Fill))
            .padding(6)
            .width(400)
            .height(Length::Fill)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(panel_bg)),
                border: Border {
                    width: 1.0,
                    radius: 4.0.into(),
                    color: border,
                },
                ..container::Style::default()
            }),
        list_controls,
    ]
    .spacing(8)
    .width(400)
    .height(Length::Fill);

    let x_unit_picker = pick_list(
        GridUnit::ALL,
        Some(state.settings_unit()),
        GerberGridEditorMessage::SetSettingsUnit,
    )
    .width(90);
    let y_unit_picker = pick_list(
        GridUnit::ALL,
        Some(state.settings_unit()),
        GerberGridEditorMessage::SetSettingsUnit,
    )
    .width(90);
    let mut settings_form = column![
        text("Grid Settings").size(13).color(text_primary),
        container(Space::new())
            .height(1)
            .width(Length::Fill)
            .style(styles::chrome_separator(tokens)),
        row![
            text("Name:").width(55).color(text_primary),
            text_input("Optional name", state.settings_name())
                .on_input(
                    GerberGridEditorMessage::SettingsNameChanged,
                )
                .width(Length::Fill),
            text("(optional)").size(10).color(text_muted),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
        row![
            text("X:").width(55).color(text_primary),
            text_input("X distance", state.x_value())
                .on_input(GerberGridEditorMessage::SettingsXChanged)
                .width(Length::Fill),
            x_unit_picker,
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
        row![
            text("Y:").width(55).color(text_primary),
            text_input("Y distance", state.y_value())
                .on_input(GerberGridEditorMessage::SettingsYChanged)
                .width(Length::Fill),
            y_unit_picker,
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(14);
    if let Some(error) = state.error()
    {
        settings_form = settings_form.push(
            text(error)
                .size(11)
                .color(Color::from_rgb8(239, 83, 80)),
        );
    }
    let settings_panel = container(settings_form)
        .padding(18)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(panel_bg)),
            border: Border {
                width: 1.0,
                radius: 4.0.into(),
                color: border,
            },
            ..container::Style::default()
        });

    let footer = row![
        button(text("Reset Grids to Defaults"))
            .on_press(GerberGridEditorMessage::ResetDefaults),
        Space::new().width(Length::Fill),
        button(text("OK"))
            .on_press(GerberGridEditorMessage::Apply),
        button(text("Cancel"))
            .on_press(GerberGridEditorMessage::Cancel),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let content = column![
        row![grids_panel, settings_panel]
            .spacing(20)
            .height(Length::Fill),
        footer,
    ]
    .spacing(14)
    .padding(16)
    .width(Length::Fill)
    .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(
                styles::ti(tokens.bg),
            )),
            ..container::Style::default()
        })
        .into()
}

fn compact_button(
    label: &'static str,
    message: Option<GerberGridEditorMessage>,
) -> iced::widget::Button<'static, GerberGridEditorMessage>
{
    button(text(label).size(14))
        .padding([5, 9])
        .on_press_maybe(message)
}
