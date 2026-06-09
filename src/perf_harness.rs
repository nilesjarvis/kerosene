use crate::api::{BookLevel, Candle, ExchangeSymbol, MarketType, OrderBook};
use crate::app_state::TradingTerminal;
use crate::chart::{CandlestickChart, ChartState, perf_probe};
use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::config::{AxisConfig, KeroseneConfig, PaneKindConfig, PaneLayoutConfig};
use crate::message::Message;
use crate::timeframe::Timeframe;
use crate::ws::{HydromancerWsMessage, LiquidationEvent, TrackedTradeEvent};
use iced::widget::canvas;
use iced::{Event, Point, Rectangle, Size, mouse};
use std::collections::HashMap;
use std::hint::black_box;
use std::time::{Duration, Instant};

const DEFAULT_RUNS: usize = 5;
const DEFAULT_INTERACTIONS_PER_RUN: usize = 360;
const DEFAULT_CHART_CANDLES: usize = 4_000;
const DEFAULT_SYMBOLS: usize = 260;
const DEFAULT_BOOK_LEVELS: usize = 80;
const CHART_BOUNDS: Rectangle = Rectangle {
    x: 0.0,
    y: 0.0,
    width: 1280.0,
    height: 760.0,
};

#[test]
#[ignore = "deterministic performance harness; run with `cargo test --release deterministic_market_load_latency -- --ignored --nocapture`"]
fn deterministic_market_load_latency() {
    let scenario = PerfScenario::from_env();
    println!("{}", scenario.report());

    let mut summaries = Vec::with_capacity(scenario.runs);
    for run_idx in 0..scenario.runs {
        let summary = run_once(run_idx as u64, &scenario);
        println!("{}", summary.report(run_idx + 1));
        summaries.push(summary);
    }

    let aggregate = AggregateSummary::from_runs(&summaries);
    println!("{}", aggregate.report());
}

#[derive(Debug, Clone, Copy)]
struct PerfScenario {
    kind: PerfScenarioKind,
    runs: usize,
    interactions_per_run: usize,
    chart_candles: usize,
    symbols: usize,
    book_levels: usize,
}

impl PerfScenario {
    fn from_env() -> Self {
        Self {
            kind: PerfScenarioKind::from_env(),
            runs: env_usize("KEROSENE_PERF_RUNS", DEFAULT_RUNS, 1),
            interactions_per_run: env_usize(
                "KEROSENE_PERF_INTERACTIONS",
                DEFAULT_INTERACTIONS_PER_RUN,
                1,
            ),
            chart_candles: env_usize("KEROSENE_PERF_CANDLES", DEFAULT_CHART_CANDLES, 1),
            symbols: env_usize("KEROSENE_PERF_SYMBOLS", DEFAULT_SYMBOLS, 3),
            book_levels: env_usize("KEROSENE_PERF_BOOK_LEVELS", DEFAULT_BOOK_LEVELS, 1),
        }
    }

