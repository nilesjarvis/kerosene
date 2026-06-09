use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;

mod candles;
mod detached;
mod earnings;
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
            message @ (Message::ToggleChartEarningsMarkers(_)
            | Message::ChartEarningsEventsLoaded(_, _)) => {
                return self.update_chart_earnings(message);
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
            Message::ToggleChartHeaderCollapsed(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.header_collapsed = !instance.header_collapsed;
                    self.persist_config();
                }
            }
            Message::ChartFocused(id) if self.charts.contains_key(&id) => {
                self.primary_chart_id = Some(id);
                self.focus = self
                    .panes
                    .iter()
                    .find(|(_, kind)| matches!(kind, PaneKind::Chart(chart_id) if *chart_id == id))
                    .map(|(pane, _)| *pane);
            }
            message @ (Message::ChartReload(_)
            | Message::ChartSwitchTimeframe(_, _)
            | Message::ChartCandlesLoaded(_, _)
            | Message::ChartFundingHistoryLoaded(_, _)
            | Message::ChartWsCandleUpdate(_, _, _, _)) => {
                return self.update_chart_candles(message);
            }
            Message::ChartResetView(id, surface_id) => {
                self.chart_surface_viewports.remove(&surface_id);
                if let Some(instance) = self.charts.get_mut(&id)
                    && instance.chart.surface_id() == surface_id
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
            Message::ChartHudOrderAnimationTick => {
                for instance in self.charts.values_mut() {
                    instance.chart.advance_hud_order_animation();
                }
            }
            Message::ChartHudArmToggled(id, surface_id) => {
                let now_ms = Self::now_ms();
                if let Some(instance) = self.charts.get_mut(&id)
                    && instance.chart.surface_id() == surface_id
                {
                    instance.chart.toggle_hud_armed_at(now_ms);
                }
            }
            Message::ChartHudSafetyTick => {
                let now_ms = Self::now_ms();
                for instance in self.charts.values_mut() {
                    if instance.chart.hud_safety_timeout_due(now_ms) {
                        instance.chart.set_hud_armed_at(false, now_ms);
                    }
                }
            }
            Message::ChartHoverStateChanged(
                id,
                surface_id,
                oid,
                hovering_plot,
                earnings_marker_time_ms,
            ) => {
                let now_ms = Self::now_ms();
                if let Some(instance) = self.charts.get_mut(&id)
                    && instance.chart.surface_id() == surface_id
                {
                    instance.chart.set_order_cancel_hover(oid);
                    instance
                        .chart
                        .set_earnings_marker_hover(earnings_marker_time_ms);
                    instance.chart.record_hud_activity(now_ms, hovering_plot);
                }
            }
            Message::ChartOrderCancelHoverAnimationTick => {
                for instance in self.charts.values_mut() {
                    instance.chart.advance_order_cancel_hover_animation();
                }
            }
            Message::ChartEarningsMarkerHoverAnimationTick => {
                for instance in self.charts.values_mut() {
                    instance.chart.advance_earnings_marker_hover_animation();
                }
            }
            Message::ChartWsAssetCtxUpdate(_id, symbol, ctx) => {
                if self.symbol_key_is_hidden(&symbol) {
                    return Task::none();
                }
                let fetch_liquidation_ids: Vec<_> = self
                    .charts
                    .iter()
                    .filter_map(|(chart_id, inst)| {
                        (inst.symbol == symbol
                            && inst.show_liquidations
                            && inst.liquidation_data.is_none())
                        .then_some(*chart_id)
                    })
                    .collect();
                let now_ms = Self::now_ms();
                for instance in self.charts.values_mut() {
                    if instance.symbol == symbol {
                        instance.set_asset_context_at(Some(ctx.clone()), now_ms);
                    }
                }
                if !fetch_liquidation_ids.is_empty() {
                    return Task::batch(
                        fetch_liquidation_ids
                            .into_iter()
                            .map(|chart_id| self.maybe_fetch_liquidations(chart_id)),
                    );
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
            Message::ToggleAssetVolumeNotional(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.asset_volume_as_notional = !instance.asset_volume_as_notional;
                    self.persist_config();
                }
            }
            Message::ToggleOutcomeVolumeNotional(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.outcome_volume_as_notional = !instance.outcome_volume_as_notional;
                    self.persist_config();
                }
            }
            Message::FundingRefreshTick => return self.refresh_due_funding_charts(),
            _ => {}
        }

        Task::none()
    }
}
