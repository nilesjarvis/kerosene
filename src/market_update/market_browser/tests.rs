use super::*;
use crate::chart_state::ChartInstance;
use crate::config::KeroseneConfig;
use crate::timeframe::Timeframe;

#[test]
fn papertrade_browsing_preserves_active_account_symbol_and_hyperliquid_search() {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    let active_symbol = terminal.active_symbol.clone();
    let active_account = terminal.active_account_index;
    terminal.symbol_search_query = "BTC".into();
    terminal.symbol_search_market_filter = crate::market_state::SymbolSearchMarketFilter::Spot;
    let _task = terminal.update_market(Message::MarketPickerVenueChanged(
        MarketPicker::Symbols,
        MarketVenue::Papertrade,
    ));
    assert_eq!(terminal.active_symbol, active_symbol);
    assert_eq!(terminal.active_account_index, active_account);
    assert_eq!(terminal.symbol_search_query, "BTC");
    assert_eq!(
        terminal.symbol_search_market_filter,
        crate::market_state::SymbolSearchMarketFilter::Spot
    );
    assert!(terminal.market_browser.papertrade.loading);
    let _task = terminal.update_market(Message::MarketPickerVenueChanged(
        MarketPicker::Symbols,
        MarketVenue::Hyperliquid,
    ));
    assert_eq!(
        terminal.market_browser.venue(MarketPicker::Symbols),
        MarketVenue::Hyperliquid
    );
    assert_eq!(terminal.active_symbol, active_symbol);
}

#[test]
fn papertrade_deleted_picker_cannot_start_a_request() {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    let _task = terminal.update_market(Message::MarketPickerVenueChanged(
        MarketPicker::Chart(u64::MAX),
        MarketVenue::Papertrade,
    ));
    assert!(!terminal.market_browser.papertrade.loading);
    assert!(terminal.market_browser.venues.is_empty());
}

#[test]
fn papertrade_chart_submit_cannot_select_hyperliquid_with_same_ticker() {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.exchange_symbols = vec![crate::api::ExchangeSymbol {
        key: "ETH".into(),
        ticker: "ETH".into(),
        category: "crypto".into(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 1,
        collateral_token: None,
        sz_decimals: 4,
        max_leverage: 25,
        only_isolated: false,
        growth_mode: false,
        market_type: crate::api::MarketType::Perp,
        outcome: None,
    }];
    let id = 900;
    terminal
        .charts
        .insert(id, ChartInstance::new(id, "BTC".into(), Timeframe::H1));
    for secondary in [false, true] {
        let instance = terminal.charts.get_mut(&id).expect("chart");
        instance.editor_search_query = "ETH".into();
        instance.secondary_editor_search_query = "ETH".into();
        instance.editor_selected_index = Some(0);
        instance.secondary_editor_selected_index = Some(0);
        let _task = terminal.update_market(Message::MarketPickerVenueChanged(
            MarketPicker::chart(id, secondary),
            MarketVenue::Papertrade,
        ));
        let instance = &terminal.charts[&id];
        assert_eq!(
            if secondary {
                instance.secondary_editor_selected_index
            } else {
                instance.editor_selected_index
            },
            None
        );
        let _task = terminal.update_chart(if secondary {
            Message::ChartSecondaryEditorSubmit(id)
        } else {
            Message::ChartEditorSubmit(id)
        });
        let instance = &terminal.charts[&id];
        assert_eq!(instance.symbol, "BTC");
        assert!(instance.secondary_symbol.is_none());
    }
    let _task = terminal.update_market(Message::MarketPickerVenueChanged(
        MarketPicker::Chart(id),
        MarketVenue::Hyperliquid,
    ));
    let _task = terminal.update_chart(Message::ChartEditorSubmit(id));
    assert_eq!(terminal.charts[&id].symbol, "ETH");
}

#[test]
fn papertrade_chart_browsing_does_not_change_another_picker_or_chart() {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    for id in [900, 901] {
        terminal
            .charts
            .insert(id, ChartInstance::new(id, "BTC".into(), Timeframe::H1));
    }
    let _task = terminal.update_market(Message::MarketPickerVenueChanged(
        MarketPicker::Chart(900),
        MarketVenue::Papertrade,
    ));
    for picker in [
        MarketPicker::Chart(901),
        MarketPicker::ChartSecondary(900),
        MarketPicker::Symbols,
    ] {
        assert_eq!(
            terminal.market_browser.venue(picker),
            MarketVenue::Hyperliquid
        );
    }
    terminal.charts.remove(&900);
    let _task = terminal.update_market(Message::MarketPickerVenueChanged(
        MarketPicker::Symbols,
        MarketVenue::Papertrade,
    ));
    assert!(
        !terminal
            .market_browser
            .venues
            .contains_key(&MarketPicker::Chart(900))
    );
}
