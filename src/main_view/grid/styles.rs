use iced::widget::container as container_style;
use iced::{Color, Theme};

pub(super) const PANE_BORDER_WIDTH: f32 = 1.0;

pub(super) fn pane_drag_ghost_style(theme: &Theme, corner_radius: f32) -> container_style::Style {
    let mut background = theme.palette().primary;
    background.a = 0.12;

    let mut border_color = theme.palette().primary;
    border_color.a = 0.85;

    container_style::Style {
        background: Some(background.into()),
        border: iced::Border {
            width: PANE_BORDER_WIDTH,
            color: border_color,
            radius: corner_radius.into(),
        },
        ..Default::default()
    }
}

pub(super) fn pane_drag_ghost_title_bar_style(
    theme: &Theme,
    corner_radius: f32,
) -> container_style::Style {
    let mut background = theme.palette().primary;
    background.a = 0.18;

    let mut border_color = theme.palette().primary;
    border_color.a = 0.35;

    container_style::Style {
        background: Some(background.into()),
        border: iced::Border {
            width: 0.0,
            color: border_color,
            radius: iced::border::Radius::default().top(corner_radius),
        },
        ..Default::default()
    }
}

pub(super) fn drag_ghost_title_color(theme: &Theme) -> Color {
    let mut color = theme.palette().text;
    color.a = 0.72;
    color
}

pub(super) fn pane_title_bar_style(theme: &Theme, corner_radius: f32) -> container_style::Style {
    use iced::gradient;

    let background = theme.extended_palette().background.strong.color;
    let mut separator = theme.extended_palette().background.strong.text;
    separator.a = 0.08;

    container_style::Style {
        background: Some(
            gradient::Linear::new(iced::Degrees(180.0))
                .add_stop(0.00, background)
                .add_stop(0.97, background)
                .add_stop(0.985, separator)
                .add_stop(1.00, separator)
                .into(),
        ),
        border: iced::Border {
            radius: iced::border::Radius::default().top(corner_radius),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn pane_content_style(theme: &Theme, corner_radius: f32) -> container_style::Style {
    let mut border_color = theme.extended_palette().background.strong.text;
    border_color.a = 0.10;

    container_style::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        border: iced::Border {
            width: PANE_BORDER_WIDTH,
            color: border_color,
            radius: corner_radius.into(),
        },
        ..Default::default()
    }
}

pub(super) fn subtle_pane_title_color(theme: &Theme) -> iced::Color {
    let mut color = theme.extended_palette().background.strong.text;
    color.a = 0.46;
    color
}
