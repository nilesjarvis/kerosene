use super::super::pricing::wire_market_price;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{OrderKind, float_to_wire, place_order};

use iced::Task;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NukePositionOrder {
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
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
    /// The asset is not a perpetual market — NUKE only closes perps.
    NonPerp,
    /// No mid price is currently resolvable for this symbol (degraded
    /// market-data feed).
    NoMidPrice,
    /// Order construction rejected the (mid, size, slippage) inputs as
    /// non-finite or out of range.
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
pub(crate) struct NukeSymbolInfo {
    pub(crate) asset_index: u32,
    pub(crate) sz_decimals: u32,
    pub(crate) market_type: MarketType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NukePositionClassification {
    Order(NukePositionOrder),
    Skip(NukeSkipReason),
}

/// Pure classifier: given a coin's size + the symbol + mid + slippage,
/// return either the order to submit or the reason for skipping. Lives
/// outside `TradingTerminal` so unit tests can exercise every branch with
/// fixture data.
fn classify_nuke_position(
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

fn build_nuke_position_order(
    asset: u32,
    sz_decimals: u32,
    mid: f64,
    szi: f64,
    slippage: f64,
) -> Option<NukePositionOrder> {
    if !mid.is_finite()
        || mid <= 0.0
        || !szi.is_finite()
        || szi.abs() <= 1e-12
        || !slippage.is_finite()
        || slippage < 0.0
    {
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

fn parse_nuke_position_size(coin: &str, raw_size: &str) -> Result<Option<f64>, String> {
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

impl TradingTerminal {
    /// Plan a NUKE: classify every active visible non-muted position into
    /// `(coin, order)` for submission or `(coin, reason)` for skip.
    /// Returns `Err` only when a position's `szi` field cannot be parsed
    /// (malformed account data), in which case the whole action is aborted
    /// rather than partially submitted.
    pub(crate) fn plan_nuke_positions(&self) -> Result<NukePlan, String> {
        let positions = self
            .account_data
            .as_ref()
            .map(|d| d.clearinghouse.asset_positions.clone())
            .unwrap_or_default();

        let slippage = self.market_slippage_fraction();
        let mut plan = NukePlan::default();

        for ap in positions {
            let coin = ap.position.coin.clone();
            let szi = match parse_nuke_position_size(&coin, &ap.position.szi)? {
                Some(szi) => szi,
                None => continue, // zero or near-zero — no exposure
            };

            // Apply visibility filter (mute + hidden). Audit framing: NUKE
            // closes "active visible non-muted" positions; muted/hidden
            // entries are intentionally absent from the count so the user
            // count matches the table they see.
            if self.is_ticker_muted(&coin) || self.position_is_hidden(&coin) {
                continue;
            }

            let sym = self
                .exchange_symbols
                .iter()
                .find(|s| s.key == coin)
                .map(|s| NukeSymbolInfo {
                    asset_index: s.asset_index,
                    sz_decimals: s.sz_decimals,
                    market_type: s.market_type,
                });
            let mid = self.resolve_mid_for_symbol(&coin);

            match classify_nuke_position(szi, sym, mid, slippage) {
                NukePositionClassification::Order(order) => plan.ready.push((coin, order)),
                NukePositionClassification::Skip(reason) => plan.skipped.push((coin, reason)),
            }
        }

        Ok(plan)
    }

    pub(crate) fn execute_nuke_positions(&mut self) -> Task<Message> {
        let _theme = self.theme();
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }

        let plan = match self.plan_nuke_positions() {
            Ok(plan) => plan,
            Err(e) => {
                self.order_status = Some((e, true));
                return Task::none();
            }
        };

        if plan.is_empty() {
            self.order_status = Some(("No positions to close".into(), true));
            return Task::none();
        }
        if plan.ready.is_empty() {
            // Every active position is unrouteable. Refuse to fire — surface
            // why so the user can address it (subscribe to mids, switch
            // symbol search filters, etc.) rather than seeing a silent no-op.
            self.order_status = Some((
                format!(
                    "NUKE aborted: no positions could be routed. Skipped: {}",
                    plan.format_skip_list()
                ),
                true,
            ));
            return Task::none();
        }
        self.nuke_confirmation = None;

        let ready_count = plan.ready.len();
        let skipped_count = plan.skipped.len();
        // Format the skip list before consuming `ready` in the loop below.
        let skip_summary = plan.format_skip_list();
        let NukePlan { ready, .. } = plan;

        let mut tasks = Vec::with_capacity(ready_count);
        for (_coin, order) in ready {
            let k = key.clone();
            tasks.push(Task::perform(
                place_order(
                    k.into(),
                    order.asset,
                    order.is_buy,
                    order.price,
                    order.size,
                    OrderKind::Market,
                    true,
                ),
                |r| Message::NukeResult(Box::new(r)),
            ));
        }

        let total = ready_count + skipped_count;
        let status = if skipped_count == 0 {
            format!(
                "Nuking {} position{}...",
                ready_count,
                if ready_count == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "Nuking {} of {} position{}; skipped: {}",
                ready_count,
                total,
                if total == 1 { "" } else { "s" },
                skip_summary
            )
        };
        self.order_status = Some((status, false));
        Task::batch(tasks)
    }
}
