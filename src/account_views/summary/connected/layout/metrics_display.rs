use super::super::metrics::ConnectedSummaryValues;
use crate::message::Message;

use iced::widget::{column, container, text};
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
