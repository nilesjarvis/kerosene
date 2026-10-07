use super::{action_or_panic, candle_at, chart_bounds, message_or_panic};
use crate::chart::state::HudOrderKind;
use crate::chart::{CandlestickChart, ChartState};
use crate::config::ChartCrosshairStyle;
use crate::message::Message;
use crate::timeframe::Timeframe;
use iced::{Event, Point, mouse};

fn send_event(
    chart: &CandlestickChart,
    state: &mut ChartState,
    event: mouse::Event,
) -> Option<Message> {
    chart
        .update_interaction(
            state,
            &Event::Mouse(event),
            chart_bounds(800.0, 500.0),
            mouse::Cursor::Available(Point::new(120.0, 80.0)),
        )
        .and_then(|action| action.into_inner().0)
}

fn sync_canvas(chart: &CandlestickChart, state: &mut ChartState) {
    let message = send_event(
        chart,
        state,
        mouse::Event::CursorMoved {
            position: Point::new(120.0, 80.0),
        },
    );
    assert!(!matches!(message, Some(Message::SubmitHudOrder(_))));
}

#[test]
fn symbol_switch_resets_hud_size_before_rearmed_order_in_every_mode() {
    for style in [ChartCrosshairStyle::Hud, ChartCrosshairStyle::RacingHud] {
        for mode in [
            HudOrderKind::Limit,
            HudOrderKind::Market,
            HudOrderKind::Chase,
        ] {
            // Both ordinary ticker selection and direct identity rewrites must reset size.
            for extra_view_reset in [false, true] {
                let mut chart = CandlestickChart::new(1);
                chart.set_crosshair_style(style);
                chart.set_symbol_key("DOGE".into());
                chart.set_candles(vec![candle_at(1_000, 1.0), candle_at(2_000, 1.1)]);
                let mut state = ChartState::default();
                sync_canvas(&chart, &mut state);
                state.hud_order_kind = mode;
                state.hud_size_input = "100".into();
                state.hud_size_editing = true;
                state.hud_size_replace_on_type = true;
                state.hud_size_scroll_bias = 1.0;
                chart.set_hud_armed_at(true, 1_000);

                chart.set_symbol_key("BTC".into());
                if extra_view_reset {
                    chart.request_view_reset();
                }
                chart.set_candles(vec![
                    candle_at(1_000, 90_000.0),
                    candle_at(2_000, 100_000.0),
                ]);
                chart.set_market_reference_price(Some(100_000.0));
                assert!(!chart.hud_armed());
                assert_eq!(
                    chart.chart_state_for_draw(&state).as_ref().hud_size_input,
                    "1"
                );

                // Even if rearmed before the next canvas event, a click must
                // apply the pending reset before it can publish an order.
                chart.set_hud_armed_at(true, 2_000);
                let first = send_event(
                    &chart,
                    &mut state,
                    mouse::Event::ButtonPressed(mouse::Button::Left),
                );
                assert!(!matches!(first, Some(Message::SubmitHudOrder(_))));
                assert_eq!(state.hud_size_input, "1");
                assert!(!state.hud_size_editing);
                assert!(!state.hud_size_replace_on_type);
                assert_eq!(state.hud_size_scroll_bias, 0.0);
                assert_eq!(
                    chart.chart_state_for_draw(&state).as_ref().hud_size_input,
                    "1"
                );

                let action = action_or_panic(
                    chart.update_interaction(
                        &mut state,
                        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                        chart_bounds(800.0, 500.0),
                        mouse::Cursor::Available(Point::new(120.0, 80.0)),
                    ),
                    "rearmed click should submit the reset HUD size",
                );
                match message_or_panic(action.into_inner().0, "HUD order after symbol switch") {
                    Message::SubmitHudOrder(request) => {
                        assert_eq!(request.symbol_key, "BTC");
                        assert_eq!(request.quantity, "1");
                    }
                    other => panic!("expected SubmitHudOrder, got {other:?}"),
                }
            }
        }
    }
}

#[test]
fn same_symbol_and_timeframe_view_resets_preserve_hud_size() {
    let mut chart = CandlestickChart::new(1);
    chart.set_symbol_key("BTC".into());
    let mut state = ChartState::default();
    sync_canvas(&chart, &mut state);
    state.hud_size_input = "0.125".into();

    chart.set_symbol_key("BTC".into());
    sync_canvas(&chart, &mut state);
    assert_eq!(state.hud_size_input, "0.125");

    chart.set_timeframe(Timeframe::M5);
    chart.request_view_reset();
    sync_canvas(&chart, &mut state);
    assert_eq!(state.hud_size_input, "0.125");

    chart.request_view_reset();
    sync_canvas(&chart, &mut state);
    assert_eq!(state.hud_size_input, "0.125");
}

#[test]
fn rapid_symbol_round_trip_resets_hud_size_on_each_canvas() {
    let mut chart = CandlestickChart::new(1);
    chart.set_symbol_key("BTC".into());
    let mut first = ChartState::default();
    let mut second = ChartState::default();
    sync_canvas(&chart, &mut first);
    sync_canvas(&chart, &mut second);
    first.hud_size_input = "10".into();
    second.hud_size_input = "20".into();

    chart.set_symbol_key("DOGE".into());
    chart.set_symbol_key("BTC".into());
    sync_canvas(&chart, &mut first);
    assert_eq!(first.hud_size_input, "1");
    sync_canvas(&chart, &mut second);
    assert_eq!(second.hud_size_input, "1");

    first.hud_size_input = "0.25".into();
    sync_canvas(&chart, &mut first);
    assert_eq!(first.hud_size_input, "0.25");
}
