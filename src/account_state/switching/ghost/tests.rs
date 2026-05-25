use super::*;

use zeroize::Zeroizing;

const WALLET: &str = "0x1111111111111111111111111111111111111111";

fn account(secret_id: &str, name: &str, wallet_address: &str, agent_key: &str) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: name.to_string(),
        wallet_address: wallet_address.to_string(),
        agent_key: Zeroizing::new(agent_key.to_string()),
        hydromancer_api_key: Zeroizing::new(String::new()),
    }
}

#[test]
fn ghost_wallet_lookup_ignores_saved_trading_profile_with_same_address() {
    let accounts = vec![account("saved", "Saved", WALLET, "agent-key")];
    let ghost_account_secret_ids = HashSet::new();

    assert_eq!(
        find_ghost_account_index(&accounts, &ghost_account_secret_ids, WALLET),
        None
    );
}

#[test]
fn ghost_wallet_lookup_reuses_existing_ghost_profile() {
    let accounts = vec![
        account("saved", "Saved", WALLET, "agent-key"),
        account("ghost", "Ghost", WALLET, ""),
    ];
    let ghost_account_secret_ids = HashSet::from(["ghost".to_string()]);

    assert_eq!(
        find_ghost_account_index(&accounts, &ghost_account_secret_ids, WALLET),
        Some(1)
    );
}
