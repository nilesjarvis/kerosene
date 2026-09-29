use super::*;
use crate::api::{ExchangeSymbol, MarketType};
use crate::config::KeroseneConfig;
use crate::market_state::SymbolSearchMarketFilter;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};

fn symbol(key: &str, ticker: &str, kind: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 1,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 10,
        only_isolated: false,
        growth_mode: false,
        market_type: kind,
        outcome: None,
    }
}

#[tokio::test]
async fn symbol_search_preserves_status_layout_group_order_and_row_actions() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for mode in SymbolSearchSortMode::ALL {
        for state in 0..8 {
            let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
            terminal.symbol_search_sort_mode = mode;
            terminal.spinner_phase = 0.25;
            terminal.symbols_loading = state == 1;
            terminal.symbol_search_contexts_loading = state == 4;
            terminal.symbol_search_status = match state {
                2 | 4 => Some(("Synthetic refresh status".to_string(), false)),
                5 => Some(("Synthetic refresh error".to_string(), true)),
                _ => None,
            };
            terminal.exchange_symbols = if state < 3 {
                Vec::new()
            } else {
                vec![
                    symbol("ETH", "ETH", MarketType::Perp),
                    symbol("@1", "AAA", MarketType::Spot),
                    symbol("xyz:BTC", "BTC", MarketType::Perp),
                    symbol("BTC", "BTC", MarketType::Perp),
                    symbol("abc:SOL", "SOL", MarketType::Perp),
                ]
            };
            terminal.favourite_symbols = vec!["ETH".to_string(), "BTC".to_string()];
            terminal.active_symbol = "BTC".to_string();
            if state >= 6 {
                terminal.symbol_search_market_filter = SymbolSearchMarketFilter::Hip3;
                terminal.symbol_search_hip3_dex_filter =
                    (state == 7).then(|| "missing".to_string());
            }
            terminal.refresh_symbol_search_results();
            let theme = terminal.theme();
            let size = Size::new(560.0, 500.0);
            let mut view = terminal.view_watchlist();
            let bounds = Rectangle::with_size(size);
            let mut tree = Tree::new(view.as_widget());
            let node =
                view.as_widget_mut()
                    .layout(&mut tree, &renderer, &layout::Limits::new(size, size));
            iced::advanced::Renderer::reset(&mut renderer, bounds);
            view.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style {
                    text_color: theme.palette().text,
                },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &bounds,
            );
            let pixels = renderer.screenshot(Size::new(560, 500), 1.0, theme.palette().background);
            assert_eq!(pixels.len(), 560 * 500 * 4);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
            if let Some(directory) = std::env::var_os("KEROSENE_SEARCH_VIEW_PREVIEW_DIR") {
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).expect("preview directory");
                image::save_buffer(
                    directory.join(format!("{mode:?}-{state}.png")),
                    &pixels,
                    560,
                    500,
                    image::ColorType::Rgba8,
                )
                .expect("synthetic preview");
            }
            let mut selected = Vec::new();
            let mut favourites = Vec::new();
            // Start below the picker controls to exercise rows without opening overlays.
            let first_y = if state >= 6 { 110 } else { 90 };
            for y in (first_y..490).step_by(3) {
                for x in [10.0, 100.0] {
                    let mut messages = Vec::new();
                    for event in [
                        mouse::Event::ButtonPressed(mouse::Button::Left),
                        mouse::Event::ButtonReleased(mouse::Button::Left),
                    ] {
                        view.as_widget_mut().update(
                            &mut tree,
                            &Event::Mouse(event),
                            Layout::new(&node),
                            mouse::Cursor::Available(Point::new(x, y as f32)),
                            &renderer,
                            &mut clipboard::Null,
                            &mut Shell::new(&mut messages),
                            &bounds,
                        );
                    }
                    for message in messages {
                        let (list, key) = match message {
                            Message::SymbolSelected(key) => (&mut selected, key),
                            Message::ToggleFavourite(key) => (&mut favourites, key),
                            _ => continue,
                        };
                        if list.last() != Some(&key) {
                            list.push(key);
                        }
                    }
                }
            }
            let expected = if state < 3 || state == 7 {
                vec![]
            } else if state == 6 {
                if mode == SymbolSearchSortMode::Exchange {
                    vec!["abc:SOL", "xyz:BTC"]
                } else {
                    vec!["xyz:BTC", "abc:SOL"]
                }
            } else if mode == SymbolSearchSortMode::Exchange {
                vec!["ETH", "BTC", "@1", "abc:SOL", "xyz:BTC"]
            } else {
                vec!["ETH", "BTC", "@1", "xyz:BTC", "abc:SOL"]
            };
            assert_eq!(selected, expected, "{mode:?}/{state}");
            assert_eq!(favourites, expected, "{mode:?}/{state}");
        }
    }
}
