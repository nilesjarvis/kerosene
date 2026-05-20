use super::{HotkeyAction, HotkeyConfig, HotkeyPrefixConfig};

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
fn alfred_hotkey_round_trips() {
    let hotkey = HotkeyConfig {
        action: HotkeyAction::OpenAlfred,
        key: "Space".to_string(),
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

#[test]
fn chart_timeframe_prefix_round_trips_modifiers() {
    let prefix = HotkeyPrefixConfig {
        shift: false,
        ctrl: false,
        alt: false,
        logo: true,
    };

    let json = serde_json::to_string(&prefix).expect("prefix should serialize");
    let loaded: HotkeyPrefixConfig =
        serde_json::from_str(&json).expect("prefix should deserialize");

    assert_eq!(loaded, prefix);
}
