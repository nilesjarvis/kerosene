use super::*;

#[test]
fn quick_order_limit_preview_tracks_open_form_lifecycle() {
    let mut instance = instance();

    instance.set_quick_order(quick_order_form(true, "", false, 0.0));
    assert!(instance.chart.quick_order_open);
    assert_eq!(instance.chart.quick_order_limit_price, Some(100.0));

    instance.clear_quick_order();
    assert!(!instance.chart.quick_order_open);
    assert_eq!(instance.chart.quick_order_limit_price, None);
}

#[test]
fn quick_order_reopen_values_preserve_cleared_form_for_same_symbol() {
    let mut instance = instance();

    instance.set_quick_order(quick_order_form(false, "2.5", false, 25.0));
    instance.clear_quick_order();

    assert_eq!(
        instance.quick_order_reopen_values(true),
        ("2.5".to_string(), false, 25.0, false)
    );
}

#[test]
fn quick_order_reopen_values_drop_percentage_derived_size() {
    let mut instance = instance();
    let mut form = quick_order_form(false, "2.5", false, 25.0);
    form.quantity_provenance = Some(crate::order_execution::QuickOrderQuantityProvenance {
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        account_data_revision: 7,
        symbol_key: "BTC".to_string(),
        quantity_is_usd: false,
        percentage: 25.0,
        is_limit: false,
        reference_price: Some(100.0),
        reduce_only: false,
        market_universe: crate::config::MarketUniverseConfig::default(),
    });

    instance.set_quick_order(form);
    instance.clear_quick_order();

    assert_eq!(
        instance.quick_order_reopen_values(true),
        (String::new(), false, 0.0, false)
    );
}

#[test]
fn quick_order_reopen_values_drop_size_after_symbol_change() {
    let mut instance = instance();

    instance.set_quick_order(quick_order_form(false, "2.5", false, 25.0));
    instance.clear_quick_order();
    instance.set_symbol_identity("ETH".to_string(), "ETH".to_string());

    assert_eq!(
        instance.quick_order_reopen_values(true),
        (String::new(), true, 0.0, false)
    );
}

#[test]
fn account_reset_drops_quick_order_form_and_remembered_size() {
    let mut instance = instance();

    instance.set_quick_order(quick_order_form(true, "2.5", false, 25.0));

    instance.reset_quick_order_for_account_reset();

    assert!(instance.quick_order.is_none());
    assert!(!instance.chart.quick_order_open);
    assert_eq!(instance.chart.quick_order_limit_price, None);
    assert_eq!(instance.chart.quick_order_line_phase, 0.0);
    assert_eq!(instance.last_quick_order_symbol, "");
    assert_eq!(instance.last_quick_order_quantity, "");
    assert_eq!(instance.last_quick_order_percentage, 0.0);
    assert_eq!(
        instance.quick_order_reopen_values(true),
        (String::new(), true, 0.0, true)
    );
}

#[test]
fn quick_order_market_form_does_not_show_limit_preview() {
    let mut instance = instance();

    instance.set_quick_order(quick_order_form(false, "", false, 0.0));

    assert!(instance.chart.quick_order_open);
    assert_eq!(instance.chart.quick_order_limit_price, None);
}

#[test]
fn quick_order_limit_preview_phase_only_advances_while_visible() {
    let mut instance = instance();

    instance.advance_quick_order_limit_line();
    assert_eq!(instance.chart.quick_order_line_phase, 0.0);

    instance.set_quick_order(quick_order_form(true, "", false, 0.0));
    instance.advance_quick_order_limit_line();
    assert!(instance.chart.quick_order_line_phase > 0.0);

    instance.clear_quick_order();
    assert_eq!(instance.chart.quick_order_line_phase, 0.0);
}
