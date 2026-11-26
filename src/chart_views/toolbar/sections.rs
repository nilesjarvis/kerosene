use crate::chart_state::{CandleFetchMode, ChartId, ChartInstance, ChartSurfaceId};
use crate::message::Message;
use iced::widget::{Row, button, container, rule, svg, text, tooltip};
use iced::{Color, Element, Fill, Theme};
use std::borrow::Cow;

const TRADE_MARKERS_ICON: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
    <path d="M7 2v3m0 8v3m10-8v3m0 8v3"/>
    <rect x="4.5" y="5" width="5" height="8" rx="1"/>
    <rect x="14.5" y="11" width="5" height="8" rx="1"/>
  </g>
  <g fill="currentColor">
    <circle cx="7" cy="21" r="2.2"/>
    <circle cx="17" cy="3" r="2.2"/>
  </g>
</svg>"#;

const COMPARE_ICON: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M2 13 8 6l5 3 9-7M2 21l6-5 5 3 9-7"/>
</svg>"#;

pub(super) fn chart_toolbar_strip(controls: Row<'_, Message>) -> Element<'_, Message> {
    container(controls.width(Fill).wrap().vertical_spacing(0))
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
    label: impl Into<Cow<'static, str>>,
    active: bool,
    msg: Message,
) -> Element<'static, Message> {
    button(text(label.into()).size(11).center())
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
        let label = match instance
            .candle_fetch_request
            .as_ref()
            .map(|request| request.mode)
        {
            Some(CandleFetchMode::WarmupOlder) => "Loading indicator history",
            Some(CandleFetchMode::BackfillOlder) => "Loading history",
            Some(CandleFetchMode::Refresh)
                if instance.candle_history_verified_at_ms.is_none()
                    && instance.candle_ws_updated_at_ms.is_none() =>
            {
                "Cached · verifying"
            }
            Some(CandleFetchMode::Refresh) if instance.candle_history_verified_at_ms.is_none() => {
                "Live · verifying history"
            }
            _ => "Refreshing",
        };
        Some(chart_toolbar_status_label(label, theme.palette().warning))
    } else if has_candles && instance.candle_fetch_error.is_some() {
        Some(chart_toolbar_status_label(
            "Candles stale",
            theme.palette().danger,
        ))
    } else if has_candles
        && instance.candle_history_verified_at_ms.is_none()
        && instance.candle_ws_updated_at_ms.is_none()
    {
        Some(chart_toolbar_status_label(
            "Cached · unverified",
            theme.palette().danger,
        ))
    } else if instance.candle_interval_gap {
        Some(chart_toolbar_status_label(
            "Candle gaps",
            theme.palette().warning,
        ))
    } else if instance.secondary_candle_fetch_request.is_some() {
        Some(chart_toolbar_status_label(
            "CMP Refreshing",
            theme.palette().warning,
        ))
    } else if instance.secondary_candle_fetch_error.is_some() {
        Some(chart_toolbar_status_label(
            "CMP Stale",
            theme.palette().danger,
        ))
    } else {
        None
    }
}

pub(super) fn push_chart_mode_buttons<'a>(
    toolbar: Row<'a, Message>,
    chart_id: ChartId,
    instance: &ChartInstance,
) -> Row<'a, Message> {
    let mut toolbar = toolbar
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
            chart_toolbar_icon_button(
                TRADE_MARKERS_ICON,
                instance.chart.show_trade_markers,
                Message::ToggleChartTradeMarkers(chart_id),
            ),
            text(if instance.chart.show_trade_markers {
                "Hide account buy/sell fills"
            } else {
                "Show account buy/sell fills"
            })
            .size(10)
            .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Top,
        ));

    toolbar = toolbar.push(chart_toolbar_separator()).push(tooltip(
        chart_toolbar_icon_button(
            COMPARE_ICON,
            instance.secondary_symbol.is_some() || instance.secondary_editor_open,
            Message::ChartSecondaryOpenEditor(chart_id),
        ),
        text(if instance.secondary_symbol.is_some() {
            "Change comparison symbol"
        } else {
            "Add comparison symbol"
        })
        .size(10)
        .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Top,
    ));

    if instance.secondary_symbol.is_some() {
        toolbar = toolbar.push(tooltip(
            chart_toolbar_button("×", false, Message::ChartSecondarySymbolRemoved(chart_id)),
            text("Remove comparison symbol")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Top,
        ));
    }

    toolbar
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

fn chart_toolbar_status_label(label: &'static str, color: Color) -> Element<'static, Message> {
    container(text(label).size(10).color(color))
        .padding([3, 8])
        .into()
}

fn chart_toolbar_icon_button(
    icon: &'static [u8],
    active: bool,
    msg: Message,
) -> Element<'static, Message> {
    // Match the text buttons' line height so wrapped toolbar rows stay aligned.
    let icon_size = text::LineHeight::default().to_absolute(11.into());
    let icon = svg(svg::Handle::from_memory(icon))
        .width(icon_size)
        .height(icon_size)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(toolbar_foreground(theme, active)),
        });
    button(icon)
        .on_press(msg)
        .padding([3, 8])
        .style(move |theme, status| compact_toolbar_button_style(theme, status, active))
        .into()
}

fn toolbar_foreground(theme: &Theme, active: bool) -> Color {
    if active {
        theme.palette().primary
    } else {
        theme.extended_palette().background.weak.text
    }
}

pub(super) fn compact_toolbar_button_style(
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
        text_color: toolbar_foreground(theme, active),
        border: iced::Border {
            radius: 0.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
