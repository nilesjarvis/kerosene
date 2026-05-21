use crate::annotations::DrawingTool;
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId};
use crate::message::Message;
use iced::widget::{Row, button, container, rule, text, tooltip};
use iced::{Color, Element, Fill, Theme};

pub(super) fn chart_toolbar_strip<'a>(content: Row<'a, Message>) -> Element<'a, Message> {
    container(content.width(Fill).wrap().vertical_spacing(0))
        .width(Fill)
        .style(|theme: &Theme| {
            let background = Color {
                a: 0.04,
                ..theme.extended_palette().background.weak.color
            };
            container::Style {
                background: Some(background.into()),
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn chart_toolbar_button(
    label: &'static str,
    active: bool,
    msg: Message,
) -> Element<'static, Message> {
    button(text(label).size(11).center())
        .on_press(msg)
        .padding([3, 8])
        .style(move |theme: &Theme, status| compact_toolbar_button_style(theme, status, active))
        .into()
}

pub(super) fn chart_reload_button(chart_id: ChartId) -> Element<'static, Message> {
    button(text("\u{27F3}").size(12))
        .on_press(Message::ChartReload(chart_id))
        .padding([3, 8])
        .style(|theme: &Theme, status| compact_toolbar_button_style(theme, status, false))
        .into()
}

pub(super) fn chart_reset_view_button(
    chart_id: ChartId,
    surface_id: ChartSurfaceId,
) -> Element<'static, Message> {
    chart_toolbar_button(
        "Reset View",
        false,
        Message::ChartResetView(chart_id, surface_id),
    )
}

pub(super) fn chart_fetch_status_label(
    has_candles: bool,
    instance: &ChartInstance,
    theme: &Theme,
) -> Option<Element<'static, Message>> {
    if has_candles && instance.candle_fetch_request.is_some() {
        Some(chart_toolbar_status_label(
            "Refreshing",
            theme.palette().warning,
        ))
    } else if has_candles && instance.candle_fetch_error.is_some() {
        Some(chart_toolbar_status_label("Stale", theme.palette().danger))
    } else {
        None
    }
}

pub(super) fn push_drawing_tool_buttons<'a>(
    toolbar: Row<'a, Message>,
    chart_id: ChartId,
    surface_id: ChartSurfaceId,
    active_tool: Option<DrawingTool>,
) -> Row<'a, Message> {
    toolbar
        .push(chart_toolbar_separator())
        .push(drawing_tool_button(
            "\u{2014}",
            chart_id,
            surface_id,
            active_tool,
            DrawingTool::HorizontalLevel,
        ))
        .push(chart_toolbar_separator())
        .push(drawing_tool_button(
            "\u{2571}",
            chart_id,
            surface_id,
            active_tool,
            DrawingTool::TrendLine,
        ))
        .push(chart_toolbar_separator())
        .push(drawing_tool_button(
            "\u{2717}",
            chart_id,
            surface_id,
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
        .push(tooltip(
            chart_toolbar_button(
                "\u{21C5}",
                instance.chart.inverted,
                Message::ToggleChartInvert(chart_id),
            ),
            text("Invert price axis")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Top,
        ))
        .push(chart_toolbar_separator())
        .push(tooltip(
            chart_toolbar_button(
                "FILL",
                instance.chart.show_trade_markers,
                Message::ToggleChartTradeMarkers(chart_id),
            ),
            text("Show account buy/sell fills")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Top,
        ))
}

pub(super) fn chart_toolbar_separator() -> Element<'static, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.12,
            ..theme.extended_palette().background.weak.text
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .height(14)
    .width(1)
    .into()
}

fn drawing_tool_button(
    label: &'static str,
    chart_id: ChartId,
    surface_id: ChartSurfaceId,
    active_tool: Option<DrawingTool>,
    tool: DrawingTool,
) -> Element<'static, Message> {
    chart_toolbar_button(
        label,
        active_tool == Some(tool),
        Message::SetDrawingTool(
            chart_id,
            surface_id,
            if active_tool == Some(tool) {
                None
            } else {
                Some(tool)
            },
        ),
    )
}

fn chart_toolbar_status_label(label: &'static str, color: Color) -> Element<'static, Message> {
    container(text(label).size(10).color(color))
        .padding([3, 8])
        .into()
}

fn compact_toolbar_button_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let background = if active {
        Color {
            a: 0.10,
            ..theme.palette().primary
        }
    } else {
        match status {
            button::Status::Hovered => Color {
                a: 0.55,
                ..theme.extended_palette().background.strong.color
            },
            _ => Color::TRANSPARENT,
        }
    };

    button::Style {
        background: Some(background.into()),
        text_color: if active {
            theme.palette().primary
        } else {
            theme.extended_palette().background.weak.text
        },
        border: iced::Border {
            radius: 0.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
