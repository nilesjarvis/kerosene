use super::*;
use crate::chart_state::ChartSurfaceId;
use futures::StreamExt as _;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{Layout, layout};
use iced::{Font, Pixels, Rectangle, Renderer, Size};
use std::time::Duration;

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

/// Exercises the production subscription, websocket parser, message mapping,
/// chart update, and actual header layout against public market data.
#[tokio::test]
#[ignore = "requires live public Hyperliquid WebSocket access; run explicitly"]
async fn live_native_funding_reaches_chart_header_with_hydromancer_selected() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "unused-test-key".to_string().into();
    for (id, symbol) in [(1, "BTC"), (2, "xyz:NVDA")] {
        let mut chart = ChartInstance::new(id, symbol.to_string(), Timeframe::H1);
        chart
            .chart
            .candles
            .push(crate::api::Candle::test_price(0, 100.0));
        terminal.charts.insert(id, chart);
    }
    let source_context = terminal.market_data_source_context();
    let expected = [(1, "BTC"), (2, "xyz:NVDA")]
        .into_iter()
        .flat_map(|(id, symbol)| {
            subscription_hashes(
                Subscription::run_with((id, symbol.to_string()), ws_asset_ctx_stream_keyed)
                    .with(source_context)
                    .map(chart_asset_ctx_stream_event_message),
            )
        })
        .collect::<Vec<_>>();
    let mut subscriptions = Vec::new();
    terminal.push_chart_market_subscriptions(&mut subscriptions);
    // Execute only the real chart context recipes: no candle, account, or
    // authenticated Hydromancer requests are needed for this verification.
    let streams = into_recipes(Subscription::batch(subscriptions))
        .into_iter()
        .filter(|recipe| {
            let mut hasher = Hasher::default();
            recipe.hash(&mut hasher);
            expected.contains(&hasher.finish())
        })
        .map(|recipe| recipe.stream(Box::pin(futures::stream::pending())))
        .collect::<Vec<_>>();
    assert_eq!(streams.len(), 2, "both charts use native asset context");
    let mut stream = futures::stream::select_all(streams);
    tokio::time::timeout(Duration::from_secs(20), async {
        while terminal
            .charts
            .values()
            .any(|chart| chart.asset_ctx.is_none())
        {
            let message = stream
                .next()
                .await
                .expect("asset context stream stays open");
            let _task = terminal.update_chart(message);
        }
    })
    .await
    .expect("both charts receive live asset context within 20 seconds");

    let renderer = Renderer::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for (id, chart) in &terminal.charts {
        let rate = chart
            .asset_ctx
            .as_ref()
            .expect("live context")
            .funding
            .as_deref()
            .expect("native funding field")
            .parse::<f64>()
            .expect("numeric funding rate");
        assert!(rate.is_finite());
        let expected = format!("{:.4}%", rate * 100.0);
        let mut header = terminal.view_chart_header(*id, chart, ChartSurfaceId::Docked(*id));
        let mut tree = Tree::new(header.as_widget());
        let node = header.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(320.0, 500.0)),
        );
        let mut text = HeaderText::default();
        header
            .as_widget_mut()
            .operate(&mut tree, Layout::new(&node), &renderer, &mut text);
        let (_, bounds) = text
            .0
            .iter()
            .find(|(value, _)| value == &expected)
            .expect("live funding percentage rendered in header");
        assert!(bounds.width > 0.0 && bounds.height > 0.0);
        assert!(bounds.x >= 0.0 && bounds.x + bounds.width <= 320.1);
        println!(
            "{}: live funding {expected} rendered inside 320px header",
            chart.symbol
        );
    }
}
