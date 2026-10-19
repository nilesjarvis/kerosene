use crate::api;
use crate::app_state::TradingTerminal;
use crate::chart_state::{OPEN_INTEREST_CAP_REFRESH_MS, OpenInterestCapStreamEvent};
use crate::message::Message;
#[cfg(test)]
use crate::pane_state::PaneKind;
use iced::Task;
use std::collections::{HashMap, HashSet};

impl TradingTerminal {
    pub(crate) fn queue_chart_open_interest_caps(&mut self, now_ms: u64) -> Vec<Task<Message>> {
        let dexes = self.chart_open_interest_cap_dexes();
        if dexes.is_empty()
            || self
                .chart_open_interest_caps
                .live
                .as_ref()
                .is_some_and(|snapshot| !snapshot.is_fresh(now_ms))
        {
            self.apply_chart_open_interest_cap_stream(
                OpenInterestCapStreamEvent::Unavailable,
                now_ms,
            );
        }
        let live = self.chart_open_interest_caps.live.is_some();

        // Keep pending entries until completion, even if their last chart closes.
        // Reopening a chart then shares that request instead of racing a late result.
        self.chart_open_interest_caps
            .dexes
            .retain(|dex, entry| entry.in_flight || entry.status.in_flight || dexes.contains(dex));
        let mut tasks = Vec::new();
        for dex in dexes {
            let entry = self
                .chart_open_interest_caps
                .dexes
                .entry(dex.clone())
                .or_default();
            if !live && !entry.status.in_flight && now_ms >= entry.status.next_attempt_ms {
                entry.status.in_flight = true;
                entry.status.requested_at_ms = Some(now_ms);
                let status_dex = dex.clone();
                tasks.push(Task::perform(
                    api::fetch_perps_at_open_interest_cap(status_dex.clone()),
                    move |result| {
                        Message::ChartOpenInterestCapStatusLoaded(status_dex.clone(), result)
                    },
                ));
            }
            // Native perpetuals expose cap status, but no numeric limit endpoint.
            if !dex.is_empty() && !entry.in_flight && now_ms >= entry.next_attempt_ms {
                entry.in_flight = true;
                tasks.push(Task::perform(
                    api::fetch_open_interest_caps(dex.clone()),
                    move |result| Message::ChartOpenInterestCapsLoaded(dex.clone(), result),
                ));
            }
        }
        tasks
    }

    pub(crate) fn apply_chart_open_interest_caps(
        &mut self,
        dex: &str,
        result: Result<HashMap<String, f64>, String>,
        now_ms: u64,
    ) {
        let Some(entry) = self.chart_open_interest_caps.dexes.get_mut(dex) else {
            return;
        };
        if !entry.in_flight {
            return;
        }
        entry.in_flight = false;
        let delay = match result {
            Ok(caps) => {
                entry.caps = caps;
                entry.updated_at_ms = Some(now_ms);
                entry.failures = 0;
                OPEN_INTEREST_CAP_REFRESH_MS
            }
            Err(_) => {
                entry.caps.clear();
                entry.updated_at_ms = None;
                entry.failures = entry.failures.saturating_add(1);
                OPEN_INTEREST_CAP_REFRESH_MS
                    .saturating_mul(1 << entry.failures.saturating_sub(1).min(3))
                    .min(5 * OPEN_INTEREST_CAP_REFRESH_MS)
            }
        };
        entry.next_attempt_ms = now_ms.saturating_add(delay);
    }

    pub(crate) fn apply_chart_open_interest_cap_stream(
        &mut self,
        event: OpenInterestCapStreamEvent,
        now_ms: u64,
    ) {
        match event {
            OpenInterestCapStreamEvent::Snapshot(snapshot) => {
                if !snapshot.is_fresh(now_ms) || self.chart_open_interest_cap_dexes().is_empty() {
                    return;
                }
                if self
                    .chart_open_interest_caps
                    .live
                    .as_ref()
                    .is_some_and(|current| snapshot.server_time_ms < current.server_time_ms)
                {
                    return;
                }
                self.chart_open_interest_caps.last_live_update_ms = Some(now_ms);
                self.chart_open_interest_caps.live = Some(snapshot);
                for entry in self.chart_open_interest_caps.dexes.values_mut() {
                    entry.status.coins.clear();
                    entry.status.updated_at_ms = None;
                    entry.status.failures = 0;
                    entry.status.next_attempt_ms = now_ms;
                }
            }
            OpenInterestCapStreamEvent::Unavailable => {
                if self.chart_open_interest_caps.live.take().is_some() {
                    for entry in self.chart_open_interest_caps.dexes.values_mut() {
                        entry.status.next_attempt_ms = now_ms;
                    }
                }
            }
        }
    }

    pub(crate) fn apply_chart_open_interest_cap_status(
        &mut self,
        dex: &str,
        result: Result<HashSet<String>, String>,
        now_ms: u64,
    ) {
        let Some(entry) = self.chart_open_interest_caps.dexes.get_mut(dex) else {
            return;
        };
        let status = &mut entry.status;
        if !status.in_flight {
            return;
        }
        status.in_flight = false;
        let requested_at_ms = status.requested_at_ms.take();
        if self
            .chart_open_interest_caps
            .last_live_update_ms
            .is_some_and(|last| requested_at_ms.is_none_or(|requested| requested <= last))
        {
            // An HTTP request begun before a live push must not resurrect its older list.
            status.next_attempt_ms = now_ms;
            return;
        }
        let delay = match result {
            Ok(coins) => {
                status.coins = coins;
                status.updated_at_ms = Some(now_ms);
                status.failures = 0;
                OPEN_INTEREST_CAP_REFRESH_MS
            }
            Err(_) => {
                status.coins.clear();
                status.updated_at_ms = None;
                status.failures = status.failures.saturating_add(1);
                OPEN_INTEREST_CAP_REFRESH_MS
                    .saturating_mul(1 << status.failures.saturating_sub(1).min(3))
                    .min(5 * OPEN_INTEREST_CAP_REFRESH_MS)
            }
        };
        status.next_attempt_ms = now_ms.saturating_add(delay);
    }
}

#[cfg(test)]
mod tests;
