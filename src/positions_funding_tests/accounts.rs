use crate::app_state::TradingTerminal;
use crate::config::{self, AccountProfile};
use crate::journal;
use std::collections::{HashMap, HashSet};

fn account_profile(secret_id: &str, name: &str, wallet_address: &str) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: name.to_string(),
        wallet_address: wallet_address.to_string(),
        agent_key: String::new().into(),
        hydromancer_api_key: String::new().into(),
    }
}

#[test]
fn ghost_accounts_are_excluded_from_persisted_account_snapshot() {
    let mut main = account_profile("main", "Main", "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    main.agent_key = "key".to_string().into();
    let ghost = account_profile(
        "ghost",
        "Ghost: Beta",
        "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    let mut ghost_ids = HashSet::new();
    ghost_ids.insert(ghost.secret_id.clone());

    let persisted = TradingTerminal::persisted_accounts_from(&[main.clone(), ghost], &ghost_ids);

    assert_eq!(persisted, vec![main]);
}

#[test]
fn ghost_active_account_saves_last_persisted_account_index() {
    let main = account_profile("main", "Main", "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let secondary = account_profile(
        "secondary",
        "Secondary",
        "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    let persisted_accounts = vec![main, secondary];

    let active_index = TradingTerminal::persisted_active_account_index_from_ids(
        &persisted_accounts,
        None,
        Some("secondary"),
    );

    assert_eq!(active_index, 1);
}

#[test]
fn ghost_journal_entries_are_excluded_from_persisted_snapshot() {
    let mut journal = journal::JournalState::new_for_account(
        Some("ghost".to_string()),
        HashMap::new(),
        HashMap::new(),
    );
    journal.entries.insert(
        "trade-a".to_string(),
        journal::JournalNote {
            open: "session only".to_string(),
            close: String::new(),
        },
    );
    journal.account_states.insert(
        "main".to_string(),
        journal::JournalAccountState {
            entries: HashMap::from([(
                "trade-b".to_string(),
                journal::JournalNote {
                    open: "persisted".to_string(),
                    close: String::new(),
                },
            )]),
            ..journal::JournalAccountState::default()
        },
    );
    let mut ghost_ids = HashSet::new();
    ghost_ids.insert("ghost".to_string());

    let persisted =
        TradingTerminal::persisted_journal_entries_by_account_from(&journal, &ghost_ids);

    assert!(!persisted.contains_key("ghost"));
    assert_eq!(
        persisted
            .get("main")
            .and_then(|entries| entries.get("trade-b"))
            .map(|entry| entry.open.as_str()),
        Some("persisted")
    );
}

#[test]
fn encrypted_credentials_popup_only_needed_when_saved_credentials_are_locked() {
    assert!(TradingTerminal::encrypted_credentials_locked_for(
        config::CredentialStorageMode::EncryptedConfig,
        true,
        false,
    ));
    assert!(!TradingTerminal::encrypted_credentials_locked_for(
        config::CredentialStorageMode::EncryptedConfig,
        true,
        true,
    ));
    assert!(!TradingTerminal::encrypted_credentials_locked_for(
        config::CredentialStorageMode::EncryptedConfig,
        false,
        false,
    ));
    assert!(!TradingTerminal::encrypted_credentials_locked_for(
        config::CredentialStorageMode::OsKeychain,
        true,
        false,
    ));
}
