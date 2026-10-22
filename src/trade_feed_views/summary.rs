use super::controls::tip;
use super::{trade_notional_label, trade_quote_currency};
use crate::app_state::TradingTerminal;
use crate::config::OrderBookSymbolModeConfig;
use crate::feed_views::FEED_SUMMARY_TIMEFRAMES;
use crate::helpers::design::UiColors;
use crate::message::Message;
use crate::trade_feed_state::TradeFeedInstance;
use iced::widget::{Column, Space, column, container, row, text};
use iced::{Color, Element, Fill, Length};

impl TradingTerminal {
    pub(super) fn view_trade_feed_summary(
        &self,
        feed: &TradeFeedInstance,
        width: f32,
    ) -> Element<'static, Message> {
        let requested = match &feed.mode {
            OrderBookSymbolModeConfig::Active => &self.active_symbol,
            OrderBookSymbolModeConfig::Fixed(key) => key,
        };
        let quote = trade_quote_currency(self.exchange_symbol_for_key(requested));
        let colors = UiColors::new(&self.theme());
        let columns = if width >= 600.0 { 4 } else { 2 };
        let mut summary = Column::new().spacing(8).width(Fill);
        for timeframes in FEED_SUMMARY_TIMEFRAMES.chunks(columns) {
            let mut cells = row![].spacing(12);
            for &(minutes, label) in timeframes {
                let (buy, sell) = feed.calculate_summary(minutes, self.status_bar_now_ms);
                let total = buy + sell;
                let has_data = total > 0.0;
                let amount = |value, color| {
                    container(
                        text(trade_notional_label(value, quote))
                            .size(11)
                            .font(crate::app_fonts::monospace_font())
                            .color(color)
                            .wrapping(text::Wrapping::None),
                    )
                    .align_right(Fill)
                    .clip(true)
                };
                let detail = format!(
                    "{label} · Buy: {} · Sell: {}. Received trades only; includes the current minute.",
                    trade_notional_label(buy, quote),
                    trade_notional_label(sell, quote)
                );
                let sides = row![
                    text("B").size(11).color(if has_data {
                        colors.positive
                    } else {
                        colors.muted
                    }),
                    amount(
                        buy,
                        if has_data {
                            colors.positive
                        } else {
                            colors.muted
                        }
                    ),
                    text("S").size(11).color(if has_data {
                        colors.negative
                    } else {
                        colors.muted
                    }),
                    amount(
                        sell,
                        if has_data {
                            colors.negative
                        } else {
                            colors.muted
                        }
                    ),
                ]
                .spacing(4)
                .align_y(iced::Alignment::Center);
                let cell = column![
                    row![
                        text(label).size(11).color(colors.muted),
                        amount(total, if has_data { colors.text } else { colors.muted })
                    ]
                    .spacing(4),
                    sides,
                    ratio_bar(buy, sell, colors),
                ]
                .spacing(4)
                .width(Fill);
                cells = cells.push(tip(cell.into(), detail, colors, self.pane_corner_radius));
            }
            summary = summary.push(cells);
        }
        container(summary).padding(8).width(Fill).into()
    }
}

fn ratio_bar(buy: f64, sell: f64, colors: UiColors) -> Element<'static, Message> {
    let total = buy + sell;
    let bar = |portion: u16, color: Color| {
        container(Space::new())
            .width(Length::FillPortion(portion))
            .height(4)
            .style(move |_| container::Style {
                background: Some(color.into()),
                ..Default::default()
            })
    };
    if total <= 0.0 {
        return bar(1, colors.divider).into();
    }
    let buy_portion = (buy / total * 1000.0).round() as u16;
    let mut bars = row![].height(4).width(Fill);
    if buy > 0.0 {
        bars = bars.push(bar(buy_portion.max(1), colors.positive));
    }
    if sell > 0.0 {
        bars = bars.push(bar((1000 - buy_portion).max(1), colors.negative));
    }
    bars.into()
}

#[cfg(test)]
mod tests;
