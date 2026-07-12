use crate::app_state::TradingTerminal;
use crate::pane_state::PaneKind;

pub(super) struct AddWidgetMenuContext {
    pub(super) can_add_pane: bool,
    pub(super) can_add_income: bool,
    pub(super) positions_history_open: bool,
    pub(super) portfolio_open: bool,
    pub(super) income_open: bool,
    pub(super) calendar_open: bool,
    pub(super) liquidations_open: bool,
    pub(super) liquidations_distribution_open: bool,
    pub(super) tracked_trades_open: bool,
    pub(super) telegram_feed_open: bool,
    pub(super) outcomes_open: bool,
    pub(super) hype_etfs_open: bool,
    pub(super) hype_unstaking_queue_open: bool,
    pub(super) ticker_tape_open: bool,
    pub(super) journal_open: bool,
    pub(super) wallet_tracker_open: bool,
    pub(super) wallet_clusters_open: bool,
    pub(super) screener_open: bool,
    pub(super) settings_open: bool,
}

impl AddWidgetMenuContext {
    pub(super) fn new(terminal: &TradingTerminal, can_add_income: bool) -> Self {
        Self {
            can_add_pane: terminal.add_target_pane().is_some(),
            can_add_income,
            positions_history_open: terminal
                .pane_is_open(|kind| matches!(kind, PaneKind::BottomTabs { .. })),
            portfolio_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::Portfolio)),
            income_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::Income)),
            calendar_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::Calendar)),
            liquidations_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::Liquidations)),
            liquidations_distribution_open: terminal
                .pane_is_open(|kind| matches!(kind, PaneKind::LiquidationsDistribution)),
            tracked_trades_open: terminal
                .pane_is_open(|kind| matches!(kind, PaneKind::TrackedTrades)),
            telegram_feed_open: terminal
                .pane_is_open(|kind| matches!(kind, PaneKind::TelegramFeed)),
            outcomes_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::Outcomes)),
            hype_etfs_open: terminal.pane_is_open(|kind| matches!(kind, PaneKind::HypeEtfs)),
            hype_unstaking_queue_open: terminal
                .pane_is_open(|kind| matches!(kind, PaneKind::HypeUnstakingQueue)),
            ticker_tape_open: terminal.ticker_tape_enabled,
            journal_open: terminal.journal.window_id.is_some(),
            wallet_tracker_open: terminal.wallet_tracker.window_id.is_some(),
            wallet_clusters_open: terminal.wallet_clusters.window_id.is_some(),
            screener_open: terminal.screener.window_id.is_some(),
            settings_open: terminal.settings_window_id.is_some(),
        }
    }
}