    fn report(&self) -> String {
        format!(
            "Scenario:\n- name: {}\n- focus: {}\n- runs: {}\n- interactions/run: {}\n- candles/symbol/timeframe: {}\n- symbols: {}\n- book levels/side: {}\n- chart bounds: {:.0}x{:.0}",
            self.kind.label(),
            self.kind.focus(),
            self.runs,
            self.interactions_per_run,
            self.chart_candles,
            self.symbols,
            self.book_levels,
            CHART_BOUNDS.width,
            CHART_BOUNDS.height,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PerfScenarioKind {
    MixedMarketLoad,
    ChartInteraction,
    Switching,
    MarketBurst,
    OrderTypingUnderLoad,
    Startup,
}

impl PerfScenarioKind {
    fn from_env() -> Self {
        match std::env::var("KEROSENE_PERF_SCENARIO")
            .unwrap_or_else(|_| "mixed_market_load".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "chart_interaction" => Self::ChartInteraction,
            "switching" => Self::Switching,
            "market_burst" => Self::MarketBurst,
            "order_typing_under_load" => Self::OrderTypingUnderLoad,
            "startup" => Self::Startup,
            _ => Self::MixedMarketLoad,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::MixedMarketLoad => "mixed_market_load",
            Self::ChartInteraction => "chart_interaction",
            Self::Switching => "switching",
            Self::MarketBurst => "market_burst",
            Self::OrderTypingUnderLoad => "order_typing_under_load",
            Self::Startup => "startup",
        }
    }

    fn focus(self) -> &'static str {
        match self {
            Self::MixedMarketLoad => {
                "chart interaction, symbol/timeframe switching, market updates, and order edits"
            }
            Self::ChartInteraction => "chart hover, pan, zoom, resize, and visible candle geometry",
            Self::Switching => "symbol and timeframe switching with cached candle reuse",
            Self::MarketBurst => {
                "mids, orderbook, and feed bursts while chart and watchlist views rebuild"
            }
            Self::OrderTypingUnderLoad => {
                "order price/quantity edits interleaved with market data updates"
            }
            Self::Startup => "boot/config/layout construction and first useful view",
        }
    }

    fn scripted_interaction(self, step: usize) -> ScriptedInteraction {
        match self {
            Self::MixedMarketLoad => match step % 12 {
                0..=2 => ScriptedInteraction::ChartHover,
                3 => ScriptedInteraction::ChartZoomIn,
                4 => ScriptedInteraction::ChartZoomOut,
                5 => ScriptedInteraction::ChartPan,
                6 => ScriptedInteraction::MidsUpdate,
                7 => ScriptedInteraction::BookUpdate,
                8 => ScriptedInteraction::OrderPriceEdit,
                9 => ScriptedInteraction::OrderQuantityEdit,
                10 => ScriptedInteraction::TimeframeSwitch,
                _ => ScriptedInteraction::SymbolSwitch,
            },
            Self::ChartInteraction => match step % 8 {
                0..=3 => ScriptedInteraction::ChartHover,
                4 => ScriptedInteraction::ChartZoomIn,
                5 => ScriptedInteraction::ChartZoomOut,
                6 => ScriptedInteraction::ChartPan,
                _ => ScriptedInteraction::ChartResize,
            },
            Self::Switching => {
                if step.is_multiple_of(2) {
                    ScriptedInteraction::TimeframeSwitch
                } else {
                    ScriptedInteraction::SymbolSwitch
                }
            }
            Self::MarketBurst => match step % 7 {
                0 | 4 => ScriptedInteraction::MidsUpdate,
                1 | 5 => ScriptedInteraction::BookUpdate,
                2 => ScriptedInteraction::LiquidationUpdate,
                3 => ScriptedInteraction::TrackedTradeUpdate,
                _ => ScriptedInteraction::ChartHover,
            },
            Self::OrderTypingUnderLoad => match step % 4 {
                0 => ScriptedInteraction::MidsUpdate,
                1 => ScriptedInteraction::OrderPriceEdit,
                2 => ScriptedInteraction::BookUpdate,
                _ => ScriptedInteraction::OrderQuantityEdit,
            },
            Self::Startup => ScriptedInteraction::Startup,
        }
    }
}

fn env_usize(name: &str, default: usize, min: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value >= min)
        .unwrap_or(default)
}

fn run_once(seed: u64, scenario: &PerfScenario) -> RunSummary {
    if scenario.kind == PerfScenarioKind::Startup {
        return run_startup_once(seed, scenario);
    }

    let mut terminal = deterministic_terminal(seed, scenario);
    let chart_id = terminal.primary_chart_id.unwrap_or(0);
    let order_book_id = 0;
    let mut chart_state = ChartState::default();
    let mut samples = Vec::with_capacity(scenario.interactions_per_run);
    let mut samples_by_kind: [Vec<Duration>; InteractionKind::COUNT] =
        std::array::from_fn(|_| Vec::new());
    let mut total_visible_candles = 0usize;
    let mut checksum = 0.0_f64;

    for step in 0..scenario.interactions_per_run {
        let scripted_interaction = scenario.kind.scripted_interaction(step);
        let bounds = chart_bounds_for_step(step, scripted_interaction);
        let start = Instant::now();
        let reseed_after_sample = drive_scripted_interaction(
            ScriptedInteractionInput {
                terminal: &mut terminal,
                chart_state: &mut chart_state,
                scenario,
                bounds,
                chart_id,
                order_book_id,
                seed,
                step,
            },
            scripted_interaction,
        );
        let interaction_kind = scripted_interaction.kind();

        perf_probe::set_market_load_view(
            &mut chart_state,
            2.0 + (step % 4) as f32 * 0.35,
            (step % 180) as f32,
        );
        let probe = {
            let chart = &terminal
                .charts
                .get(&chart_id)
                .expect("deterministic chart should exist")
                .chart;
            perf_probe::chart_layout_hot_path(chart, &chart_state, bounds)
        };
        total_visible_candles += probe.visible_candles;
        checksum += probe.checksum;

        {
            let element = terminal.view_main();
            black_box(element);
        }

        let elapsed = start.elapsed();
        samples.push(elapsed);
        samples_by_kind[interaction_kind.index()].push(elapsed);
        if reseed_after_sample {
            reseed_loaded_chart_and_book(&mut terminal, chart_id, seed, step, scenario);
        }
    }

    RunSummary::from_samples(samples, samples_by_kind, total_visible_candles, checksum)
}

fn run_startup_once(seed: u64, scenario: &PerfScenario) -> RunSummary {
    let mut samples = Vec::with_capacity(scenario.interactions_per_run);
    let mut samples_by_kind: [Vec<Duration>; InteractionKind::COUNT] =
        std::array::from_fn(|_| Vec::new());
    let mut total_visible_candles = 0usize;
    let mut checksum = 0.0_f64;

    for step in 0..scenario.interactions_per_run {
        let start = Instant::now();
        let terminal = deterministic_terminal(seed + step as u64, scenario);
        let chart_id = terminal.primary_chart_id.unwrap_or(0);
        let mut chart_state = ChartState::default();
        perf_probe::set_market_load_view(
            &mut chart_state,
            2.0 + (step % 4) as f32 * 0.35,
            (step % 180) as f32,
        );
        if let Some(instance) = terminal.charts.get(&chart_id) {
            let probe =
                perf_probe::chart_layout_hot_path(&instance.chart, &chart_state, CHART_BOUNDS);
            total_visible_candles += probe.visible_candles;
            checksum += probe.checksum;
        }
        let element = terminal.view_main();
        black_box(element);

        let elapsed = start.elapsed();
        samples.push(elapsed);
        samples_by_kind[InteractionKind::Startup.index()].push(elapsed);
    }

    RunSummary::from_samples(samples, samples_by_kind, total_visible_candles, checksum)
}

fn market_burst_pane_layout() -> PaneLayoutConfig {
    split(
        AxisConfig::Horizontal,
        0.72,
        split(
            AxisConfig::Vertical,
            0.56,
            leaf(PaneKindConfig::Chart { chart_id: 0 }),
            split(
                AxisConfig::Vertical,
                0.55,
                leaf(PaneKindConfig::OrderBook { id: 0 }),
                leaf(PaneKindConfig::Watchlist),
            ),
        ),
        split(
            AxisConfig::Vertical,
            0.50,
            leaf(PaneKindConfig::Liquidations),
            leaf(PaneKindConfig::TrackedTrades),
        ),
    )
}

fn split(
    axis: AxisConfig,
    ratio: f32,
    a: PaneLayoutConfig,
    b: PaneLayoutConfig,
) -> PaneLayoutConfig {
    PaneLayoutConfig::Split {
        axis,
        ratio,
        a: Box::new(a),
        b: Box::new(b),
    }
}

fn leaf(kind: PaneKindConfig) -> PaneLayoutConfig {
    PaneLayoutConfig::Leaf(kind)
}

#[derive(Debug, Clone, Copy)]
enum ScriptedInteraction {
    ChartHover,
    ChartZoomIn,
    ChartZoomOut,
    ChartPan,
    ChartResize,
    MidsUpdate,
    BookUpdate,
    LiquidationUpdate,
    TrackedTradeUpdate,
    OrderPriceEdit,
    OrderQuantityEdit,
    TimeframeSwitch,
    SymbolSwitch,
    Startup,
}

impl ScriptedInteraction {
    fn kind(self) -> InteractionKind {
        match self {
            Self::ChartHover => InteractionKind::ChartHover,
            Self::ChartZoomIn | Self::ChartZoomOut => InteractionKind::ChartZoom,
            Self::ChartPan => InteractionKind::ChartPan,
            Self::ChartResize => InteractionKind::ChartResize,
            Self::MidsUpdate => InteractionKind::MidsUpdate,
            Self::BookUpdate => InteractionKind::BookUpdate,
            Self::LiquidationUpdate => InteractionKind::LiquidationUpdate,
            Self::TrackedTradeUpdate => InteractionKind::TrackedTradeUpdate,
            Self::OrderPriceEdit | Self::OrderQuantityEdit => InteractionKind::OrderEdit,
            Self::TimeframeSwitch => InteractionKind::TimeframeSwitch,
            Self::SymbolSwitch => InteractionKind::SymbolSwitch,
            Self::Startup => InteractionKind::Startup,
        }
    }
}

fn chart_bounds_for_step(step: usize, interaction: ScriptedInteraction) -> Rectangle {
    if !matches!(interaction, ScriptedInteraction::ChartResize) {
        return CHART_BOUNDS;
    }

    let width = match step % 3 {
        0 => 980.0,
        1 => 1280.0,
        _ => 1540.0,
    };
    let height = match step % 3 {
        0 => 620.0,
        1 => 760.0,
        _ => 880.0,
    };
    Rectangle {
        width,
        height,
        ..CHART_BOUNDS
    }
}

struct ScriptedInteractionInput<'a> {
    terminal: &'a mut TradingTerminal,
    chart_state: &'a mut ChartState,
    scenario: &'a PerfScenario,
    bounds: Rectangle,
    chart_id: ChartId,
    order_book_id: u64,
    seed: u64,
    step: usize,
}

