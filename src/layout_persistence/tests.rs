use super::*;
use crate::canvas_state::WorkspaceId;
use crate::config::{
    AxisConfig, CanvasConfig, KeroseneConfig, OrderBookConfig, OrderBookSymbolModeConfig,
    PaneKindConfig, PaneLayoutConfig, PositioningInfoConfig, SortDirection, WatchlistPresetConfig,
};
use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};
use crate::pane_state::PaneKind;
use crate::positioning_state::{PositioningInfoInstance, PositioningInfoSortField};

fn widget_layout(positioning_id: u64, book_id: u64) -> PaneLayoutConfig {
    PaneLayoutConfig::Split {
        axis: AxisConfig::Vertical,
        ratio: 0.5,
        a: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::PositioningInfo {
            id: positioning_id,
        })),
        b: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::OrderBook {
            id: book_id,
        })),
    }
}

fn widget_config() -> KeroseneConfig {
    let mut canvas: CanvasConfig =
        serde_json::from_str(r#"{"id":1}"#).expect("minimal canvas config");
    canvas.pane_layout = Some(PaneLayoutConfig::Split {
        axis: AxisConfig::Horizontal,
        ratio: 0.5,
        a: Box::new(widget_layout(42, 43)),
        b: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::LiveWatchlist {
            id: 45,
        })),
    });
    KeroseneConfig {
        active_symbol: "HYPE".to_string(),
        muted_tickers: vec!["ETH".to_string()],
        pane_layout: Some(widget_layout(7, 9)),
        canvases: vec![canvas],
        book_tick_size: 0.25,
        live_watchlists: Vec::new(),
        watchlist_presets: vec![WatchlistPresetConfig {
            id: 5,
            name: "Test preset".to_string(),
            symbols: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()],
        }],
        positioning_infos: serde_json::from_value(serde_json::json!([
            {
                "id": 7, "symbol": "BTC", "page": "Change", "side": "Short",
                "sort_field": "EntryPrice", "sort_direction": "Ascending",
                "entry_min": " 20 ", "entry_max": "30.5", "change_timeframe": "FourHours",
                "change_sort_field": "CurrentUsd", "change_sort_direction": "Ascending"
            },
            {"id": 18, "symbol": "  BTC  ", "sort_field": "CopyScore", "sort_direction": "Ascending"},
            {"id": 21, "symbol": "ETH"}
        ]))
        .expect("positioning configs"),
        order_books: serde_json::from_value(serde_json::json!([
            {
                "id": 9, "mode": {"Fixed": "@0"}, "tick_size": 0,
                "display_mode": "DomLadder", "center_on_mid": false,
                "reverse_side": true, "show_spread_chart": true, "spread_chart_height": 120
            },
            {
                "id": 19, "mode": {"Fixed": "ETH"}, "tick_size": 1.5,
                "display_mode": "DepthChart", "spread_chart_height": 10
            },
            {"id": 20, "tick_size": 0.5}
        ]))
        .expect("order book configs"),
        ..KeroseneConfig::default()
    }
}

