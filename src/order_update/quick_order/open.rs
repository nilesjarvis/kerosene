use crate::api::OrderBook;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use crate::order_execution::QuickOrderForm;
use crate::pane_state::PaneKind;

use iced::Task;

pub(crate) struct QuickOrderOpenRequest {
    pub(crate) chart_id: ChartId,
    pub(crate) surface_id: ChartSurfaceId,
    pub(crate) price: f64,
    pub(crate) click_x: f32,
    pub(crate) click_y: f32,
    pub(crate) chart_w: f32,
    pub(crate) chart_h: f32,
}

impl TradingTerminal {
    pub(crate) fn handle_open_quick_order(
        &mut self,
        request: QuickOrderOpenRequest,
    ) -> Task<Message> {
        let QuickOrderOpenRequest {
            chart_id,
            surface_id,
            price,
            click_x,
            click_y,
            chart_w,
            chart_h,
        } = request;

        if self.primary_chart_id != Some(chart_id) {
            let target_pane = self
                .panes
                .iter()
                .find(|(_, kind)| matches!(kind, PaneKind::Chart(id) if *id == chart_id))
                .map(|(pane, _)| *pane);
            if self.charts.contains_key(&chart_id) {
                self.primary_chart_id = Some(chart_id);
            }
            if let Some(pane) = target_pane {
                self.focus = Some(pane);

                let chart_sym = self.charts.get(&chart_id).and_then(|inst| {
                    let sym = inst.symbol.clone();
                    let display = inst.symbol_display.clone();
                    if !sym.is_empty() && sym != self.active_symbol {
                        Some((sym, display))
                    } else {
                        None
                    }
                });

                if let Some((sym, display)) = chart_sym {
                    if let Some(symbol) = self.resolve_exchange_symbol_by_key_or_ticker(&sym) {
                        if let Err(message) =
                            self.validate_exchange_symbol_orderable(symbol, "Chart")
                        {
                            self.order_status = Some((message, true));
                            return Task::none();
                        }
                    } else if self.symbol_key_is_hidden(&sym) {
                        self.order_status =
                            Some(("Chart ticker is hidden in Settings > Risk".into(), true));
                        return Task::none();
                    }
                    self.apply_active_symbol_selection(sym, display);
                    for inst in self.order_books.values_mut() {
                        if inst.mode == OrderBookSymbolMode::Active {
                            inst.set_book(OrderBook::empty());
                        }
                    }
                    self.sync_all_chart_overlays();
                    self.persist_config();
                    for inst in self.order_books.values_mut() {
                        if inst.mode == OrderBookSymbolMode::Active {
                            inst.book_loading = true;
                        }
                    }
                }
            } else {
                self.focus = None;
            }
        }

        if self.connected_address.is_none() || self.wallet_key_input.trim().is_empty() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }

        let chart_symbol = self
            .charts
            .get(&chart_id)
            .map(|inst| inst.symbol.clone())
            .unwrap_or_default();
        if chart_symbol.is_empty() {
            return Task::none();
        }
        if !price.is_finite() || price <= 0.0 {
            self.order_status = Some(("Invalid quick-order price".into(), true));
            return Task::none();
        }
        if self.symbol_key_is_hidden(&chart_symbol) {
            self.order_status = Some(("Chart ticker is hidden in Settings > Risk".into(), true));
            return Task::none();
        }
        if self.is_outcome_coin(&chart_symbol) {
            self.outcome_read_only_status("quick trading");
            return Task::none();
        }

        let fallback_quantity_is_usd = self.order_quantity_is_usd;
        if let Some(instance) = self.charts.get_mut(&chart_id) {
            let (quantity, quantity_is_usd, percentage, is_limit) =
                instance.quick_order_reopen_values(fallback_quantity_is_usd);
            instance.set_quick_order(QuickOrderForm {
                price,
                quantity,
                quantity_is_usd,
                percentage,
                is_limit,
                click_x,
                click_y,
                chart_w,
                chart_h,
            });
            self.chart_quick_order_surface.insert(chart_id, surface_id);
            return iced::widget::operation::focus(iced::widget::Id::from(format!(
                "quick_order_qty_{}",
                chart_id
            )));
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType};
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    fn symbol(key: &str) -> ExchangeSymbol {
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
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    #[test]
    fn open_quick_order_reuses_last_type_and_size_for_same_chart_symbol() {
        let chart_id = 7;
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
        terminal.wallet_key_input = "agent-key".to_string().into();
        terminal.exchange_symbols = vec![symbol("BTC")];
        terminal.primary_chart_id = Some(chart_id);

        let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
        instance.last_quick_order_symbol = "BTC".to_string();
        instance.last_quick_order_quantity = "2.5".to_string();
        instance.last_quick_order_quantity_is_usd = false;
        instance.last_quick_order_percentage = 25.0;
        instance.last_quick_order_is_limit = false;
        terminal.charts.insert(chart_id, instance);

        let _task = terminal.handle_open_quick_order(QuickOrderOpenRequest {
            chart_id,
            surface_id: ChartSurfaceId::Docked(chart_id),
            price: 101.0,
            click_x: 10.0,
            click_y: 20.0,
            chart_w: 300.0,
            chart_h: 200.0,
        });

        let instance = terminal.charts.get(&chart_id).expect("chart instance");
        let form = instance.quick_order.as_ref().expect("quick order form");
        assert!(!form.is_limit);
        assert_eq!(form.quantity, "2.5");
        assert!(!form.quantity_is_usd);
        assert_eq!(form.percentage, 25.0);
        assert_eq!(form.price, 101.0);
        assert_eq!(instance.chart.quick_order_limit_price, None);
    }
}
