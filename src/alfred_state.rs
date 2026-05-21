use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::NukePlan;
use crate::pane_state::PaneKind;

mod position_close;
mod trading;

// ---------------------------------------------------------------------------
// Alfred state and command catalog
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub(crate) struct AlfredState {
    pub(crate) open: bool,
    pub(crate) query: String,
    pub(crate) selected_index: usize,
}

impl AlfredState {
    pub(crate) fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected_index = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlfredSelectionStep {
    Previous,
    Next,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlfredCommandId {
    AddCandlestickChart,
    AddComparisonChart,
    AddPairRatioChart,
    AddPortfolioPane,
    AddIncomePane,
    AddOutcomesPane,
    AddHypeEtfsPane,
    AddLiquidationsPane,
    AddTrackedTradesPane,
    AddCalendarPane,
    AddOrderBookPane,
    AddLiveWatchlistPane,
    ToggleTickerTape,
    AddPositioningInfoPane,
    AddAdvancedOrdersPane,
    OpenTradingJournal,
    OpenWalletTrackerWindow,
    OpenSettingsWindow,
    NaturalLanguageTrading,
    NukePositions,
    ClosePosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlfredCommandKind {
    AddWidget,
    OpenWindow,
    Trading,
}

#[derive(Debug, Clone)]
pub(crate) struct AlfredCommand {
    pub(crate) id: AlfredCommandId,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) tag: String,
    pub(crate) icon_symbol: Option<String>,
    pub(crate) icon_title_anchor: Option<String>,
    pub(crate) kind: AlfredCommandKind,
    pub(crate) enabled: bool,
    pub(crate) disabled_reason: Option<String>,
    pub(crate) message: Option<Message>,
    aliases: &'static [&'static str],
}

impl AlfredCommand {
    fn new(
        id: AlfredCommandId,
        title: &'static str,
        detail: &'static str,
        tag: &'static str,
        kind: AlfredCommandKind,
        message: Option<Message>,
        aliases: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            title: title.to_string(),
            detail: detail.to_string(),
            tag: tag.to_string(),
            icon_symbol: None,
            icon_title_anchor: None,
            kind,
            enabled: true,
            disabled_reason: None,
            message,
            aliases,
        }
    }

    fn disabled(mut self, reason: &'static str) -> Self {
        self.enabled = false;
        self.disabled_reason = Some(reason.to_string());
        self.message = None;
        self
    }

    fn with_dynamic_text(mut self, title: String, detail: String, tag: String) -> Self {
        self.title = title;
        self.detail = detail;
        self.tag = tag;
        self
    }

    fn with_title_icon(
        mut self,
        icon_symbol: Option<String>,
        icon_title_anchor: Option<String>,
    ) -> Self {
        self.icon_symbol = icon_symbol;
        self.icon_title_anchor = icon_title_anchor;
        self
    }

    fn disabled_with_message(mut self, reason: String) -> Self {
        self.enabled = false;
        self.disabled_reason = Some(reason);
        self.message = None;
        self
    }

    fn matches_query(&self, query: &str) -> bool {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return self.kind != AlfredCommandKind::Trading;
        }