#[test]
fn widget_restoration_preserves_settings_and_recovers_canvas_instances() {
    let config = widget_config();
    let mut expected_positioning = config.positioning_infos.clone();
    expected_positioning[1].symbol = "BTC".to_string();
    expected_positioning[1].sort_field = PositioningInfoSortField::UnrealizedPnl;
    expected_positioning[1].sort_direction = SortDirection::Descending;
    expected_positioning[2].symbol = "HYPE".to_string();
    expected_positioning.push(
        serde_json::from_str::<PositioningInfoConfig>(r#"{"id":42,"symbol":"HYPE"}"#)
            .expect("default positioning config"),
    );
    let mut expected_books = config.order_books.clone();
    expected_books[0].tick_size = 0.25;
    expected_books[1].mode = OrderBookSymbolModeConfig::Active;
    expected_books[1].spread_chart_height = 30.0;
    expected_books.push(
        serde_json::from_str::<OrderBookConfig>(r#"{"id":43,"tick_size":0.25}"#)
            .expect("default book config"),
    );

    let (mut terminal, _) = TradingTerminal::boot_from_config(config.clone());
    for boot in [true, false] {
        if !boot {
            let mut layout = terminal.saved_layout_snapshot("Test".to_string());
            layout.positioning_infos = config.positioning_infos.clone();
            layout.order_books = config.order_books.clone();
            layout.book_tick_size = config.book_tick_size;
            terminal
                .positioning_infos
                .insert(99, PositioningInfoInstance::new(99, "SOL".to_string()));
            terminal.order_books.insert(
                99,
                OrderBookInstance::new(99, OrderBookSymbolMode::Active, 1.0),
            );
            terminal.next_positioning_info_id = 100;
            terminal.next_order_book_id = 100;
            terminal
                .positioning_info_pending
                .insert("stale".to_string(), vec![99]);

            let _ = terminal.restore_layout_positioning_infos(&layout);
            let _ = terminal.restore_layout_order_books(&layout);
            assert!(terminal.positioning_info_pending.is_empty());
        }

        assert_eq!(
            terminal.positioning_info_configs_snapshot(),
            expected_positioning
        );
        let mut books = terminal.order_book_configs_snapshot();
        books.sort_by_key(|book| book.id);
        assert_eq!(books, expected_books);
        assert_eq!(terminal.next_positioning_info_id, 43);
        assert_eq!(terminal.next_order_book_id, 44);
    }
}

#[test]
fn ensuring_widget_instances_preserves_existing_runtime_state() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(widget_config());
    let positioning = terminal
        .positioning_infos
        .get_mut(&42)
        .expect("canvas positioning");
    positioning.search_query = "draft search".to_string();
    positioning.pending_key = Some("in-flight".to_string());
    let book = terminal.order_books.get_mut(&43).expect("canvas book");
    book.book_loading = false;
    book.book_error = Some("existing error".to_string());
    book.tick_size = 2.0;
    terminal
        .live_watchlists
        .get_mut(&45)
        .expect("canvas watchlist")
        .search_query = "watchlist search".to_string();

    terminal.ensure_positioning_info_pane_instances();
    terminal.ensure_order_book_pane_instances(0.01);
    terminal.ensure_live_watchlist_pane_instances();

    let positioning = &terminal.positioning_infos[&42];
    assert_eq!(positioning.search_query, "draft search");
    assert_eq!(positioning.pending_key.as_deref(), Some("in-flight"));
    let book = &terminal.order_books[&43];
    assert!(!book.book_loading);
    assert_eq!(book.book_error.as_deref(), Some("existing error"));
    assert_eq!(book.tick_size, 2.0);
    assert_eq!(
        terminal.live_watchlists[&45].search_query,
        "watchlist search"
    );
    assert_eq!(terminal.next_positioning_info_id, 43);
    assert_eq!(terminal.next_order_book_id, 44);
}

#[test]
fn default_watchlist_uses_visible_preset_symbols_at_boot_layout_and_add() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(widget_config());
    let boot_snapshot = terminal.live_watchlist_configs_snapshot();
    assert_eq!(boot_snapshot.len(), 1);
    assert_eq!(boot_snapshot[0].id, 45);
    assert_eq!(boot_snapshot[0].preset_id, Some(5));
    assert_eq!(boot_snapshot[0].symbols, ["BTC", "SOL"]);
    assert_eq!(
        boot_snapshot[0].visible_columns,
        config::default_live_watchlist_columns()
    );

    let mut layout = terminal.saved_layout_snapshot("Test".to_string());
    layout.live_watchlists.clear();
    terminal.restore_layout_live_watchlists(&layout);
    assert_eq!(terminal.live_watchlist_configs_snapshot(), boot_snapshot);

    terminal.add_widget_workspace = WorkspaceId::Canvas(1);
    let _ = terminal.update_live_watchlist_market(Message::AddLiveWatchlistPane);
    assert_eq!(terminal.live_watchlists.len(), 2);
    let added = terminal
        .live_watchlists
        .values()
        .find(|watchlist| watchlist.id != 45)
        .expect("added watchlist");
    assert_eq!(added.preset_id, Some(5));
    assert_eq!(added.symbols, ["BTC", "SOL"]);
    assert!(terminal.workspace_pane_kinds().any(|(workspace, _, kind)| {
        workspace == WorkspaceId::Canvas(1)
            && matches!(kind, PaneKind::LiveWatchlist(id) if *id == added.id)
    }));
}

#[test]
fn missing_watchlist_creates_one_preset_when_none_exist() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(widget_config());
    terminal.watchlist_presets.clear();
    terminal.live_watchlists.clear();

    terminal.ensure_live_watchlist_pane_instances();
    terminal.ensure_live_watchlist_pane_instances();

    assert_eq!(terminal.watchlist_presets.len(), 1);
    let watchlist = &terminal.live_watchlists[&45];
    assert_eq!(watchlist.preset_id, Some(terminal.watchlist_presets[0].id));
    assert!(watchlist.symbols.is_empty());
}
