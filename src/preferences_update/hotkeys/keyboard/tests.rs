use super::{ChartEditorSelectionStep, next_chart_editor_selection};
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartInstance, ChartSurfaceId, DetachedChartWindowState};
use crate::timeframe::Timeframe;
use crate::{config, message::Message};

fn key_pressed(
    key: iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
) -> iced::keyboard::Event {
    iced::keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyK),
        location: iced::keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    }
}

fn symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 50,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

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
fn chart_editor_keyboard_handles_detached_window_event() {
    let mut terminal = TradingTerminal::boot().0;
    let main_window_id = iced::window::Id::unique();
    let detached_window_id = iced::window::Id::unique();
    let chart_id = 7;
    terminal.main_window_id = Some(main_window_id);
    terminal.primary_chart_id = None;
    terminal.charts.clear();
    terminal.detached_chart_windows.clear();
    terminal.exchange_symbols = vec![symbol("BTC"), symbol("ETH")];

    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance
        .chart
        .set_surface_id(ChartSurfaceId::Detached(detached_window_id));
    instance.editor_open = true;
    terminal.charts.insert(chart_id, instance);
    terminal
        .detached_chart_windows
        .insert(detached_window_id, DetachedChartWindowState::new(chart_id));

    let _ = terminal.handle_hotkey_keyboard_event(Message::KeyboardEvent(
        detached_window_id,
        key_pressed(
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown),
            iced::keyboard::Modifiers::NONE,
        ),
        iced::event::Status::Captured,
    ));

    assert_eq!(
        terminal
            .charts
            .get(&chart_id)
            .and_then(|instance| instance.editor_selected_index),
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
        iced::window::Id::unique(),
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
        iced::window::Id::unique(),
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

#[test]
fn configured_hotkey_ignores_auxiliary_window_keyboard_event() {
    let mut terminal = TradingTerminal::boot().0;
    let main_window_id = iced::window::Id::unique();
    let aux_window_id = iced::window::Id::unique();
    terminal.main_window_id = Some(main_window_id);
    terminal.hotkeys = vec![config::HotkeyConfig {
        action: config::HotkeyAction::OpenAlfred,
        key: "K".to_string(),
        shift: false,
        ctrl: true,
        alt: false,
        logo: false,
    }];

    let _ = terminal.handle_hotkey_keyboard_event(Message::KeyboardEvent(
        aux_window_id,
        key_pressed(
            iced::keyboard::Key::Character("k".into()),
            iced::keyboard::Modifiers::CTRL,
        ),
        iced::event::Status::Ignored,
    ));

    assert!(!terminal.alfred.open);
}
