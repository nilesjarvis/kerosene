use iced::widget::container as container_style;
use iced::widget::pane_grid;
use iced::{Color, Theme};

#[cfg(test)]
mod tests;

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
    border_color.a = 0.85;

    container_style::Style {
        background: Some(background.into()),
        border: iced::Border {
            width: PANE_BORDER_WIDTH,
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

pub(super) fn pane_title_bar_style(
    theme: &Theme,
    corner_radius: f32,
    dividers_enabled: bool,
) -> container_style::Style {
    // The pane outline is stroked by `pane_content_style` around the whole
    // pane, but iced paints the title bar background over it. Stroke the same
    // border here so the header shares the widget outline; the bottom edge
    // doubles as the header/body separator.
    let background = theme.extended_palette().background.strong.color;
    let mut border_color = theme.extended_palette().background.strong.text;
    border_color.a = 0.10;

    container_style::Style {
        background: Some(background.into()),
        border: iced::Border {
            width: PANE_BORDER_WIDTH,
            color: if dividers_enabled {
                border_color
            } else {
                Color::TRANSPARENT
            },
            radius: iced::border::Radius::default().top(corner_radius),
        },
        ..Default::default()
    }
}

pub(super) fn pane_content_style(
    theme: &Theme,
    corner_radius: f32,
    dividers_enabled: bool,
) -> container_style::Style {
    let mut border_color = theme.extended_palette().background.strong.text;
    border_color.a = 0.10;

    container_style::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        border: iced::Border {
            width: PANE_BORDER_WIDTH,
            color: if dividers_enabled {
                border_color
            } else {
                Color::TRANSPARENT
            },
            radius: corner_radius.into(),
        },
        ..Default::default()
    }
}

/// Outlines the pane body below the title bar. iced strokes container borders
/// before children, so full-bleed widget backgrounds paint over the outline of
/// the pane's own container; this wrapper sits above the body content and
/// keeps the side/bottom lines continuous with the header.
pub(super) fn pane_body_style(
    theme: &Theme,
    corner_radius: f32,
    dividers_enabled: bool,
) -> container_style::Style {
    let mut border_color = theme.extended_palette().background.strong.text;
    border_color.a = 0.10;

    container_style::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        border: iced::Border {
            width: PANE_BORDER_WIDTH,
            color: if dividers_enabled {
                border_color
            } else {
                Color::TRANSPARENT
            },
            radius: iced::border::Radius::default().bottom(corner_radius),
        },
        ..Default::default()
    }
}

pub(super) fn pane_grid_style(
    theme: &Theme,
    corner_radius: f32,
    divider_width: f32,
    dividers_enabled: bool,
) -> pane_grid::Style {
    let primary = theme.palette().primary;
    let split_color = if dividers_enabled {
        primary
    } else {
        Color::TRANSPARENT
    };

    pane_grid::Style {
        hovered_region: pane_grid::Highlight {
            background: primary.into(),
            border: iced::Border {
                width: PANE_BORDER_WIDTH,
                color: primary,
                radius: corner_radius.into(),
            },
        },
        picked_split: pane_grid::Line {
            color: split_color,
            width: divider_width,
        },
        hovered_split: pane_grid::Line {
            color: split_color,
            width: divider_width,
        },
    }
}

pub(super) fn subtle_pane_title_color(theme: &Theme) -> iced::Color {
    let mut color = theme.extended_palette().background.strong.text;
    color.a = 0.46;
    color
}
