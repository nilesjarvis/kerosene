use super::*;
use crate::config::{AccountProfile, KeroseneConfig};
use crate::wallet_cluster_state::{WalletCluster, WalletClusterMember};

fn terminal_with_members(ids: &[&str]) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.accounts = [
        ("valid", format!(" {ADDRESS} ")),
        ("invalid", "bad address".into()),
        ("outside", ADDRESS.into()),
    ]
    .into_iter()
    .map(|(id, address)| AccountProfile {
        secret_id: id.into(),
        name: id.into(),
        wallet_address: address,
        master_address: None,
        agent_key: String::new().into(),
        hydromancer_api_key: String::new().into(),
    })
    .collect();
    terminal.wallet_clusters.clusters = vec![WalletCluster {
        id: "cluster".into(),
        name: "Keep name".into(),
        members: ids
            .iter()
            .map(|id| WalletClusterMember {
                profile_secret_id: (*id).into(),
                weight: 0.0,
                weight_input: "keep draft".into(),
            })
            .collect(),
    }];
    terminal.wallet_clusters.selected_cluster_id = Some("cluster".into());
    terminal.wallet_clusters.status = Some(("previous status".into(), false));
    let previous_context = terminal.read_data_request_context();
    for id in ["valid", "invalid", "missing", "outside"] {
        let mut data = empty_details();
        data.fetched_at_ms = 42;
        data.warnings = vec!["cached warning".into()];
        terminal.wallet_clusters.member_data.insert(
            id.into(),
            WalletClusterMemberData {
                address: "previous address".into(),
                data: Some(data),
                loading: true,
                loading_context: Some(previous_context),
                error: Some("previous error".into()),
                positions_refreshed_ms: Some(42),
                stale: true,
            },
        );
    }
    terminal.read_data_provider_generation += 1;
    terminal
}

fn assert_unchanged(state: &WalletClusterMemberData) {
    assert_eq!(state.address, "previous address");
    assert!(state.loading);
    assert_eq!(state.error.as_deref(), Some("previous error"));
    assert!(state.stale);
    assert_eq!(state.positions_refreshed_ms, Some(42));
    let data = state.data.as_ref().expect("cached data");
    assert_eq!(data.fetched_at_ms, 42);
    assert_eq!(data.warnings, ["cached warning"]);
}

fn assert_refreshing(terminal: &TradingTerminal) {
    let state = &terminal.wallet_clusters.member_data["valid"];
    assert_eq!(state.address, ADDRESS);
    assert!(state.loading);
    assert_eq!(
        state.loading_context,
        Some(terminal.read_data_request_context())
    );
    assert_eq!(state.error, None);
    assert!(!state.stale);
    assert_eq!(state.positions_refreshed_ms, Some(42));
    let data = state.data.as_ref().expect("cached data");
    assert_eq!(data.fetched_at_ms, 42);
    assert_eq!(data.warnings, ["cached warning"]);
}

fn assert_invalid(state: &WalletClusterMemberData) {
    assert!(state.address.is_empty());
    assert!(state.data.is_none());
    assert!(!state.loading);
    assert_eq!(state.loading_context, None);
    assert_eq!(
        state.error.as_deref(),
        Some("Profile is missing a valid wallet address")
    );
    assert_eq!(state.positions_refreshed_ms, None);
    assert!(!state.stale);
}

#[test]
fn cluster_refresh_keeps_repeated_members_and_cached_data_without_weight_or_loading_gates() {
    let ids = ["missing", "valid", "invalid", "missing", "valid"];
    let mut terminal = terminal_with_members(&ids);
    let task = terminal.refresh_selected_wallet_cluster();
    assert_eq!(task.units(), 2);
    assert_refreshing(&terminal);
    assert_invalid(&terminal.wallet_clusters.member_data["invalid"]);
    assert!(!terminal.wallet_clusters.member_data.contains_key("missing"));
    assert_unchanged(&terminal.wallet_clusters.member_data["outside"]);
    assert_eq!(
        terminal.wallet_clusters.status,
        Some(("2 cluster member profiles were removed".into(), true))
    );
    let cluster = terminal
        .wallet_clusters
        .selected_cluster()
        .expect("cluster");
    assert_eq!(cluster.name, "Keep name");
    assert_eq!(
        cluster
            .members
            .iter()
            .map(|m| m.profile_secret_id.as_str())
            .collect::<Vec<_>>(),
        ids
    );
    assert!(
        cluster
            .members
            .iter()
            .all(|m| m.weight == 0.0 && m.weight_input == "keep draft")
    );
}

#[test]
fn cluster_member_refresh_selects_every_matching_entry_and_leaves_other_rows_alone() {
    for target in ["valid", "invalid", "missing"] {
        let mut terminal =
            terminal_with_members(&["missing", "valid", "invalid", "missing", "valid"]);
        let task = terminal.refresh_wallet_cluster_member(target.into());
        assert_eq!(task.units(), if target == "valid" { 2 } else { 0 });
        match target {
            "valid" => assert_refreshing(&terminal),
            "invalid" => assert_invalid(&terminal.wallet_clusters.member_data[target]),
            "missing" => assert!(!terminal.wallet_clusters.member_data.contains_key(target)),
            _ => unreachable!("test cases"),
        }
        for id in ["valid", "invalid", "missing", "outside"] {
            if id != target {
                assert_unchanged(&terminal.wallet_clusters.member_data[id]);
            }
        }
        assert_eq!(
            terminal.wallet_clusters.status,
            Some(if target == "missing" {
                ("2 cluster member profiles were removed".into(), true)
            } else {
                ("previous status".into(), false)
            })
        );
    }
}

#[test]
fn cluster_refresh_without_a_selected_member_preserves_rows_and_status() {
    for selection in [None, Some("unknown"), Some("cluster")] {
        for single in [false, true] {
            let mut terminal = terminal_with_members(&[]);
            terminal.wallet_clusters.selected_cluster_id = selection.map(str::to_string);
            let task = if single {
                terminal.refresh_wallet_cluster_member("outside".into())
            } else {
                terminal.refresh_selected_wallet_cluster()
            };
            assert_eq!(task.units(), 0);
            for state in terminal.wallet_clusters.member_data.values() {
                assert_unchanged(state);
            }
            assert_eq!(
                terminal.wallet_clusters.status,
                Some(("previous status".into(), false))
            );
        }
    }
}

#[test]
fn cluster_refresh_counts_missing_entries_without_editing_membership() {
    for count in [1, 2] {
        let mut terminal = terminal_with_members(&vec!["missing"; count]);
        let task = terminal.refresh_selected_wallet_cluster();
        assert_eq!(task.units(), 0);
        let expected = if count == 1 {
            "1 cluster member profile was removed"
        } else {
            "2 cluster member profiles were removed"
        };
        assert_eq!(
            terminal
                .wallet_clusters
                .status
                .as_ref()
                .map(|(message, error)| (message.as_str(), *error)),
            Some((expected, true))
        );
        assert_eq!(terminal.wallet_clusters.clusters[0].members.len(), count);
    }
}
