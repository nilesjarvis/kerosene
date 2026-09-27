use crate::account::AssetContext;
use crate::api::Candle;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartInstance, ChartSurfaceId};
use crate::timeframe::Timeframe;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{Layout, layout};
use iced::{Font, Pixels, Rectangle, Renderer, Size};
use serde_json::json;

#[derive(Default)]
struct HeaderText(Vec<(String, Rectangle)>);

impl Operation for HeaderText {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, text: &str) {
        self.0.push((text.to_string(), bounds));
    }
}

fn header_text(symbol: &str, ctx: Option<AssetContext>, width: f32) -> HeaderText {
    let terminal = TradingTerminal::boot().0;
    let mut instance = ChartInstance::new(7, symbol.to_string(), Timeframe::H1);
    instance.chart.candles.push(Candle::test_price(0, 100.0));
    instance.set_asset_context_at(ctx, 0);
    let mut header =
        terminal.view_chart_header_sized(7, &instance, ChartSurfaceId::Docked(7), width);
    let renderer = futures::executor::block_on(Renderer::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("software renderer for chart header layout");
    let mut tree = Tree::new(header.as_widget());
    let node = header.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(width, 500.0)),
    );
    let mut text = HeaderText::default();
    header
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut text);
    text
}

fn context(funding: Option<&str>) -> AssetContext {
    serde_json::from_value(json!({
        "funding": funding,
        "openInterest": "1234",
        "markPx": "100",
        "oraclePx": "100",
        "dayBaseVlm": "10000",
        "dayNtlVlm": "1000000"
    }))
    .expect("public asset context fixture")
}

#[test]
fn perpetual_funding_stays_visible_and_inside_narrow_headers() {
    for symbol in ["BTC", "xyz:NVDA"] {
        for width in [240.0, 320.0, 420.0, 459.0, 460.0, 520.0, 760.0, 1200.0] {
            let text = header_text(symbol, Some(context(Some("0.0000125"))), width);
            for expected in ["Funding (", "0.0013%"] {
                let (_, bounds) = text
                    .0
                    .iter()
                    .find(|(value, _)| value.starts_with(expected))
                    .unwrap_or_else(|| panic!("missing {expected} for {symbol} at width {width}"));
                assert!(bounds.width > 0.0 && bounds.height > 0.0);
                assert!(bounds.x >= 0.0 && bounds.x + bounds.width <= width + 0.1);
            }
        }
    }
}

#[test]
fn missing_funding_does_not_hide_perpetual_metrics() {
    let text = header_text("BTC", Some(context(None)), 1200.0);
    assert!(
        text.0
            .iter()
            .any(|(value, _)| value.starts_with("Funding ("))
    );
    assert!(text.0.iter().any(|(value, _)| value == "-"));
    assert!(text.0.iter().any(|(value, _)| value == "Open Interest"));
    assert!(text.0.iter().any(|(value, _)| value == "Mark / Oracle"));

    let text = header_text("xyz:NVDA", None, 320.0);
    assert!(
        text.0
            .iter()
            .any(|(value, _)| value.starts_with("Funding ("))
    );
    assert!(text.0.iter().any(|(value, _)| value == "-"));
}

#[test]
fn spot_and_outcome_headers_do_not_show_funding() {
    for symbol in ["@107", "PURR/USDC", "#1"] {
        let text = header_text(symbol, Some(context(Some("0.0000125"))), 1200.0);
        assert!(
            !text
                .0
                .iter()
                .any(|(value, _)| value.starts_with("Funding ("))
        );
    }
}

#[test]
fn zero_and_negative_funding_remain_visible() {
    for (rate, expected) in [("0", "0.0000%"), ("-0.0001", "-0.0100%")] {
        let text = header_text("BTC", Some(context(Some(rate))), 320.0);
        assert!(text.0.iter().any(|(value, _)| value == expected));
    }
}
