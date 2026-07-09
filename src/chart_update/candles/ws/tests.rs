use super::*;
use crate::chart_state::ChartInstance;
use crate::config::ReadDataProvider;
use crate::read_data_provider::MarketDataSourceContext;
use crate::timeframe::Timeframe;

fn spot_symbol(key: &str) -> crate::api::ExchangeSymbol {
    crate::api::ExchangeSymbol {
        key: key.to_string(),
        ticker: "SPOT".to_string(),
        category: "spot".to_string(),
        display_name: Some("SPOT/USDC".to_string()),
        keywords: vec!["spot".to_string()],
        asset_index: 10_003,
        collateral_token: Some(crate::api::USDC_TOKEN_INDEX),
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: crate::api::MarketType::Spot,
        outcome: None,
    }
}

fn candle(open_time: u64, close: f64) -> Candle {
    Candle::test_ohlcv(
        open_time,
        open_time + 60_000,
        [close, close, close, close],
        1.0,
    )
}

fn last_close(terminal: &TradingTerminal, id: ChartId) -> Option<f64> {
    terminal
        .charts
        .get(&id)
        .expect("chart")
        .chart
        .candles
        .last()
        .map(|candle| candle.close)
}

fn source_context(
    terminal: &TradingTerminal,
    hydromancer_key_generation: Option<u64>,
) -> MarketDataSourceContext {
    MarketDataSourceContext {
        hydromancer_key_generation,
        ..terminal.market_data_source_context()
    }
}

#[test]
fn ws_candle_update_fans_out_to_matching_chart_instances() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let mut first = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    first.chart.status = ChartStatus::Loaded;
    first.chart.set_candles(vec![candle(1_000, 100.0)]);

    let mut second = ChartInstance::new(2, "BTC".to_string(), Timeframe::H1);
    second.chart.status = ChartStatus::Loaded;
    second.chart.set_candles(vec![candle(1_000, 100.0)]);

    let mut different_timeframe = ChartInstance::new(3, "BTC".to_string(), Timeframe::M5);
    different_timeframe.chart.status = ChartStatus::Loaded;
    different_timeframe
        .chart
        .set_candles(vec![candle(1_000, 100.0)]);

    terminal.charts.insert(1, first);
    terminal.charts.insert(2, second);
    terminal.charts.insert(3, different_timeframe);

    let _task = terminal.apply_chart_ws_candle_update(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        source_context(&terminal, None),
        candle(2_000, 101.0),
    );

    assert_eq!(last_close(&terminal, 1), Some(101.0));
    assert_eq!(last_close(&terminal, 2), Some(101.0));
    assert_eq!(last_close(&terminal, 3), Some(100.0));
}

#[test]
fn ws_candle_after_large_gap_triggers_reload_instead_of_blind_append() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let mut chart = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    chart.chart.status = ChartStatus::Loaded;
    chart.chart.set_candles(vec![candle(3_600_000, 100.0)]);
    terminal.charts.insert(1, chart);

    // A live candle four hours past the tail — a reconnect after a sleep/quiet
    // outage. Blind-appending it would splice a persistent phantom gap.
    let _task = terminal.apply_chart_ws_candle_update(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        source_context(&terminal, None),
        candle(3_600_000 + 4 * 3_600_000, 200.0),
    );

    let instance = terminal.charts.get(&1).expect("chart");
    // The phantom candle was NOT appended; a reload was queued and the stale
    // series cleared so the refetch replaces rather than stitches.
    assert!(instance.candle_fetch_request.is_some());
    assert!(instance.chart.candles.is_empty());
}

#[test]
fn new_spot_candle_gap_triggers_one_reconciliation_reload() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.exchange_symbols = vec![spot_symbol("@3")];

    let mut chart = ChartInstance::new(1, "@3".to_string(), Timeframe::H1);
    chart.chart.status = ChartStatus::Loaded;
    chart.chart.set_candles(vec![candle(3_600_000, 100.0)]);
    terminal.charts.insert(1, chart);

    let _task = terminal.apply_chart_ws_candle_update(
        1,
        "@3".to_string(),
        "1h".to_string(),
        source_context(&terminal, None),
        candle(3_600_000 + 24 * 3_600_000, 200.0),
    );

    let instance = terminal.charts.get(&1).expect("chart");
    assert!(instance.candle_fetch_request.is_some());
    assert!(instance.chart.candles.is_empty());
    assert!(instance.spot_candle_gap_reloaded_at_ms.is_some());
}

