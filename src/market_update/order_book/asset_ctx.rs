use crate::account::AssetContext;

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

const SPREAD_HISTORY_WINDOW: Duration = Duration::from_secs(300);

pub(super) fn record_asset_context_spread(
    spread_history: &mut VecDeque<(Instant, f64)>,
    ctx: &AssetContext,
    now: Instant,
) {
    if let Some(spread) = impact_spread(ctx) {
        spread_history.push_front((now, spread));
        trim_spread_history(spread_history, now);
    }
}

fn impact_spread(ctx: &AssetContext) -> Option<f64> {
    let impact = ctx.impact_pxs.as_deref()?;
    if impact.len() < 2 {
        return None;
    }

    let bid = impact[0].parse::<f64>().ok()?;
    let ask = impact[1].parse::<f64>().ok()?;
    Some(ask - bid)
}

fn trim_spread_history(spread_history: &mut VecDeque<(Instant, f64)>, now: Instant) {
    let cutoff = now.checked_sub(SPREAD_HISTORY_WINDOW).unwrap_or(now);
    while spread_history
        .back()
        .is_some_and(|(time, _)| *time < cutoff)
    {
        spread_history.pop_back();
    }
}
