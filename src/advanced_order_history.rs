use crate::signing::ChaseOrder;
use crate::twap_state::{TwapOrder, TwapStatus};
use crate::{app_state::TradingTerminal, message::Message};
use iced::{Size, Task, window};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// Advanced Order History
// ---------------------------------------------------------------------------

pub(crate) const ADVANCED_ORDER_HISTORY_LIMIT: usize = 100;
const ADVANCED_ORDER_HISTORY_LOG_LIMIT: usize = 200;
const ADVANCED_ORDER_HISTORY_CHILD_LIMIT: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum AdvancedOrderHistoryKind {
    Chase,
    Twap,
}

impl AdvancedOrderHistoryKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Chase => "CHASE",
            Self::Twap => "TWAP",
        }
    }
}

fn default_history_kind() -> AdvancedOrderHistoryKind {
    AdvancedOrderHistoryKind::Twap
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AdvancedOrderHistoryLog {
    #[serde(default)]
    pub(crate) elapsed_ms: u64,
    #[serde(default)]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AdvancedOrderHistoryChild {
    #[serde(default)]
    pub(crate) index: u32,
    #[serde(default)]
    pub(crate) elapsed_ms: u64,
    #[serde(default)]
    pub(crate) planned_size: f64,
    #[serde(default)]
    pub(crate) limit_price: f64,
    #[serde(default)]
    pub(crate) filled_size: f64,
    #[serde(default)]
    pub(crate) avg_price: Option<f64>,
    #[serde(default)]
    pub(crate) fee: f64,
    #[serde(default)]
    pub(crate) oid: Option<u64>,
    #[serde(default)]
    pub(crate) cloid: Option<String>,
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) exchange_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AdvancedOrderHistoryEntry {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default = "default_history_kind")]
    pub(crate) kind: AdvancedOrderHistoryKind,
    #[serde(default)]
    pub(crate) source_id: u64,
    #[serde(default)]
    pub(crate) account_address: String,
    #[serde(default)]
    pub(crate) coin: String,
    #[serde(default)]
    pub(crate) display_coin: String,
    #[serde(default)]
    pub(crate) is_buy: bool,
    #[serde(default)]
    pub(crate) target_size: f64,
    #[serde(default)]
    pub(crate) filled_size: f64,
    #[serde(default)]
    pub(crate) remaining_size: f64,
    #[serde(default)]
    pub(crate) average_price: Option<f64>,
    #[serde(default)]
    pub(crate) min_price: Option<f64>,
    #[serde(default)]
    pub(crate) max_price: Option<f64>,
    #[serde(default)]
    pub(crate) reduce_only: bool,
    #[serde(default)]
    pub(crate) randomize: bool,
    #[serde(default)]
    pub(crate) slice_count: u32,
    #[serde(default)]
    pub(crate) slices_sent: u32,
    #[serde(default)]
    pub(crate) reprice_count: u32,
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) started_at_ms: u64,
    #[serde(default)]
    pub(crate) completed_at_ms: u64,
    #[serde(default)]
    pub(crate) logs: Vec<AdvancedOrderHistoryLog>,
    #[serde(default)]
    pub(crate) children: Vec<AdvancedOrderHistoryChild>,
}