// These scripted interactions intentionally enter the same paths a trader hits:
// chart canvas updates for hover/pan/zoom, app update messages for market data,
// order edits and symbol switching, and view_main for the post-update rebuild.
fn drive_scripted_interaction(
    input: ScriptedInteractionInput<'_>,
    interaction: ScriptedInteraction,
) -> bool {
    let ScriptedInteractionInput {
        terminal,
        chart_state,
        scenario,
        bounds,
        chart_id,
        order_book_id,
        seed,
        step,
    } = input;

    match interaction {
        ScriptedInteraction::ChartHover => {
            let point = Point::new(
                60.0 + ((step * 17) % 980) as f32,
                55.0 + ((step * 29) % 520) as f32,
            );
            drive_chart_mouse(
                terminal,
                chart_id,
                chart_state,
                Event::Mouse(mouse::Event::CursorMoved { position: point }),
                bounds,
                point,
            );
            false
        }
        ScriptedInteraction::ChartZoomIn | ScriptedInteraction::ChartZoomOut => {
            let point = Point::new(420.0 + (step % 5) as f32 * 18.0, 320.0);
            let y = if matches!(interaction, ScriptedInteraction::ChartZoomIn) {
                1.0
            } else {
                -1.0
            };
            drive_chart_mouse(
                terminal,
                chart_id,
                chart_state,
                Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Lines { x: 0.0, y },
                }),
                bounds,
                point,
            );
            false
        }
        ScriptedInteraction::ChartPan => {
            drive_chart_pan(terminal, chart_id, chart_state, bounds, step);
            false
        }
        ScriptedInteraction::ChartResize => {
            drive_chart_resize(terminal, chart_id, bounds);
            false
        }
        ScriptedInteraction::MidsUpdate => {
            let mids = synthetic_mids(seed, step, scenario.symbols);
            let _task = terminal.update(Message::AllMidsBootstrapLoaded(
                "main".to_string(),
                Ok(mids),
            ));
            false
        }
        ScriptedInteraction::BookUpdate => {
            let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
            let _task = terminal.update(Message::WsBookUpdate {
                id: order_book_id,
                coin: "BTC".to_string(),
                sigfigs,
                book: synthetic_order_book(seed, step, scenario.book_levels),
            });
            false
        }
        ScriptedInteraction::LiquidationUpdate => {
            let _task = terminal.update(Message::WsHydromancerLiquidation(
                HydromancerWsMessage::Event(synthetic_liquidation_event(seed, step)),
            ));
            false
        }
        ScriptedInteraction::TrackedTradeUpdate => {
            let _task = terminal.update(Message::WsHydromancerTrackedTrades(
                HydromancerWsMessage::TrackedTrade(synthetic_tracked_trade_event(seed, step)),
            ));
            false
        }
        ScriptedInteraction::OrderPriceEdit => {
            let price = 50_000.0 + step as f64 * 0.25;
            let _task = terminal.update(Message::OrderPriceChanged(format!("{price:.2}")));
            false
        }
        ScriptedInteraction::OrderQuantityEdit => {
            let qty = 100.0 + (step % 25) as f64 * 10.0;
            let _task = terminal.update(Message::OrderQuantityChanged(format!("{qty:.0}")));
            false
        }
        ScriptedInteraction::TimeframeSwitch => {
            let use_alt_timeframe = match scenario.kind {
                PerfScenarioKind::MixedMarketLoad => step % 24 == 10,
                PerfScenarioKind::Switching => step.is_multiple_of(4),
                _ => step % 8 < 4,
            };
            let target = if use_alt_timeframe {
                Timeframe::M5
            } else {
                Timeframe::H1
            };
            let _task = terminal.update(Message::ChartSwitchTimeframe(chart_id, target));
            true
        }
        ScriptedInteraction::SymbolSwitch => {
            let use_alt_symbol = match scenario.kind {
                PerfScenarioKind::MixedMarketLoad => step % 24 == 11,
                PerfScenarioKind::Switching => step % 4 == 1,
                _ => step % 8 < 4,
            };
            let target = if use_alt_symbol { "ETH" } else { "BTC" };
            let _task = terminal.update(Message::ChartSymbolSelected(chart_id, target.to_string()));
            true
        }
        ScriptedInteraction::Startup => false,
    }
}

