use crate::message::Message;

use iced::widget::{canvas as canvas_widget, slider};
use iced::{Color, Event, Point, Rectangle, Renderer, Size, Theme, mouse};
use std::fmt;

// ---------------------------------------------------------------------------
// Size Preset Marks
// ---------------------------------------------------------------------------

const SIZE_PRESET_MARKS: [f32; 4] = [25.0, 50.0, 75.0, 100.0];
const SIZE_PRESET_MARK_WIDTH: f32 = 3.0;
const SIZE_PRESET_MARK_HEIGHT: f32 = 16.0;
const SIZE_PRESET_HIT_WIDTH: f32 = 16.0;
const SIZE_AMOUNT_FIELD_HEIGHT: f32 = 29.0;
const SIZE_SLIDER_HANDLE_WIDTH: u16 = 7;

pub(super) const SIZE_PERCENT_LABEL_WIDTH: f32 = 38.0;
pub(super) const SIZE_SLIDER_HEIGHT: f32 = SIZE_AMOUNT_FIELD_HEIGHT;

pub(super) fn size_slider_style(theme: &Theme, status: slider::Status) -> slider::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let mut active = palette.primary;
    active.a = match status {
        slider::Status::Active => 0.45,
        slider::Status::Hovered => 0.55,
        slider::Status::Dragged => 0.68,
    };

    let mut inactive = extended.background.weak.color;
    inactive.a = 0.72;

    let handle_color = match status {
        slider::Status::Active => palette.primary,
        slider::Status::Hovered => extended.primary.strong.color,
        slider::Status::Dragged => extended.primary.weak.color,
    };

    let mut style = slider::default(theme, status);
    style.rail.width = SIZE_SLIDER_HEIGHT;
    style.rail.backgrounds = (active.into(), inactive.into());
    style.rail.border = iced::Border {
        radius: 5.0.into(),
        width: 1.0,
        color: extended.background.strong.color,
    };
    style.handle.shape = slider::HandleShape::Rectangle {
        width: SIZE_SLIDER_HANDLE_WIDTH,
        border_radius: 3.0.into(),
    };
    style.handle.background = handle_color.into();
    style.handle.border_width = 0.0;
    style.handle.border_color = handle_color;
    style
}

#[derive(Clone, Copy)]
pub(super) struct SizePresetMarks {
    pub(super) current_pct: f32,
}

impl fmt::Debug for SizePresetMarks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SizePresetMarks")
            .field("current_pct", &"<redacted>")
            .finish()
    }
}

impl canvas_widget::Program<Message> for SizePresetMarks {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas_widget::Action<Message>> {
        let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event else {
            return None;
        };
        let position = cursor.position_in(bounds)?;

        size_preset_pct_at_position(bounds, position).map(|pct| {
            canvas_widget::Action::publish(Message::OrderPercentageChanged(pct.into()))
                .and_capture()
        })
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas_widget::Geometry> {
        let mut frame = canvas_widget::Frame::new(renderer, bounds.size());
        let palette = theme.palette();
        let hovered_pct = cursor
            .position_in(bounds)
            .and_then(|position| size_preset_pct_at_position(bounds, position));

        for pct in SIZE_PRESET_MARKS {
            let selected = (self.current_pct - pct).abs() < 0.5;
            let hovered = hovered_pct.is_some_and(|hovered_pct| hovered_pct == pct);
            let center = size_preset_mark_center(bounds, pct);

            if hovered {
                let halo_origin = Point::new(center.x - 6.0, center.y - SIZE_PRESET_MARK_HEIGHT);
                let halo_size = Size::new(12.0, SIZE_PRESET_MARK_HEIGHT * 2.0);
                let halo =
                    canvas_widget::Path::rounded_rectangle(halo_origin, halo_size, 4.0.into());
                let mut halo_color = palette.primary;
                halo_color.a = if selected { 0.16 } else { 0.1 };
                frame.fill(&halo, halo_color);
            }

            let mark_height = if hovered {
                SIZE_PRESET_MARK_HEIGHT + 3.0
            } else {
                SIZE_PRESET_MARK_HEIGHT
            };
            let mark = canvas_widget::Path::rounded_rectangle(
                Point::new(
                    center.x - SIZE_PRESET_MARK_WIDTH / 2.0,
                    center.y - mark_height / 2.0,
                ),
                Size::new(SIZE_PRESET_MARK_WIDTH, mark_height),
                1.5.into(),
            );
            let mut color = if selected || hovered {
                palette.primary
            } else {
                Color {
                    a: 0.45,
                    ..palette.text
                }
            };
            if hovered && !selected {
                color.a = 0.82;
            }

            frame.fill(&mark, color);
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor
            .position_in(bounds)
            .and_then(|position| size_preset_pct_at_position(bounds, position))
            .is_some()
        {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn size_preset_mark_center(bounds: Rectangle, pct: f32) -> Point {
    let handle_width = f32::from(SIZE_SLIDER_HANDLE_WIDTH);
    let rail_width = (bounds.width - handle_width).max(0.0);
    Point::new(
        handle_width / 2.0 + rail_width * pct / 100.0,
        bounds.height / 2.0,
    )
}

fn size_preset_pct_at_position(bounds: Rectangle, position: Point) -> Option<f32> {
    SIZE_PRESET_MARKS.into_iter().find(|pct| {
        let center = size_preset_mark_center(bounds, *pct);
        (position.x - center.x).abs() <= SIZE_PRESET_HIT_WIDTH / 2.0
            && position.y >= 0.0
            && position.y <= bounds.height
    })
}

#[cfg(test)]
mod tests {
    use super::SizePresetMarks;

    #[test]
    fn size_preset_debug_redacts_order_percentage_without_changing_it() {
        let marks = SizePresetMarks {
            current_pct: 42.424_244,
        };

        let rendered = format!("{marks:?}");

        assert!(rendered.contains("<redacted>"), "{rendered}");
        assert!(!rendered.contains("42.424244"), "{rendered}");
        assert_eq!(marks.current_pct.to_bits(), 42.424_244_f32.to_bits());
    }
}
