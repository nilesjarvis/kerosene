use super::*;
use crate::account::AccountData;
use crate::api::Candle;
use crate::chart_state::ChartSurfaceId;
use crate::config::DisplayDenominationConfig;
use crate::timeframe::Timeframe;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{Layout, Shell, clipboard, layout, renderer};
use iced::{Event, Font, Pixels, Rectangle, Renderer, Size, Vector, mouse};
use serde_json::json;
use std::collections::HashMap;

fn context(rate: &str) -> AssetContext {
    serde_json::from_value(json!({
        "funding": rate,
        "oraclePx": "100",
        "markPx": "120"
    }))
    .expect("public context fixture")
}

#[test]
fn funding_projection_uses_oracle_price_hourly_rate_and_position_direction() {
    for (size, rate, sign) in [
        ("2", "0.0001", -1.0),
        ("-2", "0.0001", 1.0),
        ("2", "-0.0001", 1.0),
        ("-2", "-0.0001", -1.0),
    ] {
        let amounts = funding_projection(size, &context(rate)).expect("valid funding");
        for (actual, expected) in amounts.into_iter().zip([0.48, 3.36, 14.4, 175.2]) {
            assert!((actual - sign * expected).abs() < 1e-10);
        }
    }
    assert_eq!(funding_projection("2", &context("0")), Some([0.0; 4]));
}

#[test]
fn funding_projection_rejects_closed_positions_invalid_data_and_overflow() {
    for size in ["0", "-0", "", "bad", "NaN", "inf", "-inf", "1e308"] {
        assert!(funding_projection(size, &context("0.0001")).is_none());
    }
    for rate in ["", "bad", "NaN", "inf", "-inf", "1e308"] {
        assert!(funding_projection("2", &context(rate)).is_none());
    }
    for oracle in [
        None,
        Some("0"),
        Some("-1"),
        Some("bad"),
        Some("NaN"),
        Some("inf"),
    ] {
        let mut ctx = context("0.0001");
        ctx.oracle_px = oracle.map(str::to_string);
        assert!(funding_projection("2", &ctx).is_none());
    }
    let mut ctx = context("0.0001");
    ctx.funding = None;
    assert!(funding_projection("2", &ctx).is_none());
    // The hourly amount is finite, but the yearly projection would overflow.
    assert!(funding_projection("1e304", &context("1")).is_none());
}

fn terminal_with_position(coin: &str, size: &str) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    let data = AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: serde_json::from_value(json!({
            "marginSummary": {
                "accountValue": "1000", "totalNtlPos": "200", "totalMarginUsed": "20"
            },
            "withdrawable": "980",
            "assetPositions": [{
                "position": {
                    "coin": coin, "szi": size, "entryPx": "90",
                    "positionValue": "240", "unrealizedPnl": "60",
                    "leverage": {"type": "cross", "value": 10}
                }
            }]
        }))
        .expect("synthetic position"),
        clearinghouses_by_dex: HashMap::new(),
        spot: serde_json::from_value(json!({"balances": []})).expect("empty spot fixture"),
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: Default::default(),
        fetched_at_ms: 0,
    };
    let account = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(account.to_string());
    terminal.set_account_data_for_address_for_test(account, data);
    terminal
}

fn chart(symbol: &str) -> ChartInstance {
    let mut chart = ChartInstance::new(7, symbol.to_string(), Timeframe::H1);
    chart.chart.candles.push(Candle::test_price(0, 100.0));
    chart.set_asset_context_at(Some(context("0.0001")), 0);
    chart
}

#[test]
fn funding_projection_requires_the_connected_accounts_exact_perpetual_position() {
    let mut terminal = terminal_with_position("xyz:BTC", "2");
    assert!(
        terminal
            .chart_funding_projection(&chart("xyz:BTC"))
            .is_some()
    );
    for symbol in ["BTC", "abc:BTC", "ETH", "@107", "BTC/USDC", "#1"] {
        assert!(terminal.chart_funding_projection(&chart(symbol)).is_none());
    }
    for symbol in ["@107", "BTC/USDC", "#1"] {
        let terminal = terminal_with_position(symbol, "2");
        assert!(terminal.chart_funding_projection(&chart(symbol)).is_none());
    }
    let mut missing_context = chart("xyz:BTC");
    missing_context.asset_ctx = None;
    assert!(
        terminal
            .chart_funding_projection(&missing_context)
            .is_none()
    );
    terminal.connected_address = Some("0xdef0000000000000000000000000000000000000".into());
    assert!(
        terminal
            .chart_funding_projection(&chart("xyz:BTC"))
            .is_none()
    );
    terminal.connected_address = None;
    assert!(
        terminal
            .chart_funding_projection(&chart("xyz:BTC"))
            .is_none()
    );

    let mut terminal = terminal_with_position("BTC", "2");
    terminal.account_data = None;
    assert!(terminal.chart_funding_projection(&chart("BTC")).is_none());
}

