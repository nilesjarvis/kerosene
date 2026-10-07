use super::*;
use crate::api::{BookLevel, ExchangeSymbol, MarketType, OrderBook};
use crate::config::KeroseneConfig;
use crate::market_state::OrderBookDisplayMode;
use crate::message::Message;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Element, Event, Font, Pixels, Point, Rectangle, Size, Theme};
use std::collections::BTreeSet;

fn symbol(key: &str, name: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: Some(name.to_string()),
        keywords: Vec::new(),
        asset_index: 1,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 10,
        only_isolated: false,
        growth_mode: false,
        market_type,
        outcome: None,
    }
}

#[tokio::test]
async fn settings_preserve_symbol_order_limits_and_toggle_actions() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.exchange_symbols = [
        ("MUTED", "Muted Asset", MarketType::Perp),
        ("#1", "Unselectable Asset", MarketType::Outcome),
        ("FIRST", "First Asset", MarketType::Perp),
        ("SECOND", "Second Asset", MarketType::Perp),
        ("THIRD", "Third Asset", MarketType::Perp),
        ("FOURTH", "Fourth Asset", MarketType::Perp),
        ("FIFTH", "Fifth Asset", MarketType::Perp),
        ("SIXTH", "Sixth Asset", MarketType::Perp),
    ]
    .into_iter()
    .map(|(key, name, kind)| symbol(key, name, kind))
    .collect();
    terminal.muted_tickers.insert("MUTED".to_string());
    let theme = terminal.theme();
    let size = Size::new(440.0, 320.0);
    for (query, expected) in [
        ("", vec!["FIRST", "SECOND", "THIRD", "FOURTH", "FIFTH"]),
        ("ASSET", vec!["FIRST", "SECOND", "THIRD", "FOURTH", "FIFTH"]),
        ("second", vec!["SECOND"]),
        ("missing", vec![]),
    ] {
        for (name, mode) in [
            ("list", OrderBookDisplayMode::DepthList),
            ("dom", OrderBookDisplayMode::DomLadder),
            ("chart", OrderBookDisplayMode::DepthChart),
        ] {
            for active in [false, true] {
                let symbol_mode = if active {
                    OrderBookSymbolMode::Active
                } else {
                    OrderBookSymbolMode::Fixed("THIRD".to_string())
                };
                let mut inst = OrderBookInstance::new(91, symbol_mode, 1.0);
                inst.display_mode = mode;
                inst.search_query = query.to_string();
                inst.reverse_side = active;
                inst.show_spread_chart = !active;
                let mut view = terminal.view_order_book_settings(91, &inst);
                let (mut tree, node) = render(
                    &mut view,
                    &mut renderer,
                    &theme,
                    size,
                    &format!("settings-{query}-{name}-{active}"),
                );
                let mut keys = Vec::new();
                let mut track_active = false;
                let mut reverse = false;
                let mut spread = false;
                for y in (5..315).step_by(5) {
                    for message in click(
                        &mut view,
                        &mut tree,
                        &node,
                        &mut renderer,
                        size,
                        Point::new(200.0, y as f32),
                    ) {
                        match message {
                            Message::OrderBookSetMode(id, OrderBookSymbolMode::Active) => {
                                assert_eq!(id, 91);
                                track_active = true;
                            }
                            Message::OrderBookSetMode(id, OrderBookSymbolMode::Fixed(key)) => {
                                assert_eq!(id, 91);
                                if keys.last() != Some(&key) {
                                    keys.push(key);
                                }
                            }
                            Message::ToggleOrderBookReverseSide(id) => {
                                assert_eq!(id, 91);
                                reverse = true;
                            }
                            Message::ToggleOrderBookSpreadChart(id) => {
                                assert_eq!(id, 91);
                                spread = true;
                            }
                            _ => {}
                        }
                    }
                }
                assert_eq!(keys, expected);
                assert!(track_active && spread);
                assert_eq!(reverse, mode != OrderBookDisplayMode::DepthChart);
            }
        }
    }
}

fn book() -> OrderBook {
    OrderBook {
        bids: [99.0, 98.0, 96.0]
            .into_iter()
            .enumerate()
            .map(|(index, px)| BookLevel {
                px,
                sz: index as f64 + 1.5,
            })
            .collect(),
        asks: [101.0, 103.0, 104.0, 106.0]
            .into_iter()
            .enumerate()
            .map(|(index, px)| BookLevel {
                px,
                sz: index as f64 + 2.5,
            })
            .collect(),
    }
}

