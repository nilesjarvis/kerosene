use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{OrderBookDisplayMode, OrderBookId, OrderBookSymbolMode};
use crate::message::Message;

use iced::widget::{column, container, row, rule, text};
use iced::{Element, Fill, color};
use std::collections::HashSet;

mod controls;
mod depth;
mod settings;

impl TradingTerminal {
    pub(crate) fn view_order_book(&self, id: OrderBookId) -> Element<'_, Message> {
        let Some(inst) = self.order_books.get(&id) else {
            return container(text("Loading Order Book...").size(12).style(
                move |t: &iced::Theme| text::Style {
                    color: Some(t.extended_palette().background.weak.text),
                },
            ))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill)
            .into();
        };
        let theme = self.theme();
        let tracking_symbol = match &inst.mode {
            OrderBookSymbolMode::Active => self.active_symbol.clone(),
            OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
        };
        if !tracking_symbol.is_empty() && self.symbol_key_is_hidden(&tracking_symbol) {
            let content = column![
                text("Order Book").size(13).color(theme.palette().text),
                rule::horizontal(1),
                container(
                    text("Muted ticker")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text)
                )
                .width(Fill)
                .height(Fill)
                .center(Fill),
            ]
            .spacing(8);
            return container(content)
                .width(Fill)
                .height(Fill)
                .padding(10)
                .into();
        }

        let mid = inst.book.mid_price();
        let tick_options = helpers::book_tick_options(mid);
        let tick = Self::resolved_order_book_tick(inst, &tick_options);
        let tick_buttons = Self::view_order_book_tick_buttons(id, &tick_options, tick);
        let header = match inst.display_mode {
            OrderBookDisplayMode::DepthList => Self::view_order_book_header(),
            OrderBookDisplayMode::DomLadder => Self::view_order_book_dom_header(),
        };
        let waiting_for_selected_precision = !inst.can_render_book_at_tick(tick);

        if waiting_for_selected_precision
            || (inst.book.bids.is_empty() && inst.book.asks.is_empty())
        {
            let loading_row: Element<'_, Message> = if let Some(error) = &inst.book_error {
                column![
                    text("Order book unavailable")
                        .size(12)
                        .color(color!(0xff5555)),
                    text(error)
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(4)
                .into()
            } else if inst.book_loading {
                row![
                    self.view_spinner(18),
                    text(if waiting_for_selected_precision {
                        "Loading selected denomination..."
                    } else {
                        "Loading order book..."
                    })
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .into()
            } else {
                text(if waiting_for_selected_precision {
                    "Waiting for selected denomination"
                } else {
                    "No order book data"
                })
                .size(12)
                .color(theme.extended_palette().background.weak.text)
                .into()
            };
            let content =
                column![tick_buttons, header, rule::horizontal(1), loading_row].spacing(4);

            return container(content)
                .width(Fill)
                .height(Fill)
                .padding(10)
                .into();
        }

        let user_order_levels = self.user_order_book_levels(&tracking_symbol, tick);
        let scroll = match inst.display_mode {
            OrderBookDisplayMode::DepthList => {
                Self::view_order_book_rows(id, inst, tick, &theme, &user_order_levels)
            }
            OrderBookDisplayMode::DomLadder => {
                Self::view_order_book_dom_ladder(id, inst, tick, &theme, &user_order_levels)
            }
        };
        let title_row = self.view_order_book_title(id, inst);

        let mut content_col = column![title_row].spacing(4);
        if let Some(error) = &inst.book_error {
            content_col = content_col.push(
                text(format!("{error}; showing last snapshot"))
                    .size(11)
                    .color(color!(0xff5555)),
            );
        }

        if inst.settings_open {
            content_col = content_col.push(self.view_order_book_settings(id, inst));
        }

        content_col = content_col
            .push(tick_buttons)
            .push(header)
            .push(rule::horizontal(1))
            .push(scroll);

        if inst.show_spread_chart {
            content_col = content_col
                .push(rule::horizontal(1))
                .push(Self::view_order_book_spread_chart(id, inst));
        }

        container(content_col)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn user_order_book_levels(&self, symbol: &str, tick: f64) -> UserOrderBookLevels {
        self.account_data
            .as_ref()
            .map(|data| UserOrderBookLevels::from_orders(&data.open_orders, symbol, tick))
            .unwrap_or_default()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct UserOrderBookLevels {
    bids: HashSet<i64>,
    asks: HashSet<i64>,
}

impl UserOrderBookLevels {
    fn from_orders(orders: &[crate::account::OpenOrder], symbol: &str, tick: f64) -> Self {
        if symbol.trim().is_empty() || !helpers::valid_book_tick_size(tick) {
            return Self::default();
        }

        let mut levels = Self::default();
        for order in orders.iter().filter(|order| order.coin == symbol) {
            let Some(is_bid) = order_side_is_bid(&order.side) else {
                continue;
            };
            let Some(price) = parse_order_price(&order.limit_px) else {
                continue;
            };
            let Some(key) = order_price_bucket_key(price, tick, is_bid) else {
                continue;
            };
            if is_bid {
                levels.bids.insert(key);
            } else {
                levels.asks.insert(key);
            }
        }
        levels
    }

    pub(super) fn has_bid_at_price(&self, price: f64, tick: f64) -> bool {
        displayed_price_key(price, tick).is_some_and(|key| self.bids.contains(&key))
    }

    pub(super) fn has_ask_at_price(&self, price: f64, tick: f64) -> bool {
        displayed_price_key(price, tick).is_some_and(|key| self.asks.contains(&key))
    }
}

fn order_side_is_bid(side: &str) -> Option<bool> {
    match side {
        "B" => Some(true),
        "A" => Some(false),
        _ => None,
    }
}

fn parse_order_price(value: &str) -> Option<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|price| price.is_finite() && *price > 0.0)
}

