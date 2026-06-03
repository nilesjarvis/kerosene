use super::components::{indicator_group_label, menu_checkbox};
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;

use iced::widget::{Column, row, text};
use iced::{Alignment, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Indicator Menu Overlays
// ---------------------------------------------------------------------------

pub(super) fn overlay_group(
    chart_id: ChartId,
    instance: &ChartInstance,
    theme: &Theme,
    earnings_available: bool,
) -> Element<'static, Message> {
    let mut option_row = row![
        menu_checkbox(
            "LIQ",
            instance.show_liquidations,
            Message::ToggleLiquidationOverlay(chart_id),
        ),
        menu_checkbox(
            "HEAT",
            instance.show_heatmap,
            Message::ToggleHeatmapOverlay(chart_id),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Fill);

    if earnings_available {
        option_row = option_row.push(menu_checkbox(
            "EARN",
            instance.show_earnings_markers,
            Message::ToggleChartEarningsMarkers(chart_id),
        ));
    }

    let mut content = Column::new().spacing(2).width(Fill).push(option_row);

    if let Some(status) = overlay_status(instance, theme) {
        content = content.push(status);
    }

    row![indicator_group_label("OVR"), content]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
}

fn overlay_status(instance: &ChartInstance, theme: &Theme) -> Option<Element<'static, Message>> {
    let mut parts = Vec::new();
    let mut is_error = false;

    if instance.show_liquidations {
        if instance.liquidation_fetching {
            parts.push("LIQ loading".to_string());
        } else if let Some((status, status_is_error)) = &instance.liquidation_status {
            parts.push(status.clone());
            is_error |= *status_is_error;
        }
    }

    if instance.show_heatmap {
        if instance.heatmap_fetching {
            parts.push("HEAT loading".to_string());
        } else if let Some((status, status_is_error)) = &instance.heatmap_status {
            parts.push(status.clone());
            is_error |= *status_is_error;
        }
    }

    if instance.show_earnings_markers {
        if instance.earnings_fetching {
            parts.push("EARN loading".to_string());
        } else if let Some((status, status_is_error)) = &instance.earnings_status {
            parts.push(status.clone());
            is_error |= *status_is_error;
        }
    }

    if parts.is_empty() {
        return None;
    }

    let color = if is_error {
        theme.palette().danger
    } else {
        theme.extended_palette().background.weak.text
    };

    Some(
        text(parts.join(" / "))
            .size(9)
            .font(crate::app_fonts::monospace_font())
            .color(color)
            .into(),
    )
}
