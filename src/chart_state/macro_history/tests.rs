use super::*;
use crate::chart_indicator::ChartIndicatorId;
use crate::chart_state::ChartInstance;
use crate::config::EmaCloudConfig;

const HOUR: u64 = 60 * MINUTE_MS;
const NOW: u64 = 20_000 * HOUR + HOUR / 2;

fn terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    let mut chart = ChartInstance::new(7, "BTC".into(), Timeframe::M5);
    chart.macro_indicators.sma_50h = true;
    chart.chart.macro_indicators = chart.macro_indicators.clone();
    terminal.charts.insert(7, chart);
    terminal
}

fn pending(terminal: &TradingTerminal, tf: Timeframe) -> MacroCandleRequest {
    terminal.chart_macro_history[&("BTC".into(), tf)]
        .pending
        .clone()
        .expect("pending history request")
}

fn hour(open: u64, price: f64) -> Candle {
    Candle::test_ohlcv(open, open + HOUR - 1, [price; 4], 1.0)
}

#[test]
fn independent_sources_and_detached_charts_share_inflight_history() {
    let mut terminal = terminal();
    assert_eq!(terminal.queue_macro_history(NOW).len(), 1);
    let hourly = pending(&terminal, Timeframe::H1);
    let detached = terminal.charts[&7].clone_for_detached_window(8);
    terminal.charts.insert(8, detached);
    let _ = terminal.update(Message::ToggleMacroIndicator(7, ChartIndicatorId::Ema50d));
    let _ = terminal.update(Message::ToggleMacroIndicator(7, ChartIndicatorId::Sessions));
    assert_eq!(pending(&terminal, Timeframe::H1), hourly);
    let daily = pending(&terminal, Timeframe::D1);
    assert!(terminal.queue_macro_history(NOW).is_empty());

    let _ = terminal.apply_macro_candles(hourly, Ok(vec![hour(NOW, 71.0)]), NOW);
    let first = &terminal.charts[&7].chart.hourly_candles;
    assert_eq!(first[0].close, 71.0);
    assert!(Arc::ptr_eq(
        first,
        &terminal.charts[&8].chart.hourly_candles
    ));
    assert_eq!(pending(&terminal, Timeframe::D1), daily);
}

#[test]
fn failed_and_empty_loads_retry_without_toggling_and_ignore_late_results() {
    let mut terminal = terminal();
    let _ = terminal.queue_macro_history(NOW);
    let failed = pending(&terminal, Timeframe::H1);
    let _ = terminal.apply_macro_candles(failed.clone(), Err("offline".into()), NOW);
    assert!(terminal.queue_macro_history(NOW + MINUTE_MS - 1).is_empty());
    assert_eq!(terminal.queue_macro_history(NOW + MINUTE_MS).len(), 1);
    let retry = pending(&terminal, Timeframe::H1);
    let _ = terminal.apply_macro_candles(failed, Ok(vec![hour(NOW, 9.0)]), NOW);
    assert!(terminal.charts[&7].chart.hourly_candles.is_empty());
    let _ = terminal.apply_macro_candles(retry, Ok(vec![]), NOW + MINUTE_MS);
    assert!(terminal.queue_macro_history(NOW + 2 * MINUTE_MS).is_empty());
    assert_eq!(terminal.queue_macro_history(NOW + 3 * MINUTE_MS).len(), 1);
    let retry = pending(&terminal, Timeframe::H1);
    let bars = (0..51)
        .map(|i| hour(NOW - (50 - i) * HOUR, 100.0))
        .collect();
    let _ = terminal.apply_macro_candles(retry, Ok(bars), NOW + 3 * MINUTE_MS);
    let source = &terminal.charts[&7].chart.hourly_candles;
    assert_eq!(source.len(), 51);
    assert_eq!(crate::chart::calculate_ema(source, 50).len(), 2);
}

#[test]
fn closed_history_renders_during_outage_and_only_overlapping_tail_is_refetched() {
    let mut terminal = terminal();
    let _ = terminal.queue_macro_history(NOW);
    let initial = pending(&terminal, Timeframe::H1);
    let first = initial.start_ms / HOUR * HOUR;
    let active = NOW / HOUR * HOUR;
    let cached = (first..=active)
        .step_by(HOUR as usize)
        .map(|t| hour(t, 10.0))
        .collect();
    let _ = terminal.apply_cached_macro_candles(initial, cached);
    let tail = pending(&terminal, Timeframe::H1);
    assert_eq!(tail.start_ms, active - 2 * HOUR);
    let history = &terminal.charts[&7].chart.hourly_candles;
    assert_eq!(history.first().expect("first").open_time, first);
    assert_eq!(history.last().expect("last").open_time, active - HOUR);
    let _ = terminal.apply_macro_candles(tail, Err("offline".into()), NOW);
    assert_eq!(terminal.charts[&7].chart.hourly_candles[0].close, 10.0);
    let _ = terminal.queue_macro_history(NOW + MINUTE_MS);
    let retry = pending(&terminal, Timeframe::H1);
    assert_eq!(retry.start_ms, active - 2 * HOUR);
    let _ = terminal.apply_macro_candles(
        retry,
        Ok(vec![
            hour(active - 2 * HOUR, 10.0),
            hour(active - HOUR, 12.0),
            hour(active, 14.0),
        ]),
        NOW + MINUTE_MS,
    );
    let bars = &terminal.charts[&7].chart.hourly_candles;
    assert_eq!(bars[0].open_time, first);
    assert_eq!(bars[bars.len() - 2].close, 12.0);
    assert_eq!(bars.last().expect("active bucket").close, 14.0);
    assert!(bars.windows(2).all(|p| p[0].open_time < p[1].open_time));
    assert!(terminal.queue_macro_history(NOW + 2 * MINUTE_MS).is_empty());
    assert_eq!(terminal.queue_macro_history(NOW + 16 * MINUTE_MS).len(), 1);
    assert_eq!(pending(&terminal, Timeframe::H1).start_ms, active - HOUR);
}

