use crate::app_state::TradingTerminal;
use crate::denomination::DISPLAY_DENOMINATION_RATE_STALE_MS;
use crate::hype_unstaking_state::{
    HypeUnstakingFilter, HypeUnstakingQueueState, sort_unstaking_events, summarize_unstaking_events,
};
use crate::message::Message;

use iced::widget::{column, container, responsive, row, rule, text};
use iced::{Element, Fill};
use std::time::Instant;

mod controls;
mod rows;
mod summary;

use summary::hype_unstaking_summary_grid;

// ---------------------------------------------------------------------------
// HYPE Unstaking Queue View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_hype_unstaking_queue(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_hype_unstaking_queue_sized(size.width)).into()
    }

    fn view_hype_unstaking_queue_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let compact = available_width < 680.0;
        let now_ms = self.status_bar_now_ms;
        let denomination = self.display_denomination_context();
        let hype_mid = self.hype_unstaking_notional_mid(now_ms);

        let status_text =
            hype_unstaking_status_text(&self.hype_unstaking_queue, self.status_bar_now);
        let status_color = if self.hype_unstaking_queue.error.is_some() {
            theme.palette().danger
        } else {
            theme.extended_palette().background.weak.text
        };

        // Compute filtered events early so we can show the count in the filters
        let mine_address = if self.hype_unstaking_queue.mine_only {
            self.connected_address.as_deref()
        } else {
            None
        };
        let filtered = if let Some(data) = &self.hype_unstaking_queue.data {
            let mut filtered = data.filtered_events(HypeUnstakingFilter {
                now_ms,
                window: self.hype_unstaking_queue.window_filter,
                amount: self.hype_unstaking_queue.amount_filter,
                mine_address,
            });
            sort_unstaking_events(
                filtered.as_mut_slice(),
                self.hype_unstaking_queue.sort_field,
                self.hype_unstaking_queue.sort_direction,
            );
            Some(filtered)
        } else {
            None
        };
        let filtered_count = filtered.as_ref().map(|f| f.len()).unwrap_or(0);
        let has_data = filtered_count > 0;

        let mut content = column![
            self.view_hype_unstaking_header(compact),
            self.view_hype_unstaking_filters(filtered_count, has_data),
            text(status_text).size(10).color(status_color).width(Fill),
            rule::horizontal(1),
        ]
        .spacing(8)
        .width(Fill);

        if let Some(filtered_events) = filtered.as_deref().filter(|events| !events.is_empty()) {
            let summary = summarize_unstaking_events(filtered_events);
            content = content.push(hype_unstaking_summary_grid(
                &summary,
                now_ms,
                available_width,
                &theme,
            ));
            content = content.push(self.view_hype_unstaking_event_list(
                filtered_events,
                now_ms,
                compact,
                &denomination,
                hype_mid,
                &theme,
            ));
        } else if self.hype_unstaking_queue.loading {
            content = content.push(
                row![
                    self.view_spinner(18),
                    text("Loading unstaking queue")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            );
        } else {
            content = content.push(
                text("No unstaking queue data loaded")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn hype_unstaking_notional_mid(&self, now_ms: u64) -> Option<f64> {
        let updated_at_ms = self.all_mids_updated_at_ms.get("HYPE").copied()?;
        if now_ms.saturating_sub(updated_at_ms) > DISPLAY_DENOMINATION_RATE_STALE_MS {
            return None;
        }

        self.all_mids
            .get("HYPE")
            .copied()
            .filter(|mid| mid.is_finite() && *mid > 0.0)
    }
}

fn hype_unstaking_status_text(state: &HypeUnstakingQueueState, now: Instant) -> String {
    if state.loading && state.data.is_none() {
        "Loading queue...".to_string()
    } else if state.loading {
        "Refreshing...".to_string()
    } else if let Some(error) = &state.error {
        if state.data.is_none() {
            format!("Load failed: {error}")
        } else {
            format!("Showing last good data; refresh failed: {error}")
        }
    } else if let Some(last_fetch) = state.last_fetch {
        let age = now.saturating_duration_since(last_fetch).as_secs();
        if age < 60 {
            "Updated just now".to_string()
        } else {
            format!("Updated {}m ago", age / 60)
        }
    } else {
        "Not loaded".to_string()
    }
}

#[cfg(test)]
mod tests;