fn order_price_bucket_key(price: f64, tick: f64, is_bid: bool) -> Option<i64> {
    if !helpers::valid_book_tick_size(tick) || !price.is_finite() || price <= 0.0 {
        return None;
    }
    let scaled = price / tick;
    if !scaled.is_finite() {
        return None;
    }
    Some(if is_bid {
        scaled.floor() as i64
    } else {
        scaled.ceil() as i64
    })
}

fn displayed_price_key(price: f64, tick: f64) -> Option<i64> {
    if !helpers::valid_book_tick_size(tick) || !price.is_finite() || price <= 0.0 {
        return None;
    }
    let scaled = price / tick;
    scaled.is_finite().then_some(scaled.round() as i64)
}

#[cfg(test)]
mod tests {
    use super::UserOrderBookLevels;
    use crate::account::OpenOrder;

    fn open_order(coin: &str, side: &str, limit_px: &str, oid: u64) -> OpenOrder {
        OpenOrder {
            coin: coin.to_string(),
            side: side.to_string(),
            limit_px: limit_px.to_string(),
            sz: "1".to_string(),
            oid,
            timestamp: oid,
            reduce_only: None,
        }
    }

    #[test]
    fn user_order_levels_filter_to_symbol_and_side() {
        let orders = vec![
            open_order("BTC", "B", "99.74", 1),
            open_order("BTC", "A", "100.01", 2),
            open_order("ETH", "B", "99.5", 3),
            open_order("BTC", "X", "99.5", 4),
        ];

        let levels = UserOrderBookLevels::from_orders(&orders, "BTC", 0.5);

        assert!(levels.has_bid_at_price(99.5, 0.5));
        assert!(levels.has_ask_at_price(100.5, 0.5));
        assert!(!levels.has_bid_at_price(99.0, 0.5));
        assert!(!levels.has_ask_at_price(100.0, 0.5));
    }

    #[test]
    fn user_order_levels_collapse_multiple_orders_in_same_denomination() {
        let orders = vec![
            open_order("BTC", "B", "99.74", 1),
            open_order("BTC", "B", "99.51", 2),
            open_order("BTC", "A", "100.01", 3),
            open_order("BTC", "A", "100.49", 4),
        ];

        let levels = UserOrderBookLevels::from_orders(&orders, "BTC", 0.5);

        assert_eq!(levels.bids.len(), 1);
        assert_eq!(levels.asks.len(), 1);
        assert!(levels.has_bid_at_price(99.5, 0.5));
        assert!(levels.has_ask_at_price(100.5, 0.5));
    }

    #[test]
    fn user_order_levels_ignore_invalid_inputs() {
        let orders = vec![
            open_order("BTC", "B", "bad", 1),
            open_order("BTC", "A", "NaN", 2),
            open_order("BTC", "B", "0", 3),
        ];

        let invalid_tick = UserOrderBookLevels::from_orders(&orders, "BTC", 0.0);
        let valid_tick = UserOrderBookLevels::from_orders(&orders, "BTC", 0.5);

        assert!(invalid_tick.bids.is_empty());
        assert!(invalid_tick.asks.is_empty());
        assert!(valid_tick.bids.is_empty());
        assert!(valid_tick.asks.is_empty());
    }
}
