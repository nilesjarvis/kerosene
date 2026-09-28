use super::*;

#[test]
fn amount_scale_makes_large_unlocks_more_prominent() {
    let small = HypeUnstakingEvent {
        unlock_time_ms: 1_000,
        user: "0xsmall".to_string(),
        amount_wei: 100 * HYPE_CORE_WEI_PER_TOKEN as u64,
    };
    let large = HypeUnstakingEvent {
        unlock_time_ms: 2_000,
        user: "0xlarge".to_string(),
        amount_wei: 10_000 * HYPE_CORE_WEI_PER_TOKEN as u64,
    };
    let scale = hype_unstaking_amount_scale(&[&small, &large]);
    let small_heat = scale.heat(small.amount_wei);
    let large_heat = scale.heat(large.amount_wei);

    assert!(large_heat.fill_pct > small_heat.fill_pct);
    assert!(large_heat.alpha > small_heat.alpha);
}