fn deterministic_terminal(seed: u64, scenario: &PerfScenario) -> TradingTerminal {
    let mut cfg = KeroseneConfig {
        active_symbol: "BTC".to_string(),
        active_timeframe: Timeframe::H1.config_str().to_string(),
        ..Default::default()
    };
    if scenario.kind == PerfScenarioKind::MarketBurst {
        cfg.pane_layout = Some(market_burst_pane_layout());
    }

    let (mut terminal, _task) = TradingTerminal::boot_from_config(cfg);
    let symbols = synthetic_symbols(scenario.symbols);
    let _task = terminal.update(Message::SymbolsLoaded(Ok(symbols)));
    let _task = terminal.update(Message::AllMidsBootstrapLoaded(
        "main".to_string(),
        Ok(synthetic_mids(seed, 0, scenario.symbols)),
    ));

    for symbol in ["BTC", "ETH"] {
        for timeframe in [Timeframe::H1, Timeframe::M5] {
            let candles = synthetic_candles(symbol, timeframe, seed, scenario.chart_candles);
            terminal.cache_candles(symbol, timeframe, candles.clone());
            if let Some(instance) = terminal.charts.get_mut(&0)
                && instance.symbol == symbol
                && instance.interval == timeframe
            {
                instance.chart.set_candles(candles);
            }
        }
    }

    if let Some(instance) = terminal.charts.get_mut(&0) {
        instance.symbol = "BTC".to_string();
        instance.symbol_display = "BTC".to_string();
        instance.interval = Timeframe::H1;
        instance.chart.set_symbol_label("BTC".to_string());
        instance.chart.set_timeframe(Timeframe::H1);
        instance.chart.set_candles(synthetic_candles(
            "BTC",
            Timeframe::H1,
            seed,
            scenario.chart_candles,
        ));
    }

    if let Some(book) = terminal.order_books.get_mut(&0) {
        book.set_book(synthetic_order_book(seed, 0, scenario.book_levels));
        book.book_loading = false;
        book.book_error = None;
    }

    terminal.main_window_size = Some(Size::new(1600.0, 1000.0));
    terminal.clear_all_chart_surface_state(0);
    terminal.chart_surface_viewports.insert(
        ChartSurfaceId::Docked(0),
        crate::chart::ChartViewport {
            start_time_ms: 0,
            end_time_ms: 0,
            price_lo: 0.0,
            price_hi: 0.0,
            chart_width: CHART_BOUNDS.width,
            candle_width: crate::chart::DEFAULT_CANDLE_WIDTH,
            scroll_offset: 0.0,
            y_auto: true,
            y_scale: 1.0,
            y_offset: 0.0,
            funding_y_scale: 1.0,
            funding_y_offset: 0.0,
        },
    );

    terminal
}

