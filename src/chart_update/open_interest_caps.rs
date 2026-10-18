use crate::api;
use crate::app_state::TradingTerminal;
use crate::canvas_state::WorkspaceId;
use crate::chart_state::OPEN_INTEREST_CAP_REFRESH_MS;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;
use std::collections::{BTreeSet, HashMap};

impl TradingTerminal {
    pub(crate) fn queue_chart_open_interest_caps(&mut self, now_ms: u64) -> Vec<Task<Message>> {
        let dexes: BTreeSet<String> = self
            .workspace_pane_kinds()
            .filter(|(workspace, _, _)| match workspace {
                WorkspaceId::Main => true,
                WorkspaceId::Canvas(id) => self
                    .canvases
                    .get(id)
                    .is_some_and(|canvas| canvas.window_id.is_some()),
            })
            .filter_map(|(_, _, kind)| match kind {
                PaneKind::Chart(id) => Some(*id),
                _ => None,
            })
            .chain(
                self.detached_chart_windows
                    .values()
                    .map(|window| window.chart_id),
            )
            .filter_map(|id| self.charts.get(&id))
            .filter(|chart| {
                !chart.header_collapsed
                    && self.is_perp_coin(&chart.symbol)
                    && !self.symbol_key_is_hidden(&chart.symbol)
            })
            .filter_map(|chart| chart.symbol.split_once(':'))
            .filter(|(dex, coin)| !dex.is_empty() && !coin.is_empty())
            .map(|(dex, _)| dex.to_string())
            .collect();

        // Keep pending entries until completion, even if their last chart closes.
        // Reopening a chart then shares that request instead of racing a late result.
        self.chart_open_interest_caps
            .dexes
            .retain(|dex, entry| entry.in_flight || dexes.contains(dex));
        let mut tasks = Vec::new();
        for dex in dexes {
            let entry = self
                .chart_open_interest_caps
                .dexes
                .entry(dex.clone())
                .or_default();
            if entry.in_flight || now_ms < entry.next_attempt_ms {
                continue;
            }
            entry.in_flight = true;
            tasks.push(Task::perform(
                api::fetch_open_interest_caps(dex.clone()),
                move |result| Message::ChartOpenInterestCapsLoaded(dex.clone(), result),
            ));
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
}

#[cfg(test)]
mod tests;
