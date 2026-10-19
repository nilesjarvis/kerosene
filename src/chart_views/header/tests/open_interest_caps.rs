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
struct OpenInterestBounds(Option<Rectangle>, Option<Rectangle>);

impl Operation for OpenInterestBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, value: &str) {
        if value.starts_with("Open Interest") {
            self.0 = Some(bounds);
        } else if value == "1K" || value == "$123.4K" {
            self.1 = Some(bounds);
        }
    }
}

#[tokio::test]
async fn open_interest_cap_warning_and_tooltip_track_exchange_status_and_preserve_click_toggle() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for (symbol, cap, as_notional, now_ms, status, expect_tooltip, expect_warning) in [
        ("xyz:AAPL", Some(25_000_000.0), false, 0, None, true, false),
        ("xyz:AAPL", Some(25_000_000.0), true, 0, None, true, false),
        ("xyz:AAPL", Some(0.0), false, 0, Some(false), true, false),
        ("xyz:AAPL", None, false, 0, None, false, false),
        (
            "xyz:AAPL",
            Some(25_000_000.0),
            false,
            120_000,
            Some(true),
            false,
            false,
        ),
        (
            "abc:AAPL",
            Some(25_000_000.0),
            false,
            0,
            Some(true),
            false,
            false,
        ),
        ("BTC", Some(25_000_000.0), false, 0, None, false, false),
        // The exchange reports a cap despite OI being far below the numeric limit.
        (
            "xyz:AAPL",
            Some(25_000_000.0),
            false,
            0,
            Some(true),
            true,
            true,
        ),
        (
            "xyz:AAPL",
            Some(25_000_000.0),
            true,
            0,
            Some(true),
            true,
            true,
        ),
        // Numeric equality alone must never trigger the warning.
        (
            "xyz:AAPL",
            Some(123_400.0),
            true,
            0,
            Some(false),
            true,
            false,
        ),
        ("xyz:AAPL", None, false, 0, Some(true), true, true),
        ("BTC", None, false, 0, Some(true), true, true),
        ("BTC", None, true, 0, Some(true), true, true),
        ("BTC", None, false, 120_000, Some(true), false, false),
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
        if let Some(at_cap) = status {
            let (dex, coin) = if symbol == "BTC" {
                ("", "BTC")
            } else {
                ("xyz", "xyz:AAPL")
            };
            let entry = terminal
                .chart_open_interest_caps
                .dexes
                .entry(dex.to_string())
                .or_default();
            entry.status.updated_at_ms = Some(0);
            if at_cap {
                entry.status.coins.insert(coin.to_string());
            }
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
        let pixels = renderer.screenshot(Size::new(1200, 120), 1.0, theme.palette().background);
        let has_red_pixels = |area: Rectangle| {
            (area.y.ceil() as usize..(area.y + area.height).floor() as usize).any(|y| {
                (area.x.ceil() as usize..(area.x + area.width).floor() as usize).any(|x| {
                    let pixel = &pixels[(y * 1200 + x) * 4..][..3];
                    pixel[0] > pixel[1].saturating_add(40) && pixel[0] > pixel[2].saturating_add(40)
                })
            })
        };
        let label_bounds = bounds.0.expect("open interest label");
        let value_bounds = bounds.1.expect("open interest value");
        assert_eq!(
            has_red_pixels(label_bounds),
            expect_warning,
            "{symbol} label at {now_ms}"
        );
        assert_eq!(
            has_red_pixels(value_bounds),
            expect_warning,
            "{symbol} value at {now_ms}"
        );
        if expect_warning {
            // The warning icon occupies the space before the value and is red too.
            assert!(value_bounds.x > label_bounds.x + 10.0);
            assert!(has_red_pixels(Rectangle {
                x: label_bounds.x,
                y: value_bounds.y,
                width: value_bounds.x - label_bounds.x - 2.0,
                height: value_bounds.height,
            }));
        }
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
            && expect_warning
            && let Some(path) = std::env::var_os("KEROSENE_CHART_HEADER_PREVIEW_PATH")
        {
            let pixels = renderer.screenshot(Size::new(1200, 120), 1.0, theme.palette().background);
            image::save_buffer(path, &pixels, 1200, 120, image::ColorType::Rgba8)
                .expect("header preview");
        }
    }
}