fn reseed_loaded_chart_and_book(
    terminal: &mut TradingTerminal,
    chart_id: ChartId,
    seed: u64,
    step: usize,
    scenario: &PerfScenario,
) {
    let Some((symbol, interval)) = terminal
        .charts
        .get(&chart_id)
        .map(|instance| (instance.symbol.clone(), instance.interval))
    else {
        return;
    };

    let candles = synthetic_candles(&symbol, interval, seed, scenario.chart_candles);
    terminal.cache_candles(&symbol, interval, candles.clone());
    if let Some(instance) = terminal.charts.get_mut(&chart_id) {
        instance.chart.set_candles(candles);
    }
    if let Some(book) = terminal.order_books.get_mut(&0) {
        book.set_book(synthetic_order_book(seed, step, scenario.book_levels));
        book.book_loading = false;
        book.book_error = None;
    }
}

fn drive_chart_mouse(
    terminal: &TradingTerminal,
    chart_id: ChartId,
    state: &mut ChartState,
    event: Event,
    bounds: Rectangle,
    position: Point,
) {
    let chart = &terminal
        .charts
        .get(&chart_id)
        .expect("deterministic chart should exist")
        .chart;
    let _action = <CandlestickChart as canvas::Program<Message>>::update(
        chart,
        state,
        &event,
        bounds,
        mouse::Cursor::Available(position),
    );
}

fn drive_chart_pan(
    terminal: &TradingTerminal,
    chart_id: ChartId,
    state: &mut ChartState,
    bounds: Rectangle,
    step: usize,
) {
    let start = Point::new(360.0 + (step % 7) as f32 * 12.0, 300.0);
    let end = Point::new(start.x - 95.0, start.y + 8.0);
    drive_chart_mouse(
        terminal,
        chart_id,
        state,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        bounds,
        start,
    );
    drive_chart_mouse(
        terminal,
        chart_id,
        state,
        Event::Mouse(mouse::Event::CursorMoved { position: end }),
        bounds,
        end,
    );
    drive_chart_mouse(
        terminal,
        chart_id,
        state,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        bounds,
        end,
    );
}

fn drive_chart_resize(terminal: &mut TradingTerminal, chart_id: ChartId, bounds: Rectangle) {
    terminal.main_window_size = Some(Size::new(bounds.width + 320.0, bounds.height + 240.0));
    if let Some(viewport) = terminal
        .chart_surface_viewports
        .get_mut(&ChartSurfaceId::Docked(chart_id))
    {
        viewport.chart_width = bounds.width;
    }
}

fn synthetic_symbols(count: usize) -> Vec<ExchangeSymbol> {
    let mut symbols = Vec::with_capacity(count.max(2));
    for idx in 0..count {
        let ticker = match idx {
            0 => "BTC".to_string(),
            1 => "ETH".to_string(),
            2 => "HYPE".to_string(),
            _ => format!("SYM{idx:03}"),
        };
        symbols.push(ExchangeSymbol {
            key: ticker.clone(),
            ticker: ticker.clone(),
            category: if idx % 5 == 0 {
                "crypto".to_string()
            } else {
                "perp".to_string()
            },
            display_name: None,
            keywords: vec![format!("synthetic-{idx}")],
            asset_index: idx as u32,
            collateral_token: None,
            sz_decimals: 3,
            max_leverage: 50,
            only_isolated: false,
            market_type: MarketType::Perp,
            outcome: None,
        });
    }
    symbols
}

