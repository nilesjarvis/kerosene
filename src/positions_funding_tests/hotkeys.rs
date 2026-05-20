use crate::app_state::TradingTerminal;
use crate::config;

#[test]
fn hotkey_matching_rejects_unmodified_printable_and_navigation_keys() {
    let ctrl = iced::keyboard::Modifiers::CTRL;
    let none = iced::keyboard::Modifiers::NONE;

    assert!(TradingTerminal::hotkey_combo_is_assignable("1", ctrl));
    assert!(TradingTerminal::hotkey_combo_is_assignable("F1", none));
    assert!(!TradingTerminal::hotkey_combo_is_assignable("1", none));
    assert!(!TradingTerminal::hotkey_combo_is_assignable("Enter", ctrl));
    assert!(!TradingTerminal::hotkey_combo_is_assignable(
        "ArrowDown",
        ctrl
    ));

    let hotkey = config::HotkeyConfig {
        action: config::HotkeyAction::SwitchAccount {
            secret_id: "acct-a".to_string(),
        },
        key: "1".to_string(),
        shift: false,
        ctrl: true,
        alt: false,
        logo: false,
    };

    assert!(TradingTerminal::hotkey_matches(&hotkey, "1", ctrl));
    assert!(!TradingTerminal::hotkey_matches(&hotkey, "1", none));
}

#[test]
fn chart_timeframe_hotkey_maps_number_row_to_toolbar_timeframes() {
    assert_eq!(
        TradingTerminal::chart_timeframe_for_hotkey_key("1"),
        Some(crate::timeframe::Timeframe::M1)
    );
    assert_eq!(
        TradingTerminal::chart_timeframe_for_hotkey_key("2"),
        Some(crate::timeframe::Timeframe::M5)
    );
    assert_eq!(
        TradingTerminal::chart_timeframe_for_hotkey_key("7"),
        Some(crate::timeframe::Timeframe::W1)
    );
    assert_eq!(TradingTerminal::chart_timeframe_for_hotkey_key("8"), None);
}

#[test]
fn chart_timeframe_prefix_requires_matching_modifiers() {
    let prefix = config::HotkeyPrefixConfig {
        shift: false,
        ctrl: false,
        alt: false,
        logo: true,
    };

    assert!(TradingTerminal::hotkey_prefix_matches(
        &prefix,
        iced::keyboard::Modifiers::LOGO
    ));
    assert!(!TradingTerminal::hotkey_prefix_matches(
        &prefix,
        iced::keyboard::Modifiers::CTRL
    ));
}

#[test]
fn chart_timeframe_prefix_ignores_incidental_shift_with_primary_modifier() {
    let prefix = config::HotkeyPrefixConfig {
        shift: false,
        ctrl: false,
        alt: false,
        logo: true,
    };
    let shifted_logo = iced::keyboard::Modifiers::LOGO | iced::keyboard::Modifiers::SHIFT;

    assert!(TradingTerminal::hotkey_prefix_matches(
        &prefix,
        shifted_logo
    ));
    assert_eq!(
        TradingTerminal::normalize_chart_timeframe_hotkey_prefix(config::HotkeyPrefixConfig {
            shift: true,
            ctrl: false,
            alt: false,
            logo: true,
        }),
        Some(prefix)
    );
}
