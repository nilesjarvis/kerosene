use super::*;
use crate::config::{LiveWatchlistColumn, OrderBookSymbolModeConfig};
use crate::market_state::{
    LiveWatchlistInstance, OrderBookDisplayMode, OrderBookInstance, OrderBookSymbolMode,
};
use crate::pane_state::PaneKind;
use crate::positioning_state::{PositioningInfoInstance, PositioningInfoPage};
use crate::session_data_state::SessionDataInstance;
use crate::spaghetti_state::SpaghettiChartInstance;
use crate::trade_feed_state::TradeFeedInstance;

const PICKERS: [MarketPicker; 10] = [
    MarketPicker::Spaghetti(901),
    MarketPicker::Spaghetti(902),
    MarketPicker::OrderBook(903),
    MarketPicker::OrderBook(904),
    MarketPicker::LiveWatchlist(905),
    MarketPicker::Positioning(906),
    MarketPicker::Positioning(907),
    MarketPicker::SessionData(908),
    MarketPicker::TradeFeed(909),
    MarketPicker::LiquidationDistribution,
];

fn terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    // Root updates prune feed instances that have no open pane.
    terminal.panes = iced::widget::pane_grid::State::new(PaneKind::TradeFeed(909)).0;
    terminal.exchange_symbols = ["BTC", "ETH"]
        .into_iter()
        .enumerate()
        .map(|(index, symbol)| ExchangeSymbol {
            key: symbol.into(),
            ticker: symbol.into(),
            category: "crypto".into(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: index as u32,
            collateral_token: None,
            sz_decimals: 4,
            max_leverage: 25,
            only_isolated: false,
            growth_mode: false,
            market_type: MarketType::Perp,
            outcome: None,
        })
        .collect();
    terminal
        .spaghetti_charts
        .insert(901, SpaghettiChartInstance::new_empty(901));
    terminal
        .spaghetti_charts
        .insert(902, SpaghettiChartInstance::new_pair(902));
    for (id, display_mode) in [
        (903, OrderBookDisplayMode::DepthList),
        (904, OrderBookDisplayMode::DepthChart),
    ] {
        let mut instance =
            OrderBookInstance::new(id, OrderBookSymbolMode::Fixed("ETH".into()), 1.0);
        instance.settings_open = true;
        instance.display_mode = display_mode;
        terminal.order_books.insert(id, instance);
    }
    terminal.live_watchlists.insert(
        905,
        LiveWatchlistInstance {
            id: 905,
            preset_id: None,
            symbols: Vec::new(),
            search_query: String::new(),
            sort_column: Default::default(),
            sort_direction: Default::default(),
            visible_columns: vec![LiveWatchlistColumn::Price],
            ema: Default::default(),
            ema_period_input: "20".into(),
            row_cache: Vec::new(),
        },
    );
    for (id, page) in [
        (906, PositioningInfoPage::Positions),
        (907, PositioningInfoPage::Change),
    ] {
        let mut instance = PositioningInfoInstance::new(id, "ETH".into());
        instance.symbol_picker_open = true;
        instance.page = page;
        terminal.positioning_infos.insert(id, instance);
    }
    let mut session = SessionDataInstance::new(908, "ETH".into(), Default::default());
    session.symbol_picker_open = true;
    terminal.session_data.insert(908, session);
    let mut feed = TradeFeedInstance::new(OrderBookSymbolModeConfig::Fixed("ETH".into()));
    feed.picker_open = true;
    terminal.trade_feeds.instances.insert(909, feed);
    terminal.liquidation_distribution.symbol_picker_open = true;
    terminal
}

