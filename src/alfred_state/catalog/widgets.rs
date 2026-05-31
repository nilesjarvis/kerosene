use crate::account::AccountData;
use crate::alfred_state::{AlfredCommand, AlfredCommandId, AlfredCommandKind};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use super::availability::{AlfredCommandAvailability, income_tag, open_tag};

// ---------------------------------------------------------------------------
// Widget Commands
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn alfred_widget_commands(&self) -> Vec<AlfredCommand> {
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
        let hype_unstaking_queue_open =
            self.pane_is_open(|kind| matches!(kind, PaneKind::HypeUnstakingQueue));
        let liquidations_open = self.pane_is_open(|kind| matches!(kind, PaneKind::Liquidations));
        let liquidations_distribution_open =
            self.pane_is_open(|kind| matches!(kind, PaneKind::LiquidationsDistribution));
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
                AlfredCommandId::AddHypeUnstakingQueuePane,
                "HYPE Unstaking Queue",
                "Upcoming HYPE unlocks pane",
                open_tag(hype_unstaking_queue_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::AddHypeUnstakingQueuePane),
                &[
                    "unstake",
                    "unstaking",
                    "queue",
                    "hype",
                    "staking",
                    "unlock",
                    "widget",
                    "add",
                ],
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
                AlfredCommandId::AddLiquidationsDistributionPane,
                "Liquidations Distribution",
                "HyperDash liquidation depth pane",
                open_tag(liquidations_distribution_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::AddLiquidationsDistributionPane),
                &[
                    "liq",
                    "liquidation",
                    "distribution",
                    "depth",
                    "hyperdash",
                    "widget",
                    "add",
                ],
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
        ]
    }
}
