use super::*;
use crate::api::{BookLevel, OrderBook};
use crate::config::KeroseneConfig;
use crate::market_state::OrderBookDisplayMode;
use crate::message::Message;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Element, Event, Font, Pixels, Point, Rectangle, Size, Theme};
use std::collections::BTreeSet;

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
                        // The top of the scrollable depth list contains inert padding.
                        assert!(prices.is_empty());
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
