use super::*;

mod aggregation;

const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

fn position_member(profile: &str, dex: &str, size: f64) -> WalletClusterPositionMember {
    WalletClusterPositionMember {
        profile_secret_id: profile.to_string(),
        address: ADDRESS.to_string(),
        label: profile.to_string(),
        dex: dex.to_string(),
        size,
        entry_price: None,
        value: None,
        unrealized_pnl: None,
    }
}

fn summary(
    symbol: &str,
    members: Vec<WalletClusterPositionMember>,
) -> WalletClusterPositionSummary {
    WalletClusterPositionSummary {
        symbol: symbol.to_string(),
        net_size: 0.0,
        long_size: 0.0,
        short_size: 0.0,
        value: None,
        unrealized_pnl: None,
        members,
    }
}

#[test]
fn cluster_close_size_sums_same_side_positions_across_dexes() {
    let summaries = vec![summary(
        "BTC",
        vec![
            position_member("m1", "", 2.0),
            position_member("m1", "builder", 3.0),
            position_member("m2", "", -4.0),
        ],
    )];

    // m1 is long 2 + 3 across two dexes; full close -> 5, half close -> 2.5.
    assert_eq!(
        cluster_close_size_for_member(&summaries, "BTC", "m1", WalletClusterCloseSide::Long, 1.0),
        Some(5.0)
    );
    assert_eq!(
        cluster_close_size_for_member(&summaries, "BTC", "m1", WalletClusterCloseSide::Long, 0.5),
        Some(2.5)
    );
    // m1 has no short; m2 has no long.
    assert_eq!(
        cluster_close_size_for_member(&summaries, "BTC", "m1", WalletClusterCloseSide::Short, 1.0),
        None
    );
    assert_eq!(
        cluster_close_size_for_member(&summaries, "BTC", "m2", WalletClusterCloseSide::Short, 1.0),
        Some(4.0)
    );
    assert_eq!(
        cluster_close_size_for_member(&summaries, "BTC", "m2", WalletClusterCloseSide::Long, 1.0),
        None
    );
    // Unknown member or symbol -> None.
    assert_eq!(
        cluster_close_size_for_member(
            &summaries,
            "BTC",
            "ghost",
            WalletClusterCloseSide::Long,
            1.0
        ),
        None
    );
    assert_eq!(
        cluster_close_size_for_member(&summaries, "ETH", "m1", WalletClusterCloseSide::Long, 1.0),
        None
    );
}

#[test]
fn cluster_close_size_filters_to_requested_side_when_member_is_hedged() {
    // Same member + symbol, hedged across dexes: long on main, short on builder.
    let summaries = vec![summary(
        "BTC",
        vec![
            position_member("m1", "", 2.0),
            position_member("m1", "builder", -1.0),
        ],
    )];
    assert_eq!(
        cluster_close_size_for_member(&summaries, "BTC", "m1", WalletClusterCloseSide::Long, 1.0),
        Some(2.0)
    );
    assert_eq!(
        cluster_close_size_for_member(&summaries, "BTC", "m1", WalletClusterCloseSide::Short, 1.0),
        Some(1.0)
    );
}
