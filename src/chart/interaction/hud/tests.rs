mod size_input;

use super::{format_hud_size, hud_size_scroll_step};
use crate::chart::{CandlestickChart, ChartState};
use crate::config::ChartCrosshairStyle;
use crate::message::Message;
use crate::sound::HudUiSound;
use iced::{Point, keyboard};

fn pressed_control(
    chart: &CandlestickChart,
    state: &mut ChartState,
    key: &str,
) -> Option<(HudUiSound, bool)> {
    let action = chart
        .handle_hud_key_pressed(
            state,
            keyboard::Key::Character(key),
            Some(key),
            keyboard::Modifiers::NONE,
        )
        .expect("selector key should be handled");
    let (message, _, _) = action.into_inner();
    match message {
        Some(Message::ChartHudControlChanged(id, surface_id, control, changed)) => {
            assert_eq!(id, chart.id);
            assert_eq!(surface_id, chart.surface_id);
            Some((control, changed))
        }
        Some(other) => panic!("expected ChartHudControlChanged, got {other:?}"),
        None => None,
    }
}

#[test]
fn mode_key_always_pops_the_selector_but_clicks_only_on_change() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    let mut state = ChartState {
        cursor_position: Some(Point::ORIGIN),
        ..ChartState::default()
    };

    // Default mode is Limit, so M is a change and clicks.
    assert_eq!(
        pressed_control(&chart, &mut state, "m"),
        Some((HudUiSound::ModeMarket, true))
    );

    // Repeating M still publishes (the weapon list re-opens, Battlefield
    // style) but is flagged unchanged so no click replays.
    assert_eq!(
        pressed_control(&chart, &mut state, "m"),
        Some((HudUiSound::ModeMarket, false))
    );

    assert_eq!(
        pressed_control(&chart, &mut state, "h"),
        Some((HudUiSound::ModeChase, true))
    );
    assert_eq!(
        pressed_control(&chart, &mut state, "h"),
        Some((HudUiSound::ModeChase, false))
    );
}

#[test]
fn side_keys_publish_direction_coded_controls() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    let mut state = ChartState {
        cursor_position: Some(Point::ORIGIN),
        ..ChartState::default()
    };

    // Default side is Long, so X is a change.
    assert_eq!(
        pressed_control(&chart, &mut state, "x"),
        Some((HudUiSound::SideShort, true))
    );
    assert_eq!(
        pressed_control(&chart, &mut state, "y"),
        Some((HudUiSound::SideLong, true))
    );
}

#[test]
fn size_scroll_publishes_direction_coded_ticks() {
    let chart = {
        let mut chart = CandlestickChart::new(1);
        chart.set_crosshair_style(ChartCrosshairStyle::Hud);
        chart
    };
    let mut state = ChartState::default();

    let action = chart
        .handle_hud_size_scroll(&mut state, 1.0)
        .expect("scroll up should be handled");
    let (message, _, _) = action.into_inner();
    assert!(matches!(
        message,
        Some(Message::ChartHudControlChanged(
            _,
            _,
            HudUiSound::SizeUp,
            true
        ))
    ));

    let action = chart
        .handle_hud_size_scroll(&mut state, -1.0)
        .expect("scroll down should be handled");
    let (message, _, _) = action.into_inner();
    assert!(matches!(
        message,
        Some(Message::ChartHudControlChanged(
            _,
            _,
            HudUiSound::SizeDown,
            true
        ))
    ));
}

#[test]
fn hud_size_format_trims_fractional_padding() {
    assert_eq!(format_hud_size(1.0), "1");
    assert_eq!(format_hud_size(1.2), "1.2");
    assert_eq!(format_hud_size(0.125), "0.125");
    assert_eq!(format_hud_size(123.0), "123");
}

#[test]
fn hud_size_scroll_step_scales_with_size() {
    assert_eq!(hud_size_scroll_step(0.5), 0.01);
    assert_eq!(hud_size_scroll_step(1.0), 0.1);
    assert_eq!(hud_size_scroll_step(10.0), 1.0);
    assert_eq!(hud_size_scroll_step(100.0), 10.0);
}

#[test]
fn c_key_toggles_hud_follow_price() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    let mut state = ChartState {
        cursor_position: Some(Point::ORIGIN),
        ..ChartState::default()
    };

    let action = chart.handle_hud_key_pressed(
        &mut state,
        keyboard::Key::Character("c"),
        Some("c"),
        keyboard::Modifiers::NONE,
    );

    assert!(action.is_some());
    assert!(state.hud_follow_price);

    let action = chart.handle_hud_key_pressed(
        &mut state,
        keyboard::Key::Character("c"),
        Some("c"),
        keyboard::Modifiers::NONE,
    );

    assert!(action.is_some());
    assert!(!state.hud_follow_price);
}
