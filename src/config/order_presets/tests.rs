use super::OrderPresetsConfig;

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