impl AdvancedOrderHistoryEntry {
    pub(crate) fn from_twap(twap: &TwapOrder, completed_at_ms: u64) -> Self {
        let logs = twap
            .events
            .iter()
            .rev()
            .take(ADVANCED_ORDER_HISTORY_LOG_LIMIT)
            .rev()
            .map(|event| AdvancedOrderHistoryLog {
                elapsed_ms: event
                    .at
                    .saturating_duration_since(twap.started_at)
                    .as_millis() as u64,
                kind: format!("{:?}", event.kind),
                message: event.message.clone(),
                is_error: event.is_error,
            })
            .collect();
        let children = twap
            .child_orders
            .iter()
            .rev()
            .take(ADVANCED_ORDER_HISTORY_CHILD_LIMIT)
            .rev()
            .map(|child| AdvancedOrderHistoryChild {
                index: child.index,
                elapsed_ms: child
                    .requested_at
                    .saturating_duration_since(twap.started_at)
                    .as_millis() as u64,
                planned_size: finite_or_zero(child.planned_size),
                limit_price: finite_or_zero(child.limit_price),
                filled_size: finite_or_zero(child.filled_size),
                avg_price: child
                    .avg_price
                    .filter(|value| value.is_finite() && *value > 0.0),
                fee: finite_or_zero(child.fee),
                oid: child.oid,
                cloid: child.cloid.clone(),
                status: child.status.label().to_string(),
                exchange_summary: child.exchange_summary.clone(),
            })
            .collect();
        let summary = twap
            .events
            .last()
            .map(|event| event.message.clone())
            .unwrap_or_else(|| twap.status.label().to_string());

        Self {
            id: format!(
                "twap:{}:{}:{}",
                twap.account_address, twap.started_at_ms, twap.id
            ),
            kind: AdvancedOrderHistoryKind::Twap,
            source_id: twap.id,
            account_address: twap.account_address.clone(),
            coin: twap.coin.clone(),
            display_coin: twap.display_coin.clone(),
            is_buy: twap.is_buy,
            target_size: finite_or_zero(twap.target_size),
            filled_size: finite_or_zero(twap.filled_size),
            remaining_size: finite_or_zero(twap.remaining_size),
            average_price: twap_average_price(twap),
            min_price: Some(twap.min_price).filter(|value| value.is_finite() && *value > 0.0),
            max_price: Some(twap.max_price).filter(|value| value.is_finite() && *value > 0.0),
            reduce_only: twap.reduce_only,
            randomize: twap.randomize,
            slice_count: twap.slice_count,
            slices_sent: twap.slices_sent,
            reprice_count: 0,
            status: twap_history_status(twap.status).to_string(),
            summary,
            started_at_ms: twap.started_at_ms,
            completed_at_ms,
            logs,
            children,
        }
    }

    pub(crate) fn from_chase(chase: &ChaseOrder, completed_at_ms: u64, summary: String) -> Self {
        let status = chase
            .stop_reason
            .as_ref()
            .map(|(_, is_error)| if *is_error { "Error" } else { "Stopped" })
            .unwrap_or("Completed");
        let summary = if summary.trim().is_empty() {
            status.to_string()
        } else {
            summary
        };
        let target_size = finite_or_zero(chase.target_size);
        let filled_size = if chase.filled_size.is_finite() && chase.filled_size > 0.0 {
            if target_size > 0.0 {
                chase.filled_size.min(target_size)
            } else {
                chase.filled_size
            }
        } else if target_size > 0.0
            && chase.remaining_size.is_finite()
            && chase.remaining_size > 0.0
        {
            (target_size - chase.remaining_size).clamp(0.0, target_size)
        } else {
            0.0
        };
        let remaining_size = if target_size > 0.0 {
            (target_size - filled_size).max(0.0)
        } else {
            finite_or_zero(chase.remaining_size)
        };

        Self {
            id: format!(
                "chase:{}:{}:{}",
                chase.account_address, chase.started_at_ms, chase.id
            ),
            kind: AdvancedOrderHistoryKind::Chase,
            source_id: chase.id,
            account_address: chase.account_address.clone(),
            coin: chase.coin.clone(),
            display_coin: chase.coin.clone(),
            is_buy: chase.is_buy,
            target_size,
            filled_size,
            remaining_size,
            average_price: Some(chase.current_price)
                .filter(|value| value.is_finite() && *value > 0.0),
            min_price: None,
            max_price: None,
            reduce_only: chase.reduce_only,
            randomize: false,
            slice_count: 0,
            slices_sent: 0,
            reprice_count: chase.reprice_count,
            status: status.to_string(),
            summary: summary.clone(),
            started_at_ms: chase.started_at_ms,
            completed_at_ms,
            logs: vec![
                AdvancedOrderHistoryLog {
                    elapsed_ms: 0,
                    kind: "Started".to_string(),
                    message: "Chase started".to_string(),
                    is_error: false,
                },
                AdvancedOrderHistoryLog {
                    elapsed_ms: chase
                        .started_at
                        .elapsed()
                        .as_millis()
                        .try_into()
                        .unwrap_or(u64::MAX),
                    kind: status.to_string(),
                    message: summary,
                    is_error: status == "Error",
                },
            ],
            children: Vec::new(),
        }
    }