fn set_preview(terminal: &mut TradingTerminal, picker: MarketPicker, state: &str) {
    let catalog = &mut terminal.market_browser.papertrade;
    catalog.data = (!matches!(state, "loading" | "error")).then(|| PapertradeMarkets {
        indexed_through_block: 1234,
        markets: vec![PapertradeMarket {
            id: PapertradeMarketId(0),
            symbol: "BTC".into(),
            max_leverage: 1000,
            status: "Perp",
        }],
    });
    catalog.loading = state == "loading";
    catalog.error =
        matches!(state, "error" | "cached-error").then(|| "Papertrade markets unavailable.".into());
    let query = if state == "empty" { "missing" } else { "BTC" }.to_string();
    let message = match picker {
        MarketPicker::Spaghetti(id) => Message::SpaghettiEditorSearchChanged(id, query),
        MarketPicker::OrderBook(id) => Message::OrderBookSearchChanged(id, query),
        MarketPicker::LiveWatchlist(id) => Message::LiveWatchlistSearchChanged(id, query),
        MarketPicker::Positioning(id) => Message::PositioningInfoSearchChanged(id, query),
        MarketPicker::SessionData(id) => Message::SessionDataSearchChanged(id, query),
        MarketPicker::TradeFeed(id) => Message::TradeFeedSearchChanged(id, query),
        MarketPicker::LiquidationDistribution => {
            Message::LiquidationsDistributionSearchChanged(query)
        }
        _ => panic!("non-chart picker fixture"),
    };
    let _task = terminal.update(message);
}

fn render_and_click(
    terminal: &TradingTerminal,
    picker: MarketPicker,
    renderer: &mut iced::Renderer,
    size: Size,
) -> Vec<Message> {
    let mut view = match picker {
        MarketPicker::Spaghetti(id) => {
            terminal.view_spaghetti_editor(id, &terminal.spaghetti_charts[&id])
        }
        MarketPicker::OrderBook(id) => terminal.view_order_book(id),
        MarketPicker::LiveWatchlist(id) => terminal.view_live_watchlist(id),
        MarketPicker::Positioning(id) => terminal.view_positioning_info(id),
        MarketPicker::SessionData(id) => terminal.view_session_data(id),
        MarketPicker::TradeFeed(id) => terminal.view_trade_feed(id),
        MarketPicker::LiquidationDistribution => terminal.view_liquidations_distribution(),
        _ => panic!("non-chart picker fixture"),
    };
    let bounds = Rectangle::with_size(size);
    let mut tree = Tree::new(view.as_widget());
    let node = view
        .as_widget_mut()
        .layout(&mut tree, renderer, &layout::Limits::new(size, size));
    iced::advanced::Renderer::reset(renderer, bounds);
    let theme = terminal.theme();
    view.as_widget().draw(
        &tree,
        renderer,
        &theme,
        &renderer::Style {
            text_color: theme.palette().text,
        },
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &bounds,
    );
    let pixels = renderer.screenshot(
        Size::new(size.width as u32, size.height as u32),
        1.0,
        theme.palette().background,
    );
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));

    // Probe across the real pane, including responsive widgets whose nested
    // layout is deferred. Messages are inspected, not executed as trade actions.
    let mut messages = Vec::new();
    for x in [40.0, 140.0, 280.0, size.width - 35.0] {
        for y in (0..size.height as usize).step_by(4) {
            for event in [
                mouse::Event::ButtonPressed(mouse::Button::Left),
                mouse::Event::ButtonReleased(mouse::Button::Left),
            ] {
                view.as_widget_mut().update(
                    &mut tree,
                    &Event::Mouse(event),
                    Layout::new(&node),
                    mouse::Cursor::Available(Point::new(x, y as f32)),
                    renderer,
                    &mut clipboard::Null,
                    &mut Shell::new(&mut messages),
                    &bounds,
                );
            }
        }
    }
    messages
}

