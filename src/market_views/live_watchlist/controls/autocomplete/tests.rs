use super::live_watchlist_autocomplete_matches;
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use crate::message::Message;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};

fn outcome_symbol() -> ExchangeSymbol {
    ExchangeSymbol {
        key: "#950".to_string(),
        ticker: "OUT95-YES".to_string(),
        category: "outcome".to_string(),
        display_name: Some("YES: Will BTC close green?".to_string()),
        keywords: vec!["prediction".to_string()],
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        growth_mode: false,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 95,
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

#[test]
fn autocomplete_matches_outcome_question_text_in_display_name() {
    let symbol = outcome_symbol();

    assert!(live_watchlist_autocomplete_matches(
        &symbol,
        "btc close green"
    ));
}

#[test]
fn autocomplete_matches_keywords_key_and_ticker() {
    let symbol = outcome_symbol();

    assert!(live_watchlist_autocomplete_matches(&symbol, "prediction"));
    assert!(live_watchlist_autocomplete_matches(&symbol, "#950"));
    assert!(live_watchlist_autocomplete_matches(&symbol, "out95"));
    assert!(!live_watchlist_autocomplete_matches(&symbol, "solana"));
}

#[tokio::test]
async fn autocomplete_scrolls_to_and_selects_matches_beyond_the_first_five() {
    let renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.exchange_symbols = (0..8)
        .rev()
        .map(|index| {
            let mut symbol = outcome_symbol();
            symbol.key = format!("#{}", 950 + index);
            symbol.ticker = format!("BTC{index}");
            symbol.display_name = Some(format!("BTC market {index}"));
            symbol
        })
        .collect();

    for available_height in [240.0, 72.0] {
        let mut view = terminal.view_live_watchlist_autocomplete(7, "btc");
        let mut tree = Tree::new(view.as_widget());
        let node = view.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(300.0, available_height)),
        );
        assert_eq!(node.size().height, available_height.min(120.0));

        let bounds = Rectangle::with_size(node.size());
        let cursor = mouse::Cursor::Available(Point::new(30.0, bounds.height - 10.0));
        let mut messages = Vec::new();
        for event in [
            mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -1000.0 },
            },
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            view.as_widget_mut().update(
                &mut tree,
                &Event::Mouse(event),
                Layout::new(&node),
                cursor,
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &bounds,
            );
        }

        assert!(messages.iter().any(|message| {
            matches!(message, Message::LiveWatchlistAddSymbol(7, symbol) if symbol == "#957")
        }));
    }
}
