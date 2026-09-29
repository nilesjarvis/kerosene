use super::*;

const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

#[test]
fn add_member_rejects_profile_without_agent_key() {
    use crate::config::AccountProfile;
    use crate::wallet_cluster_state::WalletCluster;

    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![AccountProfile {
        master_address: None,
        secret_id: "keyless".to_string(),
        name: "Keyless".to_string(),
        wallet_address: ADDRESS.to_string(),
        agent_key: String::new().into(),
        hydromancer_api_key: String::new().into(),
    }];
    terminal.wallet_clusters.clusters = vec![WalletCluster {
        id: "cluster".to_string(),
        name: "Cluster".to_string(),
        members: Vec::new(),
    }];
    terminal.wallet_clusters.selected_cluster_id = Some("cluster".to_string());

    let _ = terminal.add_wallet_cluster_member("keyless".to_string());

    assert!(
        terminal.wallet_clusters.clusters[0].members.is_empty(),
        "keyless profile must not be added"
    );
    let (message, is_error) = terminal
        .wallet_clusters
        .status
        .as_ref()
        .expect("a rejection status should be set");
    assert!(is_error);
    assert!(message.contains("agent key"));
}
