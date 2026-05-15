mod data;
mod http;
mod spot;
mod types;
mod wallets;

pub use data::{fetch_account_data_scoped, fetch_all_mids};
pub use types::*;
pub use wallets::{
    fetch_wallet_details_scoped, fetch_wallet_tracker_open_order_count_scoped,
    fetch_wallet_tracker_snapshot_scoped,
};

/// Known HIP-3 perp dex names. The main dex uses "" (empty string).
pub const HIP3_DEXES: &[&str] = &["xyz", "flx", "vntl", "hyna", "km", "abcd", "cash", "para"];

pub(crate) fn normalize_dex_open_order_coin(dex: &str, order: &mut OpenOrder) {
    if dex.is_empty() || order.coin.contains(':') {
        return;
    }
    order.coin = format!("{dex}:{}", order.coin);
}

pub(crate) fn normalize_dex_open_order_coins(dex: &str, orders: &mut [OpenOrder]) {
    for order in orders {
        normalize_dex_open_order_coin(dex, order);
    }
}

pub(crate) fn normalize_dex_asset_position_coin(dex: &str, position: &mut AssetPosition) {
    if dex.is_empty() || position.position.coin.contains(':') {
        return;
    }
    position.position.coin = format!("{dex}:{}", position.position.coin);
}

pub(crate) fn normalize_dex_asset_position_coins(dex: &str, positions: &mut [AssetPosition]) {
    for position in positions {
        normalize_dex_asset_position_coin(dex, position);
    }
}
