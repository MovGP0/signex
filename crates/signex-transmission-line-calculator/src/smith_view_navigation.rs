use iced::widget::canvas;
use iced::{Event, Point, Rectangle, Vector, mouse};

use crate::SmithViewTransform;

use super::SmithChartMessage;

const LINE_ZOOM_RATE: f32 = 0.18;
const PIXEL_ZOOM_RATE: f32 = 0.003;

/// Tracks mouse-wheel zoom and middle-button pan gestures for a Smith chart.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SmithViewNavigationState {
    pan_start: Option<Point>,
    pan_transform: SmithViewTransform,
}

impl SmithViewNavigationState {
    /// Handles an input event and returns the resulting action, if any.
    pub(super) fn update(
        &mut self,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        transform: SmithViewTransform,
    ) -> Option<canvas::Action<SmithChartMessage>> {
        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let anchor = cursor.position_in(bounds)?;
                let zoom = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => (*y * LINE_ZOOM_RATE).exp(),
                    mouse::ScrollDelta::Pixels { y, .. } => (*y * PIXEL_ZOOM_RATE).exp(),
                };
                let transformed = transform.zoomed_at(anchor, zoom);
                if transformed == transform {
                    Some(canvas::Action::capture())
                } else {
                    Some(
                        canvas::Action::publish(SmithChartMessage::SmithViewTransformChanged(
                            transformed,
                        ))
                        .and_capture(),
                    )
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                let position = cursor.position_in(bounds)?;
                self.begin_pan(position, transform);
                Some(canvas::Action::capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if self.is_panning() => {
                let position = Point::new(position.x - bounds.x, position.y - bounds.y);
                let transformed = self.pan_to(position);
                Some(
                    canvas::Action::publish(SmithChartMessage::SmithViewTransformChanged(
                        transformed,
                    ))
                    .and_capture(),
                )
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                if self.end_pan() {
                    Some(canvas::Action::capture())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Returns whether a middle-button pan gesture is active.
    pub(super) fn is_panning(self) -> bool {
        self.pan_start.is_some()
    }

    /// Starts a pan gesture at the supplied screen position.
    fn begin_pan(&mut self, position: Point, transform: SmithViewTransform) {
        self.pan_start = Some(position);
        self.pan_transform = transform;
    }

    /// Returns the view transform for the current pan position.
    fn pan_to(self, position: Point) -> SmithViewTransform {
        let Some(start) = self.pan_start else {
            return self.pan_transform;
        };
        self.pan_transform
            .translated(Vector::new(position.x - start.x, position.y - start.y))
    }

    /// Ends the active pan gesture and reports whether one existed.
    fn end_pan(&mut self) -> bool {
        self.pan_start.take().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that pan is measured from gesture start and ends cleanly.
    #[test]
    fn pan_is_measured_from_gesture_start_and_ends_cleanly() {
        let mut navigation = SmithViewNavigationState::default();
        let transform = SmithViewTransform::identity().zoomed_at(Point::ORIGIN, 2.0);
        navigation.begin_pan(Point::new(20.0, 30.0), transform);

        let panned = navigation.pan_to(Point::new(37.0, 24.0));

        assert_eq!(panned.translation(), Vector::new(17.0, -6.0));
        assert!(navigation.end_pan());
        assert!(!navigation.is_panning());
        assert!(!navigation.end_pan());
    }
}
