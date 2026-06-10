use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::{CancelIntent, OrderSurface, cancel_order_task};

use iced::Task;

impl TradingTerminal {
    pub(crate) fn execute_cancel(&mut self, coin: &str, oid: u64) -> Task<Message> {
        let _theme = self.theme();
        // The chart suppresses interaction on a Cancelling line, but the
        // orders table and queued clicks can still re-dispatch; a duplicate
        // cancel surfaces a spurious red error after the first one lands.
        if self.has_pending_cancel_indicator(oid) {
            self.order_status = Some(("Cancel already pending for this order".into(), true));
            return Task::none();
        }
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() {
            self.order_status = Some(("Enter agent key to cancel orders".into(), true));
            return Task::none();
        }
        let prepared = match self.prepare_cancel_order(CancelIntent {
            surface: OrderSurface::Cancel,
            symbol_key: coin.to_string(),
            oid,
        }) {
            Ok(prepared) => prepared,
            Err(message) => {
                self.order_status = Some((message, true));
                return Task::none();
            }
        };
        let account_address = self.connected_address.clone().unwrap_or_default();
        let pending_indicator_id = if account_address.is_empty() {
            None
        } else {
            self.account_data
                .as_ref()
                .and_then(|data| data.open_orders.iter().find(|order| order.oid == oid))
                .cloned()
                .and_then(|order| {
                    self.add_pending_order_cancellation_indicator(account_address.clone(), &order)
                })
        };

        self.order_status = Some(("Cancelling order...".into(), false));
        cancel_order_task(key.into(), prepared.asset, prepared.oid, move |result| {
            Message::CancelResult {
                account_address,
                pending_indicator_id,
                result: Box::new(result),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::account::{
        AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, OpenOrder,
        SpotClearinghouseState, UserFeeRates,
    };
    use crate::api::{ExchangeSymbol, MarketType};
    use crate::app_state::{TradingTerminal, sensitive_string};

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    fn btc_symbol() -> ExchangeSymbol {
        ExchangeSymbol {
            key: "BTC".to_string(),
            ticker: "BTC".to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 4,
            max_leverage: 50,
            only_isolated: false,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    fn open_order(oid: u64) -> OpenOrder {
        OpenOrder {
            coin: "BTC".to_string(),
            side: "B".to_string(),
            limit_px: "100".to_string(),
            sz: "1".to_string(),
            oid,
            timestamp: 1,
            reduce_only: Some(false),
            is_trigger: None,
            order_type: None,
            tif: None,
            trigger_px: None,
        }
    }

    fn account_data_with_order(order: OpenOrder) -> AccountData {
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
                asset_positions: Vec::new(),
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: vec![order],
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: UserFeeRates::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: 1,
        }
    }

    fn terminal_with_cancelable_order() -> TradingTerminal {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_key_input = sensitive_string("agent-key");
        terminal.exchange_symbols = vec![btc_symbol()];
        terminal.account_data = Some(account_data_with_order(open_order(42)));
        terminal
    }

    #[test]
    fn execute_cancel_creates_cancelling_indicator() {
        let mut terminal = terminal_with_cancelable_order();

        let _task = terminal.execute_cancel("BTC", 42);

        assert!(terminal.has_pending_cancel_indicator(42));
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("Cancelling order...", false))
        );
    }

    #[test]
    fn duplicate_cancel_is_gated_while_indicator_pending() {
        let mut terminal = terminal_with_cancelable_order();

        let _task = terminal.execute_cancel("BTC", 42);
        assert_eq!(terminal.pending_order_indicators.len(), 1);

        let _task = terminal.execute_cancel("BTC", 42);

        assert_eq!(terminal.pending_order_indicators.len(), 1);
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("Cancel already pending for this order", true))
        );
    }

    #[test]
    fn cancel_of_other_oid_is_not_gated() {
        let mut terminal = terminal_with_cancelable_order();

        let _task = terminal.execute_cancel("BTC", 42);
        let _task = terminal.execute_cancel("BTC", 43);

        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, _)| message.as_str()),
            Some("Cancelling order...")
        );
    }
}
