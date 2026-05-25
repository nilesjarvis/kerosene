use crate::message::Message;

use iced::widget::canvas as canvas_widget;
use iced::{Color, Event, Point, Rectangle, Renderer, Theme, mouse};

// ---------------------------------------------------------------------------
// Size Preset Dots
// ---------------------------------------------------------------------------

const SIZE_PRESET_MARKS: [f32; 4] = [25.0, 50.0, 75.0, 100.0];
const SIZE_PRESET_DOT_SIZE: f32 = 7.0;
const SIZE_PRESET_HIT_RADIUS: f32 = 8.0;
const SIZE_SLIDER_HANDLE_RADIUS: f32 = 7.0;

pub(super) const SIZE_SLIDER_HEIGHT: f32 = 16.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct SizePresetDots {
    pub(super) current_pct: f32,
}

impl canvas_widget::Program<Message> for SizePresetDots {
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
            canvas_widget::Action::publish(Message::OrderPercentageChanged(pct)).and_capture()
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
            let center = size_preset_dot_center(bounds, pct);

            if hovered {
                let mut halo_color = palette.primary;
                halo_color.a = if selected { 0.18 } else { 0.12 };
                frame.fill(
                    &canvas_widget::Path::circle(center, SIZE_PRESET_HIT_RADIUS - 1.5),
                    halo_color,
                );

                let ring = canvas_widget::Path::circle(center, SIZE_PRESET_HIT_RADIUS - 2.0);
                let mut ring_color = palette.primary;
                ring_color.a = if selected { 0.55 } else { 0.38 };
                frame.stroke(
                    &ring,
                    canvas_widget::Stroke::default()
                        .with_width(1.0)
                        .with_color(ring_color),
                );
            }

            let dot_radius = if hovered {
                SIZE_PRESET_DOT_SIZE / 2.0 + 1.0
            } else {
                SIZE_PRESET_DOT_SIZE / 2.0
            };
            let dot = canvas_widget::Path::circle(center, dot_radius);
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

            frame.fill(&dot, color);
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

fn size_preset_dot_center(bounds: Rectangle, pct: f32) -> Point {
    let rail_width = (bounds.width - SIZE_SLIDER_HANDLE_RADIUS * 2.0).max(0.0);
    Point::new(
        SIZE_SLIDER_HANDLE_RADIUS + rail_width * pct / 100.0,
        bounds.height / 2.0,
    )
}

fn size_preset_pct_at_position(bounds: Rectangle, position: Point) -> Option<f32> {
    SIZE_PRESET_MARKS.into_iter().find(|pct| {
        position.distance(size_preset_dot_center(bounds, *pct)) <= SIZE_PRESET_HIT_RADIUS
    })
}
