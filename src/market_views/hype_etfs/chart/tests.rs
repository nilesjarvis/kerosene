use super::*;

#[test]
fn latest_daily_flows_preserves_tail_order_and_handles_empty_limits() {
    let flows = ["2026-05-20", "2026-05-18", "2026-05-20", "2026-05-19"]
        .into_iter()
        .zip([100.0, -25.0, 0.0, 40.0])
        .map(|(date, amount_usd)| HypeEtfDailyFlow {
            date: date.to_string(),
            amount_usd,
        })
        .collect::<Vec<_>>();

    for (limit, expected) in [
        (0, &flows[4..]),
        (1, &flows[3..]),
        (2, &flows[2..]),
        (4, &flows[..]),
        (usize::MAX, &flows[..]),
    ] {
        assert_eq!(latest_daily_flows(&flows, limit), expected);
    }
    assert!(latest_daily_flows(&[], 10).is_empty());
}
