use crate::app_state::TradingTerminal;
use crate::config::OrderBookSymbolModeConfig;
use crate::helpers;
use crate::message::Message;
use crate::trade_feed_state::{
    TradeFeedId, TradeFeedInstance, TradePrint, trade_feed_symbol_rank, trade_shading,
};
use crate::wallet_views::{WalletAddressActionCell, wallet_address_action_cell};
use iced::widget::{Column, button, column, container, responsive, row, rule, text, text_input};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

mod colors;

use colors::TraderColors;

const TRADER_WIDTH: f32 = 112.0;

#[derive(Clone, Copy)]
struct TapeRowLayout {
    show_time: bool,
    show_value: bool,
}

impl TradingTerminal {
    pub(crate) fn view_trade_feed(&self, id: TradeFeedId) -> Element<'_, Message> {
        let Some(feed) = self.trade_feeds.instances.get(&id) else {
            return container(text("Loading Trade Feed...").size(12))
                .center(Fill)
                .into();
        };
        container(responsive(move |size| {
            let requested = match &feed.mode {
                OrderBookSymbolModeConfig::Active => &self.active_symbol,
                OrderBookSymbolModeConfig::Fixed(key) => key,
            };
            let label = if requested.is_empty() {
                "Select asset".to_string()
            } else {
                self.display_name_for_symbol(requested)
            };
            let follows = matches!(feed.mode, OrderBookSymbolModeConfig::Active);
            let theme = self.theme();
            let trader_colors = TraderColors::new(&theme);
            let muted = theme.extended_palette().background.weak.text;
            let title = row![
                button(text(label).size(13).wrapping(text::Wrapping::None))
                    .width(Fill)
                    .on_press(Message::TradeFeedTogglePicker(id))
                    .style(button::text),
                button(text("Follow active").size(11))
                    .on_press(Message::TradeFeedSetMode(
                        id,
                        OrderBookSymbolModeConfig::Active
                    ))
                    .style(if follows {
                        button::primary
                    } else {
                        button::text
                    }),
            ]
            .spacing(4)
            .align_y(Alignment::Center);
            let mut content = column![title].spacing(4);
            if feed.picker_open {
                content = content.push(
                    text_input("Search asset...", &feed.search)
                        .size(12)
                        .padding(5)
                        .on_input(move |query| Message::TradeFeedSearchChanged(id, query)),
                );
                let query = feed.search.to_lowercase();
                let mut results = Column::new().spacing(2);
                let mut matches: Vec<_> = self
                    .exchange_symbols
                    .iter()
                    .filter(|symbol| {
                        symbol.is_user_selectable_market()
                            && !self.exchange_symbol_is_hidden(symbol)
                    })
                    .filter_map(|symbol| {
                        let display = Self::exchange_symbol_display_name(symbol);
                        trade_feed_symbol_rank(&query, &symbol.key, &display)
                            .map(|rank| (rank, symbol, display))
                    })
                    .collect();
                matches.sort_by(|a, b| (a.0, &a.2, &a.1.key).cmp(&(b.0, &b.2, &b.1.key)));
                for (_, symbol, display) in matches.into_iter().take(5) {
                    results = results.push(
                        button(text(display).size(12).wrapping(text::Wrapping::None))
                            .height(24)
                            .width(Fill)
                            .on_press(Message::TradeFeedSetMode(
                                id,
                                OrderBookSymbolModeConfig::Fixed(symbol.key.clone()),
                            ))
                            .style(button::text),
                    );
                }
                content = content.push(container(results).height(140).clip(true));
            }
            let status = if feed.symbol.is_none() {
                "Asset unavailable"
            } else {
                feed.status
            };
            let status = if feed.has_gap {
                format!("{status} · gap in history")
            } else {
                status.to_string()
            };
            let status = if feed.is_held() {
                format!("{status} · rows held")
            } else {
                status
            };
            content = content.push(text(status).size(11).color(muted));
            let layout = TapeRowLayout {
                show_time: size.width >= 400.0,
                show_value: size.width >= 540.0,
            };
            let header = tape_row(
                "Time",
                "Price",
                "Size",
                "Value",
                text("Trader")
                    .size(12)
                    .color(muted)
                    .width(TRADER_WIDTH)
                    .into(),
                layout,
                muted,
            );
            content = content
                .push(container(header).padding([0, 4]))
                .push(rule::horizontal(1));
            let available = size.height - if feed.picker_open { 255.0 } else { 80.0 };
            let row_count = ((available.max(0.0) / 26.0) as usize).min(100);
            let trades = feed.visible_trades();
            let largest = trades.iter().map(|trade| trade.value).fold(0.0, f64::max);
            for trade in trades.iter().take(row_count) {
                let time = trade_age_label(trade.time_ms, self.status_bar_now_ms);
                let value = helpers::format_size(trade.value);
                let accent = if trade.is_buy {
                    theme.palette().success
                } else {
                    theme.palette().danger
                };
                let shading = trade_shading(trade.value, largest);
                content = content.push(
                    container(tape_row(
                        &time,
                        &trade.price,
                        &trade.size,
                        &value,
                        self.view_trade_feed_trader(id, feed, trade, &theme, &trader_colors),
                        layout,
                        theme.palette().text,
                    ))
                    .height(22)
                    .padding([1, 4])
                    .style(move |_theme: &Theme| container::Style {
                        background: Some(
                            Color {
                                a: shading,
                                ..accent
                            }
                            .into(),
                        ),
                        border: iced::Border {
                            color: accent,
                            width: 0.0,
                            radius: 2.0.into(),
                        },
                        ..Default::default()
                    }),
                );
            }
            content.width(Fill).into()
        }))
        .padding(8)
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn view_trade_feed_trader(
        &self,
        id: TradeFeedId,
        feed: &TradeFeedInstance,
        trade: &TradePrint,
        theme: &Theme,
        colors: &TraderColors,
    ) -> Element<'static, Message> {
        let Some(address) = &trade.trader else {
            return text("—")
                .size(12)
                .color(theme.extended_palette().background.weak.text)
                .width(TRADER_WIDTH)
                .into();
        };
        let display = self.wallet_display(address.as_str());
        let tooltip_label = if display.has_label {
            format!("{} ({})", display.primary, address.as_str())
        } else {
            format!("Copy {}", address.as_str())
        };
        wallet_address_action_cell(WalletAddressActionCell {
            address: address.as_str().to_owned(),
            label: display.primary,
            tooltip_label,
            hover_key: feed.address_hover_key(id, trade),
            hovered_key: self.hovered_wallet_address_actions.as_deref(),
            width: TRADER_WIDTH,
            text_size: 12,
            text_color: colors.for_address(address.as_str()),
        })
    }
}

