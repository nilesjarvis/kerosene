use super::*;
use crate::account::{ClearinghouseState, MarginSummary, SpotClearinghouseState};

mod refresh;

const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

fn empty_details() -> WalletDetailsData {
    WalletDetailsData {
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "0".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "0".to_string(),
            asset_positions: Vec::new(),
        },
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        positions: Vec::new(),
        open_orders: Vec::new(),
        fills: Vec::new(),
        warnings: Vec::new(),
        fetched_at_ms: 0,
    }
}

fn member_data_at(
    address: &str,
    positions_refreshed_ms: Option<u64>,
    stale: bool,
) -> WalletClusterMemberData {
    WalletClusterMemberData {
        address: address.to_string(),
        positions_refreshed_ms,
        stale,
        ..WalletClusterMemberData::default()
    }
}

#[test]
fn snapshot_freshness_requires_recent_positions_not_stale() {
    let now = 1_000_000;
    let recent = now - 1_000;
    let old = now - (AccountData::POSITION_ACTION_MAX_AGE_MS + 1);

    // No position timestamp -> never fresh.
    assert!(!cluster_member_snapshot_is_fresh(
        &member_data_at(ADDRESS, None, false),
        now
    ));
    // Recent positions but no data -> not fresh.
    assert!(!cluster_member_snapshot_is_fresh(
        &member_data_at(ADDRESS, Some(recent), false),
        now
    ));
    // Old positions -> not fresh.
    let mut old_with_data = member_data_at(ADDRESS, Some(old), false);
    old_with_data.data = Some(empty_details());
    assert!(!cluster_member_snapshot_is_fresh(&old_with_data, now));
    // Recent positions + data + not stale -> fresh.
    let mut fresh = member_data_at(ADDRESS, Some(recent), false);
    fresh.data = Some(empty_details());
    assert!(cluster_member_snapshot_is_fresh(&fresh, now));
    // Stale flag overrides recent positions.
    fresh.stale = true;
    assert!(!cluster_member_snapshot_is_fresh(&fresh, now));
}

#[test]
fn open_orders_ws_frame_does_not_refresh_position_freshness() {
    // Regression for the freshness-gate defeat: a non-position frame
    // (open orders) must NOT bump positions_refreshed_ms or clear the
    // stale flag the close gate depends on. The timestamp/stale bookkeeping
    // runs after the optional `details` update, so it is independent of
    // whether `data` is populated.
    let mut terminal = TradingTerminal::boot().0;
    let stale_ts = 123;
    terminal.wallet_clusters.member_data.insert(
        "member".to_string(),
        member_data_at(ADDRESS, Some(stale_ts), true),
    );

    let _ = terminal.apply_wallet_cluster_ws_update(
        Some(ADDRESS.to_string()),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: Vec::new(),
        },
    );

    let state = &terminal.wallet_clusters.member_data["member"];
    assert_eq!(state.positions_refreshed_ms, Some(stale_ts));
    assert!(
        state.stale,
        "open-orders frame must not clear the stale flag"
    );
}

#[test]
fn lagged_ws_frame_invalidates_position_freshness_through_optimistic_refresh() {
    use crate::config::AccountProfile;
    use crate::wallet_cluster_state::{WalletCluster, WalletClusterMember};

    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![AccountProfile {
        master_address: None,
        secret_id: "member-profile".to_string(),
        name: "Member".to_string(),
        wallet_address: ADDRESS.to_string(),
        agent_key: "agent-key".to_string().into(),
        hydromancer_api_key: String::new().into(),
    }];
    terminal.wallet_clusters.clusters = vec![WalletCluster {
        id: "cluster".to_string(),
        name: "Cluster".to_string(),
        members: vec![WalletClusterMember {
            profile_secret_id: "member-profile".to_string(),
            weight: 1.0,
            weight_input: "1".to_string(),
        }],
    }];
    terminal.wallet_clusters.selected_cluster_id = Some("cluster".to_string());
    // Member currently looks fresh (recent positions, not stale).
    let mut data = member_data_at(ADDRESS, Some(1_000_000), false);
    data.data = Some(empty_details());
    terminal
        .wallet_clusters
        .member_data
        .insert("member-profile".to_string(), data);

    let _ = terminal.apply_wallet_cluster_ws_update(
        Some(ADDRESS.to_string()),
        WsUserData::Lagged { skipped: 7 },
    );

    // The lag-triggered refresh clears `stale` optimistically, but the
    // position timestamp is invalidated so the freshness gate stays closed
    // (a close / reduce-only re-checks before acting) until fresh data
    // actually lands.
    let state = &terminal.wallet_clusters.member_data["member-profile"];
    assert_eq!(state.positions_refreshed_ms, None);
    assert!(!cluster_member_snapshot_is_fresh(state, 1_000_000));
}
