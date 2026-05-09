use super::parse_reserve_states;

#[test]
fn reserve_states_parse_pair_arrays_and_number_fields() {
    let raw = serde_json::json!([
        [
            0,
            {
                "borrowYearlyRate": 0.12,
                "supplyYearlyRate": "0.03",
                "oraclePx": 1
            }
        ]
    ]);

    let reserves = parse_reserve_states(&raw);
    let reserve = reserves.get(&0).expect("reserve state");
    assert_eq!(reserve.borrow_yearly_rate, "0.12");
    assert_eq!(reserve.supply_yearly_rate, "0.03");
    assert_eq!(reserve.oracle_px, "1");
}

#[test]
fn reserve_states_parse_object_map_shape() {
    let raw = serde_json::json!({
        "1": {
            "borrowYearlyRate": "0.5",
            "supplyYearlyRate": "0.2",
            "oraclePx": "100"
        }
    });

    let reserves = parse_reserve_states(&raw);
    assert_eq!(
        reserves.get(&1).map(|reserve| reserve.oracle_px.as_str()),
        Some("100")
    );
}
