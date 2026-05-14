use crate::app_state::TradingTerminal;
use crate::helpers::{format_price, parse_number};
use crate::market_state::{OrderBookId, OrderBookSymbolMode};
use crate::message::Message;
use crate::signing::OrderKind;
use iced::Task;

mod quantity;

use quantity::{
    order_percentage_for_quantity, quantity_for_percentage, toggled_order_quantity_text,
};

impl TradingTerminal {
    pub(crate) fn handle_order_price_changed(&mut self, value: String) {
        self.order_price = value;
    }

    pub(crate) fn handle_set_mid_price(&mut self) {
        if let Some(mid) = self.resolve_mid_for_symbol(&self.active_symbol) {
            self.order_price = format_price(mid);
        }
    }

    pub(crate) fn handle_order_book_price_selected(
        &mut self,
        id: OrderBookId,
        price: String,
    ) -> Task<Message> {
        let selected_price = price.trim().to_string();
        if !valid_selected_order_book_price(&selected_price) {
            self.order_status = Some(("Invalid order-book price".into(), true));
            return Task::none();
        }

        let Some((target_symbol, book_mid)) = self.order_books.get(&id).and_then(|inst| {
            let target_symbol = match &inst.mode {
                OrderBookSymbolMode::Active => self.active_symbol.clone(),
                OrderBookSymbolMode::Fixed(symbol) => {
                    let symbol = symbol.trim();
                    if symbol.is_empty() {
                        return None;
                    }
                    symbol.to_string()
                }
            };
            Some((target_symbol, positive_finite_price(inst.book.mid_price())))
        }) else {
            self.order_status = Some(("Order book unavailable".into(), true));
            return Task::none();
        };

        let mut task = Task::none();
        if target_symbol != self.active_symbol {
            let previous_symbol = self.active_symbol.clone();
            task = self.switch_active_symbol_internal(target_symbol);
            if self.active_symbol == previous_symbol {
                return task;
            }
        }

        if let Some(mid) = book_mid {
            self.all_mids.insert(self.active_symbol.clone(), mid);
            self.all_mids_updated_at_ms
                .insert(self.active_symbol.clone(), Self::now_ms());
        }

        self.order_kind = OrderKind::Limit;
        self.order_price = selected_price;
        self.persist_config();
        task
    }

    pub(crate) fn handle_order_quantity_changed(&mut self, value: String) {
        self.order_quantity = if self.is_outcome_coin(&self.active_symbol) {
            Self::sanitize_outcome_quantity_input(&value)
        } else {
            value
        };

        let Some(qty) = parse_number(&self.order_quantity) else {
            self.order_percentage = 0.0;
            return;
        };

        let Some(data) = &self.account_data else {
            self.order_percentage = 0.0;
            return;
        };

        let Some(available_margin) = self.visible_available_margin_usdc(data) else {
            self.order_percentage = 0.0;
            return;
        };

        let mut max_leverage = 1.0;
        if let Some((_, lev, _)) =
            data.get_leverage_for(&self.active_symbol, &self.exchange_symbols)
        {
            max_leverage = lev as f64;
        }

        let max_notional = available_margin * max_leverage;
        if max_notional <= 0.0 {
            self.order_percentage = 0.0;
            return;
        }

        self.order_percentage = order_percentage_for_quantity(
            qty,
            self.order_quantity_is_usd,
            self.order_reference_price(),
            max_notional,
        );
    }

    pub(crate) fn handle_toggle_order_denomination(&mut self) {
        if self.is_outcome_coin(&self.active_symbol) {
            self.order_quantity_is_usd = false;
            self.order_quantity = Self::sanitize_outcome_quantity_input(&self.order_quantity);
            self.persist_config();
            return;
        }
        self.order_quantity_is_usd = !self.order_quantity_is_usd;
        self.persist_config();

        let Some(qty) = parse_number(&self.order_quantity) else {
            return;
        };

        let Some(parsed_price) = self.order_reference_price() else {
            return;
        };

        let decimals = self.active_symbol_size_decimals();
        if let Some(quantity) =
            toggled_order_quantity_text(qty, self.order_quantity_is_usd, parsed_price, decimals)
        {
            self.order_quantity = quantity;
        }
    }

    pub(crate) fn handle_order_percentage_changed(&mut self, value: f32) {
        self.order_percentage = value;
        if self.is_outcome_coin(&self.active_symbol) {
            self.order_quantity_is_usd = false;
            self.order_quantity = Self::sanitize_outcome_quantity_input(&self.order_quantity);
            return;
        }

        let Some(data) = &self.account_data else {
            return;
        };

        let Some(available_margin) = self.visible_available_margin_usdc(data) else {
            self.order_quantity = "0".to_string();
            return;
        };

        let mut max_leverage = 1.0;
        if let Some((_, lev, _)) =
            data.get_leverage_for(&self.active_symbol, &self.exchange_symbols)
        {
            max_leverage = lev as f64;
        }

        let max_notional = available_margin * max_leverage;

        self.order_quantity = quantity_for_percentage(
            value,
            max_notional,
            self.order_quantity_is_usd,
            self.order_reference_price(),
            self.active_symbol_size_decimals(),
        );
    }

    pub(crate) fn handle_set_order_kind(&mut self, kind: OrderKind) {
        self.order_kind = kind;
        let active_symbol = self.active_symbol.clone();
        self.refresh_order_price_for_symbol(&active_symbol);
        self.persist_config();
    }

