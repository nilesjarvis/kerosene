use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

mod candles;
mod detached;
mod editor;
mod macro_indicators;

impl TradingTerminal {
    pub(crate) fn update_chart(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ (Message::ToggleMacroMenu(_)
            | Message::ToggleMacroIndicator(_, _)
            | Message::MacroCandlesLoaded(_, _, _, _)) => {
                return self.update_chart_macro_indicators(message);
            }
            message @ (Message::ChartSymbolSelected(_, _)
            | Message::ToggleChartInvert(_)
            | Message::ToggleChartTradeMarkers(_)
            | Message::ChartOpenEditor(_)
            | Message::ChartCloseEditor(_)
            | Message::ChartEditorSearchChanged(_, _)
            | Message::ChartEditorSubmit(_)
            | Message::AddChart(_)) => {
                return self.update_chart_editor(message);
            }
            Message::OpenDetachedChart(id) => return self.open_detached_chart_window(id),
            message @ (Message::ChartReload(_)
            | Message::ChartSwitchTimeframe(_, _)
            | Message::ChartCandlesLoaded(_, _)
            | Message::ChartFundingHistoryLoaded(_, _)
            | Message::ChartWsCandleUpdate(_, _, _, _)) => {
                return self.update_chart_candles(message);
            }
            Message::ChartResetView(id, surface_id) => {
                let reset_epoch = self
                    .chart_surface_reset_epochs
                    .entry(surface_id)
                    .or_default();
                *reset_epoch = reset_epoch.saturating_add(1);
                self.chart_surface_viewports.remove(&surface_id);
                if !self.chart_has_detached_window(id)
                    && let Some(instance) = self.charts.get_mut(&id)
                {
                    instance.chart.request_view_reset();
                    instance.heatmap_viewport = None;
                }
            }
            Message::ChartPriceFlashTick => {
                let now_ms = Self::now_ms();
                for instance in self.charts.values_mut() {
                    instance.clear_expired_last_price_flash(now_ms);
                }
            }
            Message::ChartWsAssetCtxUpdate(id, symbol, ctx) => {
                if self.symbol_key_is_hidden(&symbol) {
                    return Task::none();
                }
                let should_fetch_liq = self.charts.get(&id).is_some_and(|inst| {
                    inst.symbol == symbol
                        && inst.show_liquidations
                        && inst.liquidation_data.is_none()
                });
                if let Some(instance) = self.charts.get_mut(&id)
                    && instance.symbol == symbol
                {
                    instance.asset_ctx = Some(ctx);
                }
                if should_fetch_liq {
                    return self.maybe_fetch_liquidations(id);
                }
            }
            Message::ChartViewportChanged(id, surface_id, viewport) => {
                self.chart_surface_viewports.insert(surface_id, viewport);
                let chart_symbol = self
                    .charts
                    .get(&id)
                    .map(|instance| instance.symbol.clone())
                    .unwrap_or_default();
                let chart_symbol_muted = self.symbol_key_is_hidden(&chart_symbol);
                let should_fetch = if let Some(instance) = self.charts.get_mut(&id) {
                    instance.heatmap_viewport = Some(viewport);
                    instance.show_heatmap && !instance.symbol.is_empty() && !chart_symbol_muted
                } else {
                    false
                };
                if should_fetch {
                    return self.maybe_fetch_heatmap(id);
                }
            }
            Message::ChartFundingPanelHeightChanged(id, height, persist) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.chart.set_funding_panel_height(height as f32);
                }
                if persist {
                    self.persist_config();
                }
            }
            Message::ToggleFundingRateDisplayMode(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.chart.toggle_funding_rate_display_mode();
                }
            }
            Message::ToggleOpenInterestNotional(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.open_interest_as_notional = !instance.open_interest_as_notional;
                    self.persist_config();
                }
            }
            Message::FundingRefreshTick => return self.refresh_due_funding_charts(),
            _ => {}
        }

        Task::none()
    }
}
