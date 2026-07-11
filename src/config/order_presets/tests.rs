use super::{OrderPreset, OrderPresetsConfig};

const LABEL: &str = "private-preset-label-sentinel";
const SIZE: f64 = 98_765.432_1;
const OFFSET: f64 = 12.345_678_9;

fn private_preset() -> OrderPreset {
    OrderPreset {
        label: LABEL.to_string(),
        size: SIZE,
        price_offset_pct: Some(OFFSET),
    }
}

#[test]
fn default_limit_presets_keep_price_offsets_for_one_click_safety() {
    let presets = OrderPresetsConfig::default();

    assert_eq!(
        presets
            .limit_usd
            .iter()
            .map(|preset| (preset.label.as_str(), preset.size, preset.price_offset_pct))
            .collect::<Vec<_>>(),
        vec![
            ("-1%", 500.0, Some(1.0)),
            ("-2%", 1000.0, Some(2.0)),
            ("-5%", 2000.0, Some(5.0)),
        ]
    );
    assert_eq!(
        presets
            .limit_coin
            .iter()
            .map(|preset| (preset.label.as_str(), preset.size, preset.price_offset_pct))
            .collect::<Vec<_>>(),
        vec![
            ("-1%", 1.0, Some(1.0)),
            ("-2%", 2.0, Some(2.0)),
            ("-5%", 5.0, Some(5.0)),
        ]
    );
}

#[test]
fn order_preset_debug_redacts_values_without_changing_serde() {
    let preset = private_preset();
    let wire_before = serde_json::to_value(&preset).expect("serialize preset");

    let rendered = format!("{preset:?}");
    let wire_after = serde_json::to_value(&preset).expect("serialize preset after formatting");
    let restored: OrderPreset = serde_json::from_value(wire_after.clone()).expect("restore preset");

    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(!rendered.contains(LABEL), "{rendered}");
    assert!(!rendered.contains(&format!("{SIZE:?}")), "{rendered}");
    assert!(!rendered.contains(&format!("{OFFSET:?}")), "{rendered}");
    assert!(rendered.contains("price_offset_pct: Some"), "{rendered}");
    assert_eq!(wire_after, wire_before);
    assert_eq!(restored.label, LABEL);
    assert_eq!(restored.size.to_bits(), SIZE.to_bits());
    assert_eq!(
        restored.price_offset_pct.map(f64::to_bits),
        Some(OFFSET.to_bits())
    );
}

#[test]
fn order_presets_config_debug_reports_only_category_counts() {
    let mut presets = OrderPresetsConfig::default();
    presets.market_usd = vec![private_preset()];
    let wire_before = serde_json::to_value(&presets).expect("serialize presets");

    let rendered = format!("{presets:?}");
    let wire_after = serde_json::to_value(&presets).expect("serialize presets after formatting");
    let restored: OrderPresetsConfig =
        serde_json::from_value(wire_after.clone()).expect("restore presets");

    assert!(rendered.contains("market_usd_len: 1"), "{rendered}");
    assert!(rendered.contains("limit_usd_len: 3"), "{rendered}");
    assert!(rendered.contains("chase_usd_len: 3"), "{rendered}");
    assert!(rendered.contains("market_coin_len: 4"), "{rendered}");
    assert!(rendered.contains("limit_coin_len: 3"), "{rendered}");
    assert!(rendered.contains("chase_coin_len: 3"), "{rendered}");
    assert!(!rendered.contains(LABEL), "{rendered}");
    assert!(!rendered.contains(&format!("{SIZE:?}")), "{rendered}");
    assert!(!rendered.contains(&format!("{OFFSET:?}")), "{rendered}");
    assert_eq!(wire_after, wire_before);
    assert_eq!(restored, presets);
}
