use super::*;
use crate::account::SpotBalance;

fn balance(total: &str, hold: &str) -> SpotBalance {
    SpotBalance {
        coin: "+650".to_string(),
        token: None,
        total: total.to_string(),
        hold: hold.to_string(),
        entry_ntl: "0".to_string(),
        supplied: None,
    }
}

#[test]
fn available_outcome_contracts_floor_available_balance() {
    assert_eq!(
        outcome_available_contracts(&balance("10.9", "0.2")),
        Some(10.0)
    );
    assert_eq!(outcome_available_contracts(&balance("1.9", "1.0")), None);
    assert_eq!(outcome_available_contracts(&balance("bad", "0")), None);
    assert_eq!(outcome_available_contracts(&balance("inf", "0")), None);
}
