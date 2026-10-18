use crate::config::updates::{
    AssetParameters, MAX_UPDATE_EVENTS, ParameterChange, ParameterDecimal, ParameterEvent,
    ParameterKind, UpdatesHistory,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    time::Instant,
};

pub(crate) const UPDATES_REFRESH_SECS: u64 = 30;
pub(crate) const UPDATES_STALE_MS: u64 = 90_000;

#[derive(Debug, Clone)]
pub(crate) struct UpdatesSnapshot {
    pub parameters: Result<BTreeMap<String, AssetParameters>, String>,
    pub caps: Result<BTreeMap<String, ParameterDecimal>, String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum UpdatesFilter {
    #[default]
    All,
    Kind(ParameterKind),
}

impl fmt::Display for UpdatesFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::All => "All changes",
            Self::Kind(kind) => kind.label(),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum UpdatesVenue {
    #[default]
    All,
    Native,
    Dex(String),
}

impl fmt::Display for UpdatesVenue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::All => "All DEXes",
            Self::Native => "Hyperliquid",
            Self::Dex(dex) => dex,
        })
    }
}

#[derive(Default)]
pub(crate) struct UpdatesState {
    pub history: UpdatesHistory,
    pub filter: UpdatesFilter,
    pub venue: UpdatesVenue,
    pub search: String,
    pub favourites_only: bool,
    pub expanded: BTreeSet<(u64, String)>,
    pub loading: bool,
    pub request_id: u64,
    pub last_attempt: Option<Instant>,
    pub failures: u8,
    pub error: Option<&'static str>,
    pub storage_error: bool,
    pub dirty: bool,
    pub saving: bool,
    pub last_save_attempt: Option<Instant>,
    pub now_ms: u64,
    parameters_continuous: bool,
    caps_continuous: bool,
}

impl UpdatesState {
    pub fn load() -> Self {
        match crate::config_persistence::updates::load() {
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

    pub fn refresh_secs(&self) -> u64 {
        (UPDATES_REFRESH_SECS * (1 << self.failures.saturating_sub(1).min(4))).min(300)
    }

    /// A family advances only after a fully validated response. New assets and
    /// reappearing unpublished fields seed silently; absence never means zero.
    pub fn apply(&mut self, snapshot: UpdatesSnapshot, now_ms: u64) {
        let parameters_failed = snapshot.parameters.is_err();
        let caps_failed = snapshot.caps.is_err();
        self.error = match (parameters_failed, caps_failed) {
            (true, true) => Some("Feed unavailable · retrying"),
            (true, false) => Some("Parameters unavailable · retrying"),
            (false, true) => Some("OI caps unavailable · retrying"),
            (false, false) => None,
        };
        self.failures = if self.error.is_some() {
            self.failures.saturating_add(1)
        } else {
            0
        };
        let mut events: BTreeMap<String, ParameterEvent> = BTreeMap::new();
        if let Ok(parameters) = snapshot.parameters {
            if let Some(previous_ms) = self.history.parameters_observed_at_ms {
                for (key, new) in &parameters {
                    let Some(old) = self.history.parameters.get(key) else {
                        continue;
                    };
                    let mut changes = Vec::new();
                    if old.max_leverage != new.max_leverage {
                        changes.push(ParameterChange::MaxLeverage {
                            old: old.max_leverage,
                            new: new.max_leverage,
                        });
                    }
                    // Suppress only a flat table that repeats the leverage change.
                    let repeats_leverage = old.max_leverage != new.max_leverage
                        && old.margin_tiers.len() == 1
                        && new.margin_tiers.len() == 1
                        && old.margin_tiers[0].max_leverage == old.max_leverage
                        && new.margin_tiers[0].max_leverage == new.max_leverage;
                    if old.margin_tiers != new.margin_tiers && !repeats_leverage {
                        changes.push(ParameterChange::MarginTiers {
                            old: old.margin_tiers.clone(),
                            new: new.margin_tiers.clone(),
                        });
                    }
                    if let (Some(old), Some(new)) = (old.margin_mode, new.margin_mode)
                        && old != new
                    {
                        changes.push(ParameterChange::MarginMode { old, new });
                    }
                    if !changes.is_empty() {
                        events.insert(
                            key.clone(),
                            ParameterEvent {
                                key: key.clone(),
                                detected_at_ms: now_ms,
                                previous_observed_at_ms: previous_ms,
                                after_gap: observation_gap(
                                    self.parameters_continuous,
                                    previous_ms,
                                    now_ms,
                                ),
                                changes,
                            },
                        );
                    }
                }
            }
            self.history.parameters = parameters;
            self.history.parameters_observed_at_ms = Some(now_ms);
            self.dirty = true;
        }
        if let Ok(caps) = snapshot.caps {
            if let Some(previous_ms) = self.history.caps_observed_at_ms {
                for (key, new) in &caps {
                    let Some(old) = self.history.caps.get(key) else {
                        continue;
                    };
                    if old == new {
                        continue;
                    }
                    let event = events.entry(key.clone()).or_insert_with(|| ParameterEvent {
                        key: key.clone(),
                        detected_at_ms: now_ms,
                        previous_observed_at_ms: previous_ms,
                        after_gap: false,
                        changes: Vec::new(),
                    });
                    event.previous_observed_at_ms = event.previous_observed_at_ms.min(previous_ms);
                    event.after_gap |= observation_gap(self.caps_continuous, previous_ms, now_ms);
                    event.changes.push(ParameterChange::OiCap {
                        old: old.clone(),
                        new: new.clone(),
                    });
                }
            }
            self.history.caps = caps;
            self.history.caps_observed_at_ms = Some(now_ms);
            self.dirty = true;
        }
        self.parameters_continuous = !parameters_failed;
        self.caps_continuous = !caps_failed;
        self.history.events.extend(events.into_values());
        self.history.events.sort_by(|a, b| {
            b.detected_at_ms
                .cmp(&a.detected_at_ms)
                .then_with(|| a.key.cmp(&b.key))
        });
        self.history.events.truncate(MAX_UPDATE_EVENTS);
        self.expanded.retain(|(at, key)| {
            self.history
                .events
                .iter()
                .any(|event| event.detected_at_ms == *at && event.key == *key)
        });
    }

    pub fn matches(&self, event: &ParameterEvent, favourite: bool) -> bool {
        let venue_matches = match &self.venue {
            UpdatesVenue::All => true,
            UpdatesVenue::Native => !event.key.contains(':'),
            UpdatesVenue::Dex(dex) => event
                .key
                .split_once(':')
                .is_some_and(|(prefix, _)| prefix == dex),
        };
        venue_matches
            && (!self.favourites_only || favourite)
            && event
                .key
                .to_ascii_lowercase()
                .contains(&self.search.trim().to_ascii_lowercase())
            && event.changes.iter().any(|change| self.includes(change))
    }

    pub fn includes(&self, change: &ParameterChange) -> bool {
        self.filter == UpdatesFilter::All || self.filter == UpdatesFilter::Kind(change.kind())
    }
}

fn observation_gap(continuous: bool, previous_ms: u64, now_ms: u64) -> bool {
    !continuous || previous_ms > now_ms || now_ms.saturating_sub(previous_ms) > UPDATES_STALE_MS
}

#[cfg(test)]
mod tests;
