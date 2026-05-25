use super::{HotkeyAction, HotkeyConfig, HotkeyPrefixConfig};

mod actions;
mod prefix;

fn round_trip_hotkey_or_panic(hotkey: &HotkeyConfig) -> HotkeyConfig {
    let json = match serde_json::to_string(hotkey) {
        Ok(json) => json,
        Err(error) => panic!("hotkey should serialize: {error}"),
    };
    match serde_json::from_str(&json) {
        Ok(loaded) => loaded,
        Err(error) => panic!("hotkey should deserialize: {error}"),
    }
}

fn round_trip_prefix_or_panic(prefix: &HotkeyPrefixConfig) -> HotkeyPrefixConfig {
    let json = match serde_json::to_string(prefix) {
        Ok(json) => json,
        Err(error) => panic!("prefix should serialize: {error}"),
    };
    match serde_json::from_str(&json) {
        Ok(loaded) => loaded,
        Err(error) => panic!("prefix should deserialize: {error}"),
    }
}
