use super::{ChartEditorSelectionStep, next_chart_editor_selection};
use crate::app_state::TradingTerminal;
use crate::{config, message::Message};

#[test]
fn chart_editor_keyboard_selection_starts_in_direction() {
    assert_eq!(
        next_chart_editor_selection(None, 4, ChartEditorSelectionStep::Next),
        Some(0)
    );
    assert_eq!(
        next_chart_editor_selection(None, 4, ChartEditorSelectionStep::Previous),
        Some(3)
    );
}

#[test]
fn chart_editor_keyboard_selection_clamps_at_edges() {
    assert_eq!(
        next_chart_editor_selection(Some(2), 3, ChartEditorSelectionStep::Next),
        Some(2)
    );
    assert_eq!(
        next_chart_editor_selection(Some(0), 3, ChartEditorSelectionStep::Previous),
        Some(0)
    );
}

#[test]
fn chart_editor_keyboard_selection_handles_empty_or_stale_index() {
    assert_eq!(
        next_chart_editor_selection(Some(2), 0, ChartEditorSelectionStep::Next),
        None
    );
    assert_eq!(
        next_chart_editor_selection(Some(99), 3, ChartEditorSelectionStep::Next),
        Some(0)
    );
}

#[test]
fn chart_timeframe_prefix_records_modifier_changed_event() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.hotkeys.clear();
    terminal.chart_timeframe_hotkey_prefix = None;
    terminal.recording_hotkey_for = Some(config::HotkeyAction::ChartTimeframePrefix);

    let _ = terminal.handle_hotkey_keyboard_event(Message::KeyboardEvent(
        iced::keyboard::Event::ModifiersChanged(iced::keyboard::Modifiers::LOGO),
        iced::event::Status::Ignored,
    ));

    assert_eq!(
        terminal.chart_timeframe_hotkey_prefix,
        Some(config::HotkeyPrefixConfig {
            shift: false,
            ctrl: false,
            alt: false,
            logo: true,
        })
    );
    assert_eq!(terminal.recording_hotkey_for, None);
}

#[test]
fn chart_timeframe_prefix_recording_drops_incidental_shift_with_command() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.hotkeys.clear();
    terminal.chart_timeframe_hotkey_prefix = None;
    terminal.recording_hotkey_for = Some(config::HotkeyAction::ChartTimeframePrefix);

    let _ = terminal.handle_hotkey_keyboard_event(Message::KeyboardEvent(
        iced::keyboard::Event::ModifiersChanged(
            iced::keyboard::Modifiers::LOGO | iced::keyboard::Modifiers::SHIFT,
        ),
        iced::event::Status::Ignored,
    ));

    assert_eq!(
        terminal.chart_timeframe_hotkey_prefix,
        Some(config::HotkeyPrefixConfig {
            shift: false,
            ctrl: false,
            alt: false,
            logo: true,
        })
    );
}
