use super::*;
use crate::account::{AccountData, AssetPosition, UserFeeRates};
use crate::api::{BookLevel, ExchangeSymbol, MarketType, OrderBook};
use crate::builder_codes::BuilderCodeConfig;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size, Vector};

fn fixture() -> (TradingTerminal, AssetPosition) {
    let mut terminal = TradingTerminal::boot_from_config(Default::default()).0;
    let address = "0x1111111111111111111111111111111111111111";
    terminal.connected_address = Some(address.into());
    terminal.account_data_address = Some(address.into());
    terminal.account_data = Some(AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: serde_json::from_value(serde_json::json!({
            "marginSummary": {"accountValue": "0", "totalNtlPos": "0", "totalMarginUsed": "0"},
            "withdrawable": "0", "assetPositions": []
        }))
        .expect("synthetic account"),
        clearinghouses_by_dex: Default::default(),
        spot: serde_json::from_value(serde_json::json!({"balances": []})).expect("empty spot"),
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates {
            user_cross_rate: "0.001".into(),
            active_referral_discount: Some("0.04".into()),
            ..Default::default()
        },
        completeness: Default::default(),
        fetched_at_ms: terminal.status_bar_now_ms,
    });
    terminal.builder_codes.config = BuilderCodeConfig {
        enabled: true,
        address: "0x2222222222222222222222222222222222222222".into(),
        fee_tenths_bps: 10,
    };
    terminal.exchange_symbols.push(ExchangeSymbol {
        key: "BTC".into(),
        ticker: "BTC".into(),
        category: "crypto".into(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: Some(0),
        sz_decimals: 2,
        max_leverage: 10,
        only_isolated: false,
        growth_mode: false,
        market_type: MarketType::Perp,
        outcome: None,
    });
    terminal.all_mids.insert("BTC".into(), 111.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".into(), terminal.status_bar_now_ms);
    terminal.market_slippage_pct = 5.0;
    terminal.position_close_preview.coin = Some("BTC".into());
    terminal.position_close_preview.fee_multiplier = Some(Ok(1.0));
    terminal.position_close_preview.book = Some((
        OrderBook {
            bids: vec![
                BookLevel { px: 110.0, sz: 1.0 },
                BookLevel { px: 108.0, sz: 4.0 },
            ],
            asks: vec![BookLevel { px: 112.0, sz: 5.0 }],
        },
        terminal.market_data_source_context(),
        terminal.status_bar_now,
    ));
    let position = serde_json::from_value(serde_json::json!({"position": {
        "coin": "BTC", "szi": "3", "entryPx": "100", "positionValue": "333",
        "unrealizedPnl": "33", "leverage": {"type": "cross", "value": 1}
    }}))
    .expect("synthetic position");
    (terminal, position)
}

#[test]
fn close_preview_includes_referral_and_builder_fees_and_rejects_stale_inputs() {
    let (mut terminal, position) = fixture();
    let estimate = terminal
        .position_close_estimate(&terminal.position_row_data(&position))
        .expect("fresh estimate");
    assert!((estimate.exit_fee - 326.0 * 0.00106).abs() < 1e-9);
    assert!((estimate.net_pnl - (26.0 - 326.0 * 0.00106)).abs() < 1e-9);
    terminal.read_data_provider_generation += 1;
    assert_eq!(
        terminal
            .position_close_estimate(&terminal.position_row_data(&position))
            .err(),
        Some("Waiting for current market data")
    );
    terminal.read_data_provider_generation -= 1;
    terminal.status_bar_now += std::time::Duration::from_secs(11);
    assert_eq!(
        terminal
            .position_close_estimate(&terminal.position_row_data(&position))
            .err(),
        Some("Order book is stale · waiting for live depth")
    );
    terminal.status_bar_now_ms += 20_000;
    assert_eq!(
        terminal
            .position_close_estimate(&terminal.position_row_data(&position))
            .err(),
        Some("Waiting for current position data")
    );
}

#[test]
fn close_preview_rejects_missing_fees_cost_basis_and_slippage_overrun() {
    let (mut terminal, mut position) = fixture();
    terminal
        .account_data
        .as_mut()
        .expect("account")
        .completeness
        .fees_complete = false;
    assert_eq!(
        terminal
            .position_close_estimate(&terminal.position_row_data(&position))
            .err(),
        Some("Account fee rate unavailable")
    );
    terminal
        .account_data
        .as_mut()
        .expect("account")
        .completeness
        .fees_complete = true;
    terminal.market_slippage_pct = 1.0;
    assert_eq!(
        terminal
            .position_close_estimate(&terminal.position_row_data(&position))
            .err(),
        Some("Full close exceeds the market order slippage limit")
    );
    position.position.entry_px.clear();
    assert_eq!(
        terminal
            .position_close_estimate(&terminal.position_row_data(&position))
            .err(),
        Some("Position cost basis unavailable")
    );
}

#[tokio::test]
async fn close_preview_hover_renders_popup_preserves_click_and_respects_privacy() {
    let (mut terminal, position) = fixture();
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let theme = terminal.theme();
    let size = Size::new(560.0, 320.0);
    let bounds = Rectangle::with_size(size);
    let cursor_point = Point::new(30.0, 20.0);
    for hidden in [false, true] {
        terminal.hide_pnl = hidden;
        let cell = super::super::cells::position_upnl_cell(
            "BTC",
            if hidden { "$***" } else { "$33.00" }.into(),
            theme.palette().success,
        );
        let mut view: Element<'_, Message> = container(terminal.view_position_close_preview(
            cell,
            &terminal.position_row_data(&position),
            &theme,
        ))
        .padding(10)
        .into();
        let mut tree = Tree::new(view.as_widget());
        let node = view.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let cursor = mouse::Cursor::Available(cursor_point);
        let mut messages = Vec::new();
        view.as_widget_mut().update(
            &mut tree,
            &Event::Mouse(mouse::Event::CursorMoved {
                position: cursor_point,
            }),
            Layout::new(&node),
            cursor,
            &renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &bounds,
        );
        assert_eq!(
            messages.iter().any(|message| matches!(message,
            Message::PositionClosePreviewHovered(coin, true) if coin == "BTC")),
            !hidden
        );
        iced::advanced::Renderer::reset(&mut renderer, bounds);
        let style = renderer::Style {
            text_color: theme.palette().text,
        };
        view.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &style,
            Layout::new(&node),
            cursor,
            &bounds,
        );
        {
            let mut overlay = view.as_widget_mut().overlay(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &bounds,
                Vector::ZERO,
            );
            assert_eq!(overlay.is_some(), !hidden);
            if let Some(overlay) = &mut overlay {
                let overlay_node = overlay.as_overlay_mut().layout(&renderer, size);
                overlay.as_overlay().draw(
                    &mut renderer,
                    &theme,
                    &style,
                    Layout::new(&overlay_node),
                    cursor,
                );
                if let Some(path) = std::env::var_os("KEROSENE_CLOSE_PREVIEW_PATH") {
                    let pixels =
                        renderer.screenshot(Size::new(560, 320), 1.0, theme.palette().background);
                    image::save_buffer(path, &pixels, 560, 320, image::ColorType::Rgba8)
                        .expect("synthetic popup screenshot");
                }
            }
        }
        messages.clear();
        for event in [
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
        assert!(
            messages.iter().any(|message| matches!(message,
            Message::OpenPnlCard(crate::pnl_card::PnlCardTarget::Position(coin)) if coin == "BTC"))
        );
        messages.clear();
        view.as_widget_mut().update(
            &mut tree,
            &Event::Mouse(mouse::Event::CursorLeft),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &bounds,
        );
        assert_eq!(
            messages.iter().any(|message| matches!(message,
            Message::PositionClosePreviewHovered(coin, false) if coin == "BTC")),
            !hidden
        );
    }
}
