use crate::account::AssetContext;
use crate::app_state::TradingTerminal;
use crate::helpers::parse_finite_number;
use crate::message::Message;
use crate::read_data_provider::MarketDataSourceContext;

use iced::Task;
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Real-Time Position PnL
// ---------------------------------------------------------------------------

const OPEN_POSITION_EPSILON: f64 = 1e-12;

impl TradingTerminal {
    pub(crate) fn hydromancer_realtime_position_pnl_available(&self) -> bool {
        self.hydromancer_realtime_position_pnl_enabled
            && !self.hydromancer_api_key.trim().is_empty()
    }

    pub(crate) fn hydromancer_realtime_position_pnl_symbols(&self) -> Vec<String> {
        if !self.hydromancer_realtime_position_pnl_available() {
            return Vec::new();
        }

        let Some((_, data)) = self.connected_order_account_snapshot() else {
            return Vec::new();
        };

        let mut symbols = BTreeSet::new();
        for ap in &data.clearinghouse.asset_positions {
            let coin = ap.position.coin.trim();
            if coin.is_empty()
                || self.symbol_key_is_hidden(coin)
                || (self.position_is_hidden(coin) && !self.show_hidden_positions)
                || self.is_outcome_coin(coin)
            {
                continue;
            }
            if parse_finite_number(&ap.position.szi)
                .is_some_and(|szi| szi.abs() > OPEN_POSITION_EPSILON && szi.is_finite())
            {
                symbols.insert(coin.to_string());
            }
        }

        symbols.into_iter().collect()
    }

    pub(super) fn apply_position_pnl_asset_ctx_update(
        &mut self,
        symbol: String,
        source_context: MarketDataSourceContext,
        ctx: AssetContext,
    ) -> Task<Message> {
        if !self.hydromancer_keyed_market_stream_source_is_current(source_context)
            || !self
                .hydromancer_realtime_position_pnl_symbols()
                .contains(&symbol)
        {
            return Task::none();
        }

        let Some(price) = ctx.live_price() else {
            return Task::none();
        };

        let now_ms = Self::now_ms();
        if let Some(&old_price) = self.all_mids.get(&symbol)
            && (price - old_price).abs() > f64::EPSILON
        {
            let direction = if price > old_price { 1 } else { -1 };
            self.live_watchlist_flashes
                .insert(symbol.clone(), (now_ms, direction));
        }
        self.all_mids.insert(symbol.clone(), price);
        self.all_mids_updated_at_ms.insert(symbol, now_ms);

        Task::none()
    }

    pub(super) fn apply_position_pnl_asset_ctx_lag(
        &mut self,
        symbol: String,
        source_context: MarketDataSourceContext,
        _skipped: u64,
    ) -> Task<Message> {
        if !self.hydromancer_keyed_market_stream_source_is_current(source_context) {
            return Task::none();
        }

        if self
            .hydromancer_realtime_position_pnl_symbols()
            .contains(&symbol)
        {
            self.all_mids_updated_at_ms.remove(&symbol);
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
        Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
    };
    use crate::api::{ExchangeSymbol, MarketType};

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    fn asset_ctx(mid_px: Option<&str>, mark_px: Option<&str>) -> AssetContext {
        AssetContext {
            funding: None,
            open_interest: None,
            oracle_px: None,
            mark_px: mark_px.map(str::to_string),
            mid_px: mid_px.map(str::to_string),
            prev_day_px: None,
            day_ntl_vlm: None,
            day_base_vlm: None,
            impact_pxs: None,
        }
    }

    fn position(coin: &str, szi: &str) -> AssetPosition {
        AssetPosition {
            position: Position {
                coin: coin.to_string(),
                szi: szi.to_string(),
                entry_px: "100".to_string(),
                position_value: "100".to_string(),
                unrealized_pnl: "0".to_string(),
                liquidation_px: None,
                leverage: PositionLeverage {
                    leverage_type: "cross".to_string(),
                    value: 1,
                },
                margin_used: "0".to_string(),
                cum_funding: None,
            },
            liquidation_px: None,
        }
    }

    fn account_data(positions: Vec<AssetPosition>) -> AccountData {
        AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "0".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
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
            fetched_at_ms: 1_000,
        }
    }

    fn connect_with_positions(terminal: &mut TradingTerminal, positions: Vec<AssetPosition>) {
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, account_data(positions));
    }

    fn source_context(
        terminal: &TradingTerminal,
        hydromancer_key_generation: Option<u64>,
    ) -> MarketDataSourceContext {
        MarketDataSourceContext {
            hydromancer_key_generation,
            ..terminal.hydromancer_keyed_market_data_source_context()
        }
    }

    fn exchange_symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: String::new(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 0,
            max_leverage: 0,
            only_isolated: false,
            market_type,
            outcome: None,
        }
    }

    #[test]
    fn realtime_position_pnl_symbols_require_enabled_key_and_open_positions() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![
            exchange_symbol("BTC", MarketType::Perp),
            exchange_symbol("ETH", MarketType::Perp),
            exchange_symbol("#650", MarketType::Outcome),
        ];
        connect_with_positions(
            &mut terminal,
            vec![
                position("BTC", "1"),
                position("ETH", "0"),
                position("#650", "2"),
                position("BAD", "not-a-number"),
            ],
        );

        assert!(
            terminal
                .hydromancer_realtime_position_pnl_symbols()
                .is_empty()
        );

        terminal.hydromancer_realtime_position_pnl_enabled = true;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();

        assert_eq!(
            terminal.hydromancer_realtime_position_pnl_symbols(),
            vec!["BTC".to_string()]
        );
    }

    #[test]
    fn realtime_position_pnl_tick_updates_position_row_pnl() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![exchange_symbol("BTC", MarketType::Perp)];
        terminal.hydromancer_realtime_position_pnl_enabled = true;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;
        connect_with_positions(&mut terminal, vec![position("BTC", "2")]);

        let _task = terminal.apply_position_pnl_asset_ctx_update(
            "BTC".to_string(),
            source_context(&terminal, Some(2)),
            asset_ctx(Some("101.5"), None),
        );

        assert_eq!(terminal.resolve_mid_for_symbol("BTC"), Some(101.5));
        assert_eq!(
            crate::account::position_notional_from_mark_or_wire(
                Some(2.0),
                Some(100.0),
                Some(101.5)
            ),
            Some(203.0)
        );
        assert_eq!(
            crate::account::position_upnl_from_mark_or_wire(
                Some(2.0),
                Some(100.0),
                Some(0.0),
                Some(101.5),
            ),
            Some(3.0)
        );
    }

    #[test]
    fn realtime_position_pnl_ignores_stale_key_generation() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![exchange_symbol("BTC", MarketType::Perp)];
        terminal.hydromancer_realtime_position_pnl_enabled = true;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;
        connect_with_positions(&mut terminal, vec![position("BTC", "2")]);

        let _task = terminal.apply_position_pnl_asset_ctx_update(
            "BTC".to_string(),
            source_context(&terminal, Some(1)),
            asset_ctx(Some("101.5"), None),
        );

        assert!(!terminal.all_mids.contains_key("BTC"));
    }
}
