use crate::config::SortDirection;
use crate::helpers::{format_decimal_with_commas, trim_decimal_zeros};

use std::time::Instant;
use std::{cmp::Ordering, collections::HashSet, fmt};

// ---------------------------------------------------------------------------
// HYPE Unstaking Queue State
// ---------------------------------------------------------------------------

pub(crate) const HYPE_CORE_WEI_DECIMALS: u32 = 8;
pub(crate) const HYPE_CORE_WEI_PER_TOKEN: u128 = 10_u128.pow(HYPE_CORE_WEI_DECIMALS);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum HypeUnstakingWindowFilter {
    OneHour,
    #[default]
    Day,
    Week,
    All,
}

impl HypeUnstakingWindowFilter {
    pub(crate) const ALL: [Self; 4] = [Self::OneHour, Self::Day, Self::Week, Self::All];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::OneHour => "1h",
            Self::Day => "24h",
            Self::Week => "7d",
            Self::All => "All",
        }
    }

    fn end_ms(self, now_ms: u64) -> Option<u64> {
        let hour_ms = 60 * 60 * 1_000;
        let day_ms = 24 * hour_ms;
        match self {
            Self::OneHour => Some(now_ms.saturating_add(hour_ms)),
            Self::Day => Some(now_ms.saturating_add(day_ms)),
            Self::Week => Some(now_ms.saturating_add(7 * day_ms)),
            Self::All => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum HypeUnstakingAmountFilter {
    #[default]
    All,
    AtLeast100,
    AtLeast1k,
    AtLeast10k,
}

impl HypeUnstakingAmountFilter {
    pub(crate) const ALL: [Self; 4] = [
        Self::All,
        Self::AtLeast100,
        Self::AtLeast1k,
        Self::AtLeast10k,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::AtLeast100 => ">=100",
            Self::AtLeast1k => ">=1k",
            Self::AtLeast10k => ">=10k",
        }
    }

    fn min_wei(self) -> u128 {
        match self {
            Self::All => 0,
            Self::AtLeast100 => 100 * HYPE_CORE_WEI_PER_TOKEN,
            Self::AtLeast1k => 1_000 * HYPE_CORE_WEI_PER_TOKEN,
            Self::AtLeast10k => 10_000 * HYPE_CORE_WEI_PER_TOKEN,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum HypeUnstakingSortField {
    #[default]
    UnlockTime,
    Amount,
}

impl HypeUnstakingSortField {
    pub(crate) fn default_direction(self) -> SortDirection {
        match self {
            Self::UnlockTime => SortDirection::Ascending,
            Self::Amount => SortDirection::Descending,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct HypeUnstakingQueueState {
    pub(crate) data: Option<HypeUnstakingQueueData>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) last_fetch: Option<Instant>,
    pub(crate) refresh_request_id: u64,
    pub(crate) window_filter: HypeUnstakingWindowFilter,
    pub(crate) amount_filter: HypeUnstakingAmountFilter,
    pub(crate) mine_only: bool,
    pub(crate) sort_field: HypeUnstakingSortField,
    pub(crate) sort_direction: SortDirection,
}

impl fmt::Debug for HypeUnstakingQueueState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HypeUnstakingQueueState")
            .field(
                "data_events_len",
                &self.data.as_ref().map(|data| data.events.len()),
            )
            .field("loading", &self.loading)
            .field("error", &self.error.as_ref().map(|_| "<redacted>"))
            .field("last_fetch", &self.last_fetch)
            .field("refresh_request_id", &self.refresh_request_id)
            .field("window_filter", &self.window_filter)
            .field("amount_filter", &self.amount_filter)
            .field("mine_only", &self.mine_only)
            .field("sort_field", &self.sort_field)
            .field("sort_direction", &self.sort_direction)
            .finish()
    }
}

impl HypeUnstakingQueueState {
    pub(crate) fn clear_filters(&mut self) {
        self.window_filter = HypeUnstakingWindowFilter::default();
        self.amount_filter = HypeUnstakingAmountFilter::default();
        self.mine_only = false;
    }

    pub(crate) fn apply_sort_change(&mut self, field: HypeUnstakingSortField) {
        if self.sort_field == field {
            self.sort_direction = match self.sort_direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_field = field;
            self.sort_direction = field.default_direction();
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct HypeUnstakingQueueData {
    pub(crate) events: Vec<HypeUnstakingEvent>,
}

impl fmt::Debug for HypeUnstakingQueueData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HypeUnstakingQueueData")
            .field("events_len", &self.events.len())
            .field(
                "first_unlock_time_ms",
                &self.events.first().map(|event| event.unlock_time_ms),
            )
            .field(
                "last_unlock_time_ms",
                &self.events.last().map(|event| event.unlock_time_ms),
            )
            .finish()
    }
}

impl HypeUnstakingQueueData {
    pub(crate) fn new(mut events: Vec<HypeUnstakingEvent>) -> Self {
        events.sort_by_key(|event| event.unlock_time_ms);
        Self { events }
    }

    pub(crate) fn retain_upcoming_events(&mut self, now_ms: u64) {
        self.events.retain(|event| event.unlock_time_ms > now_ms);
    }

    pub(crate) fn filtered_events<'a>(
        &'a self,
        filter: HypeUnstakingFilter<'_>,
    ) -> Vec<&'a HypeUnstakingEvent> {
        let max_time_ms = filter.window.end_ms(filter.now_ms);
        let min_wei = filter.amount.min_wei();

        self.events
            .iter()
            .filter(|event| {
                event.unlock_time_ms > filter.now_ms
                    && max_time_ms.is_none_or(|max_time_ms| event.unlock_time_ms <= max_time_ms)
                    && (event.amount_wei as u128) >= min_wei
                    && filter
                        .mine_address
                        .is_none_or(|address| event.user.eq_ignore_ascii_case(address))
            })
            .collect()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct HypeUnstakingEvent {
    pub(crate) unlock_time_ms: u64,
    pub(crate) user: String,
    pub(crate) amount_wei: u64,
}

impl fmt::Debug for HypeUnstakingEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HypeUnstakingEvent")
            .field("unlock_time_ms", &self.unlock_time_ms)
            .field("user", &"<redacted>")
            .field("amount_wei", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct HypeUnstakingFilter<'a> {
    pub(crate) now_ms: u64,
    pub(crate) window: HypeUnstakingWindowFilter,
    pub(crate) amount: HypeUnstakingAmountFilter,
    pub(crate) mine_address: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct HypeUnstakingSummary {
    pub(crate) event_count: usize,
    pub(crate) unique_wallet_count: usize,
    pub(crate) total_wei: u128,
    pub(crate) next_unlock_time_ms: Option<u64>,
    pub(crate) largest_amount_wei: Option<u64>,
}

pub(crate) fn summarize_unstaking_events(events: &[&HypeUnstakingEvent]) -> HypeUnstakingSummary {
    let mut unique_wallets = HashSet::new();
    let mut total_wei = 0_u128;
    let mut next_unlock_time_ms = None;
    let mut largest_amount_wei = None;

    for event in events {
        unique_wallets.insert(event.user.to_ascii_lowercase());
        total_wei += event.amount_wei as u128;
        next_unlock_time_ms = Some(
            next_unlock_time_ms.map_or(event.unlock_time_ms, |next: u64| {
                next.min(event.unlock_time_ms)
            }),
        );
        largest_amount_wei = Some(largest_amount_wei.map_or(event.amount_wei, |largest: u64| {
            largest.max(event.amount_wei)
        }));
    }

    HypeUnstakingSummary {
        event_count: events.len(),
        unique_wallet_count: unique_wallets.len(),
        total_wei,
        next_unlock_time_ms,
        largest_amount_wei,
    }
}

pub(crate) fn sort_unstaking_events(
    events: &mut [&HypeUnstakingEvent],
    field: HypeUnstakingSortField,
    direction: SortDirection,
) {
    events.sort_by(|a, b| {
        let primary = match field {
            HypeUnstakingSortField::UnlockTime => a.unlock_time_ms.cmp(&b.unlock_time_ms),
            HypeUnstakingSortField::Amount => a.amount_wei.cmp(&b.amount_wei),
        };

        let ordered = match direction {
            SortDirection::Ascending => primary,
            SortDirection::Descending => primary.reverse(),
        };
        if ordered != Ordering::Equal {
            return ordered;
        }

        match field {
            HypeUnstakingSortField::UnlockTime => b
                .amount_wei
                .cmp(&a.amount_wei)
                .then_with(|| a.user.cmp(&b.user)),
            HypeUnstakingSortField::Amount => a
                .unlock_time_ms
                .cmp(&b.unlock_time_ms)
                .then_with(|| a.user.cmp(&b.user)),
        }
    });
}

pub(crate) fn format_hype_wei(wei: u128) -> String {
    if wei == 0 {
        return "0 HYPE".to_string();
    }

    let value = wei as f64 / HYPE_CORE_WEI_PER_TOKEN as f64;
    if value < 0.0001 {
        return "<0.0001 HYPE".to_string();
    }

    let decimals = if value >= 1_000.0 {
        0
    } else if value >= 1.0 {
        2
    } else {
        4
    };
    format!(
        "{} HYPE",
        trim_decimal_zeros(format_decimal_with_commas(value, decimals))
    )
}

pub(crate) fn format_countdown(unlock_time_ms: u64, now_ms: u64) -> String {
    if unlock_time_ms <= now_ms {
        return "Unlocked".to_string();
    }

    let mut seconds = unlock_time_ms.saturating_sub(now_ms) / 1_000;
    let days = seconds / 86_400;
    seconds %= 86_400;
    let hours = seconds / 3_600;
    seconds %= 3_600;
    let minutes = seconds / 60;
    seconds %= 60;

    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

#[cfg(test)]
mod tests;
