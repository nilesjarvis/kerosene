use crate::helpers::{format_price, format_size};
use crate::message::Message;
use crate::twap_state::TwapOrder;

use super::super::super::details::{metric, section_title};
use super::super::formatting::{
    twap_next_retry_text, twap_pause_text, twap_status_check_text, weighted_average_fill_price,
};
use iced::widget::{column, row};
use iced::{Element, Theme};

// ---------------------------------------------------------------------------
// TWAP Summary
// ---------------------------------------------------------------------------

pub(in crate::order_views::twap_details) fn twap_summary<'a>(
    twap: &TwapOrder,
    theme: &Theme,
) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let duration_minutes = twap.duration.as_secs_f64() / 60.0;
    let avg_price = weighted_average_fill_price(twap)
        .map(format_price)
        .unwrap_or_else(|| "-".to_string());
    let fee: f64 = twap.child_orders.iter().map(|child| child.fee).sum();
    column![
        section_title("Summary", theme),
        row![
            metric("Filled", format_size(twap.filled_size), weak),
            metric("Target", format_size(twap.target_size), weak),
            metric(
                "Progress",
                format!("{:.1}%", twap.progress_fraction() * 100.0),
                weak
            ),
            metric("Average", avg_price, weak),
        ]
        .spacing(8),
        row![
            metric("Remaining", format_size(twap.remaining_size), weak),
            metric(
                "Slices",
                format!("{} / {}", twap.slices_sent, twap.slice_count),
                weak
            ),
            metric("Duration", format!("{duration_minutes:.1}m"), weak),
            metric("Fees", format!("{fee:.4}"), weak),
        ]
        .spacing(8),
        row![
            metric("Min", format_price(twap.min_price), weak),
            metric("Max", format_price(twap.max_price), weak),
            metric(
                "Randomize",
                if twap.randomize { "On" } else { "Off" }.to_string(),
                weak
            ),
            metric(
                "Reduce Only",
                if twap.reduce_only { "Yes" } else { "No" }.to_string(),
                weak
            ),
        ]
        .spacing(8),
        row![
            metric("Status", twap.status.label().to_string(), weak),
            metric("Pause", twap_pause_text(twap), weak),
            metric("Next Retry", twap_next_retry_text(twap), weak),
            metric("Status Check", twap_status_check_text(twap), weak),
        ]
        .spacing(8),
    ]
    .spacing(6)
    .into()
}