    pub(crate) fn side_label(&self) -> &'static str {
        if self.is_buy { "BUY" } else { "SELL" }
    }
}

pub(crate) fn upsert_advanced_order_history(
    history: &mut VecDeque<AdvancedOrderHistoryEntry>,
    entry: AdvancedOrderHistoryEntry,
) {
    if let Some(existing) = history.iter_mut().find(|existing| existing.id == entry.id) {
        *existing = entry;
    } else {
        history.push_front(entry);
    }
    prune_advanced_order_history(history);
}

pub(crate) fn prune_advanced_order_history(history: &mut VecDeque<AdvancedOrderHistoryEntry>) {
    history.retain(|entry| !entry.id.trim().is_empty());
    while history.len() > ADVANCED_ORDER_HISTORY_LIMIT {
        history.pop_back();
    }
}

fn twap_history_status(status: TwapStatus) -> &'static str {
    match status {
        TwapStatus::Running
        | TwapStatus::WaitingForMarket
        | TwapStatus::Paused
        | TwapStatus::Stopping => "Interrupted",
        TwapStatus::Stopped => "Stopped",
        TwapStatus::Completed => "Completed",
        TwapStatus::CompletedPartial => "Partial",
        TwapStatus::Error => "Error",
    }
}

fn twap_average_price(twap: &TwapOrder) -> Option<f64> {
    let mut size = 0.0;
    let mut notional = 0.0;
    for child in &twap.child_orders {
        let Some(price) = child.avg_price else {
            continue;
        };
        if child.filled_size > 0.0 && price.is_finite() && price > 0.0 {
            size += child.filled_size;
            notional += child.filled_size * price;
        }
    }
    (size > 0.0).then_some(notional / size)
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

impl TradingTerminal {
    pub(crate) fn archive_twap_if_terminal(&mut self, twap_id: u64) {
        let Some(entry) = self
            .twap_orders
            .get(&twap_id)
            .filter(|twap| twap.status.is_terminal())
            .map(|twap| AdvancedOrderHistoryEntry::from_twap(twap, Self::now_ms()))
        else {
            return;
        };
        upsert_advanced_order_history(&mut self.advanced_order_history, entry);
        self.persist_config();
    }

    pub(crate) fn archive_chase_order(&mut self, chase: &ChaseOrder, summary: String) {
        let entry = AdvancedOrderHistoryEntry::from_chase(chase, Self::now_ms(), summary);
        upsert_advanced_order_history(&mut self.advanced_order_history, entry);
        self.persist_config();
    }

    pub(crate) fn open_advanced_order_history(&mut self, entry_id: String) -> Task<Message> {
        if !self
            .advanced_order_history
            .iter()
            .any(|entry| entry.id == entry_id)
        {
            return Task::none();
        }
        if let Some(window_id) = self
            .advanced_order_history_windows
            .iter()
            .find_map(|(window_id, id)| (id == &entry_id).then_some(*window_id))
        {
            return window::gain_focus(window_id);
        }

        let settings = window::Settings {
            size: Size::new(760.0, 560.0),
            ..window::Settings::default()
        };
        let (window_id, task) = window::open(settings);
        self.advanced_order_history_windows
            .insert(window_id, entry_id);
        task.map(Message::WindowOpened)
    }
}