fn selection(message: &Message) -> Option<(MarketPicker, &str)> {
    match message {
        Message::SpaghettiAddSymbol(id, key) | Message::SpaghettiRemoveSymbol(id, key) => {
            Some((MarketPicker::Spaghetti(*id), key))
        }
        Message::OrderBookSetMode(id, OrderBookSymbolMode::Fixed(key)) => {
            Some((MarketPicker::OrderBook(*id), key))
        }
        Message::LiveWatchlistAddSymbol(id, key) => Some((MarketPicker::LiveWatchlist(*id), key)),
        Message::PositioningInfoSymbolSelected(id, key) => {
            Some((MarketPicker::Positioning(*id), key))
        }
        Message::SessionDataSymbolSelected(id, key) => Some((MarketPicker::SessionData(*id), key)),
        Message::TradeFeedSetMode(id, OrderBookSymbolModeConfig::Fixed(key)) => {
            Some((MarketPicker::TradeFeed(*id), key))
        }
        Message::LiquidationsDistributionSymbolSelected(key) => {
            Some((MarketPicker::LiquidationDistribution, key))
        }
        Message::SymbolSelected(key) | Message::ToggleFavourite(key) => {
            Some((MarketPicker::Symbols, key))
        }
        Message::ChartSymbolSelected(id, key) => Some((MarketPicker::Chart(*id), key)),
        Message::ChartSecondarySymbolSelected(id, key) => {
            Some((MarketPicker::ChartSecondary(*id), key))
        }
        _ => None,
    }
}

fn switch(
    terminal: &mut TradingTerminal,
    messages: &[Message],
    picker: MarketPicker,
    venue: MarketVenue,
) {
    let message = messages.iter().find(|message| matches!(message,
        Message::MarketPickerVenueChanged(target, selected) if *target == picker && *selected == venue
    )).unwrap_or_else(|| panic!("{venue:?} control unreachable for {picker:?}"));
    let _task = terminal.update(message.clone());
    assert_eq!(terminal.market_browser.venue(picker), venue);
    for other in PICKERS.into_iter().filter(|other| *other != picker) {
        assert_eq!(
            terminal.market_browser.venue(other),
            MarketVenue::Hyperliquid
        );
    }
}

#[tokio::test]
async fn papertrade_widget_pickers_preserve_hyperliquid_actions_and_browse_only_previews() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = terminal();
    let active_symbol = terminal.active_symbol.clone();
    let active_account = terminal.active_account_index;
    for width in [360.0, 640.0] {
        let size = Size::new(width, 480.0);
        for picker in PICKERS {
            assert_eq!(
                terminal.market_browser.venue(picker),
                MarketVenue::Hyperliquid
            );
            set_preview(&mut terminal, picker, "loaded");
            let messages = render_and_click(&terminal, picker, &mut renderer, size);
            assert!(
                messages
                    .iter()
                    .any(|message| selection(message) == Some((picker, "BTC"))),
                "Hyperliquid selection unreachable for {picker:?} at {width}"
            );
            switch(&mut terminal, &messages, picker, MarketVenue::Papertrade);
            for state in ["loaded", "empty", "loading", "error", "cached-error"] {
                set_preview(&mut terminal, picker, state);
                let messages = render_and_click(&terminal, picker, &mut renderer, size);
                assert!(
                    !messages.iter().any(|message| selection(message).is_some()),
                    "Papertrade preview emitted selection for {picker:?}/{state} at {width}"
                );
                assert!(messages.iter().any(|message| matches!(message, Message::MarketPickerVenueChanged(target, MarketVenue::Hyperliquid) if *target == picker)), "return switch unreachable for {picker:?}/{state} at {width}");
                assert_eq!(
                    messages
                        .iter()
                        .any(|message| matches!(message, Message::RefreshPapertradeMarkets)),
                    state != "loading",
                    "Refresh/Retry availability for {picker:?}/{state} at {width}"
                );
            }
            let messages = render_and_click(&terminal, picker, &mut renderer, size);
            switch(&mut terminal, &messages, picker, MarketVenue::Hyperliquid);
            let messages = render_and_click(&terminal, picker, &mut renderer, size);
            assert!(
                messages
                    .iter()
                    .any(|message| selection(message) == Some((picker, "BTC"))),
                "Hyperliquid selection not restored for {picker:?} at {width}"
            );
            assert_eq!(terminal.active_symbol, active_symbol);
            assert_eq!(terminal.active_account_index, active_account);
        }
    }
}
