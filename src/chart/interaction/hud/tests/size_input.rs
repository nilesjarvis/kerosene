use crate::chart::{CandlestickChart, ChartState};
use crate::config::ChartCrosshairStyle;
use iced::Point;
use iced::event::Status;
use iced::keyboard::{self, Key, key::Named};

fn hud_chart() -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart
}

#[test]
fn size_text_preserves_decimal_replacement_filtering_and_length_rules() {
    let chart = hud_chart();
    for (input, replace, text, expected, handled, still_replace) in [
        ("", true, ".", "0.", true, false),
        ("12.5", true, ".", "12.5", false, true),
        ("12.5", true, ".7", "7", true, false),
        ("12", true, ".5", "0.5", true, false),
        ("1.2", false, "..3", "1.23", true, false),
        ("12", false, "a-٣四 5.6x", "125.6", true, false),
        ("", false, "٠２abc", "", false, false),
        ("9", true, "abc", "9", false, true),
        ("", true, "0012", "0012", true, false),
        ("123456789012", false, "3", "123456789012", true, false),
        ("123456789012", false, ".4", "123456789012", true, false),
        ("1234567890123", false, "", "1234567890123", false, false),
        ("1234567890123", true, "7", "7", true, false),
        ("0", false, ".", "0.", true, false),
        ("", false, "2.3.4", "2.34", true, false),
        ("9", true, ".a", "0.", true, false),
    ] {
        let mut state = ChartState {
            cursor_position: Some(Point::ORIGIN),
            hud_size_input: input.into(),
            hud_size_editing: true,
            hud_size_replace_on_type: replace,
            ..Default::default()
        };
        let action = chart.handle_hud_key_pressed(
            &mut state,
            Key::Character(text),
            Some(text),
            keyboard::Modifiers::NONE,
        );
        assert_eq!(
            state.hud_size_input, expected,
            "input {input:?}, text {text:?}"
        );
        assert_eq!(state.hud_size_replace_on_type, still_replace);
        assert!(state.hud_size_editing);
        assert_eq!(action.is_some(), handled);
        if let Some(action) = action {
            let (message, _, status) = action.into_inner();
            assert!(message.is_none());
            assert_eq!(status, Status::Captured);
        }
    }
}

#[test]
fn size_edit_finish_and_delete_keys_preserve_text_and_edit_flags() {
    let chart = hud_chart();
    for key in [Named::Enter, Named::Escape, Named::Backspace, Named::Delete] {
        for (input, popped) in [("", ""), (" ", ""), ("1.25", "1.2")] {
            for replace in [false, true] {
                let mut state = ChartState {
                    cursor_position: Some(Point::ORIGIN),
                    hud_size_input: input.into(),
                    hud_size_editing: true,
                    hud_size_replace_on_type: replace,
                    ..Default::default()
                };
                let action = chart
                    .handle_hud_key_pressed(
                        &mut state,
                        Key::Named(key),
                        Some("9"),
                        keyboard::Modifiers::NONE,
                    )
                    .expect("editing key handled");
                let (message, _, status) = action.into_inner();
                let finished = matches!(key, Named::Enter | Named::Escape);
                let expected = if finished {
                    if input.trim().is_empty() { "0" } else { input }
                } else if key == Named::Backspace && !replace {
                    popped
                } else {
                    ""
                };
                assert_eq!(state.hud_size_input, expected);
                assert_eq!(state.hud_size_editing, !finished);
                assert!(!state.hud_size_replace_on_type);
                assert!(message.is_none());
                assert_eq!(status, Status::Captured);
            }
        }
    }
}