fn synthetic_candles(symbol: &str, timeframe: Timeframe, seed: u64, count: usize) -> Vec<Candle> {
    let base = match symbol {
        "ETH" => 3_200.0,
        "HYPE" => 32.0,
        _ => 50_000.0,
    };
    let interval_ms = timeframe.duration_ms();
    let end_ms = crate::app_time::now_ms()
        .saturating_sub(60_000)
        .saturating_sub(seed * 10_000);
    let span_ms = interval_ms.saturating_mul(count.saturating_sub(1) as u64);
    let start_ms = end_ms.saturating_sub(span_ms);
    let mut candles = Vec::with_capacity(count);
    for idx in 0..count {
        let wave = ((idx as f64 + seed as f64 * 3.0) / 19.0).sin() * base * 0.012;
        let drift = idx as f64 * base * 0.000_015;
        let open = base + drift + wave;
        let close = open + ((idx as f64) / 7.0).cos() * base * 0.0025;
        let high = open.max(close) + base * (0.0015 + (idx % 11) as f64 * 0.000_08);
        let low = open.min(close) - base * (0.0015 + (idx % 13) as f64 * 0.000_07);
        let volume = 50.0 + (idx % 97) as f64 * 3.5 + seed as f64;
        let open_time = start_ms + idx as u64 * interval_ms;
        candles.push(Candle {
            open_time,
            close_time: open_time + interval_ms.saturating_sub(1),
            open,
            high,
            low,
            close,
            volume,
        });
    }
    candles
}

fn synthetic_order_book(seed: u64, step: usize, levels: usize) -> OrderBook {
    let mid = 50_000.0 + seed as f64 * 3.0 + step as f64 * 0.15;
    let mut bids = Vec::with_capacity(levels);
    let mut asks = Vec::with_capacity(levels);
    for idx in 0..levels {
        let distance = (idx + 1) as f64 * 0.5;
        let size = 0.2 + ((idx + step) % 17) as f64 * 0.03;
        bids.push(BookLevel {
            px: mid - distance,
            sz: size,
        });
        asks.push(BookLevel {
            px: mid + distance,
            sz: size * 1.05,
        });
    }
    OrderBook { bids, asks }
}

fn synthetic_liquidation_event(seed: u64, step: usize) -> LiquidationEvent {
    let is_buy = step.is_multiple_of(2);
    LiquidationEvent {
        coin: if step.is_multiple_of(3) {
            "ETH".to_string()
        } else {
            "BTC".to_string()
        },
        price: 50_000.0 + seed as f64 * 2.0 + step as f64 * 0.75,
        size: 0.05 + (step % 17) as f64 * 0.01,
        is_buy,
        time_ms: crate::app_time::now_ms().saturating_sub((step % 240) as u64 * 1_000),
        method: "market".to_string(),
        liquidated_user: format!("0x{:040x}", step % 101),
        tx_index: step as u64,
    }
}

fn synthetic_tracked_trade_event(seed: u64, step: usize) -> TrackedTradeEvent {
    let is_buy = !step.is_multiple_of(2);
    let signed_position = if is_buy { 1.0 } else { -1.0 };
    TrackedTradeEvent {
        address: format!("0x{:040x}", 10_000 + step % 211),
        coin: if step.is_multiple_of(5) {
            "ETH".to_string()
        } else {
            "BTC".to_string()
        },
        price: 49_900.0 + seed as f64 * 2.0 + step as f64 * 0.55,
        size: 0.03 + (step % 23) as f64 * 0.01,
        is_buy,
        time_ms: crate::app_time::now_ms().saturating_sub((step % 240) as u64 * 1_000),
        dir: if is_buy {
            "Buy".to_string()
        } else {
            "Sell".to_string()
        },
        start_position: Some(signed_position * (step % 7) as f64 * 0.1),
        closed_pnl: (step % 13) as f64 - 6.0,
        fee: 0.01 + (step % 5) as f64 * 0.001,
        fee_token: "USDC".to_string(),
        tid: Some(1_000_000 + step as u64),
        hash: format!("0x{:064x}", 1_000_000_u64 + step as u64),
        oid: Some(2_000_000 + step as u64),
        tx_index: step as u64,
    }
}

fn synthetic_mids(seed: u64, step: usize, count: usize) -> HashMap<String, f64> {
    let mut mids = HashMap::with_capacity(count);
    for idx in 0..count {
        let key = match idx {
            0 => "BTC".to_string(),
            1 => "ETH".to_string(),
            2 => "HYPE".to_string(),
            _ => format!("SYM{idx:03}"),
        };
        let base = match idx {
            0 => 50_000.0,
            1 => 3_200.0,
            2 => 32.0,
            _ => 10.0 + idx as f64 * 0.75,
        };
        let price = base + seed as f64 * 0.03 + step as f64 * 0.01 + (idx % 19) as f64 * 0.001;
        mids.insert(key, price);
    }
    mids
}

