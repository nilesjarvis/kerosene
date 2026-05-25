use super::*;

#[test]
fn spot_balance_preserves_optional_token_index() {
    let balance = spot_balance_or_panic(serde_json::json!({
        "coin": "USDC",
        "token": 0,
        "total": "10",
        "hold": "2",
        "entryNtl": "0"
    }));

    assert_eq!(balance.token, Some(0));
}