fn tape_row(
    time: &str,
    price: &str,
    size: &str,
    value: &str,
    trader: Element<'static, Message>,
    layout: TapeRowLayout,
    color: Color,
) -> Element<'static, Message> {
    let cell = |value: &str, portions, right| {
        let text = text(value.to_owned())
            .size(12)
            .wrapping(text::Wrapping::None)
            .color(color)
            .width(Length::FillPortion(portions));
        if right {
            text.align_x(Alignment::End)
        } else {
            text
        }
    };
    let mut cells = row![].spacing(8);
    if layout.show_time {
        cells = cells.push(cell(time, 4, false));
    }
    cells = cells
        .push(trader)
        .push(cell(price, 3, true))
        .push(cell(size, 3, true));
    if layout.show_value {
        cells = cells.push(cell(value, 3, true));
    }
    cells.align_y(Alignment::Center).into()
}

fn trade_age_label(time_ms: u64, now_ms: u64) -> String {
    let seconds = now_ms.saturating_sub(time_ms) / 1_000;
    let (count, unit) = match seconds {
        0 => return "just now".into(),
        1..60 => (seconds, "second"),
        60..3_600 => (seconds / 60, "minute"),
        3_600..86_400 => (seconds / 3_600, "hour"),
        _ => (seconds / 86_400, "day"),
    };
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {unit}{plural} ago")
}

#[cfg(test)]
mod tests {
    use super::trade_age_label;

    #[test]
    fn trade_feed_age_uses_whole_units_and_handles_clock_skew() {
        let time_ms = 10_000;
        for (age_ms, expected) in [
            (0, "just now"),
            (999, "just now"),
            (1_000, "1 second ago"),
            (10_000, "10 seconds ago"),
            (59_999, "59 seconds ago"),
            (60_000, "1 minute ago"),
            (119_999, "1 minute ago"),
            (180_000, "3 minutes ago"),
            (3_599_999, "59 minutes ago"),
            (3_600_000, "1 hour ago"),
            (7_200_000, "2 hours ago"),
            (86_400_000, "1 day ago"),
            (172_800_000, "2 days ago"),
        ] {
            assert_eq!(trade_age_label(time_ms, time_ms + age_ms), expected);
        }
        assert_eq!(trade_age_label(time_ms, time_ms - 1_000), "just now");
    }
}
