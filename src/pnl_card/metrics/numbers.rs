use crate::account;
use crate::app_state::TradingTerminal;
use crate::helpers::parse_finite_number;

// ---------------------------------------------------------------------------
// Position Numbers
// ---------------------------------------------------------------------------

pub(super) struct PositionCardNumbers {
    pub(super) szi: f64,
    pub(super) entry_px: f64,
    pub(super) mark_px: f64,
    pub(super) upnl: f64,
    pub(super) margin_used: f64,
}

impl PositionCardNumbers {
    pub(super) fn from_position(
        terminal: &TradingTerminal,
        pos: &account::Position,
    ) -> Option<Self> {
        let szi = parse_pnl_card_number(&pos.szi)?;
        let entry_px = parse_pnl_card_number(&pos.entry_px)?;
        let wire_upnl = parse_pnl_card_number(&pos.unrealized_pnl);
        let mark_px = terminal
            .resolve_mid_for_symbol(&pos.coin)
            .or_else(|| mark_from_wire_upnl(szi, entry_px, wire_upnl))?;
        let upnl = szi * (mark_px - entry_px);
        let margin_used = parse_pnl_card_number(&pos.margin_used).unwrap_or_default();

        Some(Self {
            szi,
            entry_px,
            mark_px,
            upnl,
            margin_used,
        })
    }
}

fn parse_pnl_card_number(raw: &str) -> Option<f64> {
    parse_finite_number(raw)
}

pub(in crate::pnl_card) fn mark_from_wire_upnl(
    szi: f64,
    entry_px: f64,
    wire_upnl: Option<f64>,
) -> Option<f64> {
    if szi.abs() <= f64::EPSILON {
        return None;
    }
    wire_upnl.map(|upnl| entry_px + upnl / szi)
}

pub(in crate::pnl_card) fn position_asset_move_pct(
    szi: f64,
    entry_px: f64,
    mark_px: f64,
) -> Option<f64> {
    if entry_px.abs() <= f64::EPSILON {
        return None;
    }

    let side = if szi >= 0.0 { 1.0 } else { -1.0 };
    Some((mark_px - entry_px) / entry_px * 100.0 * side)
}

pub(in crate::pnl_card) fn pct_from_basis(value: f64, basis: f64) -> Option<f64> {
    (basis.abs() > f64::EPSILON).then_some(value / basis * 100.0)
}
