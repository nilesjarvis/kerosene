use super::*;
use crate::api::{ExchangeSymbol, MarketType, OutcomeVolume24h};
use std::collections::BTreeMap;

fn symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: MarketType::Outcome,
        outcome: None,
    }
}

#[test]
fn outcome_group_volume_uses_largest_side_volume_to_avoid_double_counting_pairs() {
    let yes = symbol("#670");
    let no = symbol("#671");
    let sides = vec![&yes, &no];
    let volumes = HashMap::from([
        (
            "#670".to_string(),
            OutcomeVolume24h {
                contract: 18_055.0,
                notional: 4_500.0,
            },
        ),
        (
            "#671".to_string(),
            OutcomeVolume24h {
                contract: 18_054.0,
                notional: 4_499.0,
            },
        ),
    ]);

    assert_eq!(outcome_group_volume(&sides, &volumes), Some(18_055.0));
}

#[test]
fn outcome_group_volume_ignores_missing_and_invalid_values() {
    let yes = symbol("#670");
    let no = symbol("#671");
    let sides = vec![&yes, &no];
    let volumes = HashMap::from([(
        "#670".to_string(),
        OutcomeVolume24h {
            contract: f64::NAN,
            notional: 0.0,
        },
    )]);

    assert_eq!(outcome_group_volume(&sides, &volumes), None);
}

#[test]
fn outcome_market_set_volume_sums_distinct_outcome_pair_volume() {
    let below_yes = symbol("#1010");
    let below_no = symbol("#1011");
    let above_yes = symbol("#1030");
    let above_no = symbol("#1031");
    let outcomes = BTreeMap::from([
        (101, vec![&below_yes, &below_no]),
        (103, vec![&above_yes, &above_no]),
    ]);
    let volumes = HashMap::from([
        (
            "#1010".to_string(),
            OutcomeVolume24h {
                contract: 5.0,
                notional: 2.0,
            },
        ),
        (
            "#1011".to_string(),
            OutcomeVolume24h {
                contract: 4.0,
                notional: 2.0,
            },
        ),
        (
            "#1030".to_string(),
            OutcomeVolume24h {
                contract: 3.0,
                notional: 1.0,
            },
        ),
        (
            "#1031".to_string(),
            OutcomeVolume24h {
                contract: 8.0,
                notional: 4.0,
            },
        ),
    ]);

    assert_eq!(outcome_market_set_volume(&outcomes, &volumes), Some(13.0));
}

#[test]
fn outcome_contract_volume_formatter_uses_compact_contract_units() {
    assert_eq!(format_outcome_contract_volume(0.5), "0.50 contracts");
    assert_eq!(format_outcome_contract_volume(999.0), "999 contracts");
    assert_eq!(format_outcome_contract_volume(18_055.0), "18.1K contracts");
    assert_eq!(
        format_outcome_contract_volume(2_500_000.0),
        "2.5M contracts"
    );
}
