mod instance;
mod model;

#[cfg(test)]
use crate::config;

pub(crate) use instance::PositioningInfoInstance;
pub(crate) use model::{
    PositioningInfoChangeSortField, PositioningInfoChangeTimeframe, PositioningInfoPage,
    PositioningInfoSide, PositioningInfoSortField, default_positioning_change_sort_direction,
    default_positioning_sort_direction,
};

// ---------------------------------------------------------------------------
// Positioning Information State
// ---------------------------------------------------------------------------

pub(crate) type PositioningInfoId = u64;

pub(crate) const POSITIONING_INFO_LIMIT: u32 = 30;
pub(crate) const POSITIONING_INFO_OFFSET: u32 = 0;
pub(crate) const POSITIONING_CHANGE_ROW_LIMIT: usize = 500;

#[cfg(test)]
mod tests;