#[derive(Default)]
struct TextBounds(Vec<(String, Rectangle)>);

impl Operation for TextBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, value: &str) {
        self.0.push((value.to_string(), bounds));
    }
}

#[tokio::test]
async fn funding_tooltip_formats_all_horizons_and_respects_pnl_privacy_and_denomination() {
    let renderer = Renderer::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let eur = DisplayDenominationContext::from_mids(
        DisplayDenominationConfig::eur(),
        &HashMap::from([("xyz:EUR".to_string(), 2.0)]),
        &HashMap::from([("xyz:EUR".to_string(), 0)]),
        0,
    );
    assert_eq!(eur.active_code(), "EUR");
    for (denomination, hide, expected) in [
        (DisplayDenominationContext::usd(), false, "-$0.4800"),
        (DisplayDenominationContext::usd(), true, "$***"),
        (eur, false, "-€0.2400"),
    ] {
        let mut content: Element<'_, Message> = view_funding_projection(
            [-0.48, -3.36, -14.4, -175.2],
            &denomination,
            hide,
            &Theme::Dark,
        )
        .into();
        let mut tree = Tree::new(content.as_widget());
        let node = content.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(320.0, 240.0)),
        );
        let mut text = TextBounds::default();
        content
            .as_widget_mut()
            .operate(&mut tree, Layout::new(&node), &renderer, &mut text);
        for expected in ["Estimated funding · Pay", "1d", "7d", "30d", "1y", expected] {
            assert!(text.0.iter().any(|(value, _)| value == expected));
        }
        assert!(node.size().width <= 220.0 && node.size().height < 140.0);
        if hide {
            assert_eq!(
                text.0.iter().filter(|(value, _)| value == "$***").count(),
                4
            );
            assert!(!text.0.iter().any(|(value, _)| value.contains("-$")));
        }
    }
}

#[tokio::test]
async fn funding_hover_opens_only_for_open_positions_and_dismisses_on_exit() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for (coin, position_size, symbol, expected) in [
        ("BTC", "2", "BTC", true),
        ("BTC", "-2", "BTC", true),
        ("xyz:BTC", "2", "xyz:BTC", true),
        ("BTC", "0", "BTC", false),
        ("BTC", "2", "ETH", false),
    ] {
        let terminal = terminal_with_position(coin, position_size);
        let chart = chart(symbol);
        let mut header =
            terminal.view_chart_header_sized(7, &chart, ChartSurfaceId::Docked(7), 320.0);
        let size = Size::new(320.0, 240.0);
        let viewport = Rectangle::with_size(size);
        let mut tree = Tree::new(header.as_widget());
        let node = header.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let mut text = TextBounds::default();
        header
            .as_widget_mut()
            .operate(&mut tree, Layout::new(&node), &renderer, &mut text);
        let funding_bounds = text
            .0
            .iter()
            .find(|(value, _)| value.starts_with("Funding ("))
            .expect("funding metric")
            .1;
        for (cursor, expected) in [
            (mouse::Cursor::Available(funding_bounds.center()), expected),
            (mouse::Cursor::Unavailable, false),
        ] {
            let mut messages = Vec::new();
            header.as_widget_mut().update(
                &mut tree,
                &Event::Mouse(mouse::Event::CursorMoved {
                    position: funding_bounds.center(),
                }),
                Layout::new(&node),
                cursor,
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &viewport,
            );
            assert!(messages.is_empty());
            let mut overlay = header.as_widget_mut().overlay(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &viewport,
                Vector::ZERO,
            );
            assert_eq!(overlay.is_some(), expected);
            if let Some(overlay) = &mut overlay {
                let overlay_node = overlay.as_overlay_mut().layout(&renderer, size);
                overlay.as_overlay().draw(
                    &mut renderer,
                    &terminal.theme(),
                    &renderer::Style {
                        text_color: terminal.theme().palette().text,
                    },
                    Layout::new(&overlay_node),
                    cursor,
                );
            }
        }
    }
}
