mod cancel;
mod close;
mod nuke;

use crate::account::AccountDataSection;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;

pub(crate) use nuke::NukePlan;

#[cfg(test)]
pub(crate) use nuke::{NukePositionOrder, NukeSkipReason};

pub(crate) fn reject_if_positions_incomplete_for_action(
    terminal: &mut TradingTerminal,
    action_label: &str,
) -> Option<Task<Message>> {
    if terminal.reject_if_account_reconciliation_required(action_label, "account data") {
        return Some(Task::none());
    }

    let message = {
        let (_, account_data) = terminal.connected_order_account_snapshot()?;
        if account_data.completeness.positions_complete {
            return None;
        }
        let detail = account_data
            .completeness
            .section_warning(AccountDataSection::Positions)
            .unwrap_or_else(|| {
                "Positions may be incomplete: refresh account data before relying on positions"
                    .to_string()
            });
        format!("{detail}; refresh before {action_label}")
    };
    terminal.order_status = Some((message, true));
    Some(terminal.refresh_account_data())
}
