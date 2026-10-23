//! The PCB trace calculator dialog body: editable text fields, unit
//! pickers and the two result panels. The maths lives in
//! [`super::domain`]; this file only parses text, calls it and draws.

use std::fmt;

use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Alignment, Background, Border, Element, Length, Theme};
use signex_types::theme::ThemeTokens;

use super::domain::{
    AmbientUnit, CalculationError, InputField, LayerResult, LengthUnit, RiseUnit, ThicknessUnit,
    TraceCalculation, TraceInput, WidthUnit, calculate,
};
use crate::theme_ext;

const BODY_SIZE: f32 = 11.0;
const CAPTION_SIZE: f32 = 10.0;
const CONTENT_PADDING: u16 = 12;
const SECTION_SPACING: f32 = 14.0;
const ROW_SPACING: f32 = 8.0;
const LABEL_WIDTH: f32 = 120.0;
const UNIT_PICKER_WIDTH: f32 = 90.0;
const OUTPUT_VALUE_WIDTH: f32 = 84.0;
const OUTPUT_UNIT_WIDTH: f32 = 40.0;
const PANEL_PADDING: u16 = 12;
const PANEL_RADIUS: f32 = 6.0;
const STATIC_BOX_RADIUS: f32 = 2.0;
const BUTTON_RADIUS: f32 = 4.0;
const CONTROL_PADDING: [u16; 2] = [4, 6];

/// The curves already carry about 10 % current derating.
const DERATING_NOTE: &str = "The curves include about 10 % derating; consider a further 15 % for boards up to 0.8 mm thick or copper of 108 \u{b5}m and above.";

const DEFAULT_CURRENT: &str = "5";
const DEFAULT_THICKNESS: &str = "1";
const DEFAULT_RISE: &str = "20";
const DEFAULT_AMBIENT: &str = "20";
const DEFAULT_LENGTH: &str = "50";

/// Why the result panels cannot be drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceCalculatorError {
    /// A field's text is not a number.
    InvalidNumber(InputField),
    /// The numbers parsed but the domain rejected them.
    Calculation(CalculationError),
}

