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
