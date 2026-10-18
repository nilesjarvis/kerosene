use super::*;
use crate::config::{KeroseneConfig, PaneKindConfig, PaneLayoutConfig};
use crate::trade_feed_state::{TradeFeedEvent, TradePrint};

fn terminal(mode: OrderBookSymbolModeConfig) -> TradingTerminal {
    let config = KeroseneConfig {
        active_symbol: "BTC".into(),
        pane_layout: Some(PaneLayoutConfig::Leaf(PaneKindConfig::TradeFeed { id: 7 })),
        trade_feeds: vec![TradeFeedConfig { id: 7, mode }],
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
    let config = TradeFeedConfig {
        id: 4,
        mode: OrderBookSymbolModeConfig::Fixed("@107".into()),
    };
    let json = serde_json::to_string(&config).expect("serialize");
    assert_eq!(
        serde_json::from_str::<TradeFeedConfig>(&json).expect("deserialize"),
        config
    );
}
