use crate::advanced_order_history::AdvancedOrderHistoryEntry;
use crate::message::Message;
use crate::signing::{ChaseLifecycle, ChaseOrder};
use crate::twap_state::{TwapOrder, TwapStatus};

use iced::widget::{container, row, text};
use iced::{Alignment, Element, Fill, Theme};

use super::components::{
    badge, details_button, history_info_button, history_row_container_style, row_container_style,
    spinning_gear, stop_button, stop_twap_button,
};

mod labels;

use self::labels::{
    chase_meta_label, chase_price_label, chase_size_label, history_completed_label,
    history_progress_label, history_summary_label, twap_meta_label, twap_progress_label,
    twap_status_text,
};

// ---------------------------------------------------------------------------
// Advanced Order Rows
// ---------------------------------------------------------------------------

pub(super) fn chase_order_row(
    chase: &ChaseOrder,
    theme: &Theme,
    spinner_phase: f32,
) -> Element<'static, Message> {
    let side = if chase.is_buy { "BUY" } else { "SELL" };
    let side_color = if chase.is_buy {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    let weak_text = theme.extended_palette().background.weak.text;
    let status = chase_status(chase);
    let status_color = if chase.lifecycle.is_stopping() {
        theme.palette().danger
    } else if !matches!(chase.lifecycle, ChaseLifecycle::Resting) {
        theme.palette().primary
    } else {
        weak_text
    };
    let price = chase_price_label(chase.current_price);
    let meta = chase_meta_label(chase.reprice_count, chase.reduce_only);
    let size = chase_size_label(chase.filled_size, chase.target_size, chase.remaining_size);

    container(
        row![
            spinning_gear(spinner_phase, 13, theme.palette().primary),
            badge("CHASE"),
            text(side).size(10).color(side_color),
            text(chase.coin.clone()).size(12).width(Fill),
            text(format!("{size} @ {price}")).size(11),
            text(meta).size(10).color(weak_text),
            text(status).size(10).color(status_color),
            stop_button(chase.id)
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([5, 6])
    .style(row_container_style)
    .into()
}

pub(super) fn twap_order_row(
    twap: &TwapOrder,
    theme: &Theme,
    spinner_phase: f32,
) -> Element<'static, Message> {
    let side = twap.side_label();
    let side_color = if twap.is_buy {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    let weak_text = theme.extended_palette().background.weak.text;
    let status_color = match twap.status {
        TwapStatus::Error | TwapStatus::CompletedPartial => theme.palette().danger,
        TwapStatus::Running
        | TwapStatus::WaitingForMarket
        | TwapStatus::Paused
        | TwapStatus::Stopping => theme.palette().primary,
        TwapStatus::Stopped | TwapStatus::Completed => weak_text,
    };
    let progress = twap_progress_label(twap.filled_size, twap.target_size);
    let meta = twap_meta_label(
        twap.slices_sent,
        twap.slice_count,
        twap.min_price,
        twap.max_price,
    );
    let status = twap_status_text(twap.status, twap.pause_reason);
    let stop_cell = if twap.status.is_terminal() {
        details_button(twap.id)
    } else {
        row![details_button(twap.id), stop_twap_button(twap.id)]
            .spacing(4)
            .into()
    };

    container(
        row![
            spinning_gear(spinner_phase, 13, theme.palette().primary),
            badge("TWAP"),
            text(side).size(10).color(side_color),
            text(twap.coin.clone()).size(12).width(Fill),
            text(progress).size(11),
            text(meta).size(10).color(weak_text),
            text(status).size(10).color(status_color),
            stop_cell
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([5, 6])
    .style(row_container_style)
    .into()
}

pub(super) fn history_order_row(
    entry: &AdvancedOrderHistoryEntry,
    theme: &Theme,
    now_ms: u64,
) -> Element<'static, Message> {
    let side_color = if entry.is_buy {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    let weak_text = theme.extended_palette().background.weak.text;
    let status_color = if entry.status == "Error" || entry.status == "Partial" {
        theme.palette().danger
    } else {
        weak_text
    };
    let progress = history_progress_label(entry.filled_size, entry.target_size);
    let completed = history_completed_label(entry.completed_at_ms, now_ms);
    let summary = history_summary_label(&entry.summary);

    container(
        row![
            badge(entry.kind.label()),
            text(entry.side_label()).size(10).color(side_color),
            text(entry.display_coin.clone()).size(12).width(70),
            text(summary).size(10).color(weak_text).width(Fill),
            text(progress).size(11),
            text(completed).size(10).color(weak_text),
            text(entry.status.clone()).size(10).color(status_color),
            history_info_button(entry.id.clone()),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([5, 6])
    .style(history_row_container_style)
    .into()
}

fn chase_status(chase: &ChaseOrder) -> &'static str {
    chase.lifecycle.label()
}
