use crate::account::UserFill;
use crate::helpers::positive_finite_value;
use crate::signing::ChaseOrder;

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Recent Fills
// ---------------------------------------------------------------------------

pub(super) fn prepend_recent_fills(
    existing: &mut Vec<UserFill>,
    incoming: Vec<UserFill>,
    max_len: usize,
) {
    if max_len == 0 {
        existing.clear();
        return;
    }

    let mut updated =
        Vec::with_capacity(max_len.min(existing.len().saturating_add(incoming.len())));
    updated.extend(incoming.into_iter().take(max_len));
    let remaining = max_len.saturating_sub(updated.len());
    updated.extend(existing.drain(..).take(remaining));
    *existing = updated;
}

pub(super) fn apply_fills_update<F>(
    existing: &mut Vec<UserFill>,
    fills: Vec<UserFill>,
    is_snapshot: bool,
    is_muted: F,
) -> Vec<String>
where
    F: Fn(&str) -> bool,
{
    let fills: Vec<_> = fills
        .into_iter()
        .filter(|fill| !is_muted(&fill.coin))
        .collect();
    if is_snapshot {
        *existing = fills;
        Vec::new()
    } else {
        let toast_msgs: Vec<String> = fills
            .iter()
            .map(|fill| {
                let side = if fill.side == "B" { "BUY" } else { "SELL" };
                format!("Filled {side} {} {} @ ${}", fill.sz, fill.coin, fill.px)
            })
            .collect();
        prepend_recent_fills(existing, fills, 200);
        toast_msgs
    }
}

// ---------------------------------------------------------------------------
// Chase Fill Summaries
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct ChaseFillTotals {
    pub(super) side: String,
    pub(super) coin: String,
    pub(super) filled_size: f64,
    pub(super) total_notional: f64,
}

pub(super) fn chase_fill_totals(fills: &[UserFill], oids: &[u64]) -> Option<ChaseFillTotals> {
    if oids.is_empty() {
        return None;
    }

    let mut seen = HashSet::new();
    let mut side = None;
    let mut coin = None;
    let mut filled_size = 0.0;
    let mut total_notional = 0.0;
    let mut matched = false;
    for fill in fills {
        let Some(oid) = fill.oid else {
            continue;
        };
        if !oids.contains(&oid) {
            continue;
        }
        let fill_key = (
            oid,
            fill.time,
            fill.px.as_str(),
            fill.sz.as_str(),
            fill.side.as_str(),
            fill.dir.as_str(),
            fill.closed_pnl.as_str(),
            fill.fee.as_str(),
        );
        if !seen.insert(fill_key) {
            continue;
        }
        matched = true;
        side.get_or_insert_with(|| {
            if fill.side == "B" {
                "BUY".to_string()
            } else {
                "SELL".to_string()
            }
        });
        coin.get_or_insert_with(|| fill.coin.clone());
        let Some((size, price)) = positive_fill_size_and_price(fill) else {
            continue;
        };
        filled_size += size;
        total_notional += size * price;
    }

    if !matched {
        return None;
    }

    Some(ChaseFillTotals {
        side: side.unwrap_or_else(|| "BUY".to_string()),
        coin: coin.unwrap_or_else(|| "?".to_string()),
        filled_size,
        total_notional,
    })
}

pub(super) fn chase_fill_summary_for_oids(fills: &[UserFill], oids: &[u64]) -> Option<String> {
    let totals = chase_fill_totals(fills, oids)?;

    if totals.filled_size > 0.0 {
        let avg_px = totals.total_notional / totals.filled_size;
        Some(format!(
            "Chase filled: {} {} {} @ ${}",
            totals.side,
            format_chase_fill_number(totals.filled_size),
            totals.coin,
            format_chase_fill_number(avg_px)
        ))
    } else {
        Some("Chase filled".to_string())
    }
}

pub(super) fn chase_fill_summary_for_chase(
    fills: &[UserFill],
    chase: &ChaseOrder,
) -> Option<String> {
    let oids = chase.known_oids_with_current();
    let totals = chase_fill_totals(fills, &oids)?;

    if totals.filled_size > 0.0 {
        let avg_px = totals.total_notional / totals.filled_size;
        Some(format!(
            "Chase filled: {} {} {} @ ${}",
            totals.side,
            format_chase_fill_number(totals.filled_size),
            totals.coin,
            format_chase_fill_number(avg_px)
        ))
    } else {
        Some("Chase filled".to_string())
    }
}

pub(super) fn chase_fill_summary(fills: &[UserFill], oid: u64) -> Option<String> {
    chase_fill_summary_for_oids(fills, &[oid]).map(|summary| {
        if summary == "Chase filled" {
            format!("Chase filled (oid {oid})")
        } else {
            format!("{summary} (oid {oid})")
        }
    })
}

pub(super) fn chase_completed_summary(
    fills: &[UserFill],
    chase: &ChaseOrder,
    filled_size: f64,
) -> String {
    let summary = chase_fill_summary_for_chase(fills, chase)
        .unwrap_or_else(|| "Chase completed: target size filled".to_string());
    if chase.target_size.is_finite()
        && chase.target_size > 0.0
        && filled_size > chase.target_size + f64::EPSILON
    {
        let overfill = filled_size - chase.target_size;
        format!(
            "{summary}; over target by {}",
            format_chase_fill_number(overfill)
        )
    } else {
        summary
    }
}

fn format_chase_fill_number(value: f64) -> String {
    let formatted = format!("{value:.8}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn positive_fill_size_and_price(fill: &UserFill) -> Option<(f64, f64)> {
    let size = fill
        .sz
        .parse::<f64>()
        .ok()
        .and_then(positive_finite_value)?;
    let price = fill
        .px
        .parse::<f64>()
        .ok()
        .and_then(positive_finite_value)?;
    Some((size, price))
}
