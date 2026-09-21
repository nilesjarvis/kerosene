use super::*;
use crate::config::listings::{ListingEvent, ListingKind};

#[test]
fn listings_history_round_trip_atomic_replacement_and_invalid_files() {
    let directory = std::env::temp_dir().join(format!(
        "kerosene-listings-test-{}-{}",
        std::process::id(),
        crate::app_time::now_ms()
    ));
    let path = directory.join("listings.json");
    assert_eq!(
        load_from(&path).expect("missing is empty"),
        ListingsHistory::default()
    );
    let mut history = ListingsHistory::default();
    history.perps.initialized = true;
    history.perps.seen.insert("perp:BTC".into(), true);
    history.events.push(ListingEvent {
        id: "perp:NEW".into(),
        key: "NEW".into(),
        label: "NEW".into(),
        kind: ListingKind::Perp,
        detected_at_ms: 100,
    });
    save_to(&path, history.clone()).expect("save history");
    assert_eq!(load_from(&path).expect("read history"), history);
    history.spot.initialized = true;
    save_to(&path, history.clone()).expect("replace history");
    assert_eq!(load_from(&path).expect("read replacement"), history);
    std::fs::write(&path, b"{broken").expect("write corruption");
    assert!(load_from(&path).is_err());
    std::fs::write(
        &path,
        serde_json::to_vec(&Envelope {
            version: 2,
            history,
        })
        .expect("encode future schema"),
    )
    .expect("write future schema");
    assert!(load_from(&path).is_err());
    std::fs::remove_dir_all(directory).expect("clean test directory");
}

#[test]
fn listings_tests_do_not_load_real_user_history() {
    assert_eq!(load().expect("test load"), ListingsHistory::default());
}
