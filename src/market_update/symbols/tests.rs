use super::*;
use crate::api::{
    Candle, ExchangeSymbol, ExchangeSymbolsPayload, MarketType, OutcomeSymbolInfo, WatchlistContext,
};
use crate::chart_state::{ChartInstance, ChartSurfaceId};
use crate::config::MarketUniverseConfig;
use crate::hydromancer_api::FundingRatePoint;
use crate::hyperdash_api::{
    HeatmapFetchParams, LiquidationBucket, LiquidationHeatmap, LiquidationLevel,
};
use crate::market_state::{
    LiveWatchlistInstance, OrderBookInstance, OrderBookSymbolMode, SymbolSearchMarketFilter,
    SymbolSearchSortMode,
};
use crate::message::Message;
use crate::order_execution::QuickOrderForm;
use crate::spaghetti::Series;
use crate::spaghetti_state::SpaghettiChartInstance;
use crate::timeframe::Timeframe;
use std::collections::HashMap;

mod contexts;
mod migration;
mod refresh;

fn perp_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: Some(0),
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: false,
        growth_mode: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn spot_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "HYPE".to_string(),
        category: "spot".to_string(),
        display_name: Some("HYPE/USDC".to_string()),
        keywords: vec!["spot".to_string()],
        asset_index: 10_107,
        collateral_token: Some(0),
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        growth_mode: false,
        market_type: MarketType::Spot,
        outcome: None,
    }
}

fn canonical_purr_symbol() -> ExchangeSymbol {
    ExchangeSymbol {
        key: "PURR/USDC".to_string(),
        ticker: "PURR".to_string(),
        category: "spot".to_string(),
        display_name: Some("PURR/USDC".to_string()),
        keywords: vec!["spot".to_string()],
        asset_index: 10_000,
        collateral_token: Some(0),
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: false,
        growth_mode: false,
        market_type: MarketType::Spot,
        outcome: None,
    }
}

fn outcome_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        growth_mode: false,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 95,
            contract: crate::api::OutcomeContract::verified_fixture(),
            venue: None,
            question_id: None,
            question_name: Some("Will BTC close green?".to_string()),
            question_description: None,
            question_class: None,
            question_underlying: None,
            question_expiry: None,
            question_price_thresholds: Vec::new(),
            question_period: None,
            question_named_outcomes: Vec::new(),
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: None,
            bucket_index: None,
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring".to_string(),
            description: "Will BTC close green?".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 950,
        }),
    }
}

fn context(day_vlm: f64) -> WatchlistContext {
    WatchlistContext {
        funding: None,
        prev_day_px: None,
        mark_px: None,
        day_vlm: Some(day_vlm),
        open_interest_notional: None,
    }
}

fn payload(symbols: Vec<ExchangeSymbol>) -> ExchangeSymbolsPayload {
    ExchangeSymbolsPayload {
        symbols,
        perp_dexes: None,
        loaded_from_cache: false,
        perp_meta_failed: false,
        spot_meta_failed: false,
        outcome_meta_failed: false,
    }
}

fn quick_order_form() -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: "2.5".to_string(),
        quantity_is_usd: false,
        percentage: 25.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 10.0,
        click_y: 20.0,
        chart_w: 300.0,
        chart_h: 200.0,
    }
}

#[test]
fn outcome_rules_expand_independently_and_collapse_again() {
    let mut terminal = TradingTerminal::boot().0;
    let _task = terminal.update_symbol_search_market(Message::OutcomeRulesToggled(65));
    let _task = terminal.update_symbol_search_market(Message::OutcomeRulesToggled(66));
    let _task = terminal.update_symbol_search_market(Message::OutcomeRulesToggled(65));
    assert_eq!(
        terminal.outcome_expanded_rules,
        std::collections::HashSet::from([66])
    );
}

fn symbols_refresh_terminal(with_widgets: bool) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.spaghetti_charts.clear();
    terminal.order_books.clear();
    terminal.positioning_infos.clear();
    terminal.session_data.clear();
    terminal.market_universe = MarketUniverseConfig::All;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();

    if with_widgets {
        let candles = vec![Candle::test_flat(3_600_000, 100.0)];
        let mut chart = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        chart.chart.set_candles(candles.clone());
        terminal.charts.insert(7, chart);

        let mut comparison = SpaghettiChartInstance::new_empty(8);
        comparison.canvas.series.push(Series {
            symbol: "BTC".to_string(),
            display: "BTC".to_string(),
            candles,
            color: iced::Color::WHITE,
            loaded: true,
        });
        terminal.spaghetti_charts.insert(8, comparison);

        let mut book = OrderBookInstance::new(9, OrderBookSymbolMode::Active, 1.0);
        book.book_loading = false;
        terminal.order_books.insert(9, book);
    }
    terminal
}
