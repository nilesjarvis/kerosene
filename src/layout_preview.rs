use crate::config::{AxisConfig, PaneLayoutConfig};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, column, container, row};
use iced::{Color, Element, Fill, Length, Theme};

const PREVIEW_WIDTH: f32 = 42.0;
const PREVIEW_HEIGHT: f32 = 24.0;
const PREVIEW_GAP: f32 = 2.0;
const PORTION_SCALE: f32 = 1000.0;
const MIN_SPLIT_RATIO: f32 = 0.08;
const MAX_SPLIT_RATIO: f32 = 0.92;

// ---------------------------------------------------------------------------
// Saved Layout Preview
// ---------------------------------------------------------------------------

pub(crate) fn saved_layout_preview(
    layout: Option<&PaneLayoutConfig>,
    theme: &Theme,
    is_active: bool,
) -> Element<'static, Message> {
    let palette = theme.palette();
    let surface_color = theme.extended_palette().background.weak.color;
    let block_color = with_alpha(palette.primary, 0.72);
    let block_border = with_alpha(palette.primary, 0.9);
    let border_color = if is_active {
        palette.primary
    } else {
        theme.extended_palette().background.strong.color
    };

    let preview = match layout {
        Some(layout) => layout_preview_node(layout, block_color, block_border),
        None => layout_preview_leaf(block_color, block_border),
    };

    container(preview)
        .width(Length::Fixed(PREVIEW_WIDTH))
        .height(Length::Fixed(PREVIEW_HEIGHT))
        .padding(2)
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(surface_color.into()),
            border: iced::Border {
                radius: 3.0.into(),
                width: 1.0,
                color: border_color,
            },
            ..Default::default()
        })
        .into()
}

fn layout_preview_node(
    layout: &PaneLayoutConfig,
    block_color: Color,
    block_border: Color,
) -> Element<'static, Message> {
    match layout {
        PaneLayoutConfig::Leaf(_) => layout_preview_leaf(block_color, block_border),
        PaneLayoutConfig::Split { axis, ratio, a, b } => {
            let (a_portion, b_portion) = split_portions(*ratio);
            let a = layout_preview_node(a, block_color, block_border);
            let b = layout_preview_node(b, block_color, block_border);

            match axis {
                AxisConfig::Horizontal => column![
                    container(a)
                        .width(Fill)
                        .height(Length::FillPortion(a_portion)),
                    container(b)
                        .width(Fill)
                        .height(Length::FillPortion(b_portion)),
                ]
                .spacing(PREVIEW_GAP)
                .width(Fill)
                .height(Fill)
                .into(),
                AxisConfig::Vertical => row![
                    container(a)
                        .width(Length::FillPortion(a_portion))
                        .height(Fill),
                    container(b)
                        .width(Length::FillPortion(b_portion))
                        .height(Fill),
                ]
                .spacing(PREVIEW_GAP)
                .width(Fill)
                .height(Fill)
                .into(),
            }
        }
    }
}

fn layout_preview_leaf(block_color: Color, block_border: Color) -> Element<'static, Message> {
    container(Space::new().width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(block_color.into()),
            border: iced::Border {
                radius: 2.0.into(),
                width: 1.0,
                color: block_border,
            },
            ..Default::default()
        })
        .into()
}

fn split_portions(ratio: f32) -> (u16, u16) {
    let ratio = if ratio.is_finite() { ratio } else { 0.5 };
    let ratio = ratio.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO);
    let first = (ratio * PORTION_SCALE).round() as u16;
    let second = PORTION_SCALE as u16 - first;

    (first, second)
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_portions_preserve_saved_ratio() {
        assert_eq!(split_portions(0.25), (250, 750));
        assert_eq!(split_portions(0.5), (500, 500));
        assert_eq!(split_portions(0.75), (750, 250));
    }

    #[test]
    fn split_portions_clamp_extreme_or_invalid_ratios() {
        assert_eq!(split_portions(0.0), (80, 920));
        assert_eq!(split_portions(1.0), (920, 80));
        assert_eq!(split_portions(f32::NAN), (500, 500));
    }
}
