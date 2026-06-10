use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::helpers::positive_finite_value;
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
                    let selected_symbol = sym.clone();
                    self.apply_active_symbol_selection(sym, display);
                    self.refresh_order_price_for_symbol(&selected_symbol);
                    self.reset_active_order_books_for_symbol(&selected_symbol);
                    self.sync_all_chart_overlays();
                    self.persist_config();
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
        let Some(price) = positive_finite_value(price) else {
            self.order_status = Some(("Invalid quick-order price".into(), true));
            return Task::none();
        };
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
mod tests;
