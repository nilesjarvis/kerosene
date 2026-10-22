use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::config::OrderBookSymbolModeConfig;
use crate::feed_state::TrackedTradeIntent;
use crate::helpers;
use crate::helpers::design::UiColors;
use crate::message::Message;
use crate::trade_feed_state::{TradeFeedId, TradeFeedInstance};
use iced::widget::{column, container, responsive, rule, scrollable, text};
use iced::{Element, Fill, Length};

mod colors;
mod controls;
mod layout;
mod rows;
mod summary;

impl TradingTerminal {
    pub(crate) fn view_trade_feed(&self, id: TradeFeedId) -> Element<'_, Message> {
        let Some(feed) = self.trade_feeds.instances.get(&id) else {
            return container(text("Loading trade feed…").size(12))
                .center(Fill)
                .into();
        };
        responsive(move |size| self.view_trade_feed_sized(id, feed, size.width))
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
        let colors = UiColors::new(&self.theme());
        let requested = match &feed.mode {
            OrderBookSymbolModeConfig::Active => &self.active_symbol,
            OrderBookSymbolModeConfig::Fixed(key) => key,
        };
        let quote = trade_quote_currency(self.exchange_symbol_for_key(requested));
        let tape = responsive(move |size| {
            // The picker and filters may scroll in short panes. Reserve a header
            // and one complete row rather than subtracting guessed toolbar heights.
            let controls = container(
                scrollable(self.view_trade_feed_controls(id, feed, quote, size.width, colors))
                    .height(Length::Shrink),
            )
            .max_height((size.height - layout::ROW_HEIGHT * 2.0 - 1.0).max(24.0))
            .width(Fill);
            let layout =
                layout::TapeRowLayout::new(size.width, feed.show_context, feed.size_in_notional);
            let body = responsive(move |body| {
                self.view_trade_feed_rows(id, feed, layout, body.height, quote)
            })
            .width(Fill)
            .height(Fill);
            let table = column![
                rows::view_header(id, feed, layout, colors, self.pane_corner_radius),
                divider(colors),
                body,
            ]
            .spacing(0)
            .height(Fill)
            .width(Fill);
            column![controls, table]
                .spacing(0)
                .width(Fill)
                .height(Fill)
                .into()
        })
        .width(Fill)
        .height(Fill);
        column![
            container(tape).width(Fill).height(Fill).clip(true),
            divider(colors),
            self.view_trade_feed_summary(feed, width),
        ]
        .spacing(0)
        .width(Fill)
        .height(Fill)
        .into()
    }
}

fn divider(colors: UiColors) -> Element<'static, Message> {
    rule::horizontal(1)
        .style(move |_| rule::Style {
            color: colors.divider,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
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
