use crate::account::{AccountData, ClearinghouseState};
use crate::api::USDH_TOKEN_INDEX;
use crate::app_state::TradingTerminal;
use crate::helpers::{format_decimal_with_commas, format_price, parse_number};
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

        self.refresh_order_percentage_for_current_quantity();
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
            self.update_order_quantity_for_percentage(value);
            return;
        }

        self.update_order_quantity_for_percentage(value);
    }

    pub(crate) fn handle_set_order_kind(&mut self, kind: OrderKind) {
        self.order_kind = kind;
        let active_symbol = self.active_symbol.clone();
        self.refresh_order_price_for_symbol(&active_symbol);
        self.persist_config();
    }

    pub(crate) fn handle_toggle_reduce_only(&mut self) {
        self.order_reduce_only = !self.order_reduce_only;
        if self.order_percentage > 0.0 {
            self.update_order_quantity_for_percentage(self.order_percentage);
        } else {
            self.refresh_order_percentage_for_current_quantity();
        }
        self.persist_config();
    }

    pub(crate) fn handle_dismiss_order_status(&mut self) {
        self.order_status = None;
    }

    fn order_reference_price(&self) -> Option<f64> {
        if matches!(self.order_kind, OrderKind::Limit | OrderKind::LimitIoc) {
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

    fn refresh_order_percentage_for_current_quantity(&mut self) {
        let Some(qty) = parse_number(&self.order_quantity) else {
            self.order_percentage = 0.0;
            return;
        };

        let Some(data) = &self.account_data else {
            self.order_percentage = 0.0;
            return;
        };

        let Some(sizing_basis) = self.order_sizing_basis(data) else {
            self.order_percentage = 0.0;
            return;
        };

        self.order_percentage = sizing_basis.percentage_for_quantity(
            qty,
            self.order_quantity_is_usd,
            self.order_reference_price(),
        );
    }

    fn update_order_quantity_for_percentage(&mut self, percentage: f32) {
        let Some(data) = &self.account_data else {
            return;
        };

        let Some(sizing_basis) = self.order_sizing_basis(data) else {
            self.order_quantity = "0".to_string();
            return;
        };

        self.order_quantity = sizing_basis.quantity_for_percentage(
            percentage,
            self.order_quantity_is_usd,
            self.order_reference_price(),
            self.active_symbol_size_decimals(),
        );
    }

    fn order_sizing_basis(&self, data: &AccountData) -> Option<OrderSizingBasis> {
        if self.reduce_only_position_sizing_enabled() {
            return position_size_for_symbol(
                self.visible_clearinghouse_state(data),
                &self.active_symbol,
            )
            .map(|position_size| OrderSizingBasis::ReduceOnlyPosition {
                position_size_coin: position_size,
            });
        }

        let available_margin = if self.is_outcome_coin(&self.active_symbol) {
            data.available_margin_for_token(USDH_TOKEN_INDEX)?
        } else {
            self.visible_available_margin_usdc(data)?
        };
        if !available_margin.is_finite() || available_margin <= 0.0 {
            return None;
        }

        let max_leverage = data
            .get_leverage_for(&self.active_symbol, &self.exchange_symbols)
            .map(|(_, leverage, _)| leverage as f64)
            .unwrap_or(1.0);
        let max_notional = available_margin * max_leverage;
        (max_notional.is_finite() && max_notional > 0.0)
            .then_some(OrderSizingBasis::MarginNotional { max_notional })
    }

    fn reduce_only_position_sizing_enabled(&self) -> bool {
        self.order_reduce_only
            && !self.is_spot_coin(&self.active_symbol)
            && !self.is_outcome_coin(&self.active_symbol)
    }
}

#[derive(Debug, Clone, Copy)]
enum OrderSizingBasis {
    MarginNotional { max_notional: f64 },
    ReduceOnlyPosition { position_size_coin: f64 },
}

impl OrderSizingBasis {
    fn percentage_for_quantity(
        self,
        quantity: f64,
        quantity_is_usd: bool,
        reference_price: Option<f64>,
    ) -> f32 {
        match self {
            Self::MarginNotional { max_notional } => order_percentage_for_quantity(
                quantity,
                quantity_is_usd,
                reference_price,
                max_notional,
            ),
            Self::ReduceOnlyPosition { position_size_coin } => percentage_for_position_quantity(
                quantity,
                position_size_coin,
                quantity_is_usd,
                reference_price,
            ),
        }
    }

    fn quantity_for_percentage(
        self,
        percentage: f32,
        quantity_is_usd: bool,
        reference_price: Option<f64>,
        decimals: usize,
    ) -> String {
        match self {
            Self::MarginNotional { max_notional } => quantity_for_percentage(
                percentage,
                max_notional,
                quantity_is_usd,
                reference_price,
                decimals,
            ),
            Self::ReduceOnlyPosition { position_size_coin } => position_quantity_for_percentage(
                percentage,
                position_size_coin,
                quantity_is_usd,
                reference_price,
                decimals,
            ),
        }
    }
}

fn percentage_for_position_quantity(
    quantity: f64,
    position_size_coin: f64,
    quantity_is_usd: bool,
    reference_price: Option<f64>,
) -> f32 {
    if !quantity.is_finite()
        || quantity <= 0.0
        || !position_size_coin.is_finite()
        || position_size_coin <= 0.0
    {
        return 0.0;
    }

    let max_quantity = if quantity_is_usd {
        let Some(reference_price) =
            reference_price.filter(|price| price.is_finite() && *price > 0.0)
        else {
            return 0.0;
        };
        position_size_coin * reference_price
    } else {
        position_size_coin
    };

    if !max_quantity.is_finite() || max_quantity <= 0.0 {
        return 0.0;
    }

    (((quantity / max_quantity) * 100.0) as f32).clamp(0.0, 100.0)
}

fn position_quantity_for_percentage(
    percentage: f32,
    position_size_coin: f64,
    quantity_is_usd: bool,
    reference_price: Option<f64>,
    decimals: usize,
) -> String {
    if !percentage.is_finite() || !position_size_coin.is_finite() || position_size_coin <= 0.0 {
        return "0".to_string();
    }

    let target_coin = position_size_coin * (percentage.clamp(0.0, 100.0) as f64 / 100.0);
    if quantity_is_usd {
        if let Some(reference_price) =
            reference_price.filter(|price| price.is_finite() && *price > 0.0)
        {
            return format_decimal_with_commas(target_coin * reference_price, 2);
        }
        "0".to_string()
    } else {
        format_decimal_with_commas(target_coin, decimals)
    }
}

fn position_size_for_symbol(
    clearinghouse: &ClearinghouseState,
    active_symbol: &str,
) -> Option<f64> {
    let asset_position = clearinghouse
        .asset_positions
        .iter()
        .find(|asset_position| asset_position.position.coin == active_symbol)
        .or_else(|| {
            clearinghouse.asset_positions.iter().find(|asset_position| {
                position_coin_matches(&asset_position.position.coin, active_symbol)
            })
        })?;

    parse_number(&asset_position.position.szi)
        .map(f64::abs)
        .filter(|size| size.is_finite() && *size > 0.0)
}

fn position_coin_matches(position_coin: &str, active_symbol: &str) -> bool {
    if position_coin == active_symbol {
        return true;
    }

    match (position_coin.split_once(':'), active_symbol.split_once(':')) {
        (None, Some((_, active_suffix))) => position_coin == active_suffix,
        _ => false,
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
    use crate::account::{
        AccountDataCompleteness, AssetPosition, MarginSummary, Position, PositionLeverage,
        SpotClearinghouseState, UserFeeRates,
    };
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

    fn account_data_with_positions(positions: Vec<AssetPosition>) -> AccountData {
        AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "1000".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "1000".to_string(),
                asset_positions: positions,
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: Vec::new(),
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: UserFeeRates::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: TradingTerminal::now_ms(),
        }
    }

    fn asset_position(coin: &str, szi: &str) -> AssetPosition {
        AssetPosition {
            position: Position {
                coin: coin.to_string(),
                szi: szi.to_string(),
                entry_px: "100".to_string(),
                position_value: "0".to_string(),
                unrealized_pnl: "0".to_string(),
                liquidation_px: None,
                leverage: PositionLeverage {
                    leverage_type: "cross".to_string(),
                    value: 10,
                },
                margin_used: "0".to_string(),
                cum_funding: None,
            },
            liquidation_px: None,
        }
    }

    fn terminal_with_position(coin: &str, szi: &str) -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.active_symbol = "BTC".to_string();
        terminal.active_symbol_display = "BTC".to_string();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.order_kind = OrderKind::Market;
        terminal.order_price.clear();
        terminal.account_data = Some(account_data_with_positions(vec![asset_position(coin, szi)]));
        terminal
    }

    #[test]
    fn reduce_only_slider_sizes_coin_quantity_from_position() {
        let mut terminal = terminal_with_position("BTC", "2.5");
        terminal.order_reduce_only = true;
        terminal.order_quantity_is_usd = false;

        terminal.handle_order_percentage_changed(50.0);

        assert_eq!(terminal.order_quantity, "1.25000");
    }

    #[test]
    fn reduce_only_slider_sizes_usd_quantity_from_position_notional() {
        let mut terminal = terminal_with_position("BTC", "2");
        terminal.order_reduce_only = true;
        terminal.order_quantity_is_usd = true;
        terminal.order_kind = OrderKind::Limit;
        terminal.order_price = "100".to_string();

        terminal.handle_order_percentage_changed(25.0);

        assert_eq!(terminal.order_quantity, "50.00");
    }

    #[test]
    fn reduce_only_manual_quantity_updates_percentage_from_position() {
        let mut terminal = terminal_with_position("BTC", "-2");
        terminal.order_reduce_only = true;
        terminal.order_quantity_is_usd = false;

        terminal.handle_order_quantity_changed("0.5".to_string());

        assert_eq!(terminal.order_percentage, 25.0);
    }

    #[test]
    fn reduce_only_toggle_resizes_existing_slider_percentage_to_position() {
        let mut terminal = terminal_with_position("BTC", "2");
        terminal.order_reduce_only = false;
        terminal.order_quantity_is_usd = false;
        terminal.order_percentage = 50.0;

        terminal.handle_toggle_reduce_only();

        assert!(terminal.order_reduce_only);
        assert_eq!(terminal.order_quantity, "1.00000");
    }

    #[test]
    fn reduce_only_slider_without_active_position_does_not_use_opening_margin() {
        let mut terminal = terminal_with_position("ETH", "2");
        terminal.order_reduce_only = true;
        terminal.order_quantity_is_usd = false;

        terminal.handle_order_percentage_changed(50.0);

        assert_eq!(terminal.order_quantity, "0");
    }

    #[test]
    fn reduce_only_position_lookup_prefers_exact_active_symbol() {
        let clearinghouse = ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "0".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "0".to_string(),
            asset_positions: vec![asset_position("BTC", "1"), asset_position("xyz:BTC", "3")],
        };

        assert_eq!(
            position_size_for_symbol(&clearinghouse, "xyz:BTC"),
            Some(3.0)
        );
        assert_eq!(position_size_for_symbol(&clearinghouse, "BTC"), Some(1.0));
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

    #[test]
    fn limit_ioc_reference_price_uses_order_price_not_market_mid() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.active_symbol = "BTC".to_string();
        terminal.order_kind = OrderKind::LimitIoc;
        terminal.order_price = "99.5".to_string();
        terminal.all_mids.insert("BTC".to_string(), 101.25);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());

        assert_eq!(terminal.order_reference_price(), Some(99.5));
    }
}
