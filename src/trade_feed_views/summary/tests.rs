use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::config::{KeroseneConfig, OrderBookSymbolModeConfig};
use crate::feed_state::TrackedTradeIntent;
use crate::helpers::design::UiColors;
use crate::message::Message;
use crate::trade_feed_state::{TradeFeedEvent, TradeFeedInstance, TradePrint};
use crate::trade_feed_views::layout::TapeRowLayout;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Element, Event, Font, Pixels, Point, Rectangle, Size, Theme};
use std::collections::BTreeSet;

fn fixture() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.active_symbol = "BTC".into();
    terminal.exchange_symbols.push(ExchangeSymbol {
        key: "BTC".into(),
        ticker: "BTC".into(),
        category: "crypto".into(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 40,
        only_isolated: false,
        growth_mode: false,
        market_type: MarketType::Perp,
        outcome: None,
    });
    terminal.status_bar_now_ms = 120 * 60_000;
    let mut feed = TradeFeedInstance::new(Default::default());
    feed.symbol = Some("BTC".into());
    feed.apply(TradeFeedEvent::Trades(
        (0..100)
            .map(|tid| {
                let price = 60_000.0 + (tid / 2) as f64;
                let size = (tid % 4 + 1) as f64 / 100.0;
                TradePrint {
                    time_ms: terminal
                        .status_bar_now_ms
                        .saturating_sub(tid / 2 * 10_000 + tid % 2 * 300),
                    tid,
                    is_buy: tid / 2 % 3 != 0,
                    price: format!("{price:.2}"),
                    size: format!("{size:.3}"),
                    value: price * size,
                    trader: Some(format!("0x{:040x}", tid / 2 % 5 + 1).into()),
                    intent: if tid / 2 % 3 == 0 {
                        TrackedTradeIntent::Reducing
                    } else {
                        TrackedTradeIntent::Opening
                    },
                }
            })
            .collect(),
    ));
    terminal.trade_feeds.instances.insert(7, feed);
    terminal
}

#[tokio::test]
async fn trade_feed_summary_stays_at_bottom_when_resized_or_picker_opens() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = fixture();
    for theme_name in [
        "Custom: Kerosene",
        "Custom: Hyperliquid",
        "Custom: Papertrade",
        "Custom: Papertrade UwU",
        "Light",
    ] {
        terminal.active_theme = theme_name.into();
        for (width, height, picker, grouped, context) in [
            (280.0, 360.0, false, true, true),
            (320.0, 240.0, false, false, false),
            (400.0, 360.0, false, false, false),
            (640.0, 360.0, false, false, false),
            (640.0, 360.0, true, true, true),
            (1_000.0, 600.0, false, true, true),
        ] {
            let feed = terminal.trade_feeds.instances.get_mut(&7).expect("feed");
            feed.picker_open = picker;
            feed.group_trades = grouped;
            feed.show_context = context;
            feed.min_group_notional_input = if grouped { "1k".into() } else { String::new() };
            feed.min_group_notional = if grouped { 1_000.0 } else { 0.0 };
            let theme = terminal.theme();
            let mut view =
                terminal.view_trade_feed_sized(7, &terminal.trade_feeds.instances[&7], width);
            let name = format!(
                "{}-{width}-{height}-picker-{picker}-grouped-{grouped}-context-{context}",
                theme_name.replace([' ', ':'], "-")
            );
            let (_, node) = render(
                &mut view,
                &mut renderer,
                &theme,
                Size::new(width, height),
                &name,
                1.0,
            );
            let footer = node.children().last().expect("summary footer").bounds();
            assert!((footer.y + footer.height - height).abs() < 0.01);
            assert!(footer.height > 30.0 && footer.height < height / 2.0);
            let tape = node.children()[0].bounds();
            assert!(tape.y + tape.height <= footer.y);
        }
    }
    // UI scale changes physical output, not the logical column grid or controls.
    for scale in [1.25, 1.5] {
        let theme = terminal.theme();
        let mut view =
            terminal.view_trade_feed_sized(7, &terminal.trade_feeds.instances[&7], 400.0);
        render(
            &mut view,
            &mut renderer,
            &theme,
            Size::new(400.0, 400.0),
            &format!("scale-{scale}"),
            scale,
        );
    }
}

