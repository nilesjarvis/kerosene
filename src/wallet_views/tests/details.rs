use super::*;
use crate::account::{WalletDetailsData, WalletOpenOrderDetail, WalletPositionDetail};
use crate::api::{ExchangeSymbol, MarketType};
use crate::config::KeroseneConfig;
use crate::wallet_state::WalletDetailsWindowState;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};
use serde_json::json;

fn fixture() -> (TradingTerminal, WalletDetailsData) {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.status_bar_now_ms = 10_000;
    terminal.muted_tickers.insert("HIDDEN".into());
    terminal.exchange_symbols = [("@107", "HYPE"), ("@200", "UNPRICED")]
        .into_iter()
        .map(|(key, ticker)| ExchangeSymbol {
            key: key.into(),
            ticker: ticker.into(),
            category: "spot".into(),
            display_name: Some(format!("{ticker}/USDC")),
            keywords: vec![],
            asset_index: 10_000,
            collateral_token: None,
            sz_decimals: 2,
            max_leverage: 1,
            only_isolated: false,
            growth_mode: false,
            market_type: MarketType::Spot,
            outcome: None,
        })
        .collect();
    terminal.all_mids.insert("@107".into(), 3.0);
    terminal
        .all_mids_updated_at_ms
        .insert("@107".into(), TradingTerminal::now_ms());
    let positions = [
        ("xyz", "GOLD", "2"),
        ("", "ETH", "-3"),
        ("", "BTC", "1"),
        ("", "BTC", "-4"),
        ("", "INVALID", "NaN"),
        ("", "ZERO", "0"),
        ("", "TINY", "1e-20"),
        ("xyz", "HIDDEN", "5"),
    ]
    .into_iter()
    .map(|(dex, coin, size)| WalletPositionDetail {
        dex: dex.into(),
        asset_position: serde_json::from_value(json!({"position": {
            "coin": coin, "szi": size, "entryPx": "10", "positionValue": "20",
            "unrealizedPnl": "-2", "leverage": {"type": "cross", "value": 5},
            "liquidationPx": "2"
        }}))
        .expect("position fixture"),
    })
    .collect();
    let data = WalletDetailsData {
        clearinghouse: serde_json::from_value(json!({
            "marginSummary": {"accountValue": "100", "totalNtlPos": "40", "totalMarginUsed": "5"},
            "withdrawable": "95", "assetPositions": []
        }))
        .expect("clearinghouse fixture"),
        spot: serde_json::from_value(json!({"balances": [
            {"coin": "HYPE", "total": "2", "hold": "0", "entryNtl": "4"},
            {"coin": "UNPRICED", "total": "3", "hold": "0", "entryNtl": "0"},
            {"coin": "USDC", "total": "10", "hold": "0", "entryNtl": "0"},
            {"coin": "HYPE", "total": "0", "hold": "0", "entryNtl": "0"},
            {"coin": "UNKNOWN", "total": "1", "hold": "0", "entryNtl": "0"}
        ]}))
        .expect("spot fixture"),
        positions,
        open_orders: vec![WalletOpenOrderDetail {
            dex: "xyz".into(),
            order: serde_json::from_value(json!({
                "coin": "GOLD", "side": "B", "limitPx": "12", "sz": "3",
                "oid": 42, "timestamp": 1_000
            }))
            .expect("order fixture"),
        }],
        fills: vec![],
        warnings: vec!["Synthetic snapshot warning".into()],
        fetched_at_ms: 1_000,
    };
    (terminal, data)
}

#[test]
fn wallet_detail_positions_preserve_source_order_and_append_synthesized_spot() {
    let (terminal, data) = fixture();
    let rows = terminal.wallet_position_details_with_spot(&data);
    assert_eq!(rows.len(), data.positions.len() + 2);
    for (row, original) in rows.iter().zip(&data.positions) {
        assert_eq!(row.dex, original.dex);
        let actual = &row.asset_position.position;
        let expected = &original.asset_position.position;
        assert_eq!(actual.coin, expected.coin);
        assert_eq!(actual.szi, expected.szi);
        assert_eq!(actual.entry_px, expected.entry_px);
        assert_eq!(actual.position_value, expected.position_value);
        assert_eq!(actual.unrealized_pnl, expected.unrealized_pnl);
        assert_eq!(actual.liquidation_px, expected.liquidation_px);
        assert_eq!(
            actual.leverage.leverage_type,
            expected.leverage.leverage_type
        );
        assert_eq!(actual.leverage.value, expected.leverage.value);
    }
    let spot = &rows[data.positions.len()];
    assert!(spot.dex.is_empty());
    let position = &spot.asset_position.position;
    assert_eq!(position.coin, "@107");
    assert_eq!(position.szi, "2");
    assert_eq!(position.entry_px, "2");
    assert_eq!(position.position_value, "6");
    assert_eq!(position.unrealized_pnl, "2");
    assert_eq!(position.leverage.leverage_type, "spot");
    let unpriced = &rows[data.positions.len() + 1].asset_position.position;
    assert_eq!(unpriced.coin, "@200");
    assert!(unpriced.entry_px.is_empty());
    assert!(unpriced.position_value.is_empty());
    assert!(unpriced.unrealized_pnl.is_empty());
}

#[tokio::test]
async fn wallet_details_render_states_and_preserve_position_selection_order() {
    let (mut terminal, data) = fixture();
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let theme = terminal.theme();
    let size = Size::new(1200.0, 1100.0);
    {
        let mut view = terminal.view_wallet_positions_table(&data);
        let (mut tree, node) = render(&mut view, &mut renderer, &theme, size, "positions");
        let table = Layout::new(&node).children().next().expect("table column");
        let points: Vec<_> = table
            .children()
            .skip(3)
            .map(|row| {
                row.children()
                    .next()
                    .expect("symbol button")
                    .bounds()
                    .center()
            })
            .collect();
        let expected = ["@107", "@200", "BTC", "BTC", "ETH", "INVALID", "xyz:GOLD"];
        assert_eq!(points.len(), expected.len());
        for (point, symbol) in points.into_iter().zip(expected) {
            let messages = click(&mut view, &mut tree, &node, &mut renderer, size, point);
            assert!(
                matches!(messages.as_slice(), [Message::SymbolSelected(actual)] if actual == symbol)
            );
        }
    }
    let id = window::Id::unique();
    let mut state =
        WalletDetailsWindowState::new("0x1111111111111111111111111111111111111111".into());
    for (name, loading, error, snapshot) in [
        ("loading", true, None, None),
        ("waiting", false, None, None),
        ("error", false, Some("Synthetic refresh error"), None),
        (
            "populated",
            false,
            Some("Synthetic refresh error"),
            Some(data),
        ),
    ] {
        state.loading = loading;
        state.error = error.map(str::to_string);
        state.data = snapshot;
        terminal.wallet_detail_windows.insert(id, state.clone());
        render(
            &mut terminal.view_wallet_details(id),
            &mut renderer,
            &theme,
            size,
            name,
        );
    }
    let snapshot = state.data.as_mut().expect("populated snapshot");
    snapshot.positions.clear();
    snapshot.spot.balances.clear();
    snapshot.open_orders.clear();
    snapshot.warnings.clear();
    state.error = None;
    terminal.wallet_detail_windows.insert(id, state);
    render(
        &mut terminal.view_wallet_details(id),
        &mut renderer,
        &theme,
        size,
        "empty",
    );
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
    if let Some(directory) = std::env::var_os("KEROSENE_WALLET_DETAIL_PREVIEW_DIR") {
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
