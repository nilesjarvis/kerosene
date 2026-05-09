use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartId;

use iced::Task;

mod execution;
mod legs;
mod plan;

use execution::execute_pair_order_sequence;
use legs::build_pair_leg_order;
use plan::{missing_pair_mid_status, pair_direction_label, pair_leg_sides, parse_pair_notional};

impl TradingTerminal {
    pub(crate) fn execute_pair_trade(
        &mut self,
        id: SpaghettiChartId,
        long_a_short_b: bool,
    ) -> Task<Message> {
        let _theme = self.theme();
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }

        let Some(inst_ro) = self.spaghetti_charts.get(&id) else {
            return Task::none();
        };
        if inst_ro.pair_pending {
            return Task::none();
        }

        if inst_ro.canvas.series.len() < 2 {
            self.order_status = Some(("Pair trading needs two symbols".into(), true));
            return Task::none();
        }

        let symbol_a = inst_ro.canvas.series[0].symbol.clone();
        let symbol_b = inst_ro.canvas.series[1].symbol.clone();
        if self.is_ticker_muted(&symbol_a) || self.is_ticker_muted(&symbol_b) {
            self.order_status = Some(("Pair trade includes a muted ticker".into(), true));
            return Task::none();
        }
        let Some(notional) = parse_pair_notional(&inst_ro.pair_notional) else {
            self.order_status = Some(("Invalid pair notional".into(), true));
            return Task::none();
        };

        let Some(sym_a) = self.exchange_symbols.iter().find(|s| s.key == symbol_a) else {
            self.order_status = Some(("Pair symbol A metadata unavailable".into(), true));
            return Task::none();
        };
        let Some(sym_b) = self.exchange_symbols.iter().find(|s| s.key == symbol_b) else {
            self.order_status = Some(("Pair symbol B metadata unavailable".into(), true));
            return Task::none();
        };

        if sym_a.market_type != MarketType::Perp || sym_b.market_type != MarketType::Perp {
            self.order_status = Some(("Pair trading is perp-only".into(), true));
            return Task::none();
        }

        let mid_a = self.resolve_mid_for_symbol(&symbol_a).unwrap_or(f64::NAN);
        let mid_b = self.resolve_mid_for_symbol(&symbol_b).unwrap_or(f64::NAN);

        if let Some(status) = missing_pair_mid_status(
            &symbol_a,
            &symbol_b,
            mid_a,
            mid_b,
            &self.mid_candidates_for_symbol(&symbol_a),
            &self.mid_candidates_for_symbol(&symbol_b),
        ) {
            self.order_status = Some((status, true));
            let mut tasks = self.mids_bootstrap_tasks();
            if tasks.is_empty() {
                return Task::none();
            }
            return Task::batch(std::mem::take(&mut tasks));
        }

        let (a_is_buy, b_is_buy) = pair_leg_sides(long_a_short_b);
        let slippage = self.market_slippage_fraction();

        let leg_a = build_pair_leg_order(
            symbol_a.clone(),
            sym_a.asset_index,
            sym_a.sz_decimals,
            mid_a,
            notional,
            a_is_buy,
            slippage,
        );
        let leg_b = build_pair_leg_order(
            symbol_b.clone(),
            sym_b.asset_index,
            sym_b.sz_decimals,
            mid_b,
            notional,
            b_is_buy,
            slippage,
        );
        let (Some(leg_a), Some(leg_b)) = (leg_a, leg_b) else {
            self.order_status = Some(("Pair size calculation failed".into(), true));
            return Task::none();
        };

        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.pair_pending = true;
        }
        let dir = pair_direction_label(&symbol_a, &symbol_b, long_a_short_b);
        self.order_status = Some((format!("Placing pair trade: {dir}..."), false));

        Task::perform(
            execute_pair_order_sequence(key.into(), leg_a, leg_b),
            move |r| Message::PairExecutionDone(id, Box::new(r)),
        )
    }
}
