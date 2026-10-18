//! Transient, pinned L4 inspector. Full books live in the stream task; the UI
//! receives only a bounded page of orders for the selected price bucket.
mod book;

use crate::message::RedactedAddress;
pub(crate) use book::{Block, Book, Diff, Snapshot};
use iced::window;
use std::fmt;
use tokio::sync::watch;

pub(crate) const PAGE_SIZE: usize = 50;
pub(crate) const SCALE: u64 = 100_000_000;
pub(crate) const MAX_ORDERS: usize = 200_000;

#[derive(Clone)]
pub(crate) struct RestingOrder {
    pub(crate) oid: u64,
    pub(crate) wallet: RedactedAddress,
    pub(crate) is_bid: bool,
    pub(crate) price: u64,
    pub(crate) size: u64,
}

impl fmt::Debug for RestingOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RestingOrder(<redacted>)")
    }
}

/// Eight-decimal fixed point matches the provider's reconstruction model.
/// Reject excess precision and overflow instead of silently rounding orders.
pub(crate) fn parse_decimal(value: &str) -> Result<u64, String> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || fraction.trim_end_matches('0').len() > 8
    {
        return Err("Invalid L4 decimal".into());
    }
    let whole = whole.parse::<u64>().map_err(|_| "Invalid L4 decimal")?;
    let fraction = &fraction[..fraction.len().min(8)];
    let fractional = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u64>().map_err(|_| "Invalid L4 decimal")?
            * 10u64.pow(8 - fraction.len() as u32)
    };
    whole
        .checked_mul(SCALE)
        .and_then(|n| n.checked_add(fractional))
        .ok_or_else(|| "L4 decimal overflow".into())
}

pub(crate) fn format_decimal(value: u64) -> String {
    if value.is_multiple_of(SCALE) {
        return (value / SCALE).to_string();
    }
    format!("{}.{:08}", value / SCALE, value % SCALE)
        .trim_end_matches('0')
        .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Selection {
    pub(crate) price: f64,
    pub(crate) tick: f64,
    /// None is a DOM row containing both sides.
    pub(crate) is_bid: Option<bool>,
    pub(crate) page: usize,
    pub(crate) revision: u64,
}

impl Selection {
    pub(crate) fn valid(self) -> bool {
        self.price.is_finite()
            && self.price >= 0.0
            && crate::helpers::valid_book_tick_size(self.tick)
            && self.price / self.tick < i64::MAX as f64
    }

    pub(crate) fn contains(self, order: &RestingOrder) -> bool {
        self.is_bid.is_none_or(|side| side == order.is_bid)
            && crate::helpers::book_bucket_key(
                order.price as f64 / SCALE as f64,
                self.tick,
                order.is_bid,
            ) == crate::helpers::book_bucket_key(self.price, self.tick, order.is_bid)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Page {
    pub(crate) revision: u64,
    pub(crate) height: u64,
    pub(crate) timestamp_ms: u64,
    pub(crate) orders: Vec<RestingOrder>,
    pub(crate) total_orders: usize,
    pub(crate) wallets: usize,
    pub(crate) total_size: f64,
    pub(crate) notional: f64,
    pub(crate) page: usize,
}

#[derive(Debug, Clone)]
pub(crate) enum InspectorEvent {
    Page(Page),
    /// Pages retained by the UI become explicitly stale until a new live page.
    Status(InspectorStatus),
}

#[derive(Debug, Clone)]
pub(crate) enum InspectorStatus {
    Connecting,
    Waiting(u64),
    Synchronizing,
    Recovering(String),
    Unavailable(String),
}

impl InspectorStatus {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Connecting => "Connecting to Hydromancer…".into(),
            Self::Waiting(seconds) => format!("Waiting {seconds}s for the L4 snapshot budget…"),
            Self::Synchronizing => "Synchronizing L4 order book…".into(),
            Self::Recovering(message) | Self::Unavailable(message) => message.clone(),
        }
    }

    pub(crate) fn can_retry(&self) -> bool {
        matches!(self, Self::Recovering(_) | Self::Unavailable(_))
    }
}

pub(crate) struct BookInspectorState {
    pub(crate) window_id: window::Id,
    pub(crate) coin: String,
    pub(crate) selection: Selection,
    pub(crate) run_id: u64,
    pub(crate) selection_tx: watch::Sender<Selection>,
    pub(crate) selection_rx: watch::Receiver<Selection>,
    pub(crate) page: Option<Page>,
    pub(crate) status: Option<InspectorStatus>,
}

impl BookInspectorState {
    pub(crate) fn new(window_id: window::Id, coin: String, selection: Selection) -> Self {
        let (selection_tx, selection_rx) = watch::channel(selection);
        Self {
            window_id,
            coin,
            selection,
            run_id: 0,
            selection_tx,
            selection_rx,
            page: None,
            status: Some(InspectorStatus::Connecting),
        }
    }

    pub(crate) fn select(&mut self, mut selection: Selection) {
        selection.revision = self.selection.revision.wrapping_add(1);
        self.selection = selection;
        self.page = None;
        self.selection_tx.send_replace(selection);
    }

    pub(crate) fn restart(&mut self) {
        self.run_id = self.run_id.wrapping_add(1);
        self.page = None;
        self.status = Some(InspectorStatus::Connecting);
    }
}

#[cfg(test)]
mod tests;