#[derive(Debug, Clone, Copy)]
enum InteractionKind {
    ChartHover,
    ChartZoom,
    ChartPan,
    ChartResize,
    MidsUpdate,
    BookUpdate,
    LiquidationUpdate,
    TrackedTradeUpdate,
    OrderEdit,
    TimeframeSwitch,
    SymbolSwitch,
    Startup,
}

impl InteractionKind {
    const COUNT: usize = 12;
    const ALL: [Self; Self::COUNT] = [
        Self::ChartHover,
        Self::ChartZoom,
        Self::ChartPan,
        Self::ChartResize,
        Self::MidsUpdate,
        Self::BookUpdate,
        Self::LiquidationUpdate,
        Self::TrackedTradeUpdate,
        Self::OrderEdit,
        Self::TimeframeSwitch,
        Self::SymbolSwitch,
        Self::Startup,
    ];

    fn index(self) -> usize {
        match self {
            Self::ChartHover => 0,
            Self::ChartZoom => 1,
            Self::ChartPan => 2,
            Self::ChartResize => 3,
            Self::MidsUpdate => 4,
            Self::BookUpdate => 5,
            Self::LiquidationUpdate => 6,
            Self::TrackedTradeUpdate => 7,
            Self::OrderEdit => 8,
            Self::TimeframeSwitch => 9,
            Self::SymbolSwitch => 10,
            Self::Startup => 11,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::ChartHover => "chart hover",
            Self::ChartZoom => "chart zoom",
            Self::ChartPan => "chart pan",
            Self::ChartResize => "chart resize",
            Self::MidsUpdate => "mids update",
            Self::BookUpdate => "book update",
            Self::LiquidationUpdate => "liquidation update",
            Self::TrackedTradeUpdate => "tracked trade update",
            Self::OrderEdit => "order edit",
            Self::TimeframeSwitch => "timeframe switch",
            Self::SymbolSwitch => "symbol switch",
            Self::Startup => "startup",
        }
    }
}

#[derive(Debug, Clone)]
struct KindSummary {
    kind: InteractionKind,
    samples: usize,
    p50: Duration,
    p95: Duration,
    p99: Duration,
    max: Duration,
    over_16_7_ms: usize,
    over_33_ms: usize,
    over_100_ms: usize,
}

#[derive(Debug, Clone)]
struct RunSummary {
    p50: Duration,
    p95: Duration,
    p99: Duration,
    max: Duration,
    over_16_7_ms: usize,
    over_33_ms: usize,
    over_100_ms: usize,
    interactions: usize,
    total_visible_candles: usize,
    checksum: f64,
    samples_by_kind: [Vec<Duration>; InteractionKind::COUNT],
}

impl RunSummary {
    fn from_samples(
        mut samples: Vec<Duration>,
        samples_by_kind: [Vec<Duration>; InteractionKind::COUNT],
        total_visible_candles: usize,
        checksum: f64,
    ) -> Self {
        samples.sort_unstable();
        let interactions = samples.len();
        let p50 = percentile(&samples, 50.0);
        let p95 = percentile(&samples, 95.0);
        let p99 = percentile(&samples, 99.0);
        let max = *samples.last().unwrap_or(&Duration::ZERO);
        let over_16_7_ms = count_over(&samples, Duration::from_micros(16_700));
        let over_33_ms = count_over(&samples, Duration::from_millis(33));
        let over_100_ms = count_over(&samples, Duration::from_millis(100));

        Self {
            p50,
            p95,
            p99,
            max,
            over_16_7_ms,
            over_33_ms,
            over_100_ms,
            interactions,
            total_visible_candles,
            checksum,
            samples_by_kind,
        }
    }

    fn report(&self, run_number: usize) -> String {
        let mut by_kind = String::new();
        for summary in self.by_kind() {
            if summary.samples == 0 {
                continue;
            }
            by_kind.push_str(&format!(
                "\n  - {}: p50 {:.3}ms, p95 {:.3}ms, p99 {:.3}ms, max {:.3}ms, >16.7ms {}, >33ms {}, >100ms {}, samples {}",
                summary.kind.label(),
                millis(summary.p50),
                millis(summary.p95),
                millis(summary.p99),
                millis(summary.max),
                summary.over_16_7_ms,
                summary.over_33_ms,
                summary.over_100_ms,
                summary.samples,
            ));
        }

        format!(
            "Run {run_number}:\n- p50: {:.3}ms\n- p95: {:.3}ms\n- p99: {:.3}ms\n- max: {:.3}ms\n- >16.7ms: {}\n- >33ms: {}\n- >100ms: {}\n- interactions: {}\n- avg visible candles: {:.1}\n- checksum: {:.3}\n- by-kind:{}",
            millis(self.p50),
            millis(self.p95),
            millis(self.p99),
            millis(self.max),
            self.over_16_7_ms,
            self.over_33_ms,
            self.over_100_ms,
            self.interactions,
            self.total_visible_candles as f64 / self.interactions.max(1) as f64,
            self.checksum,
            by_kind,
        )
    }

