use crate::api::OUTCOME_ASSET_ID_OFFSET;

#[cfg(test)]
mod tests;

pub(super) fn outcome_encoding(outcome: u32, side: u32) -> u32 {
    outcome.saturating_mul(10).saturating_add(side)
}

pub(super) fn outcome_coin_key(encoding: u32) -> String {
    format!("#{encoding}")
}

pub(super) fn outcome_asset_index(encoding: u32) -> u32 {
    OUTCOME_ASSET_ID_OFFSET.saturating_add(encoding)
}