#[test]
fn sparse_spot_gap_during_backoff_appends_without_reload_churn() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.exchange_symbols = vec![spot_symbol("@3")];

    let mut chart = ChartInstance::new(1, "@3".to_string(), Timeframe::H1);
    chart.chart.status = ChartStatus::Loaded;
    chart.chart.set_candles(vec![candle(3_600_000, 100.0)]);
    chart.spot_candle_gap_reloaded_at_ms = Some(TradingTerminal::now_ms());
    terminal.charts.insert(1, chart);

    let _task = terminal.apply_chart_ws_candle_update(
        1,
        "@3".to_string(),
        "1h".to_string(),
        source_context(&terminal, None),
        candle(3_600_000 + 24 * 3_600_000, 200.0),
    );

    let instance = terminal.charts.get(&1).expect("chart");
    assert!(instance.candle_fetch_request.is_none());
    assert_eq!(instance.chart.candles.len(), 2);
    assert_eq!(last_close(&terminal, 1), Some(200.0));
}

#[test]
fn ws_candle_one_interval_ahead_appends_without_reload() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let mut chart = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    chart.chart.status = ChartStatus::Loaded;
    chart.chart.set_candles(vec![candle(3_600_000, 100.0)]);
    terminal.charts.insert(1, chart);

    // The very next candle (exactly one interval later) is normal — append it.
    let _task = terminal.apply_chart_ws_candle_update(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        source_context(&terminal, None),
        candle(7_200_000, 101.0),
    );

    let instance = terminal.charts.get(&1).expect("chart");
    assert!(instance.candle_fetch_request.is_none());
    assert_eq!(last_close(&terminal, 1), Some(101.0));
}

#[test]
fn ws_candle_lagged_queues_reload_for_matching_chart_instances() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let mut first = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    first.chart.status = ChartStatus::Loaded;
    first.chart.set_candles(vec![candle(1_000, 100.0)]);

    let mut second = ChartInstance::new(2, "BTC".to_string(), Timeframe::H1);
    second.chart.status = ChartStatus::Loaded;
    second.chart.set_candles(vec![candle(1_000, 100.0)]);

    let mut different_timeframe = ChartInstance::new(3, "BTC".to_string(), Timeframe::M5);
    different_timeframe.chart.status = ChartStatus::Loaded;
    different_timeframe
        .chart
        .set_candles(vec![candle(1_000, 100.0)]);

    terminal.charts.insert(1, first);
    terminal.charts.insert(2, second);
    terminal.charts.insert(3, different_timeframe);

    let _task = terminal.apply_chart_ws_candle_lagged(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        source_context(&terminal, None),
        3,
    );

    assert!(terminal.charts[&1].candle_fetch_request.is_some());
    assert!(terminal.charts[&2].candle_fetch_request.is_some());
    assert!(terminal.charts[&3].candle_fetch_request.is_none());
}

#[test]
fn stale_hydromancer_ws_candle_generation_does_not_update_chart() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;

    let mut chart = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    chart.chart.status = ChartStatus::Loaded;
    chart.chart.set_candles(vec![candle(1_000, 100.0)]);
    terminal.charts.insert(1, chart);

    let _task = terminal.apply_chart_ws_candle_update(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        source_context(&terminal, Some(1)),
        candle(2_000, 101.0),
    );

    assert_eq!(last_close(&terminal, 1), Some(100.0));
}

#[test]
fn stale_hyperliquid_ws_candle_generation_does_not_update_chart() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let mut chart = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    chart.chart.status = ChartStatus::Loaded;
    chart.chart.set_candles(vec![candle(1_000, 100.0)]);
    terminal.charts.insert(1, chart);
    let stale_context = source_context(&terminal, None);
    terminal.bump_read_data_provider_generation();

    let _task = terminal.apply_chart_ws_candle_update(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        stale_context,
        candle(2_000, 101.0),
    );

    assert_eq!(last_close(&terminal, 1), Some(100.0));
}

