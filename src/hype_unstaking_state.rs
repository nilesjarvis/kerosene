use crate::config::SortDirection;
use crate::helpers::{format_decimal_with_commas, trim_decimal_zeros};

use std::cmp::Ordering;
use std::collections::HashSet;
use std::time::Instant;

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

#[derive(Debug, Clone, Default)]
pub(crate) struct HypeUnstakingQueueState {
    pub(crate) data: Option<HypeUnstakingQueueData>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) last_fetch: Option<Instant>,
    pub(crate) window_filter: HypeUnstakingWindowFilter,
    pub(crate) amount_filter: HypeUnstakingAmountFilter,
    pub(crate) mine_only: bool,
    pub(crate) sort_field: HypeUnstakingSortField,
    pub(crate) sort_direction: SortDirection,
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

#[derive(Debug, Clone, Default)]
pub(crate) struct HypeUnstakingQueueData {
    pub(crate) events: Vec<HypeUnstakingEvent>,
}

impl HypeUnstakingQueueData {
    pub(crate) fn new(mut events: Vec<HypeUnstakingEvent>) -> Self {
        events.sort_by_key(|event| event.unlock_time_ms);
        Self { events }
    }

    pub(crate) fn filtered_events<'a>(
        &'a self,
        filter: HypeUnstakingFilter<'_>,
    ) -> Vec<&'a HypeUnstakingEvent> {
        let mine_address = filter.mine_address.map(str::to_ascii_lowercase);
        let max_time_ms = filter.window.end_ms(filter.now_ms);
        let min_wei = filter.amount.min_wei();

        self.events
            .iter()
            .filter(|event| {
                event.unlock_time_ms > filter.now_ms
                    && max_time_ms.is_none_or(|max_time_ms| event.unlock_time_ms <= max_time_ms)
                    && (event.amount_wei as u128) >= min_wei
                    && mine_address
                        .as_ref()
                        .is_none_or(|address| event.user.eq_ignore_ascii_case(address))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HypeUnstakingEvent {
    pub(crate) unlock_time_ms: u64,
    pub(crate) user: String,
    pub(crate) amount_wei: u64,
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
mod tests {
    use super::*;

    fn event(unlock_time_ms: u64, user: &str, hype: u64) -> HypeUnstakingEvent {
        HypeUnstakingEvent {
            unlock_time_ms,
            user: user.to_string(),
            amount_wei: hype * HYPE_CORE_WEI_PER_TOKEN as u64,
        }
    }

    #[test]
    fn data_sorts_events_by_unlock_time() {
        let data = HypeUnstakingQueueData::new(vec![
            event(3_000, "0x3", 1),
            event(1_000, "0x1", 1),
            event(2_000, "0x2", 1),
        ]);

        assert_eq!(
            data.events
                .iter()
                .map(|event| event.unlock_time_ms)
                .collect::<Vec<_>>(),
            vec![1_000, 2_000, 3_000]
        );
    }

    #[test]
    fn filtering_excludes_past_events() {
        let data = HypeUnstakingQueueData::new(vec![
            event(900, "0xpast", 100),
            event(2_000, "0xfuture", 100),
        ]);

        let filtered = data.filtered_events(HypeUnstakingFilter {
            now_ms: 1_000,
            window: HypeUnstakingWindowFilter::All,
            amount: HypeUnstakingAmountFilter::All,
            mine_address: None,
        });

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].user, "0xfuture");
    }

    #[test]
    fn filtering_excludes_events_past_window_end() {
        let data = HypeUnstakingQueueData::new(vec![
            event(2_000, "0xinside", 100),
            event(3_700_000, "0xlate", 100),
        ]);

        let filtered = data.filtered_events(HypeUnstakingFilter {
            now_ms: 1_000,
            window: HypeUnstakingWindowFilter::OneHour,
            amount: HypeUnstakingAmountFilter::All,
            mine_address: None,
        });

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].user, "0xinside");
    }

    #[test]
    fn filtering_excludes_events_below_amount_floor() {
        let data = HypeUnstakingQueueData::new(vec![
            event(2_000, "0xsmall", 99),
            event(3_000, "0xbig", 100),
        ]);

        let filtered = data.filtered_events(HypeUnstakingFilter {
            now_ms: 1_000,
            window: HypeUnstakingWindowFilter::All,
            amount: HypeUnstakingAmountFilter::AtLeast100,
            mine_address: None,
        });

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].user, "0xbig");
    }

    #[test]
    fn filtering_supports_mine_only() {
        let data = HypeUnstakingQueueData::new(vec![
            event(2_000, "0xAAA111", 1),
            event(2_000, "0xBBB222", 1),
            event(2_000, "0xAAA333", 1),
        ]);

        let mine = data.filtered_events(HypeUnstakingFilter {
            now_ms: 1_000,
            window: HypeUnstakingWindowFilter::All,
            amount: HypeUnstakingAmountFilter::All,
            mine_address: Some("0xbbb222"),
        });
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].user, "0xBBB222");
    }

    #[test]
    fn summary_aggregates_filtered_events() {
        let first = event(2_000, "0xAAA", 10);
        let second = event(3_000, "0xaaa", 25);
        let events = vec![&first, &second];

        assert_eq!(
            summarize_unstaking_events(&events),
            HypeUnstakingSummary {
                event_count: 2,
                unique_wallet_count: 1,
                total_wei: 35 * HYPE_CORE_WEI_PER_TOKEN,
                next_unlock_time_ms: Some(2_000),
                largest_amount_wei: Some(25 * HYPE_CORE_WEI_PER_TOKEN as u64),
            }
        );
    }

    #[test]
    fn sort_change_amount_defaults_descending_and_toggles() {
        let mut state = HypeUnstakingQueueState::default();

        state.apply_sort_change(HypeUnstakingSortField::Amount);
        assert_eq!(state.sort_field, HypeUnstakingSortField::Amount);
        assert_eq!(state.sort_direction, SortDirection::Descending);

        state.apply_sort_change(HypeUnstakingSortField::Amount);
        assert_eq!(state.sort_direction, SortDirection::Ascending);
    }

    #[test]
    fn amount_sort_orders_full_filtered_set() {
        let small = event(2_000, "0xsmall", 10);
        let large = event(4_000, "0xlarge", 1_000);
        let mid = event(3_000, "0xmid", 100);
        let mut events = vec![&small, &mid, &large];

        sort_unstaking_events(
            events.as_mut_slice(),
            HypeUnstakingSortField::Amount,
            SortDirection::Descending,
        );

        assert_eq!(
            events
                .iter()
                .map(|event| event.user.as_str())
                .collect::<Vec<_>>(),
            vec!["0xlarge", "0xmid", "0xsmall"]
        );
    }

    #[test]
    fn formats_hype_wei_amounts() {
        assert_eq!(format_hype_wei(123_450_000_000), "1,234 HYPE");
        assert_eq!(format_hype_wei(150_000_000), "1.5 HYPE");
        assert_eq!(format_hype_wei(12_345), "0.0001 HYPE");
        assert_eq!(format_hype_wei(1), "<0.0001 HYPE");
    }

    #[test]
    fn formats_countdowns() {
        assert_eq!(format_countdown(1_000, 1_000), "Unlocked");
        assert_eq!(format_countdown(91_000, 1_000), "1m 30s");
        assert_eq!(format_countdown(3_661_000, 1_000), "1h 1m");
    }
}
