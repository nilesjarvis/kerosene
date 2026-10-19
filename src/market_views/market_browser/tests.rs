use super::*;
use crate::api::{ExchangeSymbol, MarketType};
use crate::chart_state::ChartInstance;
use crate::config::KeroseneConfig;
use crate::papertrade::PapertradeMarkets;
use crate::papertrade::markets::{PapertradeMarket, PapertradeMarketId};
use crate::timeframe::Timeframe;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};

mod widgets;

#[tokio::test]
async fn papertrade_picker_preview_renders_and_never_emits_symbol_selection() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    let chart_id = 900;
    terminal.charts.insert(
        chart_id,
        ChartInstance::new(chart_id, "BTC".into(), Timeframe::H1),
    );
    terminal.exchange_symbols = ["BTC", "ETH"]
        .into_iter()
        .enumerate()
        .map(|(index, ticker)| ExchangeSymbol {
            key: ticker.into(),
            ticker: ticker.into(),
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
    terminal.refresh_symbol_search_results();
    for venue in MarketVenue::ALL {
        for picker in [
            MarketPicker::Chart(chart_id),
            MarketPicker::ChartSecondary(chart_id),
            MarketPicker::Symbols,
        ] {
            terminal.market_browser.venues.insert(picker, venue);
            for (state, query) in [
                ("loaded", ""),
                ("empty", "missing"),
                ("error", ""),
                ("loading", ""),
            ] {
                terminal.market_browser.papertrade.data = Some(PapertradeMarkets {
                    indexed_through_block: 1234,
                    markets: ["BTC", "ETH"]
                        .into_iter()
                        .enumerate()
                        .map(|(id, symbol)| PapertradeMarket {
                            id: PapertradeMarketId(id as u32),
                            symbol: symbol.into(),
                            max_leverage: 1000,
                            status: "Perp",
                        })
                        .collect(),
                });
                terminal.market_browser.papertrade.loading = state == "loading";
                terminal.market_browser.papertrade.error =
                    (state == "error").then(|| "Papertrade markets unavailable.".into());
                if state == "loading" {
                    terminal.market_browser.papertrade.data = None;
                }
                terminal.symbol_search_query = query.into();
                terminal.refresh_symbol_search_results();
                let instance = terminal.charts.get_mut(&chart_id).expect("chart");
                instance.editor_search_query = query.into();
                instance.secondary_editor_search_query = query.into();

                let theme = terminal.theme();
                let size = Size::new(360.0, 320.0);
                let bounds = Rectangle::with_size(size);
                let mut view = match picker {
                    MarketPicker::Chart(_) => {
                        terminal.view_chart_editor(chart_id, &terminal.charts[&chart_id])
                    }
                    MarketPicker::ChartSecondary(_) => {
                        terminal.view_chart_secondary_editor(chart_id, &terminal.charts[&chart_id])
                    }
                    _ => terminal.view_watchlist(),
                };
                let mut tree = Tree::new(view.as_widget());
                let node = view.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(size, size),
                );
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
                let pixels =
                    renderer.screenshot(Size::new(360, 320), 1.0, theme.palette().background);
                assert_eq!(pixels.len(), 360 * 320 * 4);
                assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
                if let Some(directory) = std::env::var_os("KEROSENE_PAPERTRADE_PREVIEW_DIR") {
                    let directory = std::path::PathBuf::from(directory);
                    std::fs::create_dir_all(&directory).expect("preview directory");
                    image::save_buffer(
                        directory.join(format!("{picker:?}-{venue:?}-{state}.png")),
                        &pixels,
                        360,
                        320,
                        image::ColorType::Rgba8,
                    )
                    .expect("synthetic preview");
                }
                let mut saw_switch = false;
                let mut saw_selection = false;
                for y in (0..310).step_by(3) {
                    let mut messages = Vec::new();
                    for event in [
                        mouse::Event::ButtonPressed(mouse::Button::Left),
                        mouse::Event::ButtonReleased(mouse::Button::Left),
                    ] {
                        view.as_widget_mut().update(
                            &mut tree,
                            &Event::Mouse(event),
                            Layout::new(&node),
                            mouse::Cursor::Available(Point::new(140.0, y as f32)),
                            &renderer,
                            &mut clipboard::Null,
                            &mut Shell::new(&mut messages),
                            &bounds,
                        );
                    }
                    for message in messages {
                        saw_switch |= matches!(message, Message::MarketPickerVenueChanged(target, MarketVenue::Papertrade) if target == picker);
                        saw_selection |= matches!(
                            message,
                            Message::ChartSymbolSelected(_, _)
                                | Message::ChartSecondarySymbolSelected(_, _)
                                | Message::SymbolSelected(_)
                                | Message::ToggleFavourite(_)
                        );
                    }
                }
                assert!(
                    saw_switch,
                    "venue control unreachable for {picker:?}/{venue:?}/{state}"
                );
                if venue == MarketVenue::Papertrade {
                    assert!(!saw_selection, "preview emitted a Hyperliquid selection");
                } else if state != "empty" {
                    assert!(
                        saw_selection,
                        "Hyperliquid rows no longer selectable for {picker:?}/{state}"
                    );
                }
            }
        }
    }
}