impl fmt::Display for TraceCalculatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber(field) => write!(f, "Enter a numeric value for {field}."),
            Self::Calculation(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for TraceCalculatorError {}

impl From<CalculationError> for TraceCalculatorError {
    fn from(error: CalculationError) -> Self {
        Self::Calculation(error)
    }
}

#[derive(Debug, Clone)]
pub enum TraceCalculatorMessage {
    CurrentChanged(String),
    ThicknessChanged(String),
    ThicknessUnitChanged(ThicknessUnit),
    RiseChanged(String),
    RiseUnitChanged(RiseUnit),
    AmbientChanged(String),
    AmbientUnitChanged(AmbientUnit),
    LengthChanged(String),
    LengthUnitChanged(LengthUnit),
    InnerWidthUnitChanged(WidthUnit),
    OuterWidthUnitChanged(WidthUnit),
    ResetPressed,
}

#[derive(Debug, Clone)]
pub struct TraceCalculatorControl {
    pub current: String,
    pub thickness: String,
    pub thickness_unit: ThicknessUnit,
    pub rise: String,
    pub rise_unit: RiseUnit,
    pub ambient: String,
    pub ambient_unit: AmbientUnit,
    pub length: String,
    pub length_unit: LengthUnit,
    pub inner_width_unit: WidthUnit,
    pub outer_width_unit: WidthUnit,
    /// The failure last handed to the caller by [`Self::update`], so a
    /// persisting error is reported once and not on every keystroke.
    reported: Option<TraceCalculatorError>,
}

impl Default for TraceCalculatorControl {
    fn default() -> Self {
        Self {
            current: DEFAULT_CURRENT.to_string(),
            thickness: DEFAULT_THICKNESS.to_string(),
            thickness_unit: ThicknessUnit::OuncePerSquareFoot,
            rise: DEFAULT_RISE.to_string(),
            rise_unit: RiseUnit::Celsius,
            ambient: DEFAULT_AMBIENT.to_string(),
            ambient_unit: AmbientUnit::Celsius,
            length: DEFAULT_LENGTH.to_string(),
            length_unit: LengthUnit::Millimeter,
            inner_width_unit: WidthUnit::Millimeter,
            outer_width_unit: WidthUnit::Millimeter,
            reported: None,
        }
    }
}

impl TraceCalculatorControl {
    /// Apply an edit. Returns the failure this edit newly produced, if
    /// any: the dialog already shows it, and the caller is expected to
    /// route it to the application log as well. `None` when the state is
    /// valid or the same failure was already returned for an earlier edit.
    pub fn update(&mut self, message: TraceCalculatorMessage) -> Option<TraceCalculatorError> {
        match message {
            TraceCalculatorMessage::CurrentChanged(value) => self.current = value,
            TraceCalculatorMessage::ThicknessChanged(value) => self.thickness = value,
            TraceCalculatorMessage::ThicknessUnitChanged(unit) => self.thickness_unit = unit,
            TraceCalculatorMessage::RiseChanged(value) => self.rise = value,
            TraceCalculatorMessage::RiseUnitChanged(unit) => self.rise_unit = unit,
            TraceCalculatorMessage::AmbientChanged(value) => self.ambient = value,
            TraceCalculatorMessage::AmbientUnitChanged(unit) => self.ambient_unit = unit,
            TraceCalculatorMessage::LengthChanged(value) => self.length = value,
            TraceCalculatorMessage::LengthUnitChanged(unit) => self.length_unit = unit,
            TraceCalculatorMessage::InnerWidthUnitChanged(unit) => self.inner_width_unit = unit,
            TraceCalculatorMessage::OuterWidthUnitChanged(unit) => self.outer_width_unit = unit,
            TraceCalculatorMessage::ResetPressed => *self = Self::default(),
        }
        let failure = self.calculation().err();
        if failure == self.reported {
            return None;
        }
        self.reported.clone_from(&failure);
        failure
    }

    /// Parse the fields, convert to canonical units and run the domain.
    ///
    /// # Errors
    ///
    /// A field that is not a number, or whatever the domain rejects.
    pub fn calculation(&self) -> Result<TraceCalculation, TraceCalculatorError> {
        let current = parse_field(InputField::Current, &self.current)?;
        let thickness = parse_field(InputField::CopperThickness, &self.thickness)?;
        let rise = parse_field(InputField::TemperatureRise, &self.rise)?;
        let ambient = parse_field(InputField::AmbientTemperature, &self.ambient)?;
        let length = parse_field(InputField::TraceLength, &self.length)?;
        Ok(calculate(TraceInput {
            current_a: current,
            copper_thickness_m: self.thickness_unit.to_metres(thickness),
            temperature_rise_c: self.rise_unit.to_celsius_degrees(rise),
            ambient_c: self.ambient_unit.to_celsius(ambient),
            length_m: self.length_unit.to_metres(length),
        })?)
    }

    pub fn view<'a>(&'a self, tokens: &'a ThemeTokens) -> Element<'a, TraceCalculatorMessage> {
        let secondary = theme_ext::text_secondary(tokens);
        let header = row![
            column![
                text("IPC-2221 conductor sizing")
                    .size(BODY_SIZE)
                    .color(secondary),
                text("IPC-2221 curve fit; IPC-2152 supersedes it for current capacity.")
                    .size(CAPTION_SIZE)
                    .color(secondary),
                text(DERATING_NOTE).size(CAPTION_SIZE).color(secondary),
            ]
            .spacing(2)
            .width(Length::Fill),
            button(text("Reset").size(BODY_SIZE))
                .padding(CONTROL_PADDING)
                .on_press(TraceCalculatorMessage::ResetPressed)
                .style(move |_theme: &Theme, status| button_style(tokens, status)),
        ]
        .align_y(Alignment::Start);

        let results = match self.calculation() {
            Ok(calculation) => self.results_view(&calculation, tokens),
            Err(error) => message_panel(error.to_string(), theme_ext::error_color(tokens), tokens),
        };

        let content = column![header, self.inputs_view(tokens), results]
            .spacing(SECTION_SPACING)
            .padding(CONTENT_PADDING);
        scrollable(content).height(Length::Shrink).into()
    }

    fn inputs_view<'a>(&'a self, tokens: &'a ThemeTokens) -> Element<'a, TraceCalculatorMessage> {
        use TraceCalculatorMessage as M;
        column![
            input_row(
                "Current",
                &self.current,
                static_unit_box("A", tokens),
                M::CurrentChanged,
                tokens,
            ),
            input_row(
                "Copper thickness",
                &self.thickness,
                unit_picker(
                    ThicknessUnit::ALL,
                    self.thickness_unit,
                    M::ThicknessUnitChanged
                ),
                M::ThicknessChanged,
                tokens,
            ),
            input_row(
                "Temperature rise",
                &self.rise,
                unit_picker(RiseUnit::ALL, self.rise_unit, M::RiseUnitChanged),
                M::RiseChanged,
                tokens,
            ),
            input_row(
                "Ambient temperature",
                &self.ambient,
                unit_picker(AmbientUnit::ALL, self.ambient_unit, M::AmbientUnitChanged),
                M::AmbientChanged,
                tokens,
            ),
            input_row(
                "Trace length",
                &self.length,
                unit_picker(LengthUnit::ALL, self.length_unit, M::LengthUnitChanged),
                M::LengthChanged,
                tokens,
            ),
        ]
        .spacing(ROW_SPACING)
        .into()
    }

    fn results_view<'a>(
        &'a self,
        calculation: &TraceCalculation,
        tokens: &'a ThemeTokens,
    ) -> Element<'a, TraceCalculatorMessage> {
        let panels = row![
            result_panel(
                "Inner layer",
                calculation.inner,
                self.inner_width_unit,
                TraceCalculatorMessage::InnerWidthUnitChanged,
                tokens,
            ),
            result_panel(
                "Outer layer",
                calculation.outer,
                self.outer_width_unit,
                TraceCalculatorMessage::OuterWidthUnitChanged,
                tokens,
            ),
        ]
        .spacing(ROW_SPACING);
        if calculation.warnings.is_empty() {
            return panels.into();
        }
        let warnings = calculation
            .warnings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        column![
            panels,
            message_panel(
                format!("{warnings} The result is an extrapolation."),
                theme_ext::warning_color(tokens),
                tokens,
            ),
        ]
        .spacing(ROW_SPACING)
        .into()
    }
}

