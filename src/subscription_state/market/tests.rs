use super::chart::chart_asset_ctx_stream_event_message;
use super::position_pnl::position_pnl_book_stream_event_message;
use super::positioning_info::positioning_asset_ctx_stream_event_message;
use super::*;
use crate::api::{ExchangeSymbol, MarketType};
use crate::chart::ChartStatus;
use crate::chart_state::ChartInstance;
use crate::config::ReadDataProvider;
use crate::positioning_state::PositioningInfoInstance;
use crate::spaghetti::{Series, SpaghettiCanvas};
use crate::spaghetti_state::SpaghettiChartInstance;
use crate::timeframe::Timeframe;
use iced::Color;
use iced::widget::pane_grid;

mod charts;
mod order_books;
mod position_pnl;
mod positioning_info;
mod spaghetti;

fn spaghetti_series(symbol: &str, loaded: bool) -> Series {
    Series {
        symbol: symbol.to_string(),
        display: symbol.to_string(),
        candles: Vec::new(),
        color: Color::WHITE,
        loaded,
    }
}

fn exchange_symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: String::new(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 0,
        only_isolated: false,
        growth_mode: false,
        market_type,
        outcome: None,
    }
}

fn split_positioning_pane(
    panes: &mut pane_grid::State<PaneKind>,
    root_pane: pane_grid::Pane,
    axis: pane_grid::Axis,
    id: u64,
) {
    if panes
        .split(axis, root_pane, PaneKind::PositioningInfo(id))
        .is_none()
    {
        panic!("split should add positioning pane {id}");
    }
}

#[test]
fn canvas_live_watchlist_enables_refresh_timer() {
    let (mut terminal, _) =
        TradingTerminal::boot_from_config(crate::config::KeroseneConfig::default());
    let mut baseline = Vec::new();
    terminal.push_market_subscriptions(&mut baseline);

    terminal.insert_test_canvas_pane(7, PaneKind::LiveWatchlist(17));
    let mut with_canvas = Vec::new();
    terminal.push_market_subscriptions(&mut with_canvas);

    assert_eq!(with_canvas.len(), baseline.len() + 1);
}
