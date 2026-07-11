mod creation;
mod data;
mod editor;
mod pair;
mod session;
mod style;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_spaghetti(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddComparisonChart => self.add_comparison_chart(),
            Message::AddPairRatioChart => self.add_pair_ratio_chart(),
            Message::SpaghettiReload(id) => self.reload_spaghetti_chart(id),
            Message::SpaghettiSwitchTimeframe(id, tf) => self.switch_spaghetti_timeframe(id, tf),
            Message::SpaghettiCandlesLoaded(request, result) => {
                self.apply_spaghetti_candles_loaded(request, result.into_result())
            }
            Message::SpaghettiWsCandleUpdate(context, candle) => {
                self.apply_spaghetti_ws_candle_update(context, candle)
            }
            Message::SpaghettiWsCandleLagged(context, _skipped) => {
                self.reload_spaghetti_chart_after_ws_lag(context)
            }
            Message::SpaghettiOpenEditor(id) => self.open_spaghetti_editor(id),
            Message::SpaghettiCloseEditor(id) => self.close_spaghetti_editor(id),
            Message::SpaghettiEditorSearchChanged(id, query) => {
                self.update_spaghetti_editor_search(id, query)
            }
            Message::SpaghettiAddSymbol(id, key) => self.add_spaghetti_symbol(id, key),
            Message::SpaghettiRemoveSymbol(id, symbol) => self.remove_spaghetti_symbol(id, symbol),
            Message::SpaghettiSetSession(id, session) => self.set_spaghetti_session(id, session),
            Message::SpaghettiSetSessionGranularityAuto(id) => {
                self.set_spaghetti_session_granularity_auto(id)
            }
            Message::SpaghettiResetView(id) => self.reset_spaghetti_view(id),
            Message::ToggleSpaghettiStyleMenu(id) => self.toggle_spaghetti_style_menu(id),
            Message::ToggleSpaghettiLabels(id) => self.toggle_spaghetti_labels(id),
            Message::SpaghettiSetColorMode(id, mode) => self.set_spaghetti_color_mode(id, mode),
            Message::PairSetCandleMode(id, enabled) => self.set_pair_candle_mode(id, enabled),
            _ => Task::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Candle;
    use crate::config::{ChartBackfillSource, ReadDataProvider};
    use crate::spaghetti::{Series, Session};
    use crate::spaghetti_state::{SpaghettiChartInstance, SpaghettiWsCandleContext};
    use crate::timeframe::Timeframe;
    use iced::Color;

    #[test]
    fn spaghetti_candle_lagged_reloads_chart_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.series.push(Series {
            symbol: "BTC".to_string(),
            display: "BTC".to_string(),
            candles: vec![Candle::test_ohlcv(
                1_000,
                61_000,
                [100.0, 100.0, 100.0, 100.0],
                1.0,
            )],
            color: Color::BLACK,
            loaded: true,
        });
        terminal.spaghetti_charts.insert(7, instance);

        let task = terminal.update_spaghetti(Message::SpaghettiWsCandleLagged(
            ws_context(&terminal, 7, "BTC", None),
            2,
        ));

        assert_eq!(task.units(), 1);
        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert!(series.candles.is_empty());
        assert!(!series.loaded);
    }

    #[test]
    fn queued_spaghetti_candle_lags_reload_chart_once() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.series.push(loaded_series("BTC"));
        instance.canvas.series.push(loaded_series("ETH"));
        terminal.spaghetti_charts.insert(7, instance);

        let first_task = terminal.update_spaghetti(Message::SpaghettiWsCandleLagged(
            ws_context(&terminal, 7, "BTC", None),
            2,
        ));
        let second_task = terminal.update_spaghetti(Message::SpaghettiWsCandleLagged(
            ws_context(&terminal, 7, "ETH", None),
            3,
        ));

        assert_eq!(first_task.units(), 2);
        assert_eq!(second_task.units(), 0);
        for series in &terminal.spaghetti_charts[&7].canvas.series {
            assert!(series.candles.is_empty());
            assert!(!series.loaded);
        }
    }

    #[test]
    fn stale_spaghetti_candle_lag_for_removed_symbol_does_not_reload_chart() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let task = terminal.update_spaghetti(Message::SpaghettiWsCandleLagged(
            ws_context(&terminal, 7, "ETH", None),
            2,
        ));

        assert_eq!(task.units(), 0);
        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert!(series.loaded);
        assert!(!series.candles.is_empty());
    }

    #[test]
    fn stale_spaghetti_candle_lag_for_hidden_symbol_does_not_reload_chart() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();
        terminal.muted_tickers.insert("BTC".to_string());

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let task = terminal.update_spaghetti(Message::SpaghettiWsCandleLagged(
            ws_context(&terminal, 7, "BTC", None),
            2,
        ));

        assert_eq!(task.units(), 0);
        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert!(series.loaded);
        assert!(!series.candles.is_empty());
    }

    #[test]
    fn prior_series_request_does_not_consume_readded_spaghetti_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.series.push(Series {
            symbol: "BTC".to_string(),
            display: "BTC".to_string(),
            candles: Vec::new(),
            color: Color::BLACK,
            loaded: false,
        });
        let old_request_id = instance
            .begin_spaghetti_candle_request("BTC")
            .expect("old request owner");
        let current_request_id = instance
            .begin_spaghetti_candle_request("BTC")
            .expect("replacement request owner");
        terminal.spaghetti_charts.insert(7, instance);

        let old_request = crate::spaghetti_state::SpaghettiCandleFetch {
            chart_id: 7,
            chart_instance_generation: terminal.chart_instance_generation,
            request_id: old_request_id,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            session: None,
            session_granularity: None,
        };
        let current_request = crate::spaghetti_state::SpaghettiCandleFetch {
            request_id: current_request_id,
            ..old_request.clone()
        };

        let _task = terminal.apply_spaghetti_candles_loaded(
            old_request.clone(),
            Ok(vec![Candle::test_flat(1_000, 100.0)]),
        );

        let instance = &terminal.spaghetti_charts[&7];
        assert_eq!(
            instance.pending_spaghetti_candle_request_id("BTC"),
            Some(current_request_id)
        );
        let series = &instance.canvas.series[0];
        assert!(series.candles.is_empty());
        assert!(!series.loaded);

        let _task = terminal.apply_spaghetti_candles_loaded(
            current_request,
            Ok(vec![Candle::test_flat(2_000, 200.0)]),
        );

        let instance = &terminal.spaghetti_charts[&7];
        assert_eq!(instance.pending_spaghetti_candle_request_id("BTC"), None);
        let series = &instance.canvas.series[0];
        assert_eq!(series.candles.len(), 1);
        assert_eq!(series.candles[0].close, 200.0);
        assert!(series.loaded);
        assert_eq!(
            terminal
                .get_cached_candles("BTC", Timeframe::H1)
                .and_then(|candles| candles.last().map(|candle| candle.close)),
            Some(200.0)
        );

        let _task = terminal
            .apply_spaghetti_candles_loaded(old_request, Err("stale fetch error".to_string()));

        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert_eq!(series.candles.len(), 1);
        assert_eq!(series.candles[0].close, 200.0);
        assert!(series.loaded);
        assert_eq!(
            terminal
                .get_cached_candles("BTC", Timeframe::H1)
                .and_then(|candles| candles.last().map(|candle| candle.close)),
            Some(200.0)
        );
    }

    #[test]
    fn remove_and_readd_spaghetti_series_installs_distinct_request_owner() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();
        terminal
            .spaghetti_charts
            .insert(7, SpaghettiChartInstance::new_empty(7));

        let first_task =
            terminal.update_spaghetti(Message::SpaghettiAddSymbol(7, "BTC".to_string()));
        let first_request_id = terminal.spaghetti_charts[&7]
            .pending_spaghetti_candle_request_id("BTC")
            .expect("first request owner");

        let _task = terminal.update_spaghetti(Message::SpaghettiRemoveSymbol(7, "BTC".to_string()));
        assert_eq!(
            terminal.spaghetti_charts[&7].pending_spaghetti_candle_request_id("BTC"),
            None
        );

        let second_task =
            terminal.update_spaghetti(Message::SpaghettiAddSymbol(7, "BTC".to_string()));
        let second_request_id = terminal.spaghetti_charts[&7]
            .pending_spaghetti_candle_request_id("BTC")
            .expect("replacement request owner");

        assert_eq!(first_task.units(), 1);
        assert_eq!(second_task.units(), 1);
        assert_ne!(first_request_id, second_request_id);
    }

    #[test]
    fn prior_chart_incarnation_result_does_not_consume_recreated_spaghetti_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();
        terminal.chart_instance_generation = 2;

        let mut replacement = SpaghettiChartInstance::new_empty(7);
        replacement.canvas.series.push(Series {
            symbol: "BTC".to_string(),
            display: "BTC".to_string(),
            candles: Vec::new(),
            color: Color::BLACK,
            loaded: false,
        });
        let request_id = replacement
            .begin_spaghetti_candle_request("BTC")
            .expect("replacement request owner");
        terminal.spaghetti_charts.insert(7, replacement);

        let old_request = crate::spaghetti_state::SpaghettiCandleFetch {
            chart_id: 7,
            chart_instance_generation: 1,
            request_id,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            session: None,
            session_granularity: None,
        };
        let current_request = crate::spaghetti_state::SpaghettiCandleFetch {
            chart_instance_generation: 2,
            ..old_request.clone()
        };

        let _task = terminal
            .apply_spaghetti_candles_loaded(old_request, Ok(vec![Candle::test_flat(1_000, 100.0)]));

        let instance = &terminal.spaghetti_charts[&7];
        assert_eq!(
            instance.pending_spaghetti_candle_request_id("BTC"),
            Some(request_id)
        );
        assert!(instance.canvas.series[0].candles.is_empty());

        let _task = terminal.apply_spaghetti_candles_loaded(
            current_request,
            Ok(vec![Candle::test_flat(2_000, 200.0)]),
        );

        let instance = &terminal.spaghetti_charts[&7];
        assert_eq!(instance.pending_spaghetti_candle_request_id("BTC"), None);
        assert_eq!(instance.canvas.series[0].candles[0].close, 200.0);
    }

    #[test]
    fn stale_hydromancer_generation_does_not_update_spaghetti_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();
        terminal.hydromancer_key_generation = 2;
        terminal.chart_backfill_source = ChartBackfillSource::Hydromancer;

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.interval = Timeframe::H1;
        instance.canvas.series.push(Series {
            symbol: "BTC".to_string(),
            display: "BTC".to_string(),
            candles: Vec::new(),
            color: Color::BLACK,
            loaded: false,
        });
        let request_id = instance
            .begin_spaghetti_candle_request("BTC")
            .expect("request owner");
        terminal.spaghetti_charts.insert(7, instance);

        let request = crate::spaghetti_state::SpaghettiCandleFetch {
            chart_id: 7,
            chart_instance_generation: terminal.chart_instance_generation,
            request_id,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            source: ChartBackfillSource::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: 1,
            session: None,
            session_granularity: None,
        };

        let _task = terminal.update_spaghetti(Message::SpaghettiCandlesLoaded(
            request,
            Ok(vec![Candle::test_flat(0, 100.0)]).into(),
        ));

        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert!(series.candles.is_empty());
        assert!(!series.loaded);
    }

    #[test]
    fn stale_hyperliquid_generation_does_not_update_spaghetti_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.interval = Timeframe::H1;
        instance.canvas.series.push(Series {
            symbol: "BTC".to_string(),
            display: "BTC".to_string(),
            candles: Vec::new(),
            color: Color::BLACK,
            loaded: false,
        });
        let request_id = instance
            .begin_spaghetti_candle_request("BTC")
            .expect("request owner");
        terminal.spaghetti_charts.insert(7, instance);

        let request = crate::spaghetti_state::SpaghettiCandleFetch {
            chart_id: 7,
            chart_instance_generation: terminal.chart_instance_generation,
            request_id,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            session: None,
            session_granularity: None,
        };

        terminal.bump_read_data_provider_generation();
        let _task = terminal.update_spaghetti(Message::SpaghettiCandlesLoaded(
            request,
            Ok(vec![Candle::test_flat(0, 100.0)]).into(),
        ));

        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert!(series.candles.is_empty());
        assert!(!series.loaded);
    }

    #[test]
    fn stale_hydromancer_ws_generation_does_not_update_spaghetti_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let _task = terminal.update_spaghetti(Message::SpaghettiWsCandleUpdate(
            ws_context(&terminal, 7, "BTC", Some(1)),
            Candle::test_flat(2_000, 110.0),
        ));

        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert_eq!(series.candles.len(), 1);
        assert_eq!(series.candles[0].close, 100.0);
    }

    #[test]
    fn stale_hyperliquid_ws_generation_does_not_update_spaghetti_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);
        let stale_context = ws_context(&terminal, 7, "BTC", None);
        terminal.bump_read_data_provider_generation();

        let _task = terminal.update_spaghetti(Message::SpaghettiWsCandleUpdate(
            stale_context,
            Candle::test_flat(2_000, 110.0),
        ));

        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert_eq!(series.candles.len(), 1);
        assert_eq!(series.candles[0].close, 100.0);
    }

    #[test]
    fn stale_hydromancer_ws_lag_does_not_reload_spaghetti_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let task = terminal.update_spaghetti(Message::SpaghettiWsCandleLagged(
            ws_context(&terminal, 7, "BTC", Some(1)),
            2,
        ));

        assert_eq!(task.units(), 0);
        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert!(series.loaded);
        assert!(!series.candles.is_empty());
    }

    #[test]
    fn spaghetti_ws_update_gates_provider_source() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();
        terminal.hydromancer_key_generation = 2;

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let _task = terminal.update_spaghetti(Message::SpaghettiWsCandleUpdate(
            ws_context(&terminal, 7, "BTC", Some(2)),
            Candle::test_flat(2_000, 110.0),
        ));

        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert_eq!(series.candles.len(), 1);
        assert_eq!(series.candles[0].close, 100.0);

        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        let _task = terminal.update_spaghetti(Message::SpaghettiWsCandleUpdate(
            ws_context(&terminal, 7, "BTC", None),
            Candle::test_flat(3_000, 120.0),
        ));

        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert_eq!(series.candles.len(), 2);
        assert_eq!(series.candles[1].close, 120.0);
    }

    #[test]
    fn spaghetti_ws_lag_gates_provider_source() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();
        terminal.hydromancer_key_generation = 2;

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let task = terminal.update_spaghetti(Message::SpaghettiWsCandleLagged(
            ws_context(&terminal, 7, "BTC", Some(2)),
            2,
        ));

        assert_eq!(task.units(), 0);
        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert!(series.loaded);
        assert!(!series.candles.is_empty());

        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        let task = terminal.update_spaghetti(Message::SpaghettiWsCandleLagged(
            ws_context(&terminal, 7, "BTC", None),
            2,
        ));

        assert_eq!(task.units(), 1);
        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert!(!series.loaded);
        assert!(series.candles.is_empty());
    }

    #[test]
    fn stale_spaghetti_ws_timeframe_does_not_update_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.interval = Timeframe::M5;
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let _task = terminal.update_spaghetti(Message::SpaghettiWsCandleUpdate(
            ws_context(&terminal, 7, "BTC", None),
            Candle::test_flat(2_000, 110.0),
        ));

        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert_eq!(series.candles.len(), 1);
        assert_eq!(series.candles[0].close, 100.0);
    }

    #[test]
    fn stale_spaghetti_ws_timeframe_lag_does_not_reload_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.interval = Timeframe::M5;
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let task = terminal.update_spaghetti(Message::SpaghettiWsCandleLagged(
            ws_context(&terminal, 7, "BTC", None),
            2,
        ));

        assert_eq!(task.units(), 0);
        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert!(series.loaded);
        assert!(!series.candles.is_empty());
    }

    #[test]
    fn stale_spaghetti_ws_session_context_does_not_update_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.active_session = Some(Session::UtcDay);
        instance.session_granularity = Some(Timeframe::H1);
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let _task = terminal.update_spaghetti(Message::SpaghettiWsCandleUpdate(
            ws_context(&terminal, 7, "BTC", None),
            Candle::test_flat(2_000, 110.0),
        ));

        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert_eq!(series.candles.len(), 1);
        assert_eq!(series.candles[0].close, 100.0);
    }

    #[test]
    fn stale_spaghetti_ws_session_context_lag_does_not_reload_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.active_session = Some(Session::UtcDay);
        instance.session_granularity = Some(Timeframe::H1);
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let task = terminal.update_spaghetti(Message::SpaghettiWsCandleLagged(
            ws_context(&terminal, 7, "BTC", None),
            2,
        ));

        assert_eq!(task.units(), 0);
        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert!(series.loaded);
        assert!(!series.candles.is_empty());
    }

    #[test]
    fn stale_spaghetti_ws_session_granularity_does_not_update_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.active_session = Some(Session::UtcDay);
        instance.session_granularity = Some(Timeframe::H1);
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let mut context = ws_context(&terminal, 7, "BTC", None);
        context.session = Some(Session::UtcDay);
        context.session_granularity = None;
        let _task = terminal.update_spaghetti(Message::SpaghettiWsCandleUpdate(
            context,
            Candle::test_flat(2_000, 110.0),
        ));

        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert_eq!(series.candles.len(), 1);
        assert_eq!(series.candles[0].close, 100.0);
    }

    #[test]
    fn stale_spaghetti_ws_session_granularity_lag_does_not_reload_series() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();

        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.active_session = Some(Session::UtcDay);
        instance.session_granularity = Some(Timeframe::H1);
        instance.canvas.series.push(loaded_series("BTC"));
        terminal.spaghetti_charts.insert(7, instance);

        let mut context = ws_context(&terminal, 7, "BTC", None);
        context.session = Some(Session::UtcDay);
        context.session_granularity = None;
        let task = terminal.update_spaghetti(Message::SpaghettiWsCandleLagged(context, 2));

        assert_eq!(task.units(), 0);
        let series = &terminal.spaghetti_charts[&7].canvas.series[0];
        assert!(series.loaded);
        assert!(!series.candles.is_empty());
    }

    fn loaded_series(symbol: &str) -> Series {
        Series {
            symbol: symbol.to_string(),
            display: symbol.to_string(),
            candles: vec![Candle::test_ohlcv(
                1_000,
                61_000,
                [100.0, 100.0, 100.0, 100.0],
                1.0,
            )],
            color: Color::BLACK,
            loaded: true,
        }
    }

    fn ws_context(
        terminal: &TradingTerminal,
        chart_id: u64,
        symbol: &str,
        hydromancer_key_generation: Option<u64>,
    ) -> SpaghettiWsCandleContext {
        SpaghettiWsCandleContext {
            chart_id,
            symbol: symbol.to_string(),
            timeframe: Timeframe::H1,
            source_context: crate::read_data_provider::MarketDataSourceContext {
                hydromancer_key_generation,
                ..terminal.market_data_source_context()
            },
            session: None,
            session_granularity: None,
        }
    }
}
