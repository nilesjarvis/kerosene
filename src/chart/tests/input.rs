use super::{action_or_panic, candle_at, chart_bounds, message_or_panic};
use crate::chart::{CandlestickChart, OrderOverlay, OrderOverlayPendingState};
use crate::message::Message;
use iced::Point;

mod left_click;
mod right_click;

const CHART_W: f32 = 400.0;
const CHART_H: f32 = 240.0;
const SURFACE_W: f32 = 420.0;
const SURFACE_H: f32 = 260.0;

fn chart_with_input_candles() -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 100.0), candle_at(2_000, 110.0)]);
    chart
}

fn quick_order_chart() -> CandlestickChart {
    let mut chart = chart_with_input_candles();
    chart.quick_order_open = true;
    chart
}

fn btc_buy_order(oid: u64) -> OrderOverlay {
    OrderOverlay {
        coin: "BTC".to_string(),
        limit_px: 105.0,
        sz: 1.0,
        is_buy: true,
        oid,
        is_moving: false,
        pending_state: None,
    }
}

fn pending_btc_buy_order(oid: u64) -> OrderOverlay {
    OrderOverlay {
        pending_state: Some(OrderOverlayPendingState::Cancelling),
        ..btc_buy_order(oid)
    }
}

fn assert_open_quick_order_message(
    message: Option<Message>,
    chart: &CandlestickChart,
    click: Point,
) {
    match message_or_panic(message, "open quick-order message") {
        Message::OpenQuickOrder(id, surface_id, price, click_x, click_y, chart_w, chart_h) => {
            assert_eq!(id, chart.id);
            assert_eq!(surface_id, chart.surface_id);
            assert!(price.is_finite() && price > 0.0);
            assert_eq!(click_x, click.x);
            assert_eq!(click_y, click.y);
            assert_eq!(chart_w, CHART_W);
            assert_eq!(chart_h, CHART_H);
        }
        other => panic!("expected OpenQuickOrder, got {other:?}"),
    }
}
