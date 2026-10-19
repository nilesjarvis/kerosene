use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::config::OrderBookSymbolModeConfig;
use crate::feed_state::TrackedTradeIntent;
use crate::helpers;
use crate::market_state::market_browser::{MarketPicker, MarketVenue};
use crate::message::Message;
use crate::trade_feed_state::{
    TradeFeedId, TradeFeedInstance, TradeFeedSource, TradePrint, parse_trade_feed_minimum,
    trade_feed_symbol_rank, trade_shading,
};
use crate::wallet_views::{WalletAddressActionCell, wallet_address_action_cell};
use iced::widget::{
    Column, button, column, container, responsive, row, rule, text, text_input, tooltip,
};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

mod colors;
mod summary;

use colors::TraderColors;

const TIME_WIDTH: f32 = 112.0;
const TRADER_WIDTH: f32 = 112.0;
const GROUP_COUNT_WIDTH: f32 = 32.0;
const CONTEXT_WIDTH: f32 = 44.0;

#[derive(Clone, Copy)]
struct TapeRowLayout {
    show_time: bool,
    show_price: bool,
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
            self.view_trade_feed_sized(id, feed, size.width)
        }))
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn view_trade_feed_sized<'a>(
        &'a self,
        id: TradeFeedId,
        feed: &'a TradeFeedInstance,
        width: f32,
    ) -> Element<'a, Message> {
        let tape = container(responsive(move |size| {
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
                tooltip(
                    button(text("Group trades").size(11))
                        .on_press(Message::TradeFeedToggleGrouping(id))
                        .style(if feed.group_trades { button::primary } else { button::text }),
                    text("Same trader and side within 1 second; total size and weighted average price").size(11),
                    tooltip::Position::Bottom,
                ),
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
                content = content.push(self.view_market_venue_switch(MarketPicker::TradeFeed(id)));
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
                let results: Element<'_, Message> = if self.market_browser.venue(MarketPicker::TradeFeed(id)) == MarketVenue::Papertrade {
                    self.view_papertrade_markets(&feed.search)
                } else {
                    results.into()
                };
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
            let provider = match feed.source {
                TradeFeedSource::Hydromancer { .. } => "Hydromancer",
                TradeFeedSource::Hyperliquid => "Hyperliquid",
            };
            let context_hint = if self.hydromancer_api_key.trim().is_empty() {
                "Trade context requires a Hydromancer key in Settings > Integrations. Using Hyperliquid."
            } else if feed.failed_context_generation.is_some() {
                "Hydromancer context unavailable. Using Hyperliquid; Retry to reconnect."
            } else {
                "Show Open / Add / Reduce / Close / Flip using Hydromancer. Off uses Hyperliquid."
            };
            let status_label = if feed.show_context && matches!(feed.source, TradeFeedSource::Hyperliquid) {
                format!("{status} · {provider} · no context")
            } else {
                format!("{status} · {provider}")
            };
            let mut controls = row![
                text(status_label).size(11).color(muted).wrapping(text::Wrapping::None).width(Fill),
                tooltip(
                    button(text("Context").size(11))
                        .padding([2, 4])
                        .on_press(Message::TradeFeedToggleContext(id))
                        .style(if feed.show_context { button::primary } else { button::text }),
                    container(text(context_hint).size(11))
                        .padding([5, 8]).max_width(280).style(container::rounded_box),
                    tooltip::Position::Bottom,
                ),
            ].spacing(4).align_y(Alignment::Center);
            if feed.show_context && feed.failed_context_generation.is_some()
                && !self.hydromancer_api_key.trim().is_empty()
            {
                controls = controls.push(button(text("Retry").size(11)).padding([2, 4])
                    .style(button::text).on_press(Message::TradeFeedRetryContext(id)));
            }
            content = content.push(controls);
            let quote = trade_quote_currency(self.exchange_symbol_for_key(requested));
            if feed.group_trades {
                content = content.push(view_trade_feed_minimum(id, feed, quote));
            }
            let context_width = if feed.show_context { CONTEXT_WIDTH } else { 0.0 };
            let layout = TapeRowLayout {
                show_time: size.width >= context_width + if feed.group_trades { 432.0 } else { 400.0 },
                show_price: !feed.show_context
                    || size.width >= if feed.group_trades { 388.0 } else { 356.0 },
                show_value: !feed.size_in_notional
                    && size.width >= context_width + if feed.group_trades { 572.0 } else { 540.0 },
            };
            let size_header = tooltip(
                view_trade_size_cell(id, if feed.size_in_notional { "Notional" } else { "Size" }, muted),
                text(if feed.size_in_notional { "Show asset quantity" } else { "Show notional value" }).size(11),
                tooltip::Position::Bottom,
            );
            let mut trader_header = row![text("Trader").size(12).color(muted)
                .width(TRADER_WIDTH + if feed.group_trades { GROUP_COUNT_WIDTH } else { 0.0 })];
            if feed.show_context {
                trader_header = trader_header.push(text("Action").size(11).color(muted).width(CONTEXT_WIDTH));
            }
            let header = tape_row(
                "Time",
                if feed.group_trades { "Avg price" } else { "Price" },
                size_header.into(),
                "Value",
                trader_header.into(),
                layout,
                muted,
            );
            content = content
                .push(container(header).padding([0, 4]))
                .push(rule::horizontal(1));
            let available = size.height - if feed.picker_open { 291.0 } else { 90.0 }
                - if feed.group_trades { 28.0 } else { 0.0 };
            let row_count = ((available.max(0.0) / 26.0) as usize).min(100);
            let rows = feed.visible_rows();
            if rows.is_empty() && feed.group_trades && feed.min_group_notional > 0.0 {
                content = content.push(text("No groups meet the minimum").size(12).color(muted));
            }
            let largest = rows.iter().map(|row| row.value).fold(0.0, f64::max);
            for item in rows.iter().take(row_count) {
                let trade = item.latest;
                let time = trade_age_label(trade.time_ms, self.status_bar_now_ms);
                let value = trade_value_label(item.value);
                let size_label = if feed.size_in_notional {
                    trade_notional_label(item.value, quote)
                } else {
                    item.size_label().into_owned()
                };
                let accent = if trade.is_buy {
                    theme.palette().success
                } else {
                    theme.palette().danger
                };
                let shading = trade_shading(item.value, largest);
                let mut trader = row![self.view_trade_feed_trader(id, feed, trade, &theme, &trader_colors)]
                    .align_y(Alignment::Center);
                if feed.group_trades {
                    trader = trader.push(text(format!("×{}", item.count)).size(11).color(muted).width(GROUP_COUNT_WIDTH));
                }
                if feed.show_context {
                    trader = trader.push(text(trade_intent_label(trade.intent)).size(11)
                        .color(muted).width(CONTEXT_WIDTH).wrapping(text::Wrapping::None));
                }
                content = content.push(
                    container(tape_row(
                        &time,
                        &item.price_label(),
                        view_trade_size_cell(id, &size_label, theme.palette().text),
                        &value,
                        trader.into(),
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
        .clip(true);

        column![
            tape,
            helpers::section_separator(),
            self.view_trade_feed_summary(feed, width),
        ]
        .spacing(0)
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

fn view_trade_feed_minimum(
    id: TradeFeedId,
    feed: &TradeFeedInstance,
    quote: &str,
) -> Element<'static, Message> {
    let invalid = parse_trade_feed_minimum(&feed.min_group_notional_input).is_none();
    let unit = if trade_uses_dollar_quote(quote) {
        "$"
    } else {
        quote
    };
    let hint = if invalid {
        format!(
            "Invalid amount. Current minimum: {}. Use 10000, 10,000, or 10k.",
            trade_notional_label(feed.min_group_notional, quote)
        )
    } else {
        "Minimum combined notional per group. Use 10000, 10,000, or 10k. Blank or 0 shows all."
            .into()
    };
    let input = text_input("0 (all)", &feed.min_group_notional_input)
        .size(11)
        .padding([2, 4])
        .width(110)
        .on_input(move |value| Message::TradeFeedMinimumChanged(id, value))
        .style(move |theme: &Theme, status| {
            let mut style = text_input::default(theme, status);
            if invalid {
                style.border.color = theme.palette().danger;
            }
            style
        });
    let mut controls = row![
        text(format!("Min {unit}")).size(11),
        tooltip(
            input,
            container(text(hint).size(11))
                .padding([5, 8])
                .max_width(280)
                .style(container::rounded_box),
            tooltip::Position::Bottom
        ),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if !feed.min_group_notional_input.is_empty() {
        controls = controls.push(
            button(text("Clear").size(11))
                .padding([2, 4])
                .style(button::text)
                .on_press(Message::TradeFeedMinimumChanged(id, String::new())),
        );
    }
    if invalid {
        controls = controls.push(text("Invalid amount").size(11).style(text::danger));
    }
    container(controls).height(24).width(Fill).into()
}

fn tape_row(
    time: &str,
    price: &str,
    size: Element<'static, Message>,
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
        // Anchor the suffix while the counter gains digits (for example, 9 → 10).
        cells = cells.push(
            text(time.to_owned())
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .wrapping(text::Wrapping::None)
                .color(color)
                .width(TIME_WIDTH)
                .align_x(Alignment::End),
        );
    }
    cells = cells.push(trader);
    if layout.show_price {
        cells = cells.push(cell(price, 3, true));
    }
    cells = cells.push(size);
    if layout.show_value {
        cells = cells.push(cell(value, 3, true));
    }
    cells.align_y(Alignment::Center).into()
}

fn view_trade_size_cell(id: TradeFeedId, label: &str, color: Color) -> Element<'static, Message> {
    button(
        text(label.to_owned())
            .size(12)
            .wrapping(text::Wrapping::None)
            .color(color)
            .width(Fill)
            .align_x(Alignment::End),
    )
    .padding(0)
    .width(Length::FillPortion(3))
    .on_press(Message::TradeFeedToggleSizeUnit(id))
    .style(button::text)
    .into()
}

fn trade_quote_currency(symbol: Option<&ExchangeSymbol>) -> &str {
    symbol
        .and_then(|symbol| match symbol.market_type {
            MarketType::Perp => Some("USD"),
            MarketType::Spot => symbol
                .display_name
                .as_deref()
                .and_then(|display| display.rsplit_once('/'))
                .map(|(_, quote)| quote),
            MarketType::Outcome => symbol
                .outcome
                .as_ref()
                .map(|info| info.quote_symbol.as_str()),
        })
        .filter(|quote| !quote.is_empty())
        .unwrap_or("quote")
}

fn trade_intent_label(intent: TrackedTradeIntent) -> &'static str {
    match intent {
        TrackedTradeIntent::Opening => "Open",
        TrackedTradeIntent::Increasing => "Add",
        TrackedTradeIntent::Reducing => "Reduce",
        TrackedTradeIntent::Closing => "Close",
        TrackedTradeIntent::Reversing => "Flip",
        TrackedTradeIntent::Unknown => "—",
    }
}

fn trade_notional_label(value: f64, quote: &str) -> String {
    let amount = trade_value_label(value);
    if trade_uses_dollar_quote(quote) {
        format!("${amount}")
    } else {
        format!("{amount} {quote}")
    }
}

fn trade_uses_dollar_quote(quote: &str) -> bool {
    matches!(quote, "USD" | "USDC" | "USDE" | "USDT0" | "USDH")
}

fn trade_value_label(value: f64) -> String {
    if value >= 1_000.0 {
        crate::denomination::format_compact(value)
    } else {
        helpers::format_size(value)
    }
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
    use super::*;

    #[test]
    fn trade_feed_notional_keeps_the_quote_currency_and_formats_dollars_compactly() {
        for quote in ["USD", "USDC", "USDE", "USDT0", "USDH"] {
            assert_eq!(trade_notional_label(1_000.0, quote), "$1.0K");
            assert_eq!(trade_notional_label(12_500.0, quote), "$12.5K");
            assert_eq!(trade_notional_label(1_250_000.0, quote), "$1.25M");
            assert_eq!(trade_notional_label(2_500_000_000.0, quote), "$2.50B");
            assert_eq!(trade_notional_label(0.125, quote), "$0.1250");
        }
        assert_eq!(trade_notional_label(12_500.0, "UBTC"), "12.5K UBTC");
        assert_eq!(trade_notional_label(0.125, "UBTC"), "0.1250 UBTC");
        assert_eq!(trade_quote_currency(None), "quote");
        assert_eq!(
            trade_notional_label(12.5, trade_quote_currency(None)),
            "12.50 quote"
        );

        let mut symbol = ExchangeSymbol {
            key: "BTC".into(),
            ticker: "BTC".into(),
            category: "crypto".into(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 5,
            max_leverage: 40,
            only_isolated: false,
            growth_mode: false,
            market_type: MarketType::Perp,
            outcome: None,
        };
        assert_eq!(trade_quote_currency(Some(&symbol)), "USD");
        symbol.market_type = MarketType::Spot;
        assert_eq!(trade_quote_currency(Some(&symbol)), "quote");
        symbol.display_name = Some("UETH/UBTC".into());
        assert_eq!(trade_quote_currency(Some(&symbol)), "UBTC");
        symbol.display_name = Some("HYPE/USDC".into());
        assert_eq!(trade_quote_currency(Some(&symbol)), "USDC");
        symbol.market_type = MarketType::Outcome;
        assert_eq!(trade_quote_currency(Some(&symbol)), "quote");
    }

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
