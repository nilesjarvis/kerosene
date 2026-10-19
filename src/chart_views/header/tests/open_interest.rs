use crate::account::AssetContext;
use crate::api::Candle;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartInstance, ChartSurfaceId};
use crate::timeframe::Timeframe;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{Layout, layout};
use iced::{Font, Pixels, Rectangle, Renderer, Size};

#[derive(Default)]
struct OpenInterestText {
    label: Option<String>,
    value: Option<String>,
}

impl Operation for OpenInterestText {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _id: Option<&Id>, _bounds: Rectangle, value: &str) {
        if value.starts_with("Open Interest") {
            self.label = Some(value.to_string());
        } else if self.label.is_some() && self.value.is_none() {
            self.value = Some(value.to_string());
        }
    }
}

fn context(oi: &str, mark: Option<&str>) -> AssetContext {
    serde_json::from_value(serde_json::json!({
        "openInterest": oi, "markPx": mark, "oraclePx": "80",
        "funding": "0", "dayBaseVlm": "10000", "dayNtlVlm": "1000000"
    }))
    .expect("public asset context fixture")
}

fn assert_open_interest(
    terminal: &TradingTerminal,
    chart: &ChartInstance,
    renderer: &Renderer,
    expected: &str,
) {
    let mut header = terminal.view_chart_header_sized(7, chart, ChartSurfaceId::Docked(7), 1200.0);
    let mut tree = Tree::new(header.as_widget());
    let node = header.as_widget_mut().layout(
        &mut tree,
        renderer,
        &layout::Limits::new(Size::ZERO, Size::new(1200.0, 500.0)),
    );
    let mut text = OpenInterestText::default();
    header
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), renderer, &mut text);
    assert_eq!(
        text.label.as_deref(),
        Some(if chart.open_interest_as_notional {
            "Open Interest $"
        } else {
            "Open Interest"
        })
    );
    assert_eq!(text.value.as_deref(), Some(expected), "{}", chart.symbol);
}

#[tokio::test]
async fn perpetual_open_interest_uses_context_mark_and_can_exceed_cap() {
    let renderer = Renderer::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot().0;
    let caps = terminal
        .chart_open_interest_caps
        .dexes
        .entry("para".to_string())
        .or_default();
    caps.caps.insert("para:PAPER".to_string(), 5_000_000.0);
    caps.updated_at_ms = Some(0);

    for symbol in ["BTC", "para:PAPER"] {
        let mut chart = ChartInstance::new(7, symbol.to_string(), Timeframe::H1);
        chart.chart.set_clock_now_ms(0);
        chart.open_interest_as_notional = true;
        for (oi, mark, candle_price, expected) in [
            ("100000", "40", 100.0, "$4.00M"),
            ("100000", "40", 200.0, "$4.00M"),
            ("200000", "30", 200.0, "$6.00M"),
        ] {
            chart.chart.candles = vec![Candle::test_price(0, candle_price)];
            chart.set_asset_context_at(Some(context(oi, Some(mark))), 0);
            assert_open_interest(&terminal, &chart, &renderer, expected);
        }
    }
}

#[tokio::test]
async fn invalid_mark_disables_dollar_open_interest_but_preserves_coin_units() {
    let renderer = Renderer::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let terminal = TradingTerminal::boot().0;
    for symbol in ["BTC", "para:PAPER"] {
        let mut chart = ChartInstance::new(7, symbol.to_string(), Timeframe::H1);
        chart.chart.candles.push(Candle::test_price(0, 100.0));
        chart.chart.set_clock_now_ms(0);
        for mark in [
            None,
            Some("bad"),
            Some("NaN"),
            Some("inf"),
            Some("0"),
            Some("-1"),
        ] {
            chart.set_asset_context_at(Some(context("100000", mark)), 0);
            chart.open_interest_as_notional = true;
            assert_open_interest(&terminal, &chart, &renderer, "Invalid data");
            chart.open_interest_as_notional = false;
            assert_open_interest(&terminal, &chart, &renderer, "100K");
        }
    }
}