    pub(crate) fn handle_toggle_reduce_only(&mut self) {
        self.order_reduce_only = !self.order_reduce_only;
        self.persist_config();
    }

    pub(crate) fn handle_dismiss_order_status(&mut self) {
        self.order_status = None;
    }

    fn order_reference_price(&self) -> Option<f64> {
        if self.order_kind == OrderKind::Limit || self.order_kind == OrderKind::Chase {
            parse_number(&self.order_price).filter(|price| price.is_finite() && *price > 0.0)
        } else {
            self.resolve_mid_for_symbol(&self.active_symbol)
                .filter(|price| price.is_finite() && *price > 0.0)
        }
    }

    fn active_symbol_size_decimals(&self) -> usize {
        self.exchange_symbols
            .iter()
            .find(|s| s.key == self.active_symbol)
            .map(|s| s.sz_decimals)
            .unwrap_or(4) as usize
    }
}

fn valid_selected_order_book_price(price: &str) -> bool {
    parse_number(price)
        .and_then(positive_finite_price)
        .is_some()
}

fn positive_finite_price(price: f64) -> Option<f64> {
    (price.is_finite() && price > 0.0).then_some(price)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{BookLevel, ExchangeSymbol, MarketType, OrderBook};
    use crate::market_state::OrderBookInstance;
    use crate::order_execution::PendingOrderAction;

    fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 5,
            max_leverage: 50,
            only_isolated: false,
            market_type,
            outcome: None,
        }
    }

    fn book(best_bid: f64, best_ask: f64) -> OrderBook {
        OrderBook {
            bids: vec![BookLevel {
                px: best_bid,
                sz: 1.0,
            }],
            asks: vec![BookLevel {
                px: best_ask,
                sz: 1.0,
            }],
        }
    }

    fn terminal_with_order_book(mode: OrderBookSymbolMode) -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.order_books.clear();
        terminal.active_symbol = "BTC".to_string();
        terminal.active_symbol_display = "BTC".to_string();
        terminal.order_kind = OrderKind::Market;
        terminal.order_price.clear();
        terminal
            .order_books
            .insert(7, OrderBookInstance::new(7, mode, 1.0));
        terminal
    }

    #[test]
    fn order_book_price_selected_sets_limit_price_for_active_symbol_book() {
        let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Active);

        let _task = terminal.handle_order_book_price_selected(7, "101.25".to_string());

        assert_eq!(terminal.active_symbol, "BTC");
        assert_eq!(terminal.order_kind, OrderKind::Limit);
        assert_eq!(terminal.order_price, "101.25");
    }

    #[test]
    fn order_book_price_selected_switches_to_fixed_symbol_before_setting_price() {
        let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Fixed("ETH".to_string()));

        let _task = terminal.handle_order_book_price_selected(7, "2500.5".to_string());

        assert_eq!(terminal.active_symbol, "ETH");
        assert_eq!(terminal.active_symbol_display, "ETH");
        assert_eq!(terminal.order_kind, OrderKind::Limit);
        assert_eq!(terminal.order_price, "2500.5");
    }

    #[test]
    fn order_book_price_selected_rejects_missing_book_without_mutating_form() {
        let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Active);

        let _task = terminal.handle_order_book_price_selected(99, "101.25".to_string());

        assert_eq!(terminal.active_symbol, "BTC");
        assert_eq!(terminal.order_kind, OrderKind::Market);
        assert!(terminal.order_price.is_empty());
        assert_eq!(
            terminal.order_status,
            Some(("Order book unavailable".to_string(), true))
        );
    }

    #[test]
    fn order_book_price_selected_rejects_invalid_price_without_mutating_form() {
        let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Active);

        let _task = terminal.handle_order_book_price_selected(7, "0".to_string());

        assert_eq!(terminal.active_symbol, "BTC");
        assert_eq!(terminal.order_kind, OrderKind::Market);
        assert!(terminal.order_price.is_empty());
        assert_eq!(
            terminal.order_status,
            Some(("Invalid order-book price".to_string(), true))
        );
    }

    #[test]
    fn order_book_price_selected_seeds_mid_for_immediate_limit_submission() {
        let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Active);
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.connected_address = Some("0xabc".to_string());
        terminal.wallet_key_input = "agent-key".to_string().into();
        terminal.order_quantity = "1".to_string();
        terminal.order_quantity_is_usd = false;
        terminal
            .order_books
            .get_mut(&7)
            .expect("test order book")
            .set_book(book(99.0, 101.0));

        let _task = terminal.handle_order_book_price_selected(7, "100".to_string());
        let _task = terminal.execute_order(true);

        assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
        assert_eq!(
            terminal.order_status,
            Some(("Placing order...".to_string(), false))
        );
    }

    #[test]
    fn order_book_price_selected_allows_immediate_usd_limit_submission() {
        let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Active);
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.connected_address = Some("0xabc".to_string());
        terminal.wallet_key_input = "agent-key".to_string().into();
        terminal.order_quantity = "100".to_string();
        terminal.order_quantity_is_usd = true;
        terminal
            .order_books
            .get_mut(&7)
            .expect("test order book")
            .set_book(book(99.0, 101.0));

        let _task = terminal.handle_order_book_price_selected(7, "100".to_string());
        let _task = terminal.execute_order(true);

        assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
        assert_eq!(
            terminal.order_status,
            Some(("Placing order...".to_string(), false))
        );
    }
}
