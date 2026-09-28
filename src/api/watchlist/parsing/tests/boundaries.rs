use super::*;

fn context(volume: f64) -> WatchlistContext {
    WatchlistContext {
        funding: None,
        prev_day_px: None,
        mark_px: None,
        day_vlm: Some(volume),
        open_interest_notional: None,
    }
}

fn snapshot(map: &HashMap<String, WatchlistContext>) -> Value {
    serde_json::to_value(map).expect("synthetic context map")
}

#[test]
fn context_parsers_preserve_envelope_error_priority_and_existing_data() {
    let requested = HashSet::from(["BTC".to_string()]);
    for (raw, expected) in [
        (serde_json::json!(null), "expected [meta, contexts] array"),
        (serde_json::json!({}), "expected [meta, contexts] array"),
        (
            serde_json::json!([]),
            "expected [meta, contexts] array with two entries",
        ),
        (
            serde_json::json!([null]),
            "expected [meta, contexts] array with two entries",
        ),
        (
            serde_json::json!([null, null, null]),
            "expected [meta, contexts] array with two entries",
        ),
        (serde_json::json!([null, null]), "expected meta object"),
        (serde_json::json!([[], []]), "expected meta object"),
        (serde_json::json!([{}, null]), "expected contexts array"),
        (
            serde_json::json!([{"universe": []}, {}]),
            "expected contexts array",
        ),
        (serde_json::json!([{}, []]), "expected meta.universe array"),
        (
            serde_json::json!([{"universe": null}, []]),
            "expected meta.universe array",
        ),
        (
            serde_json::json!([{"universe": {}}, []]),
            "expected meta.universe array",
        ),
    ] {
        for (spot, scoped) in [(false, false), (false, true), (true, false), (true, true)] {
            let mut map = HashMap::from([("existing".to_string(), context(99.0))]);
            let before = snapshot(&map);
            let result = match (spot, scoped) {
                (false, false) => append_perp_contexts(raw.clone(), None, &mut map),
                (false, true) => {
                    append_perp_contexts_for_symbols(raw.clone(), None, &requested, &mut map)
                }
                (true, false) => append_spot_contexts(raw.clone(), &mut map),
                (true, true) => append_spot_contexts_for_symbols(raw.clone(), &requested, &mut map),
            };

            assert_eq!(
                result,
                Err(expected.to_string()),
                "spot={spot}, scoped={scoped}"
            );
            assert_eq!(snapshot(&map), before);
        }
    }
}

#[test]
fn perp_context_aliases_preserve_overwrite_priority_and_literal_names() {
    let raw = serde_json::json!([
        {"universe": [
            {"name": "BTC"}, {"name": "BTC"}, {"name": "xyz:BTC"},
            {"name": ""}, {"name": "é"}, {"name": "nested:ETH"}
        ]},
        [
            {"dayNtlVlm": 1}, {"dayNtlVlm": 2}, {"dayNtlVlm": 3},
            {"dayNtlVlm": 4}, {"dayNtlVlm": 5}, {"dayNtlVlm": 6}
        ]
    ]);
    let initial = HashMap::from([
        ("BTC".to_string(), context(99.0)),
        ("xyz:BTC".to_string(), context(98.0)),
        ("existing".to_string(), context(97.0)),
    ]);
    for (dex, scoped, expected_entries) in [
        (
            None,
            false,
            vec![
                ("BTC", 2.0),
                ("xyz:BTC", 3.0),
                ("", 4.0),
                ("é", 5.0),
                ("nested:ETH", 6.0),
            ],
        ),
        (
            Some("xyz"),
            false,
            vec![
                ("BTC", 99.0),
                ("xyz:BTC", 3.0),
                ("xyz:", 4.0),
                ("", 4.0),
                ("xyz:é", 5.0),
                ("é", 5.0),
                ("nested:ETH", 6.0),
            ],
        ),
        (
            Some("xyz"),
            true,
            vec![
                ("BTC", 99.0),
                ("xyz:BTC", 3.0),
                ("xyz:", 4.0),
                ("", 4.0),
                ("xyz:é", 5.0),
                ("é", 5.0),
                ("nested:ETH", 6.0),
            ],
        ),
        (
            Some(""),
            false,
            vec![
                (":BTC", 2.0),
                ("BTC", 99.0),
                ("xyz:BTC", 3.0),
                (":", 4.0),
                ("", 4.0),
                (":é", 5.0),
                ("é", 5.0),
                ("nested:ETH", 6.0),
            ],
        ),
    ] {
        let mut map = initial.clone();
        let requested = ["BTC", "xyz:BTC", "", "é", "nested:ETH"]
            .map(str::to_string)
            .into_iter()
            .collect();
        let result = if scoped {
            append_perp_contexts_for_symbols(raw.clone(), dex, &requested, &mut map)
        } else {
            append_perp_contexts(raw.clone(), dex, &mut map)
        };
        let mut expected = HashMap::from([("existing".to_string(), context(97.0))]);
        expected.extend(
            expected_entries
                .into_iter()
                .map(|(key, volume)| (key.to_string(), context(volume))),
        );

        assert_eq!(result, Ok(6));
        assert_eq!(
            snapshot(&map),
            snapshot(&expected),
            "dex={dex:?}, scoped={scoped}"
        );
    }

    let mut map = initial.clone();
    let requested = HashSet::from(["xyz:BTC".to_string()]);
    assert_eq!(
        append_perp_contexts_for_symbols(raw, Some("xyz"), &requested, &mut map),
        Ok(3)
    );
    let mut expected = initial;
    expected.insert("xyz:BTC".to_string(), context(3.0));
    assert_eq!(snapshot(&map), snapshot(&expected));
}

#[test]
fn perp_contexts_preserve_atomic_failure_and_scoped_missing_rows() {
    let raw = serde_json::json!([
        {"universe": [{"name": "BTC"}, {"name": "BAD"}, {"name": "ETH"}]},
        [{"dayNtlVlm": 1}, null, {"dayNtlVlm": 3}]
    ]);
    let initial = HashMap::from([("existing".to_string(), context(99.0))]);
    let mut strict = initial.clone();
    assert_eq!(
        append_perp_contexts(raw.clone(), Some("xyz"), &mut strict),
        Err("expected context object for BAD".to_string())
    );
    assert_eq!(snapshot(&strict), snapshot(&initial));

    let mut scoped = initial.clone();
    let requested = ["xyz:BTC", "xyz:BAD", "ETH"]
        .map(str::to_string)
        .into_iter()
        .collect();
    assert_eq!(
        append_perp_contexts_for_symbols(raw, Some("xyz"), &requested, &mut scoped),
        Ok(2)
    );
    let mut expected = initial;
    for (key, volume) in [
        ("xyz:BTC", 1.0),
        ("BTC", 1.0),
        ("xyz:ETH", 3.0),
        ("ETH", 3.0),
    ] {
        expected.insert(key.to_string(), context(volume));
    }
    assert_eq!(snapshot(&scoped), snapshot(&expected));
}
