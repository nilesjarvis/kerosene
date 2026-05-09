use crate::annotations::DrawingTool;
use crate::chart_state::{ChartId, ChartInstance};
use crate::helpers::timeframe_button;
use crate::message::Message;
use iced::widget::{Row, button, container, rule, text};
use iced::{Color, Element, Theme};

pub(super) fn chart_reload_button(chart_id: ChartId) -> Element<'static, Message> {
    button(text("\u{27F3}").size(12))
        .on_press(Message::ChartReload(chart_id))
        .padding([2, 4])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn chart_reset_view_button(chart_id: ChartId) -> Element<'static, Message> {
    timeframe_button("Reset View", false, Message::ChartResetView(chart_id))
}

pub(super) fn chart_fetch_status_label(
    has_candles: bool,
    instance: &ChartInstance,
    theme: &Theme,
) -> Option<Element<'static, Message>> {
    if has_candles && instance.candle_fetch_request.is_some() {
        Some(
            text("Refreshing")
                .size(10)
                .color(theme.palette().warning)
                .into(),
        )
    } else if has_candles && instance.candle_fetch_error.is_some() {
        Some(text("Stale").size(10).color(theme.palette().danger).into())
    } else {
        None
    }
}

pub(super) fn push_drawing_tool_buttons<'a>(
    toolbar: Row<'a, Message>,
    chart_id: ChartId,
    active_tool: Option<DrawingTool>,
) -> Row<'a, Message> {
    toolbar
        .push(chart_toolbar_separator())
        .push(drawing_tool_button(
            "\u{2014}",
            chart_id,
            active_tool,
            DrawingTool::HorizontalLevel,
        ))
        .push(drawing_tool_button(
            "\u{2571}",
            chart_id,
            active_tool,
            DrawingTool::TrendLine,
        ))
        .push(drawing_tool_button(
            "\u{2717}",
            chart_id,
            active_tool,
            DrawingTool::Eraser,
        ))
}

pub(super) fn push_chart_mode_buttons<'a>(
    toolbar: Row<'a, Message>,
    chart_id: ChartId,
    instance: &ChartInstance,
) -> Row<'a, Message> {
    toolbar
        .push(chart_toolbar_separator())
        .push(timeframe_button(
            "INV",
            instance.chart.inverted,
            Message::ToggleChartInvert(chart_id),
        ))
}

pub(super) fn push_market_overlay_buttons<'a>(
    toolbar: Row<'a, Message>,
    chart_id: ChartId,
    instance: &ChartInstance,
    is_perp_chart: bool,
    theme: &Theme,
) -> Row<'a, Message> {
    if !is_perp_chart {
        return toolbar;
    }

    let mut toolbar = toolbar
        .push(chart_toolbar_separator())
        .push(timeframe_button(
            "LIQ",
            instance.show_liquidations,
            Message::ToggleLiquidationOverlay(chart_id),
        ))
        .push(timeframe_button(
            "HEAT",
            instance.show_heatmap,
            Message::ToggleHeatmapOverlay(chart_id),
        ));

    if instance.show_heatmap
        && let Some((status, is_error)) = &instance.heatmap_status
    {
        let status_color = if *is_error {
            theme.palette().danger
        } else {
            theme.extended_palette().background.weak.text
        };
        toolbar = toolbar.push(heatmap_status_label(status.clone(), status_color));
    }

    toolbar
}

fn chart_toolbar_separator() -> Element<'static, Message> {
    container(rule::vertical(1)).height(16).width(8).into()
}

fn drawing_tool_button(
    label: &'static str,
    chart_id: ChartId,
    active_tool: Option<DrawingTool>,
    tool: DrawingTool,
) -> Element<'static, Message> {
    timeframe_button(
        label,
        active_tool == Some(tool),
        Message::SetDrawingTool(
            chart_id,
            if active_tool == Some(tool) {
                None
            } else {
                Some(tool)
            },
        ),
    )
}

fn heatmap_status_label(status: String, color: Color) -> Element<'static, Message> {
    text(status)
        .size(10)
        .font(iced::Font::MONOSPACE)
        .color(color)
        .into()
}
