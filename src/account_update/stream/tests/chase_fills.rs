use super::fixtures::{
    account_data_with_timestamp, chase_order, chase_order_by_id, fill_with_oid, open_order,
};
use crate::account::{OpenOrder, UserFill};
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseOrder, ChaseStopPhase, ChaseVerificationReason};

mod completion;
mod progress;

const CONNECTED_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

fn terminal_with_chase_fills(chase: ChaseOrder, fills: Vec<UserFill>) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(chase.account_address.clone());
    terminal.chase_orders.insert(1, chase);
    let mut data = account_data_with_timestamp(1_000);
    data.fills = fills;
    terminal.account_data_address = terminal.connected_address.clone();
    terminal.account_data = Some(data);
    terminal
}

fn connected_terminal_with_chase_account(
    chase: ChaseOrder,
    fills: Vec<UserFill>,
    open_orders: Vec<OpenOrder>,
) -> TradingTerminal {
    let mut terminal = terminal_with_chase_fills(chase, fills);
    terminal.connected_address = Some(CONNECTED_ADDRESS.to_string());
    if let Some(data) = terminal.account_data.as_mut() {
        data.open_orders = open_orders;
    }
    terminal
}