#[tokio::test]
async fn trade_feed_controls_keep_instance_messages_and_picker_selection() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = fixture();
    let feed = terminal.trade_feeds.instances.get_mut(&7).expect("feed");
    feed.picker_open = true;
    feed.group_trades = true;
    feed.min_group_notional_input = "1k".into();
    for width in [280.0, 640.0] {
        let theme = terminal.theme();
        let colors = UiColors::new(&theme);
        let size = Size::new(width, 300.0);
        let mut view = terminal.view_trade_feed_controls(
            7,
            &terminal.trade_feeds.instances[&7],
            "USD",
            width,
            colors,
        );
        let (mut tree, node) = render(
            &mut view,
            &mut renderer,
            &theme,
            size,
            &format!("controls-{width}"),
            1.0,
        );
        let mut actions = BTreeSet::new();
        for x in (4..width as usize).step_by(8) {
            for y in (4..300).step_by(8) {
                for message in click(
                    &mut view,
                    &mut tree,
                    &node,
                    &renderer,
                    size,
                    Point::new(x as f32, y as f32),
                ) {
                    let (id, action) = match message {
                        Message::TradeFeedTogglePicker(id) => (id, "picker"),
                        Message::TradeFeedSetMode(id, OrderBookSymbolModeConfig::Active) => {
                            (id, "follow")
                        }
                        Message::TradeFeedSetMode(id, OrderBookSymbolModeConfig::Fixed(key)) => {
                            assert_eq!(key, "BTC");
                            (id, "select")
                        }
                        Message::TradeFeedToggleGrouping(id) => (id, "group"),
                        Message::TradeFeedToggleContext(id) => (id, "context"),
                        Message::TradeFeedMinimumChanged(id, value) => {
                            assert!(value.is_empty());
                            (id, "clear")
                        }
                        _ => continue,
                    };
                    assert_eq!(id, 7);
                    actions.insert(action);
                }
            }
        }
        assert_eq!(
            actions,
            BTreeSet::from(["picker", "follow", "select", "group", "context", "clear"])
        );
    }
}

#[tokio::test]
async fn trade_feed_header_and_rows_share_columns_and_size_toggle() {
    let mut renderer = iced::Renderer::new(Font::MONOSPACE, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = fixture();
    for width in [280.0, 400.0, 640.0] {
        for context in [false, true] {
            for notional in [false, true] {
                let feed = terminal.trade_feeds.instances.get_mut(&7).expect("feed");
                feed.show_context = context;
                feed.group_trades = true;
                feed.size_in_notional = notional;
                let feed = &terminal.trade_feeds.instances[&7];
                let theme = terminal.theme();
                let layout = TapeRowLayout::new(width, context, notional);
                let size = Size::new(width, 24.0);
                let mut header = crate::trade_feed_views::rows::view_header(
                    7,
                    feed,
                    layout,
                    UiColors::new(&theme),
                    terminal.pane_corner_radius,
                );
                let (mut header_tree, header_node) =
                    render(&mut header, &mut renderer, &theme, size, "header", 1.0);
                let mut rows = terminal.view_trade_feed_rows(7, feed, layout, 24.0, "USD");
                let (mut row_tree, row_node) =
                    render(&mut rows, &mut renderer, &theme, size, "row", 1.0);
                let header_cells = header_node.children()[0].children();
                let row_cells = row_node.children()[0].children()[0].children()[0].children();
                assert_eq!(header_cells.len(), row_cells.len());
                for (header, row) in header_cells.iter().zip(row_cells) {
                    let (header, row) = (header.bounds(), row.bounds());
                    assert!((header.x - row.x).abs() < 0.01);
                    assert!((header.width - row.width).abs() < 0.01);
                    assert!(row.x + row.width <= width - 16.0 + 0.01);
                }
                for (view, tree, node) in [
                    (&mut header, &mut header_tree, &header_node),
                    (&mut rows, &mut row_tree, &row_node),
                ] {
                    let mut size_toggle = false;
                    for x in (4..width as usize).step_by(8) {
                        size_toggle |= click(
                            view,
                            tree,
                            node,
                            &renderer,
                            size,
                            Point::new(x as f32, 12.0),
                        )
                        .iter()
                        .any(|message| matches!(message, Message::TradeFeedToggleSizeUnit(7)));
                    }
                    assert!(size_toggle);
                }
            }
        }
    }
}

#[tokio::test]
async fn trade_feed_held_wallet_actions_remain_reachable_in_grouped_rows() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = fixture();
    let feed = terminal.trade_feeds.instances.get_mut(&7).expect("feed");
    feed.group_trades = true;
    feed.show_context = true;
    let key = feed.address_hover_key(7, feed.visible_rows()[0].latest);
    feed.sync_address_hover(7, Some(&key));
    terminal.hovered_wallet_address_actions = Some(key);
    let feed = &terminal.trade_feeds.instances[&7];
    let theme = terminal.theme();
    let size = Size::new(280.0, 24.0);
    let mut view = terminal.view_trade_feed_rows(
        7,
        feed,
        TapeRowLayout::new(size.width, true, false),
        size.height,
        "USD",
    );
    let (mut tree, node) = render(
        &mut view,
        &mut renderer,
        &theme,
        size,
        "wallet-actions",
        1.0,
    );
    let mut actions = BTreeSet::new();
    for x in (4..280).step_by(4) {
        for message in click(
            &mut view,
            &mut tree,
            &node,
            &renderer,
            size,
            Point::new(x as f32, 12.0),
        ) {
            match message {
                Message::CopyToClipboard(_) => {
                    actions.insert("copy");
                }
                Message::OpenWalletDetailsWindow(_) => {
                    actions.insert("open");
                }
                Message::GhostWallet(_) => {
                    actions.insert("ghost");
                }
                _ => {}
            }
        }
    }
    assert_eq!(actions, BTreeSet::from(["copy", "open", "ghost"]));
}

