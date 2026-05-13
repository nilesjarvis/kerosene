use crate::api::OrderBook;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use crate::order_execution::QuickOrderForm;
use crate::pane_state::PaneKind;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn handle_open_quick_order(
        &mut self,
        chart_id: ChartId,
        price: f64,
        click_x: f32,
        click_y: f32,
        chart_w: f32,
        chart_h: f32,
    ) -> Task<Message> {
        if self.primary_chart_id != Some(chart_id) {
            let target_pane = self
                .panes
                .iter()
                .find(|(_, kind)| matches!(kind, PaneKind::Chart(id) if *id == chart_id))
                .map(|(pane, _)| *pane);
            if let Some(pane) = target_pane {
                self.focus = Some(pane);
                self.primary_chart_id = Some(chart_id);

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
                    if self.is_ticker_muted(&sym) {
                        self.order_status =
                            Some(("Chart ticker is muted in Settings > Risk".into(), true));
                        return Task::none();
                    }
                    self.active_symbol = sym;
                    self.active_symbol_display = display;
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
        if self.is_ticker_muted(&chart_symbol) {
            self.order_status = Some(("Chart ticker is muted in Settings > Risk".into(), true));
            return Task::none();
        }
        if self.is_outcome_coin(&chart_symbol) {
            self.outcome_read_only_status("quick trading");
            return Task::none();
        }

        if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.set_quick_order(QuickOrderForm {
                price,
                quantity: String::new(),
                is_limit: true,
                click_x,
                click_y,
                chart_w,
                chart_h,
            });
            instance.last_quick_order_is_limit = true;
            return iced::widget::operation::focus(iced::widget::Id::from(format!(
                "quick_order_qty_{}",
                chart_id
            )));
        }

        Task::none()
    }
}
