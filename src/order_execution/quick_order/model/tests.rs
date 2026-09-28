use super::{QuickOrderForm, QuickOrderQuantityProvenance, QuickOrderRecovery};
use crate::chart_state::ChartSurfaceId;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

#[test]
fn quick_order_quantity_provenance_debug_redacts_account_address() {
    let provenance = QuickOrderQuantityProvenance {
        account_address: TEST_ACCOUNT.to_string(),
        account_data_revision: 7,
        spot_balances_revision: 3,
        symbol_key: "SECRETCOIN".to_string(),
        quantity_is_usd: true,
        percentage: 42.42,
        is_limit: false,
        reference_price: Some(98765.4321),
        reduce_only: false,
        market_universe: crate::config::MarketUniverseConfig::default(),
    };

    let rendered = format!("{provenance:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains(TEST_ACCOUNT));
    assert!(!rendered.contains("SECRETCOIN"));
    assert!(!rendered.contains("42.42"));
    assert!(!rendered.contains("98765.4321"));
}

#[test]
fn quick_order_form_and_recovery_debug_redact_order_details() {
    let form = QuickOrderForm {
        price: 98765.4321,
        quantity: "quantity-secret".to_string(),
        quantity_is_usd: true,
        percentage: 42.42,
        quantity_provenance: Some(QuickOrderQuantityProvenance {
            account_address: TEST_ACCOUNT.to_string(),
            account_data_revision: 7,
            spot_balances_revision: 3,
            symbol_key: "SECRETCOIN".to_string(),
            quantity_is_usd: true,
            percentage: 42.42,
            is_limit: true,
            reference_price: Some(12345.6789),
            reduce_only: false,
            market_universe: crate::config::MarketUniverseConfig::default(),
        }),
        is_limit: true,
        click_x: 10.0,
        click_y: 20.0,
        chart_w: 400.0,
        chart_h: 300.0,
    };
    let recovery = QuickOrderRecovery {
        chart_id: 1,
        form,
        surface_id: Some(ChartSurfaceId::Docked(1)),
    };

    let rendered = format!("{recovery:?}");

    assert!(rendered.contains("price: <redacted>"));
    assert!(rendered.contains("quantity: <redacted>"));
    assert!(rendered.contains("quantity_provenance: Some(\"<redacted>\")"));
    assert!(rendered.contains("chart_id: 1"));
    for secret in [
        TEST_ACCOUNT,
        "SECRETCOIN",
        "quantity-secret",
        "98765.4321",
        "12345.6789",
        "42.42",
    ] {
        assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
    }
}
