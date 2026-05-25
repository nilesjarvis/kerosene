use super::super::super::pricing::wire_market_price;
use crate::api::MarketType;
use crate::helpers::{finite_value, positive_finite_value};
use crate::signing::float_to_wire;

// ---------------------------------------------------------------------------
// NUKE Position Planning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NukePositionOrder {
    pub(crate) asset: u32,
    pub(crate) is_buy: bool,
    pub(crate) price: String,
    pub(crate) size: String,
}

/// Reason an active visible position could not be turned into a closing
/// market order during NUKE planning. Surfaced to the user before they
/// confirm so an emergency-flatten action never silently leaves exposure
/// behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NukeSkipReason {
    /// The coin in `clearinghouse.asset_positions` has no matching entry in
    /// the cached `exchange_symbols` metadata.
    UnknownAsset,
    /// The asset is not a perpetual market; NUKE only closes perps.
    NonPerp,
    /// No mid price is currently resolvable for this symbol.
    NoMidPrice,
    /// Order construction rejected the inputs as non-finite or out of range.
    OrderBuildFailed,
}

impl NukeSkipReason {
    fn label(self) -> &'static str {
        match self {
            Self::UnknownAsset => "unknown asset",
            Self::NonPerp => "not a perpetual market",
            Self::NoMidPrice => "no mid price",
            Self::OrderBuildFailed => "invalid order params",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(in crate::order_execution::position_actions::nuke) struct NukeSymbolInfo {
    pub(in crate::order_execution::position_actions::nuke) asset_index: u32,
    pub(in crate::order_execution::position_actions::nuke) sz_decimals: u32,
    pub(in crate::order_execution::position_actions::nuke) market_type: MarketType,
}

#[derive(Debug, Clone)]
pub(in crate::order_execution::position_actions::nuke) struct NukePositionInput {
    pub(in crate::order_execution::position_actions::nuke) coin: String,
    pub(in crate::order_execution::position_actions::nuke) raw_size: String,
    pub(in crate::order_execution::position_actions::nuke) is_visible: bool,
    pub(in crate::order_execution::position_actions::nuke) sym: Option<NukeSymbolInfo>,
    pub(in crate::order_execution::position_actions::nuke) mid: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::order_execution::position_actions::nuke) enum NukePositionClassification {
    Order(NukePositionOrder),
    Skip(NukeSkipReason),
}

/// Pure classifier: given a coin's size + the symbol + mid + slippage,
/// return either the order to submit or the reason for skipping.
pub(in crate::order_execution::position_actions::nuke) fn classify_nuke_position(
    szi: f64,
    sym: Option<NukeSymbolInfo>,
    mid: Option<f64>,
    slippage: f64,
) -> NukePositionClassification {
    let Some(sym) = sym else {
        return NukePositionClassification::Skip(NukeSkipReason::UnknownAsset);
    };
    if sym.market_type != MarketType::Perp {
        return NukePositionClassification::Skip(NukeSkipReason::NonPerp);
    }
    let Some(mid) = mid else {
        return NukePositionClassification::Skip(NukeSkipReason::NoMidPrice);
    };
    match build_nuke_position_order(sym.asset_index, sym.sz_decimals, mid, szi, slippage) {
        Some(order) => NukePositionClassification::Order(order),
        None => NukePositionClassification::Skip(NukeSkipReason::OrderBuildFailed),
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct NukePlan {
    pub(crate) ready: Vec<(String, NukePositionOrder)>,
    pub(crate) skipped: Vec<(String, NukeSkipReason)>,
}

impl NukePlan {
    pub(crate) fn is_empty(&self) -> bool {
        self.ready.is_empty() && self.skipped.is_empty()
    }

    /// Comma-joined `"COIN (reason)"` list of skipped positions. Empty
    /// string when nothing was skipped.
    pub(crate) fn format_skip_list(&self) -> String {
        self.skipped
            .iter()
            .map(|(coin, reason)| format!("{coin} ({})", reason.label()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Comma-joined coin list of ready positions. Empty string when
    /// nothing is ready.
    pub(crate) fn format_ready_list(&self) -> String {
        self.ready
            .iter()
            .map(|(coin, _)| coin.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub(in crate::order_execution::position_actions::nuke) fn build_nuke_position_order(
    asset: u32,
    sz_decimals: u32,
    mid: f64,
    szi: f64,
    slippage: f64,
) -> Option<NukePositionOrder> {
    let mid = positive_finite_value(mid)?;
    let szi = finite_value(szi)?;
    let slippage = finite_value(slippage)?;
    if szi.abs() <= 1e-12 || slippage < 0.0 {
        return None;
    }

    let is_buy = szi < 0.0;
    Some(NukePositionOrder {
        asset,
        is_buy,
        price: wire_market_price(mid, is_buy, slippage, sz_decimals, false),
        size: float_to_wire(szi.abs()),
    })
}

pub(in crate::order_execution::position_actions::nuke) fn parse_nuke_position_size(
    coin: &str,
    raw_size: &str,
) -> Result<Option<f64>, String> {
    let size = raw_size
        .trim()
        .parse::<f64>()
        .map_err(|e| format!("NUKE aborted: invalid position size for {coin}: {e}"))?;
    if !size.is_finite() {
        return Err(format!("NUKE aborted: non-finite position size for {coin}"));
    }
    if size.abs() <= 1e-12 {
        Ok(None)
    } else {
        Ok(Some(size))
    }
}

pub(in crate::order_execution::position_actions::nuke) fn plan_nuke_positions_from_inputs(
    inputs: impl IntoIterator<Item = NukePositionInput>,
    slippage: f64,
) -> Result<NukePlan, String> {
    let mut plan = NukePlan::default();

    for input in inputs {
        let NukePositionInput {
            coin,
            raw_size,
            is_visible,
            sym,
            mid,
        } = input;

        if !is_visible {
            continue;
        }

        let szi = match parse_nuke_position_size(&coin, &raw_size)? {
            Some(szi) => szi,
            None => continue, // zero or near-zero: no exposure
        };

        match classify_nuke_position(szi, sym, mid, slippage) {
            NukePositionClassification::Order(order) => plan.ready.push((coin, order)),
            NukePositionClassification::Skip(reason) => plan.skipped.push((coin, reason)),
        }
    }

    Ok(plan)
}