fn result_panel<'a>(
    title: &'static str,
    result: LayerResult,
    width_unit: WidthUnit,
    on_width_unit: fn(WidthUnit) -> TraceCalculatorMessage,
    tokens: &'a ThemeTokens,
) -> Element<'a, TraceCalculatorMessage> {
    let body = column![
        row![
            text(title)
                .size(BODY_SIZE)
                .color(theme_ext::text_primary(tokens)),
            Space::new().width(Length::Fill),
            unit_picker(WidthUnit::ALL, width_unit, on_width_unit),
        ]
        .align_y(Alignment::Center),
        output_row(
            "Required width",
            result.width(width_unit),
            width_unit.to_string(),
            tokens,
        ),
        output_row("Area", result.area_mil2, "mil\u{b2}".into(), tokens),
        output_row(
            "Resistance",
            result.resistance_ohm,
            "\u{3a9}".into(),
            tokens
        ),
        output_row("Voltage drop", result.voltage_drop_v, "V".into(), tokens),
        output_row("Power loss", result.power_loss_w, "W".into(), tokens),
    ]
    .spacing(ROW_SPACING);
    container(body)
        .padding(PANEL_PADDING)
        .width(Length::Fill)
        .style(move |_theme| panel_style(tokens))
        .into()
}

fn output_row<'a>(
    label: &'static str,
    value: f64,
    unit: String,
    tokens: &'a ThemeTokens,
) -> Element<'a, TraceCalculatorMessage> {
    let primary = theme_ext::text_primary(tokens);
    row![
        text(label)
            .size(BODY_SIZE)
            .color(theme_ext::text_secondary(tokens))
            .width(Length::Fill),
        text(format_number(value))
            .size(BODY_SIZE)
            .color(primary)
            .width(OUTPUT_VALUE_WIDTH),
        text(unit)
            .size(BODY_SIZE)
            .color(primary)
            .width(OUTPUT_UNIT_WIDTH),
    ]
    .align_y(Alignment::Center)
    .spacing(ROW_SPACING)
    .into()
}

