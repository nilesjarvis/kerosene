use crate::config::listings::{ListingEvent, ListingKind, ListingsHistory, MAX_LISTING_EVENTS};
use std::time::Instant;

pub(crate) const LISTINGS_REFRESH_SECS: u64 = 30;

#[derive(Debug, Clone)]
pub(crate) struct ListingMarket {
    pub id: String,
    pub key: String,
    pub label: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ListingsSnapshot {
    pub perps: Result<Vec<ListingMarket>, String>,
    pub spot: Result<Vec<ListingMarket>, String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ListingsFilter {
    #[default]
    All,
    Perps,
    Spot,
}

impl ListingsFilter {
    pub const ALL: [Self; 3] = [Self::All, Self::Perps, Self::Spot];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Perps => "Perps",
            Self::Spot => "Spot",
        }
    }

    pub fn includes(self, kind: ListingKind) -> bool {
        matches!(
            (self, kind),
            (Self::All, _) | (Self::Perps, ListingKind::Perp) | (Self::Spot, ListingKind::Spot)
        )
    }
}

#[derive(Default)]
pub(crate) struct ListingsState {
    pub history: ListingsHistory,
    pub filter: ListingsFilter,
    pub loading: bool,
    pub request_id: u64,
    pub last_attempt: Option<Instant>,
    pub last_success_ms: Option<u64>,
    pub error: Option<&'static str>,
    pub storage_error: bool,
    pub dirty: bool,
    pub saving: bool,
    pub now_ms: u64,
}

impl ListingsState {
    pub fn load() -> Self {
        match crate::config_persistence::listings::load() {
            Ok(history) => Self {
                history,
                now_ms: crate::app_time::now_ms(),
                ..Self::default()
            },
            Err(_) => Self {
                storage_error: true,
                now_ms: crate::app_time::now_ms(),
                ..Self::default()
            },
        }
    }

    /// Accept only successful, live snapshots. Failed families keep their
    /// baseline; the first successful snapshot of each family seeds it silently.
    pub fn apply(&mut self, snapshot: ListingsSnapshot, now_ms: u64) -> bool {
        let perp_failed = snapshot.perps.is_err();
        let spot_failed = snapshot.spot.is_err();
        self.error = match (perp_failed, spot_failed) {
            (true, true) => Some("Feed unavailable · retrying"),
            (true, false) => Some("Perps unavailable · retrying"),
            (false, true) => Some("Spot unavailable · retrying"),
            (false, false) => None,
        };
        let mut added = false;
        for (kind, result) in [
            (ListingKind::Perp, snapshot.perps),
            (ListingKind::Spot, snapshot.spot),
        ] {
            let Ok(markets) = result else { continue };
            // Empty snapshots must never establish a baseline.
            if markets.is_empty() {
                continue;
            }
            let universe = match kind {
                ListingKind::Perp => &mut self.history.perps,
                ListingKind::Spot => &mut self.history.spot,
            };
            let baseline = !universe.initialized;
            for market in markets {
                let was_seen = universe.seen.get(&market.id).copied();
                if was_seen.is_none() || (was_seen == Some(false) && market.active) {
                    universe
                        .seen
                        .insert(market.id.clone(), baseline || market.active);
                    self.dirty = true;
                    if !baseline && market.active {
                        self.history.events.push(ListingEvent {
                            id: market.id,
                            key: market.key,
                            label: market.label,
                            kind,
                            detected_at_ms: now_ms,
                        });
                        added = true;
                    }
                }
            }
            self.dirty |= baseline;
            universe.initialized = true;
        }
        self.history.events.sort_by(|a, b| {
            b.detected_at_ms
                .cmp(&a.detected_at_ms)
                .then_with(|| a.id.cmp(&b.id))
        });
        self.history.events.truncate(MAX_LISTING_EVENTS);
        if !perp_failed && !spot_failed {
            self.last_success_ms = Some(now_ms);
        }
        added
    }
}

#[cfg(test)]
mod tests;
