use super::{HotkeyAction, HotkeyConfig};

#[test]
fn switch_account_hotkey_round_trips_secret_id() {
    let hotkey = HotkeyConfig {
        action: HotkeyAction::SwitchAccount {
            secret_id: "acct-a".to_string(),
        },
        key: "1".to_string(),
        shift: false,
        ctrl: true,
        alt: false,
        logo: false,
    };

    let json = serde_json::to_string(&hotkey).expect("hotkey should serialize");
    let loaded: HotkeyConfig = serde_json::from_str(&json).expect("hotkey should deserialize");

    assert_eq!(loaded, hotkey);
}

#[test]
fn settings_window_hotkey_round_trips() {
    let hotkey = HotkeyConfig {
        action: HotkeyAction::OpenSettingsWindow,
        key: ",".to_string(),
        shift: false,
        ctrl: true,
        alt: false,
        logo: false,
    };

    let json = serde_json::to_string(&hotkey).expect("hotkey should serialize");
    let loaded: HotkeyConfig = serde_json::from_str(&json).expect("hotkey should deserialize");

    assert_eq!(loaded, hotkey);
}

#[test]
fn trading_journal_hotkey_round_trips() {
    let hotkey = HotkeyConfig {
        action: HotkeyAction::OpenTradingJournal,
        key: "J".to_string(),
        shift: false,
        ctrl: true,
        alt: false,
        logo: false,
    };

    let json = serde_json::to_string(&hotkey).expect("hotkey should serialize");
    let loaded: HotkeyConfig = serde_json::from_str(&json).expect("hotkey should deserialize");

    assert_eq!(loaded, hotkey);
}

#[test]
fn switch_layout_hotkey_round_trips_layout_name() {
    let hotkey = HotkeyConfig {
        action: HotkeyAction::SwitchLayout {
            name: "Scalping".to_string(),
        },
        key: "F2".to_string(),
        shift: false,
        ctrl: false,
        alt: false,
        logo: false,
    };

    let json = serde_json::to_string(&hotkey).expect("hotkey should serialize");
    let loaded: HotkeyConfig = serde_json::from_str(&json).expect("hotkey should deserialize");

    assert_eq!(loaded, hotkey);
}
