use super::resolution::resolve_exchange_symbol;
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn reconcile_chart_symbol_metadata(&mut self) -> Vec<Task<Message>> {
        let mut tasks = Vec::new();
        let chart_backfill_request_context = self.chart_backfill_request_context();
        let hydromancer_api_key = self.hydromancer_api_key_for_task();
        let mut reset_quick_order_chart_ids = Vec::new();
        let mut chart_identity_changed = false;
        for (id, inst) in self.charts.iter_mut() {
            let symbol = resolve_exchange_symbol(&self.exchange_symbols, &inst.symbol);
            let mut primary_alias_canonicalized = false;

            if let Some(valid) = symbol {
                let display = Self::exchange_symbol_display_name(valid);
                let symbol_changed = valid.key != inst.symbol;
                primary_alias_canonicalized = symbol_changed;

                if symbol_changed || inst.symbol_display != display {
                    inst.set_symbol_identity(valid.key.clone(), display);
                }

                if symbol_changed {
                    chart_identity_changed = true;
                    inst.reset_quick_order_for_account_reset();
                    reset_quick_order_chart_ids.push(*id);
                    inst.chart.status = ChartStatus::Loading;
                    inst.chart.candles.clear();
                    inst.chart.clear_macro_candles();
                    inst.chart.candle_cache.clear();
                    inst.set_asset_context(None);
                    inst.candle_fetch_error = None;
                    Self::clear_chart_symbol_display_state(inst);
                    let request = Self::build_candle_fetch_request(
                        *id,
                        &valid.key,
                        inst.interval,
                        chart_backfill_request_context,
                        None,
                        0,
                    )
                    .with_moving_average_history(&inst.macro_indicators);
                    inst.candle_fetch_request = Some(request.clone());
                    let mut chart_tasks = vec![Self::fetch_candles_task(
                        request,
                        hydromancer_api_key.clone(),
                    )];
                    let macro_request_id = inst.next_macro_candles_request_id();
                    chart_tasks.extend(Self::fetch_macro_candles_tasks(
                        *id,
                        macro_request_id,
                        &valid.key,
                        &inst.macro_indicators,
                    ));
                    tasks.push(Task::batch(chart_tasks));
                }
            }
            if let Some(secondary_key) = inst.secondary_symbol.as_deref()
                && let Some(valid) = resolve_exchange_symbol(&self.exchange_symbols, secondary_key)
            {
                let display = Self::exchange_symbol_display_name(valid);
                let symbol_changed = inst.secondary_symbol.as_deref() != Some(valid.key.as_str());
                let alias_collision =
                    valid.key == inst.symbol && (primary_alias_canonicalized || symbol_changed);

                if alias_collision {
                    inst.clear_secondary_symbol();
                    chart_identity_changed = true;
                    continue;
                }

                if symbol_changed
                    || inst.secondary_symbol_display.as_deref() != Some(display.as_str())
                {
                    inst.set_secondary_symbol_identity(valid.key.clone(), display);
                }

                if symbol_changed {
                    chart_identity_changed = true;
                    inst.chart.set_secondary_candles(Vec::new());
                    inst.secondary_candle_fetch_error = None;
                    let request = Self::build_candle_fetch_request(
                        *id,
                        &valid.key,
                        inst.interval,
                        chart_backfill_request_context,
                        None,
                        0,
                    );
                    inst.secondary_candle_fetch_request = Some(request.clone());
                    tasks.push(Self::fetch_secondary_candles_task(
                        request,
                        hydromancer_api_key.clone(),
                    ));
                }
            }
        }
        for chart_id in reset_quick_order_chart_ids {
            self.chart_quick_order_surface.remove(&chart_id);
        }
        if chart_identity_changed {
            self.persist_config();
        }

        tasks
    }
}
