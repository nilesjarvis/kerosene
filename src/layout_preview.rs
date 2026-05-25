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
const MAX_PREVIEW_DEPTH: usize = 5;
const MAX_PREVIEW_NODES: usize = 31;

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
        Some(layout) => {
            let mut budget = PreviewBudget::new(MAX_PREVIEW_DEPTH, MAX_PREVIEW_NODES);
            layout_preview_node(layout, block_color, block_border, 0, &mut budget)
        }
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

/// Builds a compact preview and summarizes any subtree beyond the preview budget.
fn layout_preview_node(
    layout: &PaneLayoutConfig,
    block_color: Color,
    block_border: Color,
    depth: usize,
    budget: &mut PreviewBudget,
) -> Element<'static, Message> {
    if !budget.take_node() || depth >= budget.max_depth {
        return layout_preview_leaf(block_color, block_border);
    }

    match layout {
        PaneLayoutConfig::Leaf(_) => layout_preview_leaf(block_color, block_border),
        PaneLayoutConfig::Split { axis, ratio, a, b } => {
            let (a_portion, b_portion) = split_portions(*ratio);
            let next_depth = depth + 1;
            let a = layout_preview_node(a, block_color, block_border, next_depth, budget);
            let b = layout_preview_node(b, block_color, block_border, next_depth, budget);

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

#[derive(Debug, Clone, Copy)]
struct PreviewBudget {
    max_depth: usize,
    remaining_nodes: usize,
}

impl PreviewBudget {
    fn new(max_depth: usize, max_nodes: usize) -> Self {
        Self {
            max_depth,
            remaining_nodes: max_nodes,
        }
    }

    fn take_node(&mut self) -> bool {
        let Some(remaining_nodes) = self.remaining_nodes.checked_sub(1) else {
            return false;
        };
        self.remaining_nodes = remaining_nodes;
        true
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
mod tests;
