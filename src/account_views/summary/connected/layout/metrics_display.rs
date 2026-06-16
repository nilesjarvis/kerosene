use super::super::metrics::ConnectedSummaryValues;
use crate::message::Message;

use iced::widget::{Space, column, container, text};
use iced::{Color, Element, Theme};

// ---------------------------------------------------------------------------
// Connected Summary Metric Display
// ---------------------------------------------------------------------------

pub(super) fn summary_metric(
    label: &'static str,
    value: impl ToString,
    theme: &Theme,
) -> Element<'static, Message> {
    summary_metric_with_color(label, value, None, theme)
}

pub(super) fn summary_metric_colored(
    label: &'static str,
    value: impl ToString,
    value_color: Color,
    theme: &Theme,
) -> Element<'static, Message> {
    summary_metric_with_color(label, value, Some(value_color), theme)
}

fn summary_metric_with_color(
    label: &'static str,
    value: impl ToString,
    value_color: Option<Color>,
    theme: &Theme,
) -> Element<'static, Message> {
    let value = text(value.to_string())
        .size(13)
        .font(crate::app_fonts::monospace_font());
    let value = if let Some(value_color) = value_color {
        value.color(value_color)
    } else {
        value
    };

    container(
        column![
            text(label)
                .size(10)
                .color(theme.extended_palette().background.weak.text),
            value,
        ]
        .spacing(1)
        .align_x(iced::Alignment::Start),
    )
    .into()
}

// ---------------------------------------------------------------------------
// Connected Summary Skeleton Placeholders
// ---------------------------------------------------------------------------

const SKELETON_LABEL_HEIGHT: f32 = 9.0;
const SKELETON_VALUE_HEIGHT: f32 = 13.0;
const SKELETON_LABEL_ALPHA: f32 = 0.10;

/// A single placeholder metric mirroring [`summary_metric`]'s two-line stack
/// (dim label bar over a brighter value bar), used by the loading skeleton so
/// the real label/value text lands in the same position when data arrives.
pub(super) fn skeleton_metric(
    label_w: f32,
    value_w: f32,
    value_alpha: f32,
    theme: &Theme,
) -> Element<'static, Message> {
    let neutral = theme.extended_palette().background.weak.text;

    container(
        column![
            skeleton_bar(
                label_w,
                SKELETON_LABEL_HEIGHT,
                SKELETON_LABEL_ALPHA,
                neutral
            ),
            skeleton_bar(value_w, SKELETON_VALUE_HEIGHT, value_alpha, neutral),
        ]
        .spacing(1)
        .align_x(iced::Alignment::Start),
    )
    .into()
}

fn skeleton_bar(width: f32, height: f32, alpha: f32, neutral: Color) -> Element<'static, Message> {
    container(Space::new().width(width).height(height))
        .style(move |_theme: &Theme| container::Style {
            background: Some(
                Color {
                    a: alpha,
                    ..neutral
                }
                .into(),
            ),
            border: iced::Border {
                radius: (height * 0.5).into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

pub(super) fn portfolio_margin_ratio_color(
    summary_values: &ConnectedSummaryValues,
    theme: &Theme,
) -> iced::Color {
    let Some(margin_ratio) = summary_values.portfolio_margin_ratio else {
        return theme.palette().warning;
    };
    if margin_ratio < 0.5 {
        theme.palette().success
    } else if margin_ratio < 0.8 {
        theme.palette().primary
    } else {
        theme.palette().danger
    }
}

pub(super) fn available_color(
    summary_values: &ConnectedSummaryValues,
    theme: &Theme,
) -> iced::Color {
    let (Some(margin_used), Some(available)) =
        (summary_values.margin_used, summary_values.available)
    else {
        return theme.palette().warning;
    };
    if margin_used < 1e-6 || available > margin_used * 2.0 {
        theme.palette().success
    } else if available > margin_used * 0.5 {
        theme.palette().primary
    } else {
        theme.palette().danger
    }
}
