use crate::account::{UserFill, dedupe_user_fills_preserving_order};
use crate::helpers::positive_finite_value;
use crate::signing::ChaseOrder;

use std::{collections::HashSet, fmt};

// ---------------------------------------------------------------------------
// Recent Fills
// ---------------------------------------------------------------------------

/// Matches the Hyperliquid REST `userFills` depth (the most recent 2000
/// fills). Spot cost-basis estimation replays a coin's acquisition fills from
/// `AccountData::fills`, so live fill merges must not evict REST-seeded
/// history: truncating below the REST depth killed the estimate on the first
/// live fill and it stayed dead until the next account refresh.
const MERGED_FILLS_MAX_LEN: usize = 2000;

pub(super) fn prepend_recent_fills(
    existing: &mut Vec<UserFill>,
    incoming: Vec<UserFill>,
    max_len: usize,
) {
    if max_len == 0 {
        existing.clear();
        return;
    }

    let mut seen = HashSet::with_capacity(existing.len().saturating_add(incoming.len()));
    let mut updated =
        Vec::with_capacity(max_len.min(existing.len().saturating_add(incoming.len())));
    for fill in incoming.into_iter().chain(existing.drain(..)) {
        if seen.insert(fill.dedup_key()) {
            updated.push(fill);
            if updated.len() == max_len {
                break;
            }
        }
    }
    *existing = updated;
}

pub(super) fn apply_fills_update<F>(
    existing: &mut Vec<UserFill>,
    fills: Vec<UserFill>,
    is_snapshot: bool,
    is_hidden: F,
) -> Vec<UserFill>
where
    F: Fn(&str) -> bool,
{
    if is_snapshot {
        *existing = dedupe_user_fills_preserving_order(fills);
        Vec::new()
    } else {
        let mut seen: HashSet<String> = existing.iter().map(UserFill::dedup_key).collect();
        let fills: Vec<UserFill> = fills
            .into_iter()
            .filter(|fill| seen.insert(fill.dedup_key()))
            .collect();
        let toast_fills: Vec<UserFill> = fills
            .iter()
            .filter(|fill| !is_hidden(&fill.coin))
            .cloned()
            .collect();
        prepend_recent_fills(existing, fills, MERGED_FILLS_MAX_LEN);
        toast_fills
    }
}

pub(super) fn fill_toast_message(fill: &UserFill, coin_label: &str, size_label: &str) -> String {
    let side = if fill.side == "B" { "BUY" } else { "SELL" };
    format!("Filled {side} {size_label} {coin_label} @ ${}", fill.px)
}

// ---------------------------------------------------------------------------
// Chase Fill Summaries
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(super) struct ChaseFillTotals {
    pub(super) side: String,
    pub(super) filled_size: f64,
    pub(super) total_notional: f64,
}

impl fmt::Debug for ChaseFillTotals {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let side = match self.side.as_str() {
            "BUY" => "BUY",
            "SELL" => "SELL",
            _ => "<unrecognized>",
        };
        f.debug_struct("ChaseFillTotals")
            .field("side", &side)
            .field("filled_size", &"<redacted>")
            .field("total_notional", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
pub(super) fn chase_fill_totals(fills: &[UserFill], oids: &[u64]) -> Option<ChaseFillTotals> {
    chase_fill_totals_with_cutoff(fills, oids, |_| None)
}

pub(super) fn chase_fill_totals_for_chase(
    fills: &[UserFill],
    chase: &ChaseOrder,
) -> Option<ChaseFillTotals> {
    let oids = chase.known_oids_with_current();
    chase_fill_totals_with_filter(
        fills,
        &oids,
        Some((chase.coin.as_str(), chase.is_buy)),
        |oid| chase.fill_cutoff_ms_for_oid(oid),
    )
}

#[cfg(test)]
fn chase_fill_totals_with_cutoff<F>(
    fills: &[UserFill],
    oids: &[u64],
    fill_cutoff_ms_for_oid: F,
) -> Option<ChaseFillTotals>
where
    F: Fn(u64) -> Option<u64>,
{
    chase_fill_totals_with_filter(fills, oids, None, fill_cutoff_ms_for_oid)
}

fn chase_fill_totals_with_filter<F>(
    fills: &[UserFill],
    oids: &[u64],
    expected_order: Option<(&str, bool)>,
    fill_cutoff_ms_for_oid: F,
) -> Option<ChaseFillTotals>
where
    F: Fn(u64) -> Option<u64>,
{
    if oids.is_empty() {
        return None;
    }

    let mut seen = HashSet::new();
    let mut side = None;
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
        if let Some((expected_coin, expected_is_buy)) = expected_order
            && (fill.coin != expected_coin || fill_side_is_buy(&fill.side) != Some(expected_is_buy))
        {
            continue;
        }
        if fill_cutoff_ms_for_oid(oid).is_some_and(|cutoff_ms| fill.time < cutoff_ms) {
            continue;
        }
        if !seen.insert(fill.dedup_key()) {
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
        filled_size,
        total_notional,
    })
}

#[cfg(test)]
pub(super) fn chase_fill_summary_for_oids(fills: &[UserFill], oids: &[u64]) -> Option<String> {
    let totals = chase_fill_totals(fills, oids)?;
    let coin_label = fills
        .iter()
        .find(|fill| fill.oid.is_some_and(|oid| oids.contains(&oid)))
        .map(|fill| fill.coin.clone())
        .unwrap_or_else(|| "?".to_string());

    Some(chase_fill_summary_text(&totals, &coin_label))
}

/// `display_coin` is the human-readable pair name for `chase.coin`
/// (spot fills report the raw "@{index}" key), resolved by the caller via
/// `display_coin_for_journal`.
pub(super) fn chase_fill_summary_for_chase(
    fills: &[UserFill],
    chase: &ChaseOrder,
    display_coin: &str,
) -> Option<String> {
    let totals = chase_fill_totals_for_chase(fills, chase)?;

    Some(chase_fill_summary_text(&totals, display_coin))
}

fn chase_fill_summary_text(totals: &ChaseFillTotals, coin_label: &str) -> String {
    if totals.filled_size > 0.0 {
        let avg_px = totals.total_notional / totals.filled_size;
        format!(
            "Chase filled: {} {} {} @ ${}",
            totals.side,
            format_chase_fill_number(totals.filled_size),
            coin_label,
            format_chase_fill_number(avg_px)
        )
    } else {
        "Chase filled".to_string()
    }
}

#[cfg(test)]
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
    display_coin: &str,
) -> String {
    let summary = chase_fill_summary_for_chase(fills, chase, display_coin)
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

fn fill_side_is_buy(side: &str) -> Option<bool> {
    match side {
        "B" => Some(true),
        "A" => Some(false),
        _ => None,
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
