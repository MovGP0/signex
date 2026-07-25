use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GerberMeasurement
{
    pub start: signex_gerber::Point,
    pub end: Option<signex_gerber::Point>,
}

impl GerberMeasurement
{
    pub fn delta(self) -> Option<signex_gerber::Point>
    {
        self.end.map(|end| signex_gerber::Point {
            x: end.x - self.start.x,
            y: end.y - self.start.y,
        })
    }

    pub fn distance(self) -> Option<f64>
    {
        self.delta().map(|delta| delta.x.hypot(delta.y))
    }
}

impl GerberViewerState
{
    pub fn measurement_active(&self) -> bool
    {
        self.measurement_active
    }

    pub fn measurement(&self) -> Option<GerberMeasurement>
    {
        self.measurement
    }

    pub fn toggle_measurement(&mut self)
    {
        if self.measurement_active
        {
            self.measurement_active = false;
            if self.measurement.is_some_and(|measurement| measurement.end.is_none())
            {
                self.measurement = None;
            }
            self.status = "Measurement cancelled.".into();
        }
        else
        {
            self.measurement_active = true;
            self.measurement = None;
            self.zoom_selection_active = false;
            self.status = "Select the first measurement point.".into();
        }
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
    }

    pub fn capture_measurement_point(&mut self, point: signex_gerber::Point)
    {
        if !self.measurement_active || !point.x.is_finite() || !point.y.is_finite()
        {
            return;
        }
        match self.measurement
        {
            None | Some(GerberMeasurement { end: Some(_), .. }) =>
            {
                self.measurement = Some(GerberMeasurement {
                    start: point,
                    end: None,
                });
                self.status = "Select the second measurement point.".into();
            }
            Some(mut measurement) =>
            {
                measurement.end = Some(point);
                self.measurement = Some(measurement);
                self.measurement_active = false;
                self.status = self
                    .measurement_summary()
                    .unwrap_or_else(|| "Measurement complete.".into());
            }
        }
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
    }

    pub fn reset_measurement(&mut self)
    {
        self.measurement_active = false;
        self.measurement = None;
        self.status = "Measurement reset.".into();
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
    }

    pub fn measurement_summary(&self) -> Option<String>
    {
        format_measurement(
            self.measurement?,
            self.display_unit,
            &self.decimal_separator,
        )
    }
}

pub(super) fn format_measurement(
    measurement: GerberMeasurement,
    unit: GerberDisplayUnit,
    decimal_separator: &str,
) -> Option<String>
{
    let delta = measurement.delta()?;
    let distance = measurement.distance()?;
    Some(format!(
        "Distance: {} {}  ΔX: {}  ΔY: {}",
        unit.format_value(distance, decimal_separator),
        unit.suffix(),
        unit.format_value(delta.x, decimal_separator),
        unit.format_value(delta.y, decimal_separator),
    ))
}

pub(super) fn draw_measurement(
    frame: &mut canvas::Frame,
    measurement: GerberMeasurement,
    world_to_screen: &impl Fn(signex_gerber::Point) -> Point,
    color: Color,
)
{
    let start = world_to_screen(measurement.start);
    frame.fill(&canvas::Path::circle(start, 3.5), color);
    let Some(end) = measurement.end.map(world_to_screen) else
    {
        return;
    };
    frame.fill(&canvas::Path::circle(end, 3.5), color);
    frame.stroke(
        &canvas::Path::line(start, end),
        canvas::Stroke::default()
            .with_color(color)
            .with_width(1.5),
    );
}

#[cfg(test)]
#[path = "../../tests/gerber_viewer/measurement.rs"]
mod gerber_measurement_test_definitions;

#[cfg(test)]
gerber_measurement_test_definitions::gerber_measurement_tests!();
