use crate::account::AssetContext;
use crate::api::Candle;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartInstance, ChartSurfaceId};
use crate::message::Message;
use crate::timeframe::Timeframe;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{Layout, Shell, clipboard, layout, renderer};
use iced::{Event, Font, Pixels, Rectangle, Renderer, Size, Vector, mouse};

#[derive(Default)]
struct OpenInterestBounds(Option<Rectangle>);

impl Operation for OpenInterestBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, value: &str) {
        if value.starts_with("Open Interest") {
            self.0 = Some(bounds);
        }
    }
}

#[tokio::test]
async fn open_interest_cap_tooltip_tracks_market_freshness_and_preserves_click_toggle() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for (symbol, cap, as_notional, now_ms, expect_tooltip) in [
        ("xyz:AAPL", Some(25_000_000.0), false, 0, true),
        ("xyz:AAPL", Some(25_000_000.0), true, 0, true),
        ("xyz:AAPL", Some(0.0), false, 0, true),
        ("xyz:AAPL", None, false, 0, false),
        ("xyz:AAPL", Some(25_000_000.0), false, 120_000, false),
        ("abc:AAPL", Some(25_000_000.0), false, 0, false),
        ("BTC", Some(25_000_000.0), false, 0, false),
    ] {
        let mut terminal = TradingTerminal::boot().0;
        if let Some(cap) = cap {
            let entry = terminal
                .chart_open_interest_caps
                .dexes
                .entry("xyz".to_string())
                .or_default();
            entry.caps.insert("xyz:AAPL".to_string(), cap);
            entry.updated_at_ms = Some(0);
        }
        let mut chart = ChartInstance::new(7, symbol.to_string(), Timeframe::H1);
        chart.chart.candles.push(Candle::test_price(0, 100.0));
        chart.chart.set_clock_now_ms(now_ms);
        chart.open_interest_as_notional = as_notional;
        chart.set_asset_context_at(
            Some(
                serde_json::from_value::<AssetContext>(serde_json::json!({
                    "openInterest": "1234", "markPx": "100", "oraclePx": "100",
                    "funding": "0.0000125", "dayBaseVlm": "10000", "dayNtlVlm": "1000000"
                }))
                .expect("public market fixture"),
            ),
            now_ms,
        );
        let mut header =
            terminal.view_chart_header_sized(7, &chart, ChartSurfaceId::Docked(7), 1200.0);
        let size = Size::new(1200.0, 120.0);
        let viewport = Rectangle::with_size(size);
        let mut tree = Tree::new(header.as_widget());
        let node = header.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let mut bounds = OpenInterestBounds::default();
        header
            .as_widget_mut()
            .operate(&mut tree, Layout::new(&node), &renderer, &mut bounds);
        let position = bounds.0.expect("open interest label").center();
        let cursor = mouse::Cursor::Available(position);
        let mut messages = Vec::new();
        for event in [
            mouse::Event::CursorMoved { position },
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            header.as_widget_mut().update(
                &mut tree,
                &Event::Mouse(event),
                Layout::new(&node),
                cursor,
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &viewport,
            );
        }
        assert!(matches!(
            messages.as_slice(),
            [Message::ToggleOpenInterestNotional(7)]
        ));
        let theme = terminal.theme();
        let style = renderer::Style {
            text_color: theme.palette().text,
        };
        iced::advanced::Renderer::reset(&mut renderer, viewport);
        header.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &style,
            Layout::new(&node),
            cursor,
            &viewport,
        );
        let mut overlay = header.as_widget_mut().overlay(
            &mut tree,
            Layout::new(&node),
            &renderer,
            &viewport,
            Vector::ZERO,
        );
        assert_eq!(
            overlay.is_some(),
            expect_tooltip,
            "{symbol}, cap {cap:?}, now {now_ms}"
        );
        if let Some(overlay) = &mut overlay {
            let overlay_node = overlay.as_overlay_mut().layout(&renderer, size);
            overlay.as_overlay().draw(
                &mut renderer,
                &theme,
                &style,
                Layout::new(&overlay_node),
                cursor,
            );
        }
        if symbol == "xyz:AAPL"
            && cap == Some(25_000_000.0)
            && !as_notional
            && now_ms == 0
            && let Some(path) = std::env::var_os("KEROSENE_CHART_HEADER_PREVIEW_PATH")
        {
            let pixels = renderer.screenshot(Size::new(1200, 120), 1.0, theme.palette().background);
            image::save_buffer(path, &pixels, 1200, 120, image::ColorType::Rgba8)
                .expect("header preview");
        }
    }
}
