use super::*;
use crate::config::{AccountProfile, KeroseneConfig};
use zeroize::Zeroizing;

const ACCOUNT: &str = "0x1111111111111111111111111111111111111111";
const BUILDER: &str = "0x2222222222222222222222222222222222222222";

fn terminal() -> TradingTerminal {
    let config = KeroseneConfig {
        accounts: vec![AccountProfile {
            secret_id: "builder-settings-test".into(),
            name: "Test".into(),
            wallet_address: ACCOUNT.into(),
            master_address: None,
            agent_key: Zeroizing::new("test-agent".into()),
            hydromancer_api_key: Zeroizing::new(String::new()),
        }],
        ..KeroseneConfig::default()
    };
    let mut terminal = TradingTerminal::boot_from_config(config).0;
    terminal.connected_address = Some(ACCOUNT.into());
    terminal.wallet_address_input = ACCOUNT.to_string();
    terminal
}

fn save_builder(terminal: &mut TradingTerminal) {
    let _ = terminal.update(Message::BuilderAddressChanged(BUILDER.to_string().into()));
    let _ = terminal.update(Message::BuilderFeeChanged("1.5".to_string().into()));
    let _ = terminal.update(Message::SaveBuilderCode);
}

#[test]
fn builder_settings_validate_before_saving_or_enabling() {
    let mut terminal = terminal();
    let _ = terminal.update(Message::SetBuilderCodeEnabled(true));
    assert!(!terminal.builder_codes.config.enabled);
    save_builder(&mut terminal);
    assert_eq!(terminal.builder_codes.config.address, BUILDER);
    assert_eq!(terminal.builder_codes.config.fee_tenths_bps, 15);
    assert!(!terminal.builder_codes.config.enabled);
    let _ = terminal.update(Message::SetBuilderCodeEnabled(true));
    assert!(terminal.builder_codes.config.enabled);
    let saved = terminal.builder_codes.config.clone();
    let _ = terminal.update(Message::BuilderFeeChanged("1.01".to_string().into()));
    let _ = terminal.update(Message::SaveBuilderCode);
    assert_eq!(terminal.builder_codes.config, saved);
    assert!(!terminal.builder_codes.draft_is_saved());
    let _ = terminal.update(Message::SetBuilderCodeEnabled(false));
    assert!(!terminal.builder_codes.config.enabled);
}

#[test]
fn builder_approval_callbacks_require_matching_account_generation_and_builder() {
    let mut terminal = terminal();
    save_builder(&mut terminal);
    let request = terminal
        .builder_codes
        .pending
        .clone()
        .expect("check pending");
    terminal.connected_address = Some(BUILDER.into());
    let _ = terminal.update(Message::BuilderApprovalChecked(request.clone(), Ok(100)));
    assert!(terminal.builder_codes.approval.is_none());
    terminal.connected_address = Some(ACCOUNT.into());
    terminal.builder_codes.invalidate();
    let _ = terminal.update(Message::BuilderApprovalChecked(request, Ok(100)));
    assert!(terminal.builder_codes.approval.is_none());
    let _ = terminal.refresh_builder_approval();
    let request = terminal
        .builder_codes
        .pending
        .clone()
        .expect("check pending");
    let _ = terminal.update(Message::BuilderApprovalChecked(request, Ok(14)));
    assert!(
        terminal
            .builder_codes
            .status
            .as_ref()
            .expect("approval required")
            .1
    );
    let _ = terminal.refresh_builder_approval();
    let request = terminal
        .builder_codes
        .pending
        .clone()
        .expect("check pending");
    let _ = terminal.update(Message::BuilderApprovalChecked(request, Ok(15)));
    assert!(terminal.builder_codes.status.is_none());
    assert_eq!(
        terminal
            .builder_codes
            .approval
            .as_ref()
            .expect("verified")
            .1,
        15
    );
}

#[test]
fn builder_wallet_callback_requires_independent_exchange_verification() {
    let mut terminal = terminal();
    save_builder(&mut terminal);
    terminal.builder_codes.invalidate();
    let request = terminal
        .new_builder_approval_request(true)
        .expect("wallet request");
    let _ = terminal.update(Message::BuilderWalletApprovalFinished(request, Ok(())));
    assert!(terminal.builder_codes.approval.is_none());
    assert!(
        !terminal
            .builder_codes
            .pending
            .as_ref()
            .expect("checking exchange")
            .wallet
    );
    assert!(!terminal.builder_codes.config.enabled);
}

#[test]
fn builder_wallet_approval_rejects_unsaved_drafts_and_subaccounts() {
    let mut terminal = terminal();
    save_builder(&mut terminal);
    terminal.builder_codes.invalidate();
    terminal.builder_codes.fee_input = "2".into();
    assert!(terminal.new_builder_approval_request(true).is_err());
    terminal.builder_codes.fee_input = "1.5".into();
    terminal.accounts[0].master_address = Some(BUILDER.into());
    assert!(terminal.new_builder_approval_request(true).is_err());
    assert!(terminal.builder_codes.pending.is_none());
}

#[test]
fn builder_callback_retires_pending_check_after_wallet_input_changes() {
    let mut terminal = terminal();
    save_builder(&mut terminal);
    let request = terminal
        .builder_codes
        .pending
        .clone()
        .expect("check pending");
    terminal.wallet_address_input = BUILDER.to_string();
    let _ = terminal.update(Message::BuilderApprovalChecked(request, Ok(100)));
    assert!(terminal.builder_codes.pending.is_none());
    assert!(terminal.builder_codes.approval.is_none());
}

#[test]
fn builder_configuration_persists_without_approval_runtime_state() {
    let mut terminal = terminal();
    save_builder(&mut terminal);
    let mut snapshot = None;
    terminal
        .persist_config_immediately_with(|config| {
            snapshot = Some(config.clone());
            Ok(())
        })
        .expect("snapshot");
    let config = snapshot.expect("saved");
    assert_eq!(config.builder_code, terminal.builder_codes.config);
    let reboot = TradingTerminal::boot_from_config(config).0;
    assert!(reboot.builder_codes.pending.is_none());
    assert!(reboot.builder_codes.approval.is_none());
    assert!(reboot.builder_codes.draft_is_saved());
}