#[tokio::test]
async fn denomination_switches_keep_row_geometry_and_one_sided_depth_visible() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.active_symbol = "BTC".into();
    terminal.active_symbol_display = "BTC".into();
    let theme = terminal.theme();
    let size = Size::new(420.0, 1000.0);
    let mut inst = OrderBookInstance::new(91, OrderBookSymbolMode::Active, 1.0);
    for tick in [1.0, 2.0, 5.0, 10.0, 100.0, 1.0] {
        inst.set_tick_size(tick);
        inst.set_book_with_source(
            OrderBook {
                bids: (1..=20)
                    .map(|i| BookLevel {
                        px: 80_000.0 - f64::from(i) * tick,
                        sz: f64::from(i),
                    })
                    .collect(),
                asks: (1..=20)
                    .map(|i| BookLevel {
                        px: 80_000.0 + f64::from(i) * tick,
                        sz: f64::from(i),
                    })
                    .collect(),
            },
            Some(tick),
        );
        for pending in [false, true] {
            inst.book_loading = pending;
            inst.set_tick_size(if pending {
                if tick == 100.0 { 1.0 } else { 100.0 }
            } else {
                tick
            });
            terminal.order_books.insert(91, inst);
            let mut view = terminal.view_order_book(91);
            let (mut tree, node) = render(
                &mut view,
                &mut renderer,
                &theme,
                size,
                &format!("denomination-{tick}-pending-{pending}"),
            );
            let mut prices = BTreeSet::new();
            for y in (90..990).step_by(10) {
                for message in click(
                    &mut view,
                    &mut tree,
                    &node,
                    &mut renderer,
                    size,
                    Point::new(100.0, y as f32),
                ) {
                    if let Message::OrderBookPriceSelected { price, .. } = message {
                        prices.insert(price);
                    }
                }
            }
            assert_eq!(prices.len(), 40, "tick {tick}, pending {pending}");
            assert!(prices.contains(&format!("{:.0}", 80_000.0 - tick)));
            assert!(prices.contains(&format!("{:.0}", 80_000.0 + tick)));
            drop(view);
            inst = terminal.order_books.remove(&91).expect("book");
        }
    }

    inst.book.asks.clear();
    inst.set_book_with_source(inst.book.clone(), Some(1.0));
    terminal.order_books.insert(91, inst);
    let mut view = terminal.view_order_book(91);
    let (mut tree, node) = render(&mut view, &mut renderer, &theme, size, "one-sided-depth");
    let mut prices = BTreeSet::new();
    for y in (90..990).step_by(10) {
        for message in click(
            &mut view,
            &mut tree,
            &node,
            &mut renderer,
            size,
            Point::new(100.0, y as f32),
        ) {
            if let Message::OrderBookPriceSelected { price, .. } = message {
                prices.insert(price);
            }
        }
    }
    assert_eq!(prices.len(), 20);
}

#[tokio::test]
async fn pane_composition_preserves_empty_loading_stale_and_spread_states() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.spinner_phase = 0.25;
    let theme = terminal.theme();
    let size = Size::new(600.0, 440.0);
    for (mode_name, mode) in [
        ("list", OrderBookDisplayMode::DepthList),
        ("dom", OrderBookDisplayMode::DomLadder),
        ("chart", OrderBookDisplayMode::DepthChart),
    ] {
        for settings_open in [false, true] {
            for state in 0..8 {
                let mut inst = OrderBookInstance::new(91, OrderBookSymbolMode::Active, 0.5);
                inst.display_mode = mode;
                inst.settings_open = settings_open;
                inst.show_spread_chart = true;
                inst.book_loading = matches!(state, 1 | 4 | 5 | 6);
                inst.book_error = matches!(state, 2 | 4).then(|| "synthetic failure".to_string());
                let populated = matches!(state, 3 | 4 | 6 | 7);
                let mut data = if populated {
                    book()
                } else {
                    OrderBook::empty()
                };
                if state == 7 {
                    data.asks.clear();
                }
                inst.set_book_with_source(data, matches!(state, 5 | 6).then_some(2.0));
                terminal.order_books.insert(91, inst);
                let (_, node) = render(
                    &mut terminal.view_order_book(91),
                    &mut renderer,
                    &theme,
                    size,
                    &format!("pane-{mode_name}-{settings_open}-{state}"),
                );
                let expected_children = 4
                    + usize::from(mode != OrderBookDisplayMode::DepthChart)
                    + usize::from(settings_open)
                    + 2 * usize::from(populated);
                assert_eq!(node.children()[0].children().len(), expected_children);
            }
        }
    }
    terminal.muted_tickers.insert("BTC".to_string());
    let (_, hidden) = render(
        &mut terminal.view_order_book(91),
        &mut renderer,
        &theme,
        size,
        "pane-muted",
    );
    assert_eq!(hidden.children()[0].children().len(), 4);
    terminal.order_books.remove(&91);
    render(
        &mut terminal.view_order_book(91),
        &mut renderer,
        &theme,
        size,
        "pane-missing",
    );
}