        let searchable = self.searchable_text();
        query
            .split_whitespace()
            .all(|token| searchable.contains(token))
    }

    fn searchable_text(&self) -> String {
        let mut text = format!(
            "{} {} {} {:?}",
            self.title.to_ascii_lowercase(),
            self.detail.to_ascii_lowercase(),
            self.tag.to_ascii_lowercase(),
            self.kind
        );
        for alias in self.aliases {
            text.push(' ');
            text.push_str(alias);
        }
        text
    }
}

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

        if self.wallet_key_input.trim().is_empty() || self.connected_address.is_none() {
            return Some(command.disabled("Connect wallet and enter agent key first"));
        }
        if self.account_loading {
            return Some(command.disabled("Account refresh in progress"));
        }
        if self.account_data.is_none() {
            return Some(command.disabled("No account data available"));
        }

        match self.plan_nuke_positions() {
            Ok(plan) if plan.is_empty() => Some(command.disabled("No positions to close")),
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
        command = command.with_title_icon(
            draft.icon_symbol.clone(),
            draft.icon_title_anchor.clone(),
        );

        if draft.can_submit() {
            command.message = Some(Message::AlfredSubmit);
        } else if let Some(error) = draft.error.clone() {
            command = command.disabled_with_message(error);
        } else {
            command = command.disabled("Complete the trade before submitting");
        }

        Some(command)
    }

    fn alfred_command_catalog(&self) -> Vec<AlfredCommand> {
        let target = self.add_target_pane();
        let can_add_pane = target.is_some();
        let no_pane_reason = "Alfred needs an open pane to add this widget";
        let can_add_income = self
            .account_data
            .as_ref()
            .is_some_and(AccountData::is_portfolio_margin);

        let portfolio_open = self.pane_is_open(|kind| matches!(kind, PaneKind::Portfolio));
        let income_open = self.pane_is_open(|kind| matches!(kind, PaneKind::Income));
        let outcomes_open = self.pane_is_open(|kind| matches!(kind, PaneKind::Outcomes));
        let hype_etfs_open = self.pane_is_open(|kind| matches!(kind, PaneKind::HypeEtfs));
        let liquidations_open = self.pane_is_open(|kind| matches!(kind, PaneKind::Liquidations));
        let tracked_trades_open = self.pane_is_open(|kind| matches!(kind, PaneKind::TrackedTrades));
        let calendar_open = self.pane_is_open(|kind| matches!(kind, PaneKind::Calendar));

        vec![
            AlfredCommand::new(
                AlfredCommandId::AddCandlestickChart,
                "Candlestick Chart",
                "Add chart pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                target.map(Message::AddChart),
                &["candle", "chart", "price", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddComparisonChart,
                "Comparison Chart",
                "Add multi-symbol chart pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::AddComparisonChart),
                &["compare", "spaghetti", "relative", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddPairRatioChart,
                "Pair Ratio",
                "Add ratio chart pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::AddPairRatioChart),
                &["pair", "ratio", "spread", "comparison", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddPortfolioPane,
                "Portfolio",
                "Account overview pane",
                open_tag(portfolio_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::AddPortfolioPane),
                &["account", "pnl", "equity", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddIncomePane,
                "Income",
                "Portfolio margin income pane",
                income_tag(income_open, can_add_income),
                AlfredCommandKind::AddWidget,
                Some(Message::AddIncomePane),
                &["funding", "interest", "account", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason)
            .disabled_if(
                !income_open && !can_add_income,
                "Income requires Portfolio Margin",
            ),
            AlfredCommand::new(
                AlfredCommandId::AddOutcomesPane,
                "Outcomes",
                "Prediction market feed pane",
                open_tag(outcomes_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::AddOutcomesPane),
                &["prediction", "markets", "feed", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddHypeEtfsPane,
                "HYPE ETFs",
                "ETF flow pane",
                open_tag(hype_etfs_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::AddHypeEtfsPane),
                &["etf", "flow", "feed", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddLiquidationsPane,
                "Liquidations Feed",
                "Live liquidation feed pane",
                open_tag(liquidations_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::AddLiquidationsPane),
                &["liq", "liquidation", "feed", "hydromancer", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddTrackedTradesPane,
                "Wallet Tracker",
                "Tracked trades pane",
                open_tag(tracked_trades_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::AddTrackedTradesPane),
                &[
                    "wallet", "tracker", "tracked", "trades", "feed", "widget", "add",
                ],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddCalendarPane,
                "Calendar",
                "Economic calendar pane",
                open_tag(calendar_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::AddCalendarPane),
                &["events", "macro", "economic", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddOrderBookPane,
                "Order Book",
                "Market depth pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::AddOrderBookPane),
                &["book", "depth", "dom", "ladder", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddLiveWatchlistPane,
                "Live Watchlist",
                "Symbol watchlist pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::AddLiveWatchlistPane),
                &["watch", "symbols", "list", "ticker", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::ToggleTickerTape,
                "Ticker Tape",
                "Favourites bar",
                open_tag(self.ticker_tape_enabled, "Bar"),
                AlfredCommandKind::AddWidget,
                Some(Message::ToggleTickerTape),
                &["favourites", "favorites", "bar", "symbols", "widget", "add"],
            ),
            AlfredCommand::new(
                AlfredCommandId::AddPositioningInfoPane,
                "Positioning Information",
                "Trader positioning pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::AddPositioningInfoPane),
                &[
                    "positioning",
                    "traders",
                    "hyperdash",
                    "sentiment",
                    "widget",
                    "add",
                ],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddAdvancedOrdersPane,
                "Advanced Orders",
                "Chase and TWAP controls pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::AddAdvancedOrdersPane),
                &["chase", "twap", "orders", "tools", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::OpenTradingJournal,
                "Trading Journal",
                "Open journal window",
                open_tag(self.journal.window_id.is_some(), "Window"),
                AlfredCommandKind::OpenWindow,
                Some(Message::AddTradingJournal),
                &["journal", "notes", "trades", "window", "open"],
            ),
            AlfredCommand::new(
                AlfredCommandId::OpenWalletTrackerWindow,
                "Wallet Tracker Window",
                "Open wallet tracker window",
                open_tag(self.wallet_tracker.window_id.is_some(), "Window"),
                AlfredCommandKind::OpenWindow,
                Some(Message::OpenWalletTrackerWindow),
                &["wallet", "tracker", "addresses", "window", "open"],
            ),
            AlfredCommand::new(
                AlfredCommandId::OpenSettingsWindow,
                "Settings",
                "Open settings window",
                open_tag(self.settings_window_id.is_some(), "Window"),
                AlfredCommandKind::OpenWindow,
                Some(Message::OpenSettingsWindow),
                &["preferences", "config", "hotkeys", "window", "open"],
            ),
        ]
    }
}

trait AlfredCommandAvailability {
    fn disabled_if(self, condition: bool, reason: &'static str) -> Self;
}

impl AlfredCommandAvailability for AlfredCommand {
    fn disabled_if(self, condition: bool, reason: &'static str) -> Self {
        if condition {
            self.disabled(reason)
        } else {
            self
        }
    }
}

fn open_tag(open: bool, closed_tag: &'static str) -> &'static str {
    if open { "Open" } else { closed_tag }
}

fn income_tag(open: bool, can_add_income: bool) -> &'static str {
    if open {
        "Open"
    } else if can_add_income {
        "Pane"
    } else {
        "Requires PM"
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

#[cfg(test)]
mod tests {
    use super::{AlfredCommandId, AlfredCommandKind};
    use crate::app_state::TradingTerminal;

    #[test]
    fn alfred_defaults_to_add_widget_commands() {
        let terminal = TradingTerminal::boot().0;
        let commands = terminal.alfred_filtered_commands();

        assert!(
            commands
                .iter()
                .any(|command| command.id == AlfredCommandId::AddCandlestickChart)
        );
        assert!(
            commands
                .iter()
                .all(|command| command.kind != AlfredCommandKind::Trading)
        );
    }

    #[test]
    fn alfred_shows_only_trade_draft_for_trade_queries() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.alfred.query = "buy btc".to_string();

        let commands = terminal.alfred_filtered_commands();

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, AlfredCommandId::NaturalLanguageTrading);
    }

    #[test]
    fn alfred_shows_only_trade_draft_for_chase_queries() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.alfred.query = "chase 1k HYPE".to_string();

        let commands = terminal.alfred_filtered_commands();

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, AlfredCommandId::NaturalLanguageTrading);
    }

    #[test]
    fn alfred_shows_only_nuke_command_for_nuke_query() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.alfred.query = "nuke".to_string();

        let commands = terminal.alfred_filtered_commands();

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, AlfredCommandId::NukePositions);
    }

    #[test]
    fn alfred_treats_close_all_as_nuke_command() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.alfred.query = "close all".to_string();

        let commands = terminal.alfred_filtered_commands();

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, AlfredCommandId::NukePositions);
    }

    #[test]
    fn alfred_shows_only_close_position_command_for_close_queries() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.alfred.query = "close HYPE".to_string();

        let commands = terminal.alfred_filtered_commands();

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, AlfredCommandId::ClosePosition);
    }
}