#[test]
fn one_second_ws_candle_update_accepts_hydromancer_keyed_context() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.read_data_provider = ReadDataProvider::Hyperliquid;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;

    let mut chart = ChartInstance::new(1, "BTC".to_string(), Timeframe::S1);
    chart.chart.status = ChartStatus::Loaded;
    chart.chart.set_candles(vec![candle(1_000, 100.0)]);
    terminal.charts.insert(1, chart);

    let _task = terminal.apply_chart_ws_candle_update(
        1,
        "BTC".to_string(),
        "1s".to_string(),
        terminal.hydromancer_keyed_market_data_source_context(),
        candle(2_000, 101.0),
    );

    assert_eq!(last_close(&terminal, 1), Some(101.0));
}

#[test]
fn orderbook_tick_price_updates_matching_tick_chart() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let mut tick_chart = ChartInstance::new(1, "BTC".to_string(), Timeframe::Tick);
    tick_chart.chart.status = ChartStatus::Loaded;
    let mut minute_chart = ChartInstance::new(2, "BTC".to_string(), Timeframe::M1);
    minute_chart.chart.status = ChartStatus::Loaded;
    minute_chart.chart.set_candles(vec![candle(1_000, 50.0)]);
    terminal.charts.insert(1, tick_chart);
    terminal.charts.insert(2, minute_chart);

    terminal.apply_orderbook_tick_price_to_charts("BTC", 100.0, 10_000);
    terminal.apply_orderbook_tick_price_to_charts("BTC", 101.0, 10_000);

    let tick = &terminal.charts[&1].chart.candles;
    assert_eq!(tick.len(), 2);
    assert_eq!(tick[0].open_time, 10_000);
    assert_eq!(tick[0].close, 100.0);
    assert_eq!(tick[1].open_time, 10_001);
    assert_eq!(tick[1].close, 101.0);
    assert_eq!(last_close(&terminal, 2), Some(50.0));
}

#[test]
fn ws_candle_update_gates_provider_source() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.hydromancer_key_generation = 2;

    let mut chart = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    chart.chart.status = ChartStatus::Loaded;
    chart.chart.set_candles(vec![candle(1_000, 100.0)]);
    terminal.charts.insert(1, chart);

    let _task = terminal.apply_chart_ws_candle_update(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        source_context(&terminal, Some(2)),
        candle(2_000, 101.0),
    );
    assert_eq!(last_close(&terminal, 1), Some(100.0));

    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    let _task = terminal.apply_chart_ws_candle_update(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        source_context(&terminal, None),
        candle(3_000, 102.0),
    );
    assert_eq!(last_close(&terminal, 1), Some(102.0));
}

#[test]
fn stale_hydromancer_ws_candle_lag_does_not_reload_chart() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;

    let mut chart = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    chart.chart.status = ChartStatus::Loaded;
    chart.chart.set_candles(vec![candle(1_000, 100.0)]);
    terminal.charts.insert(1, chart);

    let _task = terminal.apply_chart_ws_candle_lagged(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        source_context(&terminal, Some(1)),
        3,
    );

    assert!(terminal.charts[&1].candle_fetch_request.is_none());
}

#[test]
fn ws_candle_lag_gates_provider_source() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.hydromancer_key_generation = 2;

    let mut chart = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    chart.chart.status = ChartStatus::Loaded;
    chart.chart.set_candles(vec![candle(1_000, 100.0)]);
    terminal.charts.insert(1, chart);

    let _task = terminal.apply_chart_ws_candle_lagged(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        source_context(&terminal, Some(2)),
        3,
    );
    assert!(terminal.charts[&1].candle_fetch_request.is_none());

    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    let _task = terminal.apply_chart_ws_candle_lagged(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        source_context(&terminal, None),
        3,
    );
    assert!(terminal.charts[&1].candle_fetch_request.is_some());
}
