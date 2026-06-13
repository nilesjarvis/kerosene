use crate::app_state::TradingTerminal;
use crate::chart::HudSelectorKind;
use crate::chart_state::ChartInstance;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::sound;
use iced::Task;

mod candles;
mod detached;
mod earnings;
mod editor;
mod macro_indicators;

impl TradingTerminal {
    pub(crate) fn clear_chart_market_display_state(instance: &mut ChartInstance) {
        instance.heatmap_last_fetch = None;
        instance.heatmap_viewport = None;
        instance.heatmap_status = None;
        instance.heatmap_fetching = false;
        instance.last_price_flash = None;
        Self::clear_heatmap_display(instance);
        Self::clear_liquidation_display(instance);
        Self::clear_funding_display(instance);
    }

    pub(crate) fn clear_chart_symbol_display_state(instance: &mut ChartInstance) {
        Self::clear_chart_market_display_state(instance);
        Self::clear_earnings_display(instance);
    }

    pub(crate) fn update_chart(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ (Message::ToggleMacroMenu(_)
            | Message::ToggleMacroIndicator(_, _)
            | Message::MacroCandlesLoaded(_, _, _, _, _)) => {
                return self.update_chart_macro_indicators(message);
            }
            message @ (Message::ToggleChartEarningsMarkers(_)
            | Message::ChartEarningsEventsLoaded(_, _, _)) => {
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
            | Message::ChartWsCandleUpdate(_, _, _, _, _)
            | Message::ChartWsCandleLagged(_, _, _, _, _)) => {
                return self.update_chart_candles(message);
            }
            Message::ChartResetView(id, surface_id) => {
                self.chart_surface_viewports.remove(&surface_id);
                let should_reset = self
                    .charts
                    .get(&id)
                    .is_some_and(|instance| instance.chart.surface_id() == surface_id);
                if should_reset {
                    self.clear_chart_heatmap_pending_request_state(id);
                }
                if let Some(instance) = self.charts.get_mut(&id)
                    && should_reset
                {
                    instance.chart.request_view_reset();
                    instance.heatmap_viewport = None;
                    instance.heatmap_last_fetch = None;
                    instance.heatmap_fetching = false;
                    instance.heatmap_status = None;
                    Self::clear_heatmap_display(instance);
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
                    let sound = if instance.chart.hud_armed() {
                        sound::HudUiSound::Arm
                    } else {
                        sound::HudUiSound::Disarm
                    };
                    self.play_hud_ui_sound(sound);
                }
            }
            Message::ChartHudControlChanged(id, surface_id, control, changed) => {
                if changed {
                    self.play_hud_ui_sound(control);
                }
                if let Some(instance) = self.charts.get_mut(&id)
                    && instance.chart.surface_id() == surface_id
                    && let Some(kind) = HudSelectorKind::for_control(control)
                {
                    instance.chart.open_hud_weapon_selector(kind, changed);
                }
            }
            Message::ChartHudSafetyTick => {
                let now_ms = Self::now_ms();
                let mut auto_disarmed = false;
                for instance in self.charts.values_mut() {
                    if instance.chart.hud_safety_timeout_due(now_ms) {
                        instance.chart.set_hud_armed_at(false, now_ms);
                        auto_disarmed = true;
                    }
                }
                if auto_disarmed {
                    self.play_hud_safety_sound(sound::HudUiSound::AutoDisarm);
                } else {
                    // Consume the once-per-session warning only when its pip
                    // actually plays; a suppressed warning stays pending.
                    let mut warning_due = false;
                    for instance in self.charts.values_mut() {
                        if instance.chart.hud_safety_warning_due(now_ms) {
                            instance.chart.mark_hud_idle_warning_sounded();
                            warning_due = true;
                        }
                    }
                    if warning_due {
                        self.play_hud_safety_sound(sound::HudUiSound::IdleWarning);
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
            Message::ChartWsAssetCtxUpdate(_id, symbol, source_context, ctx) => {
                if !self.market_stream_source_is_current(source_context) {
                    return Task::none();
                }
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
            Message::ChartWsAssetCtxLagged(_id, symbol, source_context, _skipped) => {
                if !self.market_stream_source_is_current(source_context) {
                    return Task::none();
                }
                if self.symbol_key_is_hidden(&symbol) {
                    return Task::none();
                }
                for instance in self.charts.values_mut() {
                    if instance.symbol == symbol {
                        instance.set_asset_context(None);
                    }
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
            Message::ChartSessionPanelHeightChanged(id, height, persist) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.chart.set_session_panel_height(height as f32);
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

    fn play_hud_ui_sound(&self, sound: sound::HudUiSound) {
        if self.sound_enabled && self.chart_hud_ui_sounds {
            sound::play_hud_ui(sound, self.chart_hud_order_sound_volume);
        }
    }

    /// Arm-safety advisories bypass the control-clicks preference: turning
    /// off key-click feedback must not silence the auto-disarm warnings.
    fn play_hud_safety_sound(&self, sound: sound::HudUiSound) {
        if self.sound_enabled {
            sound::play_hud_ui(sound, self.chart_hud_order_sound_volume);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AssetContext;
    use crate::chart_state::{ChartInstance, ChartSurfaceId};
    use crate::config::ReadDataProvider;
    use crate::hyperdash_api::{HeatmapFetchParams, LiquidationHeatmap};
    use crate::timeframe::Timeframe;

    fn asset_ctx(mid_px: &str) -> AssetContext {
        AssetContext {
            funding: None,
            open_interest: None,
            oracle_px: None,
            mark_px: None,
            mid_px: Some(mid_px.to_string()),
            prev_day_px: None,
            day_ntl_vlm: None,
            day_base_vlm: None,
            impact_pxs: None,
        }
    }

    fn asset_ctx_with_impact(bid: &str, ask: &str) -> AssetContext {
        AssetContext {
            funding: None,
            open_interest: None,
            oracle_px: None,
            mark_px: None,
            mid_px: None,
            prev_day_px: None,
            day_ntl_vlm: None,
            day_base_vlm: None,
            impact_pxs: Some(vec![bid.to_string(), ask.to_string()]),
        }
    }

    fn terminal_with_chart() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal
            .charts
            .insert(7, ChartInstance::new(7, "BTC".to_string(), Timeframe::H1));
        terminal
    }

    #[test]
    fn chart_reset_view_clears_pending_heatmap_request_state() {
        let mut terminal = terminal_with_chart();
        let chart_id = 7;
        let surface_id = ChartSurfaceId::Docked(chart_id);
        let request = HeatmapFetchParams {
            coin: "BTC".to_string(),
            min_price: 1.0,
            max_price: 2.0,
            start_time: 10,
            end_time: 20,
        };
        let cache_key = request.cache_key();
        if let Some(instance) = terminal.charts.get_mut(&chart_id) {
            instance.show_heatmap = true;
            instance.heatmap_fetching = true;
            instance.heatmap_last_fetch = Some(request);
            instance.heatmap_status = Some(("HEAT refreshing hourly data".to_string(), false));
            instance.heatmap_data = Some(LiquidationHeatmap {
                rects: Vec::new(),
                max_abs_usd: 123.0,
            });
            instance.chart.heatmap_max_usd = 123.0;
        }
        terminal
            .heatmap_pending_charts
            .insert(cache_key.clone(), vec![chart_id]);

        let _task = terminal.update_chart(Message::ChartResetView(chart_id, surface_id));

        assert!(!terminal.heatmap_pending_charts.contains_key(&cache_key));
        let instance = terminal.charts.get(&chart_id).expect("chart");
        assert!(instance.heatmap_last_fetch.is_none());
        assert!(!instance.heatmap_fetching);
        assert!(instance.heatmap_status.is_none());
        assert!(instance.heatmap_data.is_none());
        assert_eq!(instance.chart.heatmap_max_usd, 0.0);
    }

    fn source_context(
        terminal: &TradingTerminal,
        hydromancer_key_generation: Option<u64>,
    ) -> crate::read_data_provider::MarketDataSourceContext {
        crate::read_data_provider::MarketDataSourceContext {
            hydromancer_key_generation,
            ..terminal.market_data_source_context()
        }
    }

    #[test]
    fn stale_hydromancer_generation_does_not_update_chart_asset_context() {
        let mut terminal = terminal_with_chart();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_chart(Message::ChartWsAssetCtxUpdate(
            7,
            "BTC".to_string(),
            source_context(&terminal, Some(1)),
            asset_ctx("100"),
        ));

        assert!(terminal.charts[&7].asset_ctx.is_none());
    }

    #[test]
    fn current_hydromancer_generation_updates_chart_asset_context() {
        let mut terminal = terminal_with_chart();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_chart(Message::ChartWsAssetCtxUpdate(
            7,
            "BTC".to_string(),
            source_context(&terminal, Some(2)),
            asset_ctx("100"),
        ));

        assert_eq!(
            terminal.charts[&7]
                .asset_ctx
                .as_ref()
                .and_then(|ctx| ctx.mid_px.as_deref()),
            Some("100")
        );
    }

    #[test]
    fn current_asset_context_lag_clears_chart_context_and_spread_history() {
        let mut terminal = terminal_with_chart();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_chart(Message::ChartWsAssetCtxUpdate(
            7,
            "BTC".to_string(),
            source_context(&terminal, Some(2)),
            asset_ctx_with_impact("99", "101"),
        ));
        assert!(terminal.charts[&7].asset_ctx.is_some());
        assert!(!terminal.charts[&7].chart.spread_history.is_empty());

        let _task = terminal.update(Message::ChartWsAssetCtxLagged(
            7,
            "BTC".to_string(),
            source_context(&terminal, Some(2)),
            5,
        ));

        assert!(terminal.charts[&7].asset_ctx.is_none());
        assert!(terminal.charts[&7].chart.spread_history.is_empty());
    }

    #[test]
    fn stale_asset_context_lag_does_not_clear_chart_context() {
        let mut terminal = terminal_with_chart();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_chart(Message::ChartWsAssetCtxUpdate(
            7,
            "BTC".to_string(),
            source_context(&terminal, Some(2)),
            asset_ctx_with_impact("99", "101"),
        ));
        let _task = terminal.update(Message::ChartWsAssetCtxLagged(
            7,
            "BTC".to_string(),
            source_context(&terminal, Some(1)),
            5,
        ));

        assert!(terminal.charts[&7].asset_ctx.is_some());
        assert!(!terminal.charts[&7].chart.spread_history.is_empty());
    }

    #[test]
    fn chart_asset_context_ignores_inactive_provider_source() {
        let mut terminal = terminal_with_chart();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_chart(Message::ChartWsAssetCtxUpdate(
            7,
            "BTC".to_string(),
            source_context(&terminal, Some(2)),
            asset_ctx("100"),
        ));

        assert!(terminal.charts[&7].asset_ctx.is_none());

        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        let _task = terminal.update_chart(Message::ChartWsAssetCtxUpdate(
            7,
            "BTC".to_string(),
            source_context(&terminal, None),
            asset_ctx("101"),
        ));

        assert!(terminal.charts[&7].asset_ctx.is_none());
    }
}
