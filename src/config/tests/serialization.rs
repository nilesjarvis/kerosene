use serde::{Serialize, de::DeserializeOwned};

mod appearance;
mod charts;
mod credentials;
mod feeds;
mod history;
mod panes;
mod preferences;

fn json_string<T: Serialize>(value: &T, context: &str) -> String {
    match serde_json::to_string(value) {
        Ok(json) => json,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn json_value<T: Serialize>(value: T, context: &str) -> serde_json::Value {
    match serde_json::to_value(value) {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn value_from_json<T: DeserializeOwned>(value: serde_json::Value, context: &str) -> T {
    match serde_json::from_value(value) {
        Ok(decoded) => decoded,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn value_from_str<T: DeserializeOwned>(json: &str, context: &str) -> T {
    match serde_json::from_str(json) {
        Ok(decoded) => decoded,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn default_config_value() -> serde_json::Value {
    json_value(
        crate::config::KeroseneConfig::default(),
        "default config should serialize",
    )
}

fn object_mut<'a>(
    value: &'a mut serde_json::Value,
    context: &str,
) -> &'a mut serde_json::Map<String, serde_json::Value> {
    match value.as_object_mut() {
        Some(object) => object,
        None => panic!("{context}"),
    }
}

fn remove_field(value: &mut serde_json::Value, field: &str, context: &str) {
    object_mut(value, context).remove(field);
}
