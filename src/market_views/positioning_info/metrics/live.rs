use crate::helpers::{parse_positive_finite_number, positive_finite_value};
use crate::hyperdash_api::TickerPositionEntry;
use crate::positioning_state::PositioningInfoInstance;

// ---------------------------------------------------------------------------
// Positioning Live Metrics
// ---------------------------------------------------------------------------

pub(in crate::market_views::positioning_info) const POSITIONING_LIVE_MARK_MAX_AGE_MS: u64 = 15_000;

pub(in crate::market_views::positioning_info) fn positioning_live_mark(
    instance: &PositioningInfoInstance,
    now_ms: u64,
) -> Option<f64> {
    let updated_at = instance.asset_ctx_updated_at_ms?;
    if now_ms.checked_sub(updated_at)? > POSITIONING_LIVE_MARK_MAX_AGE_MS {
        return None;
    }
    let ctx = instance.asset_ctx.as_ref()?;
    parse_live_ctx_price(ctx.mark_px.as_deref())
        .or_else(|| parse_live_ctx_price(ctx.mid_px.as_deref()))
}

fn parse_live_ctx_price(value: Option<&str>) -> Option<f64> {
    value.and_then(parse_positive_finite_number)
}

pub(in crate::market_views::positioning_info) fn positioning_live_notional(
    position: &TickerPositionEntry,
    live_mark: Option<f64>,
) -> Option<f64> {
    let mark = positive_finite_value(live_mark?)?;
    position
        .size
        .is_finite()
        .then_some(position.size.abs() * mark)
}

pub(in crate::market_views::positioning_info) fn positioning_live_unrealized_pnl(
    position: &TickerPositionEntry,
    live_mark: Option<f64>,
) -> Option<f64> {
    let mark = positive_finite_value(live_mark?)?;
    if position.size.is_finite() && positive_finite_value(position.entry_price).is_some() {
        Some(position.size * (mark - position.entry_price))
    } else {
        None
    }
}

pub(in crate::market_views::positioning_info) fn positioning_live_change_usd(
    value: f64,
    live_mark: Option<f64>,
) -> Option<f64> {
    let mark = positive_finite_value(live_mark?)?;
    if value.is_finite() {
        Some(value * mark)
    } else {
        None
    }
}
