use super::{SpotBalance, UserFill};
use crate::helpers::parse_finite_number;

// ---------------------------------------------------------------------------
// Spot Cost Basis From Fills
// ---------------------------------------------------------------------------

const COST_BASIS_EPSILON: f64 = 1e-12;
const BALANCE_MATCH_ABS_TOLERANCE: f64 = 1e-8;
const BALANCE_MATCH_REL_TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SpotCostBasis {
    pub(crate) entry_notional: f64,
}

/// Reconstruct a spot entry notional when Hyperliquid omits `entryNtl`.
///
/// This only returns a basis when the relevant spot fills reconcile to the live
/// balance. That keeps deposited/transferred balances from getting a misleading
/// exchange-fill basis.
pub(crate) fn derive_spot_cost_basis_from_fills(
    balance: &SpotBalance,
    trade_coin: &str,
    fills: &[UserFill],
) -> Option<SpotCostBasis> {
    let live_size = parse_finite_number(&balance.total)?.abs();
    if live_size <= COST_BASIS_EPSILON {
        return None;
    }

    let mut relevant: Vec<&UserFill> = fills
        .iter()
        .filter(|fill| fill.coin == trade_coin)
        .collect();
    if relevant.is_empty() {
        return None;
    }
    relevant.sort_by_key(|fill| (fill.time, fill.tid.unwrap_or_default()));

    let mut qty = 0.0;
    let mut cost = 0.0;
    let mut saw_fill = false;

    for fill in relevant {
        let px = parse_finite_number(&fill.px).filter(|px| *px > 0.0)?;
        let sz = parse_finite_number(&fill.sz).filter(|sz| *sz > 0.0)?;
        let fee = parse_finite_number(&fill.fee).unwrap_or(0.0).max(0.0);
        let fee_amounts =
            fill_fee_amounts(fill.fee_token.as_deref(), &balance.coin, trade_coin, fee)?;
        saw_fill = true;

        match fill.side.as_str() {
            "B" => {
                qty += (sz - fee_amounts.base).max(0.0);
                cost += px * sz + fee_amounts.quote;
            }
            "A" => {
                let sold = sz + fee_amounts.base;
                if qty <= COST_BASIS_EPSILON || sold > qty + balance_match_tolerance(qty) {
                    return None;
                }
                let ratio = (sold / qty).clamp(0.0, 1.0);
                cost -= cost * ratio;
                qty -= sold;
                if qty.abs() <= COST_BASIS_EPSILON {
                    qty = 0.0;
                    cost = 0.0;
                }
            }
            _ => return None,
        }
    }

    if !saw_fill || !balances_match(qty, live_size) || cost <= COST_BASIS_EPSILON {
        return None;
    }

    Some(SpotCostBasis {
        entry_notional: cost.abs(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FillFeeAmounts {
    base: f64,
    quote: f64,
}

fn fill_fee_amounts(
    fee_token: Option<&str>,
    balance_coin: &str,
    trade_coin: &str,
    fee: f64,
) -> Option<FillFeeAmounts> {
    if fee <= COST_BASIS_EPSILON {
        return Some(FillFeeAmounts {
            base: 0.0,
            quote: 0.0,
        });
    }

    let fee_token = fee_token.map(str::trim).unwrap_or_default();
    if fee_token.eq_ignore_ascii_case(balance_coin) || fee_token.eq_ignore_ascii_case(trade_coin) {
        return Some(FillFeeAmounts {
            base: fee,
            quote: 0.0,
        });
    }
    if is_quote_fee_token(fee_token) {
        return Some(FillFeeAmounts {
            base: 0.0,
            quote: fee,
        });
    }

    None
}

fn is_quote_fee_token(fee_token: &str) -> bool {
    ["USDC", "USDE", "USDT0", "USDH"]
        .iter()
        .any(|stable| fee_token.eq_ignore_ascii_case(stable))
}

fn balances_match(derived: f64, live: f64) -> bool {
    (derived - live).abs() <= balance_match_tolerance(live)
}

fn balance_match_tolerance(value: f64) -> f64 {
    BALANCE_MATCH_ABS_TOLERANCE.max(value.abs() * BALANCE_MATCH_REL_TOLERANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balance(total: &str) -> SpotBalance {
        SpotBalance {
            coin: "UBTC".to_string(),
            token: Some(197),
            total: total.to_string(),
            hold: "0".to_string(),
            entry_ntl: "0".to_string(),
            supplied: None,
        }
    }

    fn fill(side: &str, px: &str, sz: &str, fee: &str, fee_token: &str, time: u64) -> UserFill {
        UserFill {
            coin: "@142".to_string(),
            px: px.to_string(),
            sz: sz.to_string(),
            side: side.to_string(),
            time,
            hash: None,
            tid: Some(time),
            oid: None,
            dir: if side == "B" { "Buy" } else { "Sell" }.to_string(),
            closed_pnl: "0".to_string(),
            fee: fee.to_string(),
            fee_token: Some(fee_token.to_string()),
        }
    }

    #[test]
    fn derives_basis_when_base_token_fees_reconcile_to_live_balance() {
        let fills = vec![
            fill("B", "60191", "1.0", "0.0004", "UBTC", 1),
            fill("B", "58395", "5.753", "0.0034270968", "UBTC", 2),
        ];

        let basis = derive_spot_cost_basis_from_fills(&balance("6.7491729032"), "@142", &fills)
            .expect("basis");

        assert!((basis.entry_notional - (60_191.0 + 58_395.0 * 5.753)).abs() < 1e-9);
    }

    #[test]
    fn rejects_fill_basis_when_net_size_does_not_match_balance() {
        let fills = vec![fill("B", "60191", "1.0", "0.0004", "UBTC", 1)];

        assert!(derive_spot_cost_basis_from_fills(&balance("2.0"), "@142", &fills).is_none());
    }

    #[test]
    fn reduces_remaining_basis_for_spot_sells() {
        let fills = vec![
            fill("B", "100", "10", "0", "USDC", 1),
            fill("A", "120", "4", "0", "USDC", 2),
        ];

        let basis =
            derive_spot_cost_basis_from_fills(&balance("6"), "@142", &fills).expect("basis");

        assert!((basis.entry_notional - 600.0).abs() < 1e-9);
    }

    #[test]
    fn trims_quote_fee_token_and_includes_quote_fees() {
        let fills = vec![fill("B", "100", "1", "1", " USDC ", 1)];

        let basis =
            derive_spot_cost_basis_from_fills(&balance("1"), "@142", &fills).expect("basis");

        assert!((basis.entry_notional - 101.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_missing_nonzero_fee_token() {
        let mut fills = vec![fill("B", "100", "1", "1", "USDC", 1)];
        fills[0].fee_token = None;

        assert!(derive_spot_cost_basis_from_fills(&balance("1"), "@142", &fills).is_none());
    }

    #[test]
    fn rejects_unknown_nonzero_fee_token() {
        let fills = vec![fill("B", "100", "1", "1", "UNKNOWN", 1)];

        assert!(derive_spot_cost_basis_from_fills(&balance("1"), "@142", &fills).is_none());
    }

    #[test]
    fn rejects_sells_that_require_missing_prior_inventory() {
        let fills = vec![fill("A", "120", "1", "0", "USDC", 1)];

        assert!(derive_spot_cost_basis_from_fills(&balance("1"), "@142", &fills).is_none());
    }
}
