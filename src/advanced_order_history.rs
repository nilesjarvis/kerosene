use crate::signing::ChaseOrder;
use crate::{app_state::TradingTerminal, message::Message};
use iced::{Size, Task, window};
use std::collections::VecDeque;

mod model;
mod snapshots;
#[cfg(test)]
mod tests;

pub(crate) use model::{
    AdvancedOrderHistoryChild, AdvancedOrderHistoryEntry, AdvancedOrderHistoryKind,
    AdvancedOrderHistoryLog,
};

// ---------------------------------------------------------------------------
// Advanced Order History
// ---------------------------------------------------------------------------

pub(crate) const ADVANCED_ORDER_HISTORY_LIMIT: usize = 100;

impl AdvancedOrderHistoryEntry {
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
            ..crate::window_chrome::settings()
        };
        let (window_id, task) = window::open(settings);
        self.advanced_order_history_windows
            .insert(window_id, entry_id);
        task.map(Message::WindowOpened)
    }
}