#[tokio::test]
async fn book_rows_preserve_price_actions_through_centering_and_orientation() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let levels = UserOrderBookLevels::from_orders(
        &[
            open_order("BTC", "B", "99", 1),
            open_order("BTC", "A", "103", 2),
        ],
        "BTC",
        1.0,
    );
    for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
        for dom in [false, true] {
            for centered in [false, true] {
                for reversed in [false, true] {
                    let mut inst = OrderBookInstance::new(91, OrderBookSymbolMode::Active, 1.0);
                    inst.set_book(book());
                    inst.center_on_mid = centered;
                    inst.reverse_side = reversed;
                    let mut view = if dom {
                        TradingTerminal::view_order_book_dom_ladder(
                            91,
                            &inst,
                            1.0,
                            &theme,
                            levels.clone(),
                            false,
                        )
                    } else {
                        TradingTerminal::view_order_book_rows(
                            91,
                            &inst,
                            1.0,
                            &theme,
                            levels.clone(),
                            false,
                        )
                    };
                    let size = Size::new(600.0, 360.0);
                    let (mut tree, node) = render(
                        &mut view,
                        &mut renderer,
                        &theme,
                        size,
                        &format!("rows-{theme_name}-{dom}-{centered}-{reversed}"),
                    );
                    let mut prices = BTreeSet::new();
                    for y in (5..355).step_by(10) {
                        for message in click(
                            &mut view,
                            &mut tree,
                            &node,
                            &mut renderer,
                            size,
                            Point::new(200.0, y as f32),
                        ) {
                            if let Message::OrderBookPriceSelected { id, price } = message {
                                assert_eq!(id, 91);
                                prices.insert(price);
                            }
                        }
                    }
                    if centered {
                        assert!(prices.contains("99"));
                        assert!(prices.contains("101"));
                    } else if !dom {
                        // Twenty slots per side: padding stays inert and the
                        // first real ask enters at the bottom of this viewport.
                        assert_eq!(
                            prices,
                            BTreeSet::from(["104".to_string(), "106".to_string()])
                        );
                    } else {
                        assert!(prices.contains("180"));
                    }
                    // Re-layout the same owned closures at a different size.
                    let _ = view.as_widget_mut().layout(
                        &mut tree,
                        &renderer,
                        &layout::Limits::new(Size::new(320.0, 180.0), Size::new(320.0, 180.0)),
                    );
                }
            }
        }
    }
}

fn render(
    view: &mut Element<'_, Message>,
    renderer: &mut iced::Renderer,
    theme: &Theme,
    size: Size,
    name: &str,
) -> (Tree, layout::Node) {
    let bounds = Rectangle::with_size(size);
    let mut tree = Tree::new(view.as_widget());
    let node = view
        .as_widget_mut()
        .layout(&mut tree, renderer, &layout::Limits::new(size, size));
    iced::advanced::Renderer::reset(renderer, bounds);
    view.as_widget().draw(
        &tree,
        renderer,
        theme,
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
    assert_eq!(pixels.len(), size.width as usize * size.height as usize * 4);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
    if let Some(directory) = std::env::var_os("KEROSENE_BOOK_VIEW_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("preview directory");
        image::save_buffer(
            directory.join(format!("{name}.png")),
            &pixels,
            size.width as u32,
            size.height as u32,
            image::ColorType::Rgba8,
        )
        .expect("synthetic preview");
    }
    (tree, node)
}

fn click(
    view: &mut Element<'_, Message>,
    tree: &mut Tree,
    node: &layout::Node,
    renderer: &mut iced::Renderer,
    size: Size,
    point: Point,
) -> Vec<Message> {
    let mut messages = Vec::new();
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        view.as_widget_mut().update(
            tree,
            &Event::Mouse(event),
            Layout::new(node),
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &Rectangle::with_size(size),
        );
    }
    messages
}