fn input_row<'a>(
    label: &'static str,
    value: &'a str,
    unit: impl Into<Element<'a, TraceCalculatorMessage>>,
    on_input: fn(String) -> TraceCalculatorMessage,
    tokens: &'a ThemeTokens,
) -> Element<'a, TraceCalculatorMessage> {
    row![
        text(label)
            .size(BODY_SIZE)
            .color(theme_ext::text_primary(tokens))
            .width(LABEL_WIDTH),
        text_input("", value)
            .on_input(on_input)
            .size(BODY_SIZE)
            .padding(CONTROL_PADDING)
            .width(Length::Fill),
        unit.into(),
    ]
    .align_y(Alignment::Center)
    .spacing(ROW_SPACING)
    .into()
}

/// A unit that cannot be changed (the current is always in amperes),
/// drawn with the size and border of the pickers so the unit column lines
/// up.
fn static_unit_box<'a>(
    label: &'static str,
    tokens: &'a ThemeTokens,
) -> Element<'a, TraceCalculatorMessage> {
    container(
        text(label)
            .size(BODY_SIZE)
            .color(theme_ext::text_secondary(tokens)),
    )
    .padding(CONTROL_PADDING)
    .width(UNIT_PICKER_WIDTH)
    .style(move |_theme| container::Style {
        border: Border {
            width: 1.0,
            radius: STATIC_BOX_RADIUS.into(),
            color: theme_ext::border_color(tokens),
        },
        ..container::Style::default()
    })
    .into()
}

fn unit_picker<'a, U, const N: usize>(
    units: [U; N],
    selected: U,
    on_change: fn(U) -> TraceCalculatorMessage,
) -> Element<'a, TraceCalculatorMessage>
where
    U: Copy + PartialEq + fmt::Display + 'a,
{
    pick_list(units, Some(selected), on_change)
        .text_size(BODY_SIZE)
        .padding(CONTROL_PADDING)
        .width(UNIT_PICKER_WIDTH)
        .into()
}

/// A bordered note: the failure line or the extrapolation warning.
fn message_panel<'a>(
    message: String,
    color: iced::Color,
    tokens: &'a ThemeTokens,
) -> Element<'a, TraceCalculatorMessage> {
    container(text(message).size(BODY_SIZE).color(color))
        .padding(PANEL_PADDING)
        .width(Length::Fill)
        .style(move |_theme| panel_style(tokens))
        .into()
}

fn panel_style(tokens: &ThemeTokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme_ext::to_color(&tokens.panel_bg))),
        border: Border {
            width: 1.0,
            radius: PANEL_RADIUS.into(),
            color: theme_ext::border_color(tokens),
        },
        ..container::Style::default()
    }
}

fn button_style(tokens: &ThemeTokens, status: button::Status) -> button::Style {
    let background = if status == button::Status::Hovered {
        theme_ext::hover_color(tokens)
    } else {
        theme_ext::to_color(&tokens.panel_bg)
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: theme_ext::text_primary(tokens),
        border: Border {
            width: 1.0,
            radius: BUTTON_RADIUS.into(),
            color: theme_ext::border_color(tokens),
        },
        ..button::Style::default()
    }
}