#[test]
fn larger_cloud_demand_gets_full_warmup_and_old_symbol_cannot_repopulate_chart() {
    let mut terminal = terminal();
    let _ = terminal.queue_macro_history(NOW);
    let initial = pending(&terminal, Timeframe::H1);
    let chart = terminal.charts.get_mut(&7).expect("chart");
    chart.macro_indicators.ema_clouds.push(EmaCloudConfig {
        timeframe: EmaCloudTimeframe::Hour,
        slow_period: 200,
        ..Default::default()
    });
    let _ = terminal.apply_macro_candles(initial, Ok(vec![hour(NOW, 14.0)]), NOW);
    let wider = pending(&terminal, Timeframe::H1);
    assert_eq!(wider.period, 200);
    assert_eq!(wider.start_ms, wider.history_start());

    let _ = terminal.update(Message::ChartSymbolSelected(7, "ETH".into()));
    let _ = terminal.apply_macro_candles(wider, Ok(vec![hour(NOW, 9.0)]), NOW);
    assert_eq!(terminal.charts[&7].symbol, "ETH");
    assert!(terminal.charts[&7].chart.hourly_candles.is_empty());
}

#[test]
fn weekly_chart_requests_enough_history_for_default_200_period_average() {
    let mut terminal = terminal();
    terminal
        .charts
        .insert(7, ChartInstance::new(7, "BTC".into(), Timeframe::W1));
    let _ = terminal.update(Message::ToggleMacroIndicator(7, ChartIndicatorId::TfSma200));
    let first = terminal.charts[&7]
        .candle_fetch_request
        .clone()
        .expect("weekly warmup");
    assert!(
        first.end_ms - first.start_ms
            >= Timeframe::W1.lookback_ms() + 200 * Timeframe::W1.duration_ms()
    );
    let _ = terminal.update(Message::ToggleMacroIndicator(7, ChartIndicatorId::TfEma200));
    assert_eq!(
        terminal.charts[&7].candle_fetch_request.as_ref(),
        Some(&first)
    );
}

fn macro_history_timeframes(config: &MacroIndicatorsConfig) -> Vec<Timeframe> {
    fixed_average_periods(config).map(|(tf, _)| tf).collect()
}

