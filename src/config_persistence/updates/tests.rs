use super::*;
use crate::config::updates::{ParameterChange, ParameterDecimal, ParameterEvent};

#[test]
fn updates_history_round_trips_baselines_and_events_with_atomic_replacement() {
    let directory = std::env::temp_dir().join(format!(
        "kerosene-updates-{}-{}",
        std::process::id(),
        crate::app_time::now_ms()
    ));
    let path = directory.join("updates.json");
    assert_eq!(
        load_from(&path).expect("missing"),
        UpdatesHistory::default()
    );
    let decimal = |value: &str| ParameterDecimal::try_from(value.to_string()).expect("decimal");
    let mut history = UpdatesHistory::default();
    history.caps.insert("xyz:BTC".into(), decimal("200"));
    history.caps_observed_at_ms = Some(31_000);
    history.events.push(ParameterEvent {
        key: "xyz:BTC".into(),
        detected_at_ms: 31_000,
        previous_observed_at_ms: 1_000,
        after_gap: true,
        changes: vec![ParameterChange::OiCap {
            old: decimal("100"),
            new: decimal("200"),
        }],
    });
    save_to(&path, history.clone()).expect("save");
    assert_eq!(load_from(&path).expect("load"), history);
    history.caps_observed_at_ms = Some(61_000);
    save_to(&path, history.clone()).expect("replace");
    assert_eq!(load_from(&path).expect("replacement"), history);
    std::fs::write(&path, b"{broken").expect("corrupt");
    assert!(load_from(&path).is_err());
    std::fs::write(
        &path,
        serde_json::to_vec(&Envelope {
            version: 2,
            history,
        })
        .expect("json"),
    )
    .expect("future");
    assert!(load_from(&path).is_err());
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn updates_tests_never_load_real_user_history() {
    assert_eq!(load().expect("isolated history"), UpdatesHistory::default());
}

#[test]
fn updates_history_encoding_obeys_size_limit() {
    let envelope = Envelope {
        version: 1,
        history: UpdatesHistory::default(),
    };
    let encoded = encode(&envelope, 1_024).expect("small history");
    assert!(encode(&envelope, encoded.len() - 1).is_err());
}
