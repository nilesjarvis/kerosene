use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::ChaseOrder;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn handle_chase_resting_order(
        &mut self,
        coin: String,
        oid: u64,
        is_buy: bool,
        sz: f64,
        limit_px: f64,
    ) -> Task<Message> {
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }
        let Some(account_address) = self.connected_address.clone() else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        };
        if self.is_ticker_muted(&coin) {
            self.order_status = Some(("Order ticker is muted in Settings > Risk".into(), true));
            return Task::none();
        }
        if !sz.is_finite() || sz <= 0.0 {
            self.order_status = Some(("Cannot chase order with invalid size".into(), true));
            return Task::none();
        }
        if !limit_px.is_finite() || limit_px <= 0.0 {
            self.order_status = Some(("Cannot chase order with invalid price".into(), true));
            return Task::none();
        }

        if let Some(chase) = &self.active_chase
            && chase.current_oid == Some(oid)
        {
            return Task::none();
        }

        let stop_task = if self.active_chase.is_some() {
            self.stop_chase()
        } else {
            Task::none()
        };

        let switch_task = if self.active_symbol != coin {
            self.switch_active_symbol_internal(coin.clone())
        } else {
            Task::none()
        };

        let symbol = self.exchange_symbols.iter().find(|s| s.key == coin);
        let Some(symbol) = symbol else {
            self.order_status = Some((format!("Symbol '{coin}' not found"), true));
            return Task::batch([stop_task, switch_task]);
        };
        if symbol.market_type == MarketType::Outcome {
            self.outcome_read_only_status("chase trading");
            return Task::batch([stop_task, switch_task]);
        }

        let asset = symbol.asset_index;
        let sz_decimals = symbol.sz_decimals;
        let is_spot = symbol.market_type == MarketType::Spot;
        let reduce_only = if is_spot {
            false
        } else {
            self.order_reduce_only
        };

        self.active_chase = Some(ChaseOrder {
            coin: coin.clone(),
            account_address,
            agent_key: key.into(),
            is_buy,
            remaining_size: sz,
            asset,
            sz_decimals,
            reduce_only,
            current_oid: Some(oid),
            current_price: limit_px,
            initial_price: limit_px,
            started_at: std::time::Instant::now(),
            reprice_count: 0,
            cancel_in_flight: false,
            stop_requested: false,
            cancel_retries: 0,
            oid_confirmed: true,
        });

        self.order_status = Some((format!("Chasing resting order {oid}..."), false));
        Task::batch([stop_task, switch_task])
    }
}
