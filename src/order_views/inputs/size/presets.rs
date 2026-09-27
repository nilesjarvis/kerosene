use iced::widget::slider;
use iced::{Color, Theme};

pub(super) const SIZE_PERCENT_LABEL_WIDTH: f32 = 38.0;
pub(super) const SIZE_SLIDER_HEIGHT: f32 = 24.0;

pub(super) fn size_slider_style(theme: &Theme, status: slider::Status) -> slider::Style {
    let mut style = slider::default(theme, status);
    style.rail.width = 3.0;
    style.rail.backgrounds = (
        theme.palette().primary.into(),
        Color {
            a: 0.2,
            ..theme.palette().text
        }
        .into(),
    );
    style.handle.shape = slider::HandleShape::Circle { radius: 5.0 };
    style
}