#[tokio::test]
async fn trade_feed_feedback_and_empty_states_keep_the_summary_visible() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for name in ["invalid", "held-gap", "filtered", "unavailable", "waiting"] {
        let mut terminal = fixture();
        terminal.active_theme = "Custom: Papertrade".into();
        terminal.pane_corner_radius = 0.0;
        let feed = terminal.trade_feeds.instances.get_mut(&7).expect("feed");
        feed.group_trades = true;
        feed.show_context = true;
        match name {
            "invalid" => feed.min_group_notional_input = "invalid".into(),
            "held-gap" => {
                feed.has_gap = true;
                let key = feed.address_hover_key(7, &feed.trades[0]);
                feed.sync_address_hover(7, Some(&key));
                terminal.hovered_wallet_address_actions = Some(key);
                feed.status = "Reconnecting";
            }
            "filtered" => {
                feed.min_group_notional_input = "1M".into();
                feed.min_group_notional = 1_000_000.0;
            }
            "unavailable" | "waiting" => {
                feed.reset(Some("BTC".into()), 2);
                if name == "unavailable" {
                    feed.status = "Feed unavailable";
                }
            }
            _ => unreachable!(),
        }
        let theme = terminal.theme();
        let mut view =
            terminal.view_trade_feed_sized(7, &terminal.trade_feeds.instances[&7], 280.0);
        let (_, node) = render(
            &mut view,
            &mut renderer,
            &theme,
            Size::new(280.0, 360.0),
            name,
            1.0,
        );
        let footer = node.children().last().expect("summary footer").bounds();
        assert!((footer.y + footer.height - 360.0).abs() < 0.01);
        assert!(node.children()[0].bounds().height <= footer.y);
    }
}

fn render(
    view: &mut Element<'_, Message>,
    renderer: &mut iced::Renderer,
    theme: &Theme,
    size: Size,
    name: &str,
    scale: f32,
) -> (Tree, layout::Node) {
    let bounds = Rectangle::with_size(size);
    let mut tree = Tree::new(view.as_widget());
    let node = view
        .as_widget_mut()
        .layout(&mut tree, renderer, &layout::Limits::new(size, size));
    let mut messages = Vec::new();
    // Iced updates button/input status during RedrawRequested. Drawing without
    // this event incorrectly captures every button in its initial disabled state.
    view.as_widget_mut().update(
        &mut tree,
        &Event::Window(iced::window::Event::RedrawRequested(
            iced::time::Instant::now(),
        )),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        renderer,
        &mut clipboard::Null,
        &mut Shell::new(&mut messages),
        &bounds,
    );
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
    let physical = Size::new((size.width * scale) as u32, (size.height * scale) as u32);
    let pixels = renderer.screenshot(
        physical,
        scale,
        theme.extended_palette().background.strong.color,
    );
    assert_eq!(
        pixels.len(),
        physical.width as usize * physical.height as usize * 4
    );
    if let Some(directory) = std::env::var_os("KEROSENE_TRADE_FEED_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("preview directory");
        image::save_buffer(
            directory.join(format!("{name}.png")),
            &pixels,
            physical.width,
            physical.height,
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
    renderer: &iced::Renderer,
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
