use super::*;

#[test]
fn active_account_delete_is_blocked_while_chase_order_is_active() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.desktop_notifications = false;
    terminal.accounts = vec![
        account(
            "account-a",
            "Account A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        account(
            "account-b",
            "Account B",
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
    ];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    terminal.chase_orders.insert(
        42,
        chase_order("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );

    let _task = terminal.delete_saved_account_task(0);

    assert_eq!(terminal.active_account_index, 0);
    assert_eq!(terminal.accounts.len(), 2);
    assert_eq!(terminal.accounts[0].secret_id, "account-a");
    assert_eq!(
        terminal.wallet_address_input,
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert!(terminal.chase_orders.contains_key(&42));
    let toast = last_toast_or_panic(&terminal);
    assert!(toast.is_error);
    assert!(toast.message.contains("Stop active chase orders"));
}
