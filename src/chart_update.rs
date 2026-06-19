use crate::app_state::TradingTerminal;
use crate::chart::HudSelectorKind;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::sound;
use iced::Task;

mod candles;
mod detached;
mod earnings;
mod editor;
mod macro_indicators;

/// How often a chart whose `asset_ctx` is REST-sourced re-fetches it. Kept
/// below `MARKET_ASSET_CONTEXT_MAX_AGE_MS` (15s) so the context is refreshed
/// before the staleness expiry would blank the header metrics.
const CHART_ASSET_CONTEXT_REST_REFRESH_MS: u64 = 10_000;

impl TradingTerminal {
    pub(crate) fn clear_chart_market_display_state(instance: &mut ChartInstance) {
        instance.heatmap_last_fetch = None;
        instance.heatmap_viewport = None;
        instance.heatmap_status = None;
        instance.heatmap_fetching = false;
        instance.candle_backfill_exhausted = false;
        instance.secondary_candle_backfill_exhausted = false;
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
            | Message::ChartSecondarySymbolSelected(_, _)
            | Message::ChartSecondarySymbolRemoved(_)
            | Message::ToggleChartInvert(_)
            | Message::ToggleChartTradeMarkers(_)
            | Message::ChartOpenEditor(_)
            | Message::ChartCloseEditor(_)
            | Message::ChartEditorSearchChanged(_, _)
            | Message::ChartEditorSubmit(_)
            | Message::ChartSecondaryOpenEditor(_)
            | Message::ChartSecondaryCloseEditor(_)
            | Message::ChartSecondaryEditorSearchChanged(_, _)
            | Message::ChartSecondaryEditorSubmit(_)
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
            Message::ToggleChartDrawingToolbar(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.drawing_toolbar_collapsed = !instance.drawing_toolbar_collapsed;
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
            | Message::ChartSecondaryCandlesLoaded(_, _)
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
            Message::ChartAssetContextRestFetched(id, symbol, result) => {
                let hidden = self.symbol_key_is_hidden(&symbol);
                let now_ms = Self::now_ms();
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.asset_ctx_rest_in_flight = false;
                    if instance.symbol != symbol || hidden {
                        return Task::none();
                    }
                    // Fill only when there is no context, or the existing one is
                    // itself REST-sourced — never clobber a live WebSocket push.
                    if let Ok(Some(ctx)) = result
                        && (instance.asset_ctx.is_none() || instance.asset_ctx_from_rest)
                    {
                        instance.fill_asset_context_from_rest(ctx, now_ms);
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
                let backfill_task = self.maybe_backfill_chart_candles_for_viewport(id, viewport);
                if should_fetch {
                    return Task::batch([self.maybe_fetch_heatmap(id), backfill_task]);
                }
                return backfill_task;
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

    /// Issue REST `metaAndAssetCtxs` fetches for charts whose `asset_ctx` is
    /// missing or whose REST-sourced context is approaching staleness. This
    /// backstops the `activeAssetCtx` WebSocket stream — notably for HIP-3
    /// `dex:coin` perps, whose context the stream may not deliver — so the
    /// header's 24h-volume and open-interest metrics keep rendering. Live
    /// WebSocket context always takes precedence (see the fetch handler).
    pub(crate) fn queue_chart_asset_context_rest_fetches(
        &mut self,
        now_ms: u64,
    ) -> Vec<Task<Message>> {
        let targets: Vec<(ChartId, String)> = self
            .charts
            .values()
            .filter(|instance| {
                instance.needs_rest_asset_context(now_ms, CHART_ASSET_CONTEXT_REST_REFRESH_MS)
                    && !self.symbol_key_is_hidden(&instance.symbol)
            })
            .map(|instance| (instance.id, instance.symbol.clone()))
            .collect();

        targets
            .into_iter()
            .map(|(id, symbol)| {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.asset_ctx_rest_in_flight = true;
                }
                let fetch_symbol = symbol.clone();
                Task::perform(
                    crate::api::fetch_chart_asset_context(fetch_symbol),
                    move |result| Message::ChartAssetContextRestFetched(id, symbol.clone(), result),
                )
            })
            .collect()
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
    fn chart_asset_context_gates_provider_source() {
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

        assert_eq!(
            terminal.charts[&7]
                .asset_ctx
                .as_ref()
                .and_then(|ctx| ctx.mid_px.as_deref()),
            Some("101")
        );
    }

    fn asset_ctx_with_metrics(open_interest: &str, day_ntl_vlm: &str) -> AssetContext {
        AssetContext {
            funding: Some("0.0000125".to_string()),
            open_interest: Some(open_interest.to_string()),
            oracle_px: None,
            mark_px: None,
            mid_px: None,
            prev_day_px: None,
            day_ntl_vlm: Some(day_ntl_vlm.to_string()),
            day_base_vlm: None,
            impact_pxs: None,
        }
    }

    fn terminal_with_hip3_chart() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal.charts.insert(
            7,
            ChartInstance::new(7, "xyz:NVDA".to_string(), Timeframe::H1),
        );
        terminal
    }

    #[test]
    fn rest_fetch_fills_missing_hip3_asset_context() {
        let mut terminal = terminal_with_hip3_chart();
        if let Some(instance) = terminal.charts.get_mut(&7) {
            instance.asset_ctx_rest_in_flight = true;
        }

        let _task = terminal.update_chart(Message::ChartAssetContextRestFetched(
            7,
            "xyz:NVDA".to_string(),
            Ok(Some(asset_ctx_with_metrics("11560.744", "987654.0"))),
        ));

        let instance = &terminal.charts[&7];
        assert!(!instance.asset_ctx_rest_in_flight);
        let ctx = instance.asset_ctx.as_ref().expect("rest-filled asset_ctx");
        assert_eq!(ctx.open_interest.as_deref(), Some("11560.744"));
        assert_eq!(ctx.day_ntl_vlm.as_deref(), Some("987654.0"));
        assert!(instance.asset_ctx_from_rest);
    }

    #[test]
    fn rest_fetch_does_not_clobber_live_ws_context() {
        let mut terminal = terminal_with_hip3_chart();
        let now_ms = TradingTerminal::now_ms();
        if let Some(instance) = terminal.charts.get_mut(&7) {
            // Live WebSocket context (set_asset_context_at marks it WS-sourced).
            instance.set_asset_context_at(Some(asset_ctx_with_metrics("1.0", "2.0")), now_ms);
        }

        let _task = terminal.update_chart(Message::ChartAssetContextRestFetched(
            7,
            "xyz:NVDA".to_string(),
            Ok(Some(asset_ctx_with_metrics("9999.0", "8888.0"))),
        ));

        let instance = &terminal.charts[&7];
        let ctx = instance.asset_ctx.as_ref().expect("ws asset_ctx preserved");
        assert_eq!(ctx.open_interest.as_deref(), Some("1.0"));
        assert!(!instance.asset_ctx_from_rest);
    }

    #[test]
    fn rest_fetch_ignored_after_symbol_change() {
        let mut terminal = terminal_with_hip3_chart();
        if let Some(instance) = terminal.charts.get_mut(&7) {
            instance.asset_ctx_rest_in_flight = true;
        }

        // The fetch resolves for a symbol the chart no longer displays.
        let _task = terminal.update_chart(Message::ChartAssetContextRestFetched(
            7,
            "xyz:TSLA".to_string(),
            Ok(Some(asset_ctx_with_metrics("1.0", "2.0"))),
        ));

        let instance = &terminal.charts[&7];
        assert!(instance.asset_ctx.is_none());
        assert!(!instance.asset_ctx_rest_in_flight);
    }

    #[test]
    fn status_tick_queues_rest_fetch_only_when_context_missing() {
        let mut terminal = terminal_with_hip3_chart();
        let now_ms = TradingTerminal::now_ms();

        let tasks = terminal.queue_chart_asset_context_rest_fetches(now_ms);
        assert_eq!(tasks.len(), 1);
        assert!(terminal.charts[&7].asset_ctx_rest_in_flight);

        // A fetch already in flight is not duplicated.
        let tasks = terminal.queue_chart_asset_context_rest_fetches(now_ms);
        assert!(tasks.is_empty());
    }

    #[test]
    fn status_tick_skips_rest_fetch_when_live_ws_context_present() {
        let mut terminal = terminal_with_hip3_chart();
        let now_ms = TradingTerminal::now_ms();
        if let Some(instance) = terminal.charts.get_mut(&7) {
            instance.set_asset_context_at(Some(asset_ctx("100")), now_ms);
        }

        let tasks = terminal.queue_chart_asset_context_rest_fetches(now_ms);
        assert!(tasks.is_empty());
        assert!(!terminal.charts[&7].asset_ctx_rest_in_flight);
    }

    #[test]
    fn rest_refresh_eligibility_respects_provenance_and_age() {
        let now_ms = 1_000_000;
        let refresh_ms = CHART_ASSET_CONTEXT_REST_REFRESH_MS;
        let mut instance = ChartInstance::new(7, "xyz:NVDA".to_string(), Timeframe::H1);

        // No context yet -> eligible.
        assert!(instance.needs_rest_asset_context(now_ms, refresh_ms));

        // Fresh REST context -> not yet due; aged REST context -> due for refresh.
        instance.fill_asset_context_from_rest(asset_ctx("100"), now_ms);
        assert!(!instance.needs_rest_asset_context(now_ms, refresh_ms));
        assert!(instance.needs_rest_asset_context(now_ms + refresh_ms, refresh_ms));

        // Live WebSocket context is never refreshed by the poller, even when old.
        instance.set_asset_context_at(Some(asset_ctx("100")), now_ms);
        assert!(!instance.needs_rest_asset_context(now_ms + refresh_ms * 10, refresh_ms));

        // An in-flight fetch suppresses duplicates.
        instance.set_asset_context(None);
        instance.asset_ctx_rest_in_flight = true;
        assert!(!instance.needs_rest_asset_context(now_ms, refresh_ms));
    }
}
