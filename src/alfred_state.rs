use crate::account::AccountDataSection;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::NukePlan;

mod catalog;
mod model;
mod position_close;
mod trading;
pub(crate) use model::{
    AlfredCommand, AlfredCommandId, AlfredCommandKind, AlfredSelectionStep, AlfredState,
};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Alfred state and command catalog
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn alfred_filtered_commands(&self) -> Vec<AlfredCommand> {
        let query = self.alfred.query.trim();
        if let Some(command) = self.alfred_nuke_command(query) {
            return vec![command];
        }
        if let Some(command) = self.alfred_close_position_command(query) {
            return vec![command];
        }
        if let Some(command) = self.alfred_trade_command(query) {
            return vec![command];
        }

        self.alfred_command_catalog()
            .into_iter()
            .filter(|command| command.matches_query(query))
            .collect()
    }

    pub(crate) fn alfred_command_by_id(&self, id: AlfredCommandId) -> Option<AlfredCommand> {
        if id == AlfredCommandId::NaturalLanguageTrading {
            return self.alfred_trade_command(self.alfred.query.trim());
        }
        if id == AlfredCommandId::NukePositions {
            return self.alfred_nuke_command(self.alfred.query.trim());
        }
        if id == AlfredCommandId::ClosePosition {
            return self.alfred_close_position_command(self.alfred.query.trim());
        }

        self.alfred_command_catalog()
            .into_iter()
            .find(|command| command.id == id)
    }

    fn alfred_close_position_command(&self, query: &str) -> Option<AlfredCommand> {
        let draft = self.alfred_close_position_draft(query)?;
        let mut command = AlfredCommand::new(
            AlfredCommandId::ClosePosition,
            "Close Position",
            "Close an open position at market",
            "Close",
            AlfredCommandKind::Trading,
            None,
            &["close", "flatten", "position", "market"],
        )
        .with_dynamic_text(draft.title.clone(), draft.detail.clone(), draft.tag.clone());

        if draft.can_submit() {
            command.message = Some(Message::AlfredSubmit);
        } else if let Some(error) = draft.error {
            command = command.disabled_with_message(error);
        } else {
            command = command.disabled("Complete the close command before submitting");
        }

        Some(command)
    }

    fn alfred_nuke_command(&self, query: &str) -> Option<AlfredCommand> {
        if !alfred_query_is_nuke(query) {
            return None;
        }

        let mut command = AlfredCommand::new(
            AlfredCommandId::NukePositions,
            "NUKE positions",
            "Close all open perp positions at market",
            "NUKE",
            AlfredCommandKind::Trading,
            None,
            &["nuke", "close", "all", "flatten", "positions", "market"],
        );

        let Some(account_address) = self.connected_order_account_address() else {
            return Some(command.disabled("Connect wallet and enter agent key first"));
        };
        if !self.has_active_committed_agent_key() {
            return Some(command.disabled("Connect wallet and enter agent key first"));
        }
        if !self.active_wallet_context_matches_connected_account(&account_address) {
            return Some(command.disabled(
                "Connected wallet no longer matches the active account; reconnect before trading",
            ));
        }
        if self.has_pending_trading_request() {
            return Some(
                command.disabled("Wait for pending trading requests to finish before NUKE"),
            );
        }
        if self.account_loading {
            return Some(command.disabled("Account refresh in progress"));
        }
        if self.account_reconciliation_required {
            return Some(
                command
                    .disabled("Account refresh pending; wait for fresh account data before NUKE"),
            );
        }
        let Some((_, account_data)) = self.connected_order_account_snapshot() else {
            return Some(command.disabled("No account data available"));
        };
        if !account_data.completeness.positions_actionable {
            let detail = account_data
                .completeness
                .section_warning(AccountDataSection::Positions)
                .unwrap_or_else(|| {
                    "Positions may be incomplete: refresh account data before relying on positions"
                        .to_string()
                });
            return Some(command.disabled_with_message(format!("{detail}; refresh before NUKE")));
        }
        let now_ms = Self::now_ms();
        if !account_data.is_fresh_for_position_action(now_ms) {
            let age_label = account_data
                .position_action_snapshot_age_ms(now_ms)
                .map(|age| format!("{}s old", age.div_ceil(1000)))
                .unwrap_or_else(|| "from the future".to_string());
            return Some(command.disabled_with_message(format!(
                "Account data is stale ({age_label}); refresh before NUKE"
            )));
        }

        match self.plan_nuke_positions() {
            Ok(plan) if plan.is_empty() => Some(command.disabled("No positions to close")),
            Ok(plan) if !plan.hidden_skipped.is_empty() => {
                Some(command.disabled_with_message(format!(
                    "Cannot NUKE: hidden exposure unresolvable: {}",
                    plan.format_hidden_skip_list()
                )))
            }
            Ok(plan) if plan.ready.is_empty() => Some(
                command.disabled_with_message(format!("Cannot NUKE: {}", plan.format_skip_list())),
            ),
            Ok(plan) => {
                command = command.with_dynamic_text(
                    nuke_command_title(&plan),
                    nuke_command_detail(&plan),
                    "NUKE".to_string(),
                );
                command.message = Some(Message::AlfredSubmit);
                Some(command)
            }
            Err(error) => Some(command.disabled_with_message(error)),
        }
    }

    fn alfred_trade_command(&self, query: &str) -> Option<AlfredCommand> {
        let draft = self.alfred_trade_draft(query)?;
        let mut command = AlfredCommand::new(
            AlfredCommandId::NaturalLanguageTrading,
            "Natural Language Trading",
            "Draft trade intent",
            "Trade",
            AlfredCommandKind::Trading,
            None,
            &[
                "buy", "sell", "long", "short", "trade", "order", "market", "limit",
            ],
        )
        .with_dynamic_text(draft.title.clone(), draft.detail.clone(), draft.tag.clone());
        command =
            command.with_title_icon(draft.icon_symbol.clone(), draft.icon_title_anchor.clone());

        if draft.can_submit() {
            command.message = Some(Message::AlfredSubmit);
        } else if let Some(error) = draft.error.clone() {
            command = command.disabled_with_message(error);
        } else {
            command = command.disabled("Complete the trade before submitting");
        }

        Some(command)
    }
}

fn nuke_command_title(plan: &NukePlan) -> String {
    format!(
        "NUKE {} position{}",
        plan.ready.len(),
        if plan.ready.len() == 1 { "" } else { "s" }
    )
}

pub(crate) fn alfred_query_is_nuke(query: &str) -> bool {
    let mut tokens = query.split_whitespace().map(str::to_ascii_lowercase);
    matches!(
        (
            tokens.next().as_deref(),
            tokens.next().as_deref(),
            tokens.next(),
        ),
        (Some("nuke"), None, None) | (Some("close"), Some("all"), None)
    )
}

fn nuke_command_detail(plan: &NukePlan) -> String {
    let ready = format_position_preview(
        plan.ready.iter().map(|(coin, _)| coin.as_str()),
        plan.ready.len(),
    );
    let mut detail = format!("Market close: {ready}");
    if !plan.skipped.is_empty() {
        detail.push_str("; skipping ");
        detail.push_str(&plan.format_skip_list());
    }
    detail
}

fn format_position_preview<'a>(coins: impl Iterator<Item = &'a str>, total: usize) -> String {
    let shown: Vec<_> = coins.take(4).collect();
    let mut label = shown.join(", ");
    let remaining = total.saturating_sub(shown.len());
    if remaining > 0 {
        label.push_str(&format!(" +{remaining} more"));
    }
    label
}
