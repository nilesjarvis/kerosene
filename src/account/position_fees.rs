use super::{Position, UserFill};
use crate::helpers::{is_usd_stable_fee_token, parse_finite_number};
use std::collections::HashSet;

const FLAT_EPSILON: f64 = 1e-12;

/// Net execution fees in USD for the current position's lifetime, including
/// increases and partial closes. A reversal contributes only its opening share.
/// Return no total unless fills form a continuous chain from flat (or a flip)
/// to the live position. Recent history, transfers and independent websocket
/// lanes can otherwise make a plausible-looking total incomplete or stale.
pub(crate) fn derive_position_spent_fees(
    position: &Position,
    fills: &[UserFill],
    base_token: Option<&str>,
) -> Option<f64> {
    let mut remaining = parse_finite_number(&position.szi)?;
    if remaining.abs() <= FLAT_EPSILON {
        return None;
    }
    let mut seen = HashSet::new();
    let mut relevant: Vec<_> = fills
        .iter()
        .filter(|fill| fill.coin == position.coin && seen.insert(fill.dedup_key()))
        .collect();
    relevant.sort_by_key(|fill| std::cmp::Reverse(fill.time));
    let mut total = 0.0;
    let mut cursor = 0;
    let mut steps = Vec::new();

    while let Some(latest) = relevant.get(cursor) {
        let time = latest.time;
        let count = relevant[cursor..]
            .iter()
            .take_while(|fill| fill.time == time)
            .count();
        for fill in &relevant[cursor..cursor + count] {
            steps.push(fee_step(fill, base_token)?);
        }

        // Trade IDs do not always follow position order within one timestamp.
        // Walk the reported position chain instead, rejecting ambiguous chains.
        while !steps.is_empty() {
            let mut matching = steps
                .iter()
                .enumerate()
                .filter(|(_, step)| sizes_match(step.end, remaining));
            let (index, _) = matching.next()?;
            if matching.next().is_some() {
                return None;
            }
            let step = steps.swap_remove(index);
            let is_flip = step.start.abs() > FLAT_EPSILON
                && step.start.is_sign_positive() != step.end.is_sign_positive();
            total += if is_flip {
                step.fee_usd * step.end.abs() / (step.end - step.start).abs()
            } else {
                step.fee_usd
            };
            if !total.is_finite() {
                return None;
            }
            if is_flip || step.start.abs() <= FLAT_EPSILON {
                return Some(total);
            }
            remaining = step.start;
        }
        cursor += count;
    }
    None
}

struct FeeStep {
    start: f64,
    end: f64,
    fee_usd: f64,
}

fn fee_step(fill: &UserFill, base_token: Option<&str>) -> Option<FeeStep> {
    let start = parse_finite_number(fill.start_position.as_deref()?)?;
    let fee = parse_finite_number(&fill.fee)?;
    let token = fill.fee_token.as_deref().unwrap_or_default().trim();
    let base_fee =
        !token.is_empty() && base_token.is_some_and(|base| token.eq_ignore_ascii_case(base));
    let fee_usd = if base_fee {
        fee * parse_finite_number(&fill.px).filter(|px| *px > 0.0)?
    } else if token.is_empty() || is_usd_stable_fee_token(token) || fee == 0.0 {
        fee
    } else {
        return None;
    };
    let end = if fill.dir == "Settlement" {
        start
    } else {
        let size = parse_finite_number(&fill.sz).filter(|size| *size > 0.0)?;
        let signed_size = match fill.side.as_str() {
            "B" => size,
            "A" => -size,
            _ => return None,
        };
        // Spot base-token fees reduce inventory; negative fees are rebates.
        start + signed_size - if base_fee { fee } else { 0.0 }
    };
    (end.is_finite() && fee_usd.is_finite()).then_some(FeeStep {
        start,
        end,
        fee_usd,
    })
}

fn sizes_match(left: f64, right: f64) -> bool {
    let tolerance = FLAT_EPSILON.max(left.abs().max(right.abs()) * f64::EPSILON * 32.0);
    (left - right).abs() <= tolerance
}

#[cfg(test)]
mod tests;
