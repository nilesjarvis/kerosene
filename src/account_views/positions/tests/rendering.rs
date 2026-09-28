use super::*;
use crate::account_state::PositionsSortColumn;
use crate::api::{ExchangeSymbol, MarketType};
use crate::config::{KeroseneConfig, SortDirection};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};

#[tokio::test]
async fn position_sections_preserve_grouping_and_own_action_messages() {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.positions_sort_column = PositionsSortColumn::Symbol;
    terminal.positions_sort_direction = SortDirection::Ascending;
    terminal.exchange_symbols.push(ExchangeSymbol {
        key: "@107".into(),
        ticker: "HYPE".into(),
        category: "spot".into(),
        display_name: Some("HYPE/USDC".into()),
        keywords: vec![],
        asset_index: 10_107,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        growth_mode: false,
        market_type: MarketType::Spot,
        outcome: None,
    });
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let theme = terminal.theme();
    for (width, can_close, hide_pnl) in [
        (500.0, false, false),
        (500.0, true, true),
        (1800.0, false, true),
        (1800.0, true, false),
    ] {
        terminal.hide_pnl = hide_pnl;
        // Position input may be dropped once the widgets and messages are built.
        let mut view: Element<'_, Message> = {
            let positions: Vec<account::AssetPosition> = [
                ("#950", "3"),
                ("BTC", "1"),
                ("@107", "2"),
                ("INVALID", "NaN"),
                ("BTC", "-4"),
            ]
            .into_iter()
            .map(|(coin, size)| {
                serde_json::from_value(serde_json::json!({"position": {
                    "coin": coin, "szi": size, "entryPx": "10", "positionValue": "20",
                    "unrealizedPnl": "-2", "leverage": {"type": "cross", "value": 5}
                }}))
                .expect("position fixture")
            })
            .collect();
            terminal
                .view_position_sections(
                    &positions,
                    can_close,
                    &theme,
                    PositionColumnVisibility::for_width(width),
                    PositionNumberMode::for_width(width),
                )
                .into()
        };
        let size = Size::new(width, 500.0);
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
        let pixels = renderer.screenshot(
            Size::new(width as u32, 500),
            1.0,
            theme.palette().background,
        );
        assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
        if let Some(directory) = std::env::var_os("KEROSENE_POSITION_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).expect("preview directory");
            image::save_buffer(
                directory.join(format!("{width}-{can_close}-{hide_pnl}.png")),
                &pixels,
                width as u32,
                500,
                image::ColorType::Rgba8,
            )
            .expect("synthetic preview");
        }
        let groups: Vec<_> = Layout::new(&node).children().collect();
        assert_eq!(groups.len(), 7);
        for (group_index, symbols) in [
            (0, &["BTC", "BTC", "INVALID"][..]),
            (3, &["@107"][..]),
            (6, &["#950"][..]),
        ] {
            let rows: Vec<_> = groups[group_index].children().collect();
            assert_eq!(rows.len(), symbols.len());
            for (row, symbol) in rows.into_iter().zip(symbols) {
                let cells: Vec<_> = row
                    .children()
                    .next()
                    .expect("position row")
                    .children()
                    .collect();
                let symbol_point = cells[0]
                    .children()
                    .next()
                    .expect("symbol control")
                    .bounds()
                    .center();
                let messages = click(
                    &mut view,
                    &mut tree,
                    &node,
                    &mut renderer,
                    size,
                    symbol_point,
                );
                assert!(
                    matches!(messages.as_slice(), [Message::SymbolSelected(actual)] if actual == symbol)
                );
                let controls = cells
                    .last()
                    .expect("action cell")
                    .children()
                    .next()
                    .expect("actions");
                let closable = can_close && *symbol == "BTC";
                let hide_point = if closable {
                    let controls: Vec<_> = controls.children().collect();
                    assert_eq!(controls.len(), 2);
                    let messages = click(
                        &mut view,
                        &mut tree,
                        &node,
                        &mut renderer,
                        size,
                        controls[1].bounds().center(),
                    );
                    assert!(
                        matches!(messages.as_slice(), [Message::ToggleCloseMenu(actual)] if actual == symbol)
                    );
                    controls[0].bounds().center()
                } else {
                    controls.bounds().center()
                };
                let messages = click(&mut view, &mut tree, &node, &mut renderer, size, hide_point);
                assert!(
                    matches!(messages.as_slice(), [Message::ToggleHiddenPosition(actual)] if actual == symbol)
                );
            }
        }
    }
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