    fn by_kind(&self) -> Vec<KindSummary> {
        summarize_kinds(self.samples_by_kind.clone())
    }
}

#[derive(Debug)]
struct AggregateSummary {
    median_p50: Duration,
    median_p95: Duration,
    median_p99: Duration,
    max: Duration,
    over_16_7_ms: usize,
    over_33_ms: usize,
    over_100_ms: usize,
    interactions: usize,
    total_visible_candles: usize,
    checksum: f64,
    by_kind: Vec<KindSummary>,
    runs: usize,
}

impl AggregateSummary {
    fn from_runs(runs: &[RunSummary]) -> Self {
        let mut p50s: Vec<_> = runs.iter().map(|summary| summary.p50).collect();
        let mut p95s: Vec<_> = runs.iter().map(|summary| summary.p95).collect();
        let mut p99s: Vec<_> = runs.iter().map(|summary| summary.p99).collect();
        let mut samples_by_kind: [Vec<Duration>; InteractionKind::COUNT] =
            std::array::from_fn(|_| Vec::new());
        for summary in runs {
            for (aggregate, run_samples) in samples_by_kind
                .iter_mut()
                .zip(summary.samples_by_kind.iter())
            {
                aggregate.extend_from_slice(run_samples);
            }
        }
        p50s.sort_unstable();
        p95s.sort_unstable();
        p99s.sort_unstable();
        Self {
            median_p50: median(&p50s),
            median_p95: median(&p95s),
            median_p99: median(&p99s),
            max: runs
                .iter()
                .map(|summary| summary.max)
                .max()
                .unwrap_or(Duration::ZERO),
            over_16_7_ms: runs.iter().map(|summary| summary.over_16_7_ms).sum(),
            over_33_ms: runs.iter().map(|summary| summary.over_33_ms).sum(),
            over_100_ms: runs.iter().map(|summary| summary.over_100_ms).sum(),
            interactions: runs.iter().map(|summary| summary.interactions).sum(),
            total_visible_candles: runs
                .iter()
                .map(|summary| summary.total_visible_candles)
                .sum(),
            checksum: runs.iter().map(|summary| summary.checksum).sum(),
            by_kind: summarize_kinds(samples_by_kind),
            runs: runs.len(),
        }
    }

    fn report(&self) -> String {
        let mut by_kind = String::new();
        for summary in &self.by_kind {
            if summary.samples == 0 {
                continue;
            }
            by_kind.push_str(&format!(
                "\n  - {}: p50 {:.3}ms, p95 {:.3}ms, p99 {:.3}ms, max {:.3}ms, >16.7ms {}, >33ms {}, >100ms {}, samples {}",
                summary.kind.label(),
                millis(summary.p50),
                millis(summary.p95),
                millis(summary.p99),
                millis(summary.max),
                summary.over_16_7_ms,
                summary.over_33_ms,
                summary.over_100_ms,
                summary.samples,
            ));
        }

        format!(
            "Aggregate ({}) runs:\n- median p50: {:.3}ms\n- median p95: {:.3}ms\n- median p99: {:.3}ms\n- max: {:.3}ms\n- total >16.7ms: {}\n- total >33ms: {}\n- total >100ms: {}\n- interactions: {}\n- avg visible candles: {:.1}\n- checksum: {:.3}\n- by-kind:{}",
            self.runs,
            millis(self.median_p50),
            millis(self.median_p95),
            millis(self.median_p99),
            millis(self.max),
            self.over_16_7_ms,
            self.over_33_ms,
            self.over_100_ms,
            self.interactions,
            self.total_visible_candles as f64 / self.interactions.max(1) as f64,
            self.checksum,
            by_kind,
        )
    }
}

fn percentile(sorted: &[Duration], pct: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let rank = ((pct / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn count_over(samples: &[Duration], threshold: Duration) -> usize {
    samples
        .iter()
        .filter(|duration| **duration > threshold)
        .count()
}

fn summarize_kinds(samples_by_kind: [Vec<Duration>; InteractionKind::COUNT]) -> Vec<KindSummary> {
    InteractionKind::ALL
        .into_iter()
        .zip(samples_by_kind)
        .map(|(kind, mut samples)| {
            samples.sort_unstable();
            KindSummary {
                kind,
                samples: samples.len(),
                p50: percentile(&samples, 50.0),
                p95: percentile(&samples, 95.0),
                p99: percentile(&samples, 99.0),
                max: samples.last().copied().unwrap_or(Duration::ZERO),
                over_16_7_ms: count_over(&samples, Duration::from_micros(16_700)),
                over_33_ms: count_over(&samples, Duration::from_millis(33)),
                over_100_ms: count_over(&samples, Duration::from_millis(100)),
            }
        })
        .collect()
}

fn median(sorted: &[Duration]) -> Duration {
    percentile(sorted, 50.0)
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
