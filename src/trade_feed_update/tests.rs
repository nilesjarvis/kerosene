use super::*;
use crate::config::{KeroseneConfig, PaneKindConfig, PaneLayoutConfig};
use crate::trade_feed_state::{TradeFeedEvent, TradePrint};

fn terminal(mode: OrderBookSymbolModeConfig) -> TradingTerminal {
    let config = KeroseneConfig {
        active_symbol: "BTC".into(),
        pane_layout: Some(PaneLayoutConfig::Leaf(PaneKindConfig::TradeFeed { id: 7 })),
        trade_feeds: vec![TradeFeedConfig {
            id: 7,
            mode,
            ..Default::default()
        }],
        ..Default::default()
    };
    TradingTerminal::boot_from_config(config).0
}

fn incoming(generation: u64, symbol: &str) -> Message {
    Message::TradeFeedStream(
        7,
        generation,
        symbol.into(),
        TradeFeedEvent::Trades(vec![TradePrint {
            time_ms: 1000,
            tid: 1,
            is_buy: true,
            price: "1".into(),
            size: "2".into(),
            value: 2.0,
            trader: None,
        }]),
    )
}

#[test]
fn trade_feed_follows_active_and_rejects_old_generation_after_round_trip() {
    let mut terminal = terminal(Default::default());
    terminal.apply_active_symbol_selection("BTC".into(), "BTC".into());
    terminal.sync_trade_feeds();
    let original = terminal.trade_feeds.instances[&7].generation;
    let _ = terminal.update(incoming(original, "BTC"));
    assert_eq!(terminal.trade_feeds.instances[&7].trades.len(), 1);
    terminal.apply_active_symbol_selection("ETH".into(), "ETH".into());
    terminal.sync_trade_feeds();
    assert!(terminal.trade_feeds.instances[&7].trades.is_empty());
    assert_eq!(
        terminal.trade_feeds.instances[&7].symbol.as_deref(),
        Some("ETH")
    );
    terminal.apply_active_symbol_selection("BTC".into(), "BTC".into());
    terminal.sync_trade_feeds();
    let _ = terminal.update(incoming(original, "BTC"));
    assert!(terminal.trade_feeds.instances[&7].trades.is_empty());
    let current = terminal.trade_feeds.instances[&7].generation;
    let _ = terminal.update(incoming(current, "BTC"));
    assert_eq!(terminal.trade_feeds.instances[&7].trades.len(), 1);
}

#[test]
fn trade_feed_pinned_asset_and_layout_restore_keep_settings_but_reset_stream() {
    let mode = OrderBookSymbolModeConfig::Fixed("xyz:BTC".into());
    let mut terminal = terminal(mode.clone());
    terminal.apply_active_symbol_selection("ETH".into(), "ETH".into());
    terminal.sync_trade_feeds();
    assert_eq!(
        terminal.trade_feeds.instances[&7].symbol.as_deref(),
        Some("xyz:BTC")
    );
    let generation = terminal.trade_feeds.instances[&7].generation;
    let _ = terminal.update(incoming(generation, "xyz:BTC"));
    let snapshot = terminal.saved_layout_snapshot("Tape".into());
    let serialized = serde_json::to_string(&snapshot).expect("layout serialization");
    let restored = serde_json::from_str(&serialized).expect("layout deserialization");
    let _ = terminal.apply_layout(restored);
    assert_eq!(terminal.trade_feeds.instances[&7].mode, mode);
    assert!(terminal.trade_feeds.instances[&7].trades.is_empty());
    let _ = terminal.update(incoming(generation, "xyz:BTC"));
    assert!(terminal.trade_feeds.instances[&7].trades.is_empty());
}

#[test]
fn trade_feed_missing_config_defaults_to_active_and_closed_pane_releases_state() {
    let config = KeroseneConfig {
        pane_layout: Some(PaneLayoutConfig::Leaf(PaneKindConfig::TradeFeed { id: 12 })),
        ..Default::default()
    };
    let (mut terminal, _) = TradingTerminal::boot_from_config(config);
    assert_eq!(
        terminal.trade_feeds.instances[&12].mode,
        OrderBookSymbolModeConfig::Active
    );
    terminal.panes = iced::widget::pane_grid::State::new(PaneKind::Watchlist).0;
    terminal.sync_trade_feeds();
    assert!(terminal.trade_feeds.instances.is_empty());
}

