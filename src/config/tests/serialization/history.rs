use super::{default_config_value, json_string, remove_field, value_from_json, value_from_str};
use crate::advanced_order_history::{AdvancedOrderHistoryEntry, AdvancedOrderHistoryKind};
use crate::config::KeroseneConfig;

#[test]
fn advanced_order_history_round_trips_and_legacy_defaults_empty() {
    let config = KeroseneConfig {
        advanced_order_history: vec![AdvancedOrderHistoryEntry {
            id: "twap:acct:1000:1".to_string(),
            kind: AdvancedOrderHistoryKind::Twap,
            source_id: 1,
            account_address: "0xabc".to_string(),
            coin: "BTC".to_string(),
            display_coin: "BTC".to_string(),
            is_buy: true,
            target_size: 1.0,
            filled_size: 1.0,
            remaining_size: 0.0,
            average_price: Some(100.0),
            last_working_price: Some(100.0),
            gross_notional: 100.0,
            total_fee: 0.05,
            closed_pnl: 0.0,
            min_price: Some(99.0),
            max_price: Some(101.0),
            reduce_only: false,
            randomize: true,
            slice_count: 2,
            slices_sent: 2,
            reprice_count: 0,
            status: "Completed".to_string(),
            summary: "TWAP completed".to_string(),
            started_at_ms: 1_000,
            completed_at_ms: 2_000,
            logs: Vec::new(),
            children: Vec::new(),
        }],
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(decoded.advanced_order_history.len(), 1);
    assert_eq!(decoded.advanced_order_history[0].status, "Completed");
    assert_eq!(
        decoded.advanced_order_history[0].last_working_price,
        Some(100.0)
    );
    assert_eq!(decoded.advanced_order_history[0].gross_notional, 100.0);

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "advanced_order_history",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert!(decoded_legacy.advanced_order_history.is_empty());
}
