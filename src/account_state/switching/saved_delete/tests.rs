use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config::AccountProfile;
use crate::signing::{ChaseLifecycle, ChaseOrder};

use std::time::Instant;

mod active_chase;
mod indexes;

fn account(secret_id: &str, name: &str, wallet_address: &str) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: name.to_string(),
        wallet_address: wallet_address.to_string(),
        agent_key: sensitive_string(format!("{secret_id}-agent-key")),
        hydromancer_api_key: sensitive_string(""),
    }
}

fn chase_order(account_address: &str) -> ChaseOrder {
    ChaseOrder {
        id: 42,
        coin: "BTC".to_string(),
        account_address: account_address.to_string(),
        agent_key: sensitive_string("old-account-agent-key"),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![1001],
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(1001),
        current_price: 50_000.0,
        current_price_wire: "50000".to_string(),
        initial_price: 50_000.0,
        started_at: Instant::now(),
        started_at_ms: 1,
        fill_cutoff_ms_by_oid: Vec::new(),
        reprice_count: 0,
        lifecycle: ChaseLifecycle::Resting,
        last_reprice_at: None,
        desired_price: None,
        stop_reason: None,
        cancel_retries: 0,
    }
}

fn last_toast_or_panic(terminal: &TradingTerminal) -> &crate::notification_state::Toast {
    match terminal.toasts.last() {
        Some(toast) => toast,
        None => panic!("blocked delete should toast"),
    }
}
