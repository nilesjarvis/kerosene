use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance, PriceFlash};
use crate::helpers;
use crate::message::Message;
use iced::widget::{Row, Space, button, column, row, text};
use iced::{Alignment, Color, Element, Theme};

use super::{chart_header_changed_text, chart_header_price_flash_color};

impl TradingTerminal {
    pub(super) fn view_chart_placeholder_header<'a>(
        &'a self,
        chart_id: ChartId,
        instance: &'a ChartInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        button(self.view_chart_symbol_title(instance, theme))
            .on_press(Message::ChartOpenEditor(chart_id))
            .padding([4, 6])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    pub(super) fn view_chart_symbol_button<'a>(
        &'a self,
        chart_id: ChartId,
        instance: &'a ChartInstance,
        last_close: f64,
        price_flash: Option<PriceFlash>,
        now_ms: u64,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let sym_col = column![
            self.view_chart_symbol_title(instance, theme),
            chart_header_price_text(
                last_close,
                price_flash,
                now_ms,
                &instance.chart.display_denomination,
                theme
            ),
        ]
        .spacing(2);

        button(sym_col)
            .on_press(Message::ChartOpenEditor(chart_id))
            .padding([4, 6])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    fn view_chart_symbol_title<'a>(
        &self,
        instance: &'a ChartInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let mut title = row![];
        if let Some(icon) = helpers::symbol_icon(&instance.symbol, 18, theme.palette().text)
            .or_else(|| helpers::symbol_icon(&instance.symbol_display, 18, theme.palette().text))
        {
            title = title.push(icon).push(Space::new().width(6.0));
        }

        title = title.push(
            text(&instance.symbol_display)
                .size(14)
                .color(theme.palette().text),
        );

        if let Some(dex) = helpers::hip3_dex(&instance.symbol) {
            title = title.push(Space::new().width(4.0)).push(
                text(format!("({dex})"))
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        title.align_y(iced::Alignment::Center).into()
    }
}

fn chart_header_price_text(
    last_close: f64,
    price_flash: Option<PriceFlash>,
    now_ms: u64,
    denomination: &crate::denomination::DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let current = denomination.format_chart_price(last_close);
    let base_color = theme.palette().text;
    let Some(flash_color) = chart_header_price_flash_color(price_flash, now_ms, theme) else {
        return chart_header_price_segment(current, base_color).into();
    };
    let Some(flash) = price_flash else {
        return chart_header_price_segment(current, base_color).into();
    };
    let previous = denomination.format_chart_price(flash.previous_close);
    let Some(parts) = chart_header_changed_text(&previous, &current) else {
        return chart_header_price_segment(current, base_color).into();
    };

    let mut row = Row::new().spacing(0).align_y(Alignment::Center);
    if !parts.before.is_empty() {
        row = row.push(chart_header_price_segment(parts.before, base_color));
    }
    row = row.push(chart_header_price_segment(parts.changed, flash_color));
    if !parts.after.is_empty() {
        row = row.push(chart_header_price_segment(parts.after, base_color));
    }

    row.into()
}

fn chart_header_price_segment(content: String, color: Color) -> iced::widget::Text<'static> {
    text(content)
        .size(16)
        .font(iced::Font::MONOSPACE)
        .color(color)
}