#[test]
fn trade_feed_config_defaults_and_round_trip() {
    let config: TradeFeedConfig = serde_json::from_str(r#"{"id":4}"#).expect("legacy defaults");
    assert_eq!(config.mode, OrderBookSymbolModeConfig::Active);
    assert!(!config.group_trades);
    assert!(!config.size_in_notional);
    let config = TradeFeedConfig {
        id: 4,
        mode: OrderBookSymbolModeConfig::Fixed("@107".into()),
        group_trades: true,
        size_in_notional: true,
    };
    let json = serde_json::to_string(&config).expect("serialize");
    assert_eq!(
        serde_json::from_str::<TradeFeedConfig>(&json).expect("deserialize"),
        config
    );
}

#[test]
fn trade_feed_size_toggle_is_per_pane_and_survives_asset_and_layout_changes() {
    let mut terminal = terminal(Default::default());
    let pane = *terminal.panes.iter().next().expect("trade feed pane").0;
    terminal
        .panes
        .split(
            iced::widget::pane_grid::Axis::Vertical,
            pane,
            PaneKind::TradeFeed(8),
        )
        .expect("second trade feed pane");
    terminal
        .trade_feeds
        .instances
        .insert(8, TradeFeedInstance::new(Default::default()));
    terminal.sync_trade_feeds();
    let generation = terminal.trade_feeds.instances[&7].generation;
    let _ = terminal.update(incoming(generation, "BTC"));
    let _ = terminal.update(Message::TradeFeedToggleSizeUnit(7));
    assert!(terminal.trade_feeds.instances[&7].size_in_notional);
    assert!(!terminal.trade_feeds.instances[&8].size_in_notional);
    assert_eq!(terminal.trade_feeds.instances[&7].generation, generation);
    assert_eq!(terminal.trade_feeds.instances[&7].trades.len(), 1);

    terminal.apply_active_symbol_selection("ETH".into(), "ETH".into());
    terminal.sync_trade_feeds();
    assert!(terminal.trade_feeds.instances[&7].size_in_notional);
    let snapshot = terminal.saved_layout_snapshot("Notional tape".into());
    let json = serde_json::to_string(&snapshot).expect("serialize notional layout");
    let snapshot = serde_json::from_str(&json).expect("deserialize notional layout");
    let _ = terminal.apply_layout(snapshot);
    assert!(terminal.trade_feeds.instances[&7].size_in_notional);
    assert!(!terminal.trade_feeds.instances[&8].size_in_notional);
    let _ = terminal.update(Message::TradeFeedToggleSizeUnit(7));
    assert!(!terminal.trade_feeds.instances[&7].size_in_notional);
    assert!(!terminal.trade_feed_configs_snapshot()[0].size_in_notional);
}

#[test]
fn trade_feed_grouping_toggle_is_per_pane_and_survives_layout_restore() {
    let mut terminal = terminal(Default::default());
    let generation = terminal.trade_feeds.instances[&7].generation;
    let _ = terminal.update(incoming(generation, "BTC"));
    terminal
        .trade_feeds
        .instances
        .insert(8, TradeFeedInstance::new(Default::default()));
    let _ = terminal.update_trade_feed(Message::TradeFeedToggleGrouping(7));
    assert!(terminal.trade_feeds.instances[&7].group_trades);
    assert!(!terminal.trade_feeds.instances[&8].group_trades);
    assert_eq!(terminal.trade_feeds.instances[&7].generation, generation);
    assert_eq!(terminal.trade_feeds.instances[&7].trades.len(), 1);
    let snapshot = terminal.saved_layout_snapshot("Grouped tape".into());
    let json = serde_json::to_string(&snapshot).expect("serialize grouped layout");
    let snapshot = serde_json::from_str(&json).expect("deserialize grouped layout");
    let _ = terminal.apply_layout(snapshot);
    assert!(terminal.trade_feeds.instances[&7].group_trades);
    assert!(terminal.trade_feeds.instances[&7].trades.is_empty());
    let _ = terminal.update(Message::TradeFeedToggleGrouping(7));
    assert!(!terminal.trade_feeds.instances[&7].group_trades);
    assert!(!terminal.trade_feed_configs_snapshot()[0].group_trades);
}

#[test]
fn trade_feed_address_hover_holds_rows_while_ingestion_continues_and_resumes_on_exit() {
    let mut terminal = terminal(Default::default());
    let generation = terminal.trade_feeds.instances[&7].generation;
    let _ = terminal.update(incoming(generation, "BTC"));
    let feed = &terminal.trade_feeds.instances[&7];
    let key = feed.address_hover_key(7, &feed.trades[0]);
    let _ = terminal.update(Message::WalletAddressActionsHovered(key.clone().into()));
    assert!(terminal.trade_feeds.instances[&7].is_held());
    let mut next = incoming(generation, "BTC");
    if let Message::TradeFeedStream(_, _, _, TradeFeedEvent::Trades(trades)) = &mut next {
        trades[0].time_ms += 1;
        trades[0].tid += 1;
    }
    let _ = terminal.update(next);
    let feed = &terminal.trade_feeds.instances[&7];
    assert_eq!(feed.trades[0].tid, 2);
    assert_eq!(feed.visible_trades()[0].tid, 1);
    // An exit from another row must not release this menu's stable target.
    let _ = terminal.update(Message::WalletAddressActionsExited("another-row".into()));
    assert!(terminal.trade_feeds.instances[&7].is_held());
    let _ = terminal.update(Message::WalletAddressActionsExited(key.into()));
    let feed = &terminal.trade_feeds.instances[&7];
    assert!(!feed.is_held());
    assert_eq!(feed.visible_trades()[0].tid, 2);
}

#[test]
fn trade_feed_address_hover_is_scoped_to_pane_and_subscription_generation() {
    let mut terminal = terminal(Default::default());
    let generation = terminal.trade_feeds.instances[&7].generation;
    let _ = terminal.update(incoming(generation, "BTC"));
    let feed = &terminal.trade_feeds.instances[&7];
    let own_key = feed.address_hover_key(7, &feed.trades[0]);
    let other_key = feed.address_hover_key(8, &feed.trades[0]);
    let _ = terminal.update(Message::WalletAddressActionsHovered(other_key.into()));
    assert!(!terminal.trade_feeds.instances[&7].is_held());
    let _ = terminal.update(Message::WalletAddressActionsHovered(own_key.into()));
    assert!(terminal.trade_feeds.instances[&7].is_held());
    terminal.apply_active_symbol_selection("ETH".into(), "ETH".into());
    terminal.sync_trade_feeds();
    let feed = &terminal.trade_feeds.instances[&7];
    assert!(!feed.is_held());
    assert!(feed.visible_trades().is_empty());
}
