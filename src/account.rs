mod data;
mod http;
mod position_metrics;
mod spot;
mod types;
mod wallets;

pub(crate) use data::{
    HydromancerPortfolioState, fetch_hydromancer_frontend_open_orders_scoped,
    fetch_hydromancer_portfolio_state, fetch_hydromancer_portfolio_states,
    hydromancer_portfolio_chunk_size,
};
pub use data::{fetch_account_data_scoped_with_provider, fetch_all_mids};
pub(crate) use position_metrics::{
    position_notional_from_mark_or_wire, position_upnl_from_mark_or_wire,
};
pub use types::*;
pub use wallets::{
    fetch_wallet_details_scoped_with_provider,
    fetch_wallet_tracker_open_order_count_scoped_with_provider,
    fetch_wallet_tracker_snapshot_scoped_with_provider,
    fetch_wallet_tracker_snapshots_scoped_with_provider,
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
