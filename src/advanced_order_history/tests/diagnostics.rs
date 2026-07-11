use super::*;

#[test]
fn advanced_order_history_debug_redacts_exact_persisted_values_without_changing_serde() {
    const ID: &str = "history-entry-id-sentinel";
    const ADDRESS: &str = "history-account-sentinel";
    const COIN: &str = "history-coin-sentinel";
    const DISPLAY: &str = "history-display-sentinel";
    const STATUS: &str = "history-status-sentinel";
    const SUMMARY: &str = "history-summary-sentinel";
    const LOG_KIND: &str = "history-log-kind-sentinel";
    const LOG_MESSAGE: &str = "history-log-message-sentinel";
    const CHILD_STATUS: &str = "history-child-status-sentinel";
    const CHILD_SUMMARY: &str = "history-child-summary-sentinel";
    const CLOID: &str = "history-child-cloid-sentinel";
    const OID: u64 = 9_876_543_210_123_457;

    let log = AdvancedOrderHistoryLog {
        elapsed_ms: 1_601_601_601_603,
        kind: LOG_KIND.to_string(),
        message: LOG_MESSAGE.to_string(),
        is_error: true,
    };
    let child = AdvancedOrderHistoryChild {
        index: 19,
        elapsed_ms: 1_701_701_701_704,
        planned_size: 18_018.123_418,
        limit_price: 19_019.234_519,
        filled_size: 20_020.345_620,
        avg_price: Some(21_021.456_721),
        fee: 22_022.567_822,
        oid: Some(OID),
        cloid: Some(CLOID.to_string()),
        status: CHILD_STATUS.to_string(),
        exchange_summary: CHILD_SUMMARY.to_string(),
    };
    let mut entry = minimal_entry(ID);
    entry.kind = AdvancedOrderHistoryKind::Chase;
    entry.source_id = 17;
    entry.account_address = ADDRESS.to_string();
    entry.coin = COIN.to_string();
    entry.display_coin = DISPLAY.to_string();
    entry.target_size = 71_001.123_451;
    entry.filled_size = 62_002.234_562;
    entry.remaining_size = 53_003.345_673;
    entry.average_price = Some(44_004.456_784);
    entry.last_working_price = Some(35_005.567_895);
    entry.gross_notional = 26_006.678_906;
    entry.total_fee = 17_007.789_017;
    entry.closed_pnl = -8_008.890_128;
    entry.min_price = Some(9_009.901_239);
    entry.max_price = Some(10_010.012_340);
    entry.reduce_only = true;
    entry.randomize = true;
    entry.slice_count = 11_011;
    entry.slices_sent = 12_012;
    entry.reprice_count = 13_013;
    entry.status = STATUS.to_string();
    entry.summary = SUMMARY.to_string();
    entry.started_at_ms = 1_401_401_401_401;
    entry.completed_at_ms = 1_501_501_501_502;
    entry.logs = vec![log.clone()];
    entry.children = vec![child.clone()];
    let metrics = ChaseHistoryFillMetrics {
        filled_size: 23_023.678_923,
        gross_notional: 24_024.789_024,
        total_fee: 25_025.890_125,
        closed_pnl: -26_026.901_226,
    };

    let serialized = serde_json::to_value(&entry).expect("history entry should serialize");
    assert_eq!(serialized["id"].as_str(), Some(ID));
    assert_eq!(serialized["account_address"].as_str(), Some(ADDRESS));
    assert_eq!(serialized["target_size"].as_f64(), Some(entry.target_size));
    assert_eq!(serialized["logs"][0]["message"].as_str(), Some(LOG_MESSAGE));
    assert_eq!(serialized["children"][0]["oid"].as_u64(), Some(OID));
    assert_eq!(serialized["children"][0]["cloid"].as_str(), Some(CLOID));

    let entry_debug = format!("{entry:?}");
    let log_debug = format!("{log:?}");
    let child_debug = format!("{child:?}");
    let metrics_debug = format!("{metrics:?}");

    assert!(entry_debug.contains("kind: Chase"), "{entry_debug}");
    assert!(entry_debug.contains("source_id: 17"), "{entry_debug}");
    assert!(entry_debug.contains("logs_count: 1"), "{entry_debug}");
    assert!(entry_debug.contains("children_count: 1"), "{entry_debug}");
    assert!(log_debug.contains("is_error: true"), "{log_debug}");
    assert!(child_debug.contains("index: 19"), "{child_debug}");
    assert!(child_debug.contains("has_oid: true"), "{child_debug}");
    assert!(child_debug.contains("has_cloid: true"), "{child_debug}");

    let exact_strings = [
        ID,
        ADDRESS,
        COIN,
        DISPLAY,
        STATUS,
        SUMMARY,
        LOG_KIND,
        LOG_MESSAGE,
        CHILD_STATUS,
        CHILD_SUMMARY,
        CLOID,
    ];
    let exact_numbers = [
        entry.target_size.to_string(),
        entry.filled_size.to_string(),
        entry.remaining_size.to_string(),
        entry.average_price.expect("average price").to_string(),
        entry.last_working_price.expect("last price").to_string(),
        entry.gross_notional.to_string(),
        entry.total_fee.to_string(),
        entry.closed_pnl.to_string(),
        entry.min_price.expect("min price").to_string(),
        entry.max_price.expect("max price").to_string(),
        entry.slice_count.to_string(),
        entry.slices_sent.to_string(),
        entry.reprice_count.to_string(),
        entry.started_at_ms.to_string(),
        entry.completed_at_ms.to_string(),
        log.elapsed_ms.to_string(),
        child.elapsed_ms.to_string(),
        child.planned_size.to_string(),
        child.limit_price.to_string(),
        child.filled_size.to_string(),
        child.avg_price.expect("child average price").to_string(),
        child.fee.to_string(),
        OID.to_string(),
        metrics.filled_size.to_string(),
        metrics.gross_notional.to_string(),
        metrics.total_fee.to_string(),
        metrics.closed_pnl.to_string(),
    ];

    for debug in [&entry_debug, &log_debug, &child_debug, &metrics_debug] {
        assert!(debug.contains("<redacted>"), "{debug}");
        for exact in exact_strings {
            assert!(!debug.contains(exact), "{exact} leaked: {debug}");
        }
        for exact in &exact_numbers {
            assert!(!debug.contains(exact), "{exact} leaked: {debug}");
        }
    }

    assert_eq!(
        serde_json::to_value(&entry).expect("history entry should still serialize"),
        serialized
    );
}