/// Read a field. The text is trimmed, and one comma is accepted as the
/// decimal separator. Input that could be read two ways is refused rather
/// than guessed: several commas, a comma next to a point, and a comma
/// followed by exactly three digits (`1,000` is a thousand to some and one
/// to others).
fn parse_field(field: InputField, text: &str) -> Result<f64, TraceCalculatorError> {
    let invalid = || TraceCalculatorError::InvalidNumber(field);
    let trimmed = text.trim();
    if is_ambiguous_comma(trimmed) {
        return Err(invalid());
    }
    trimmed
        .replace(',', ".")
        .parse::<f64>()
        .map_err(|_| invalid())
}

fn is_ambiguous_comma(text: &str) -> bool {
    match text.matches(',').count() {
        0 => false,
        1 => {
            let (whole, fraction) = text.split_once(',').unwrap_or((text, ""));
            text.contains('.')
                || (fraction.len() == 3
                    && fraction.bytes().all(|b| b.is_ascii_digit())
                    && !whole.is_empty()
                    && whole.bytes().all(|b| b.is_ascii_digit()))
        }
        _ => true,
    }
}

/// About three significant figures in plain notation, scientific outside
/// 1e-4..1e6 and for a non-finite value.
pub fn format_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if !value.is_finite() {
        return value.to_string();
    }
    let exponent = value.abs().log10().floor() as i32;
    if !(-4..=5).contains(&exponent) {
        return format!("{value:.3e}");
    }
    let decimals = (2 - exponent).clamp(0, 8) as usize;
    format!("{value:.decimals$}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_calculates_cleanly_and_draws() {
        let control = TraceCalculatorControl::default();
        let calculation = control.calculation().expect("defaults are valid");
        assert!(calculation.warnings.is_empty());
        let tokens = signex_types::theme::theme_tokens(signex_types::theme::ThemeId::Signex);
        let _: Element<'_, TraceCalculatorMessage> = control.view(&tokens);
    }

    #[test]
    fn a_non_numeric_field_names_itself() {
        let mut control = TraceCalculatorControl::default();
        let failure = control.update(TraceCalculatorMessage::CurrentChanged("5 amps".into()));
        assert_eq!(
            failure,
            Some(TraceCalculatorError::InvalidNumber(InputField::Current))
        );
        assert_eq!(
            failure.expect("failure").to_string(),
            "Enter a numeric value for current."
        );
    }

    #[test]
    fn an_empty_field_is_not_a_number() {
        let mut control = TraceCalculatorControl::default();
        control.update(TraceCalculatorMessage::LengthChanged(String::new()));
        assert_eq!(
            control.calculation(),
            Err(TraceCalculatorError::InvalidNumber(InputField::TraceLength))
        );
    }

    #[test]
    fn ambiguous_comma_input_is_refused_not_guessed() {
        for text in ["1,000", "1,2,3", "1,000.5", "1.000,5", "12,345"] {
            let mut control = TraceCalculatorControl::default();
            control.update(TraceCalculatorMessage::CurrentChanged(text.into()));
            assert_eq!(
                control.calculation(),
                Err(TraceCalculatorError::InvalidNumber(InputField::Current)),
                "{text:?} must not parse"
            );
        }
        for text in ["1,5", "0,25", "1,50", "1,5000", ",5"] {
            let mut control = TraceCalculatorControl::default();
            control.update(TraceCalculatorMessage::CurrentChanged(text.into()));
            assert!(control.calculation().is_ok(), "{text:?} is a decimal comma");
        }
    }

    #[test]
    fn a_comma_decimal_separator_and_padding_are_accepted() {
        let mut control = TraceCalculatorControl::default();
        control.update(TraceCalculatorMessage::CurrentChanged("  2,5 ".into()));
        assert!(control.calculation().is_ok());
    }

    #[test]
    fn a_valid_edit_reports_no_failure() {
        let mut control = TraceCalculatorControl::default();
        assert_eq!(
            control.update(TraceCalculatorMessage::CurrentChanged("8".into())),
            None
        );
    }

    #[test]
    fn a_domain_rejection_is_surfaced_as_a_failure() {
        let mut control = TraceCalculatorControl::default();
        let failure = control.update(TraceCalculatorMessage::CurrentChanged("-1".into()));
        assert_eq!(
            failure,
            Some(TraceCalculatorError::Calculation(
                CalculationError::NonPositive(InputField::Current)
            ))
        );
    }

    #[test]
    fn a_persisting_failure_is_reported_once_and_again_after_recovery() {
        let mut control = TraceCalculatorControl::default();
        let first = control.update(TraceCalculatorMessage::CurrentChanged("x".into()));
        let repeat = control.update(TraceCalculatorMessage::CurrentChanged("xy".into()));
        let recovered = control.update(TraceCalculatorMessage::CurrentChanged("5".into()));
        let again = control.update(TraceCalculatorMessage::CurrentChanged("y".into()));

        assert!(first.is_some());
        assert!(repeat.is_none(), "same failure must not be re-reported");
        assert!(recovered.is_none());
        assert!(again.is_some(), "a new failure after recovery is reported");
    }

    #[test]
    fn switching_the_thickness_unit_changes_the_width_not_the_area() {
        let mut control = TraceCalculatorControl::default();
        control.update(TraceCalculatorMessage::ThicknessChanged("35".into()));
        control.update(TraceCalculatorMessage::ThicknessUnitChanged(
            ThicknessUnit::Micrometer,
        ));
        let micrometers = control.calculation().expect("valid");

        control.update(TraceCalculatorMessage::ThicknessChanged("1.378".into()));
        control.update(TraceCalculatorMessage::ThicknessUnitChanged(
            ThicknessUnit::Mil,
        ));
        let mils = control.calculation().expect("valid");

        assert_eq!(micrometers.inner.area_mil2, mils.inner.area_mil2);
        let ratio = micrometers.inner.width_m / mils.inner.width_m;
        assert!(
            (ratio - 1.0).abs() < 1.0e-3,
            "35 um is about 1.378 mil: {ratio}"
        );
    }

    #[test]
    fn a_unit_switch_keeps_the_typed_text() {
        let mut control = TraceCalculatorControl::default();
        control.update(TraceCalculatorMessage::LengthUnitChanged(LengthUnit::Inch));
        assert_eq!(control.length, "50");
        assert_eq!(control.length_unit, LengthUnit::Inch);
    }

    #[test]
    fn reset_restores_the_defaults_and_forgets_the_reported_failure() {
        let mut control = TraceCalculatorControl::default();
        control.update(TraceCalculatorMessage::CurrentChanged("x".into()));
        control.update(TraceCalculatorMessage::ResetPressed);
        assert_eq!(control.current, DEFAULT_CURRENT);
        let failure = control.update(TraceCalculatorMessage::CurrentChanged("x".into()));
        assert!(failure.is_some(), "the same failure is new after a reset");
    }

    #[test]
    fn format_number_handles_zero_and_the_plain_range() {
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(-0.0), "0");
        assert_eq!(format_number(5.0), "5.00");
        assert_eq!(format_number(256.27), "256");
        assert_eq!(format_number(0.0123), "0.0123");
        assert_eq!(format_number(123_456.0), "123456");
        assert_eq!(format_number(-4.5678), "-4.57");
    }

    #[test]
    fn format_number_switches_to_scientific_outside_the_plain_range() {
        assert_eq!(format_number(1.0e-5), "1.000e-5");
        assert_eq!(format_number(1.0e6), "1.000e6");
        assert_eq!(format_number(0.000_012_34), "1.234e-5");
        assert_eq!(format_number(0.000_123_4), "0.000123");
    }

    #[test]
    fn format_number_does_not_panic_on_non_finite_values() {
        assert_eq!(format_number(f64::INFINITY), "inf");
        assert_eq!(format_number(f64::NAN), "NaN");
    }
}
