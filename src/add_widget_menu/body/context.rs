use crate::app_state::TradingTerminal;
use crate::pane_management::AddWidgetPlacement;
use crate::pane_state::PaneKind;

use iced::widget::pane_grid;

pub(super) struct AddWidgetMenuContext {
    pub(super) target: Option<pane_grid::Pane>,
    pub(super) target_title: String,
    pub(super) can_add_pane: bool,
    pub(super) can_add_income: bool,
    pub(super) placement: AddWidgetPlacement,
    pub(super) portfolio_open: bool,
    pub(super) income_open: bool,
    pub(super) calendar_open: bool,
    pub(super) liquidations_open: bool,
    pub(super) tracked_trades_open: bool,
    pub(super) outcomes_open: bool,
    pub(super) hype_etfs_open: bool,
    pub(super) ticker_tape_open: bool,
    pub(super) journal_open: bool,
    pub(super) wallet_tracker_open: bool,
    pub(super) settings_open: bool,
}

impl AddWidgetMenuContext {
    pub(super) fn new(terminal: &TradingTerminal, can_add_income: bool) -> Self {
        let target = terminal.add_target_pane();

        Self {
            target,
            target_title: terminal.add_target_title(),
            can_add_pane: target.is_some(),
            can_add_income,
            placement: terminal.add_widget_placement,
            portfolio_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::Portfolio)),
            income_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::Income)),
            calendar_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::Calendar)),
            liquidations_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::Liquidations)),
            tracked_trades_open: terminal
                .pane_is_open(|kind| matches!(kind, PaneKind::TrackedTrades)),
            outcomes_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::Outcomes)),
            hype_etfs_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::HypeEtfs)),
            ticker_tape_open: terminal.ticker_tape_enabled,
            journal_open: terminal.journal.window_id.is_some(),
            wallet_tracker_open: terminal.wallet_tracker.window_id.is_some(),
            settings_open: terminal.settings_window_id.is_some(),
        }
    }
}
