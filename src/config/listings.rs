use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const MAX_LISTING_EVENTS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ListingKind {
    Perp,
    Spot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ListingEvent {
    pub id: String,
    pub key: String,
    pub label: String,
    pub kind: ListingKind,
    pub detected_at_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ListingUniverse {
    pub initialized: bool,
    /// True means previously active, already announced, or present at baseline.
    /// False tracks a newly discovered inactive perp awaiting activation.
    pub seen: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ListingsHistory {
    pub perps: ListingUniverse,
    pub spot: ListingUniverse,
    pub events: Vec<ListingEvent>,
}