#[test]
fn ema_cloud_history_uses_enabled_sources_and_custom_lookback() {
    use crate::config::{EmaCloudConfig, EmaCloudTimeframe};
    let mut config = crate::config::MacroIndicatorsConfig {
        ema_clouds: vec![
            EmaCloudConfig {
                id: 1,
                timeframe: EmaCloudTimeframe::Hour,
                slow_period: 1000,
                ..Default::default()
            },
            EmaCloudConfig {
                id: 2,
                timeframe: EmaCloudTimeframe::Hour,
                ..Default::default()
            },
            EmaCloudConfig {
                id: 3,
                timeframe: EmaCloudTimeframe::Day,
                enabled: false,
                ..Default::default()
            },
            EmaCloudConfig {
                id: 4,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert_eq!(macro_history_timeframes(&config), vec![Timeframe::H1]);
    assert_eq!(
        crate::chart_indicator::moving_average_lookback_ms(&config, Timeframe::H1, "hourly"),
        Timeframe::H1.lookback_ms() + 3000 * Timeframe::H1.duration_ms()
    );
    config.ema_clouds[2].enabled = true;
    assert_eq!(
        macro_history_timeframes(&config),
        vec![Timeframe::H1, Timeframe::D1]
    );
    config.ema_clouds[0].slow_period = 5000;
    assert_eq!(
        crate::chart_indicator::moving_average_lookback_ms(&config, Timeframe::H1, "hourly"),
        5000 * Timeframe::H1.duration_ms()
    );
}

#[test]
fn macro_history_is_only_requested_for_enabled_intervals() {
    let mut config = crate::config::MacroIndicatorsConfig::default();
    assert!(macro_history_timeframes(&config).is_empty());
    config.sma_50h = true;
    config.ema_200h = true;
    assert_eq!(
        macro_history_timeframes(&config),
        vec![crate::timeframe::Timeframe::H1]
    );
    config.sma_12m = true;
    assert_eq!(
        macro_history_timeframes(&config),
        vec![
            crate::timeframe::Timeframe::H1,
            crate::timeframe::Timeframe::Mo1
        ]
    );
}

#[test]
fn short_primary_response_fetches_missing_seed_without_panning_or_toggling() {
    let mut terminal = terminal();
    terminal
        .charts
        .insert(7, ChartInstance::new(7, "BTC".into(), Timeframe::H1));
    let _ = terminal.update(Message::ToggleMacroIndicator(7, ChartIndicatorId::TfEma50));
    let refresh = terminal.charts[&7]
        .candle_fetch_request
        .clone()
        .expect("refresh");
    let oldest = NOW - 49 * HOUR;
    let partial = (0..49).map(|i| hour(oldest + i * HOUR, 100.0)).collect();
    let _ = terminal.update(Message::ChartCandlesLoaded(refresh, Ok(partial)));
    let older = terminal.charts[&7]
        .candle_fetch_request
        .clone()
        .expect("missing seed");
    assert_eq!(older.mode, crate::chart_state::CandleFetchMode::WarmupOlder);
    assert_eq!(older.end_ms, oldest - 1);
    assert_eq!(older.end_ms - older.start_ms, 2 * HOUR);
    let _ = terminal.update(Message::ChartCandlesLoaded(
        older,
        Ok(vec![
            hour(oldest - 2 * HOUR, 100.0),
            hour(oldest - HOUR, 100.0),
        ]),
    ));
    let chart = &terminal.charts[&7];
    assert!(chart.candle_fetch_request.is_none());
    assert_eq!(
        crate::chart::calculate_ema(&chart.chart.candles, 50).len(),
        2
    );
}

#[test]
fn disabling_last_overlay_suspends_refresh_and_reuses_history_on_enable() {
    let mut terminal = terminal();
    let _ = terminal.queue_macro_history(NOW);
    let request = pending(&terminal, Timeframe::H1);
    let _ = terminal.apply_macro_candles(request, Ok(vec![hour(NOW, 100.0)]), NOW);
    let before = terminal.charts[&7].chart.hourly_candles.clone();
    terminal
        .charts
        .get_mut(&7)
        .expect("chart")
        .macro_indicators
        .sma_50h = false;
    assert!(terminal.queue_macro_history(NOW + MINUTE_MS).is_empty());
    terminal
        .charts
        .get_mut(&7)
        .expect("chart")
        .macro_indicators
        .sma_50h = true;
    assert!(terminal.queue_macro_history(NOW + 2 * MINUTE_MS).is_empty());
    assert!(Arc::ptr_eq(
        &before,
        &terminal.charts[&7].chart.hourly_candles
    ));
    terminal.charts.clear();
    assert!(terminal.queue_macro_history(NOW + HOUR).is_empty());
    assert!(terminal.chart_macro_history.is_empty());
}

#[test]
fn short_disk_cache_and_late_reply_from_replaced_request_cannot_suppress_history() {
    let mut terminal = terminal();
    let _ = terminal.queue_macro_history(NOW);
    let old = pending(&terminal, Timeframe::H1);
    terminal
        .charts
        .get_mut(&7)
        .expect("chart")
        .macro_indicators
        .sma_50h = false;
    let _ = terminal.queue_macro_history(NOW);
    terminal
        .charts
        .get_mut(&7)
        .expect("chart")
        .macro_indicators
        .sma_50h = true;
    let _ = terminal.queue_macro_history(NOW);
    let current = pending(&terminal, Timeframe::H1);
    assert_ne!(current.request_id, old.request_id);
    let _ = terminal.apply_cached_macro_candles(old, vec![hour(NOW - HOUR, 2.0)]);
    assert!(terminal.charts[&7].chart.hourly_candles.is_empty());
    let _ = terminal.apply_cached_macro_candles(current, vec![hour(NOW - HOUR, 5.0)]);
    let network = pending(&terminal, Timeframe::H1);
    assert_eq!(network.start_ms, network.history_start());
    assert_eq!(terminal.charts[&7].chart.hourly_candles[0].close, 5.0);
}

#[test]
fn a_short_full_range_reply_preserves_the_overlapping_closed_history() {
    let mut terminal = terminal();
    let _ = terminal.queue_macro_history(NOW);
    let request = pending(&terminal, Timeframe::H1);
    let cached = (1..=100)
        .rev()
        .map(|i| hour(NOW - i * HOUR, 10.0))
        .collect();
    let _ = terminal.apply_cached_macro_candles(request, cached);
    let refresh = pending(&terminal, Timeframe::H1);
    assert_eq!(refresh.start_ms, refresh.history_start());
    let _ = terminal.apply_macro_candles(
        refresh,
        Ok(vec![hour(NOW - HOUR, 12.0), hour(NOW, 14.0)]),
        NOW,
    );
    let bars = &terminal.charts[&7].chart.hourly_candles;
    assert_eq!(bars.len(), 101);
    assert_eq!(
        (bars[0].close, bars[99].close, bars[100].close),
        (10.0, 12.0, 14.0)
    );
}
