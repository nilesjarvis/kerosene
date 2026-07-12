use crate::alfred_state::{AlfredCommand, AlfredCommandId, AlfredCommandKind};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_management::AddWidgetKind;
use crate::pane_state::PaneKind;

use super::availability::{AlfredCommandAvailability, income_tag, open_tag};

// ---------------------------------------------------------------------------
// Widget Commands
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn alfred_widget_commands(&self) -> Vec<AlfredCommand> {
        let can_add_pane = self.add_target_pane().is_some();
        let no_pane_reason = "Alfred needs an open pane to add this widget";
        let can_add_income = self
            .connected_order_account_snapshot()
            .is_some_and(|(_, data)| data.is_portfolio_margin());

        let positions_history_open =
            self.pane_is_open(|kind| matches!(kind, PaneKind::BottomTabs { .. }));
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
        let telegram_feed_open = self.pane_is_open(|kind| matches!(kind, PaneKind::TelegramFeed));
        let calendar_open = self.pane_is_open(|kind| matches!(kind, PaneKind::Calendar));

        vec![
            AlfredCommand::new(
                AlfredCommandId::AddCandlestickChart,
                "Candlestick Chart",
                "Add chart pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(
                    AddWidgetKind::CandlestickChart,
                )),
                &["candle", "chart", "price", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddComparisonChart,
                "Comparison Chart",
                "Add multi-symbol chart pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(
                    AddWidgetKind::ComparisonChart,
                )),
                &["compare", "spaghetti", "relative", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddPairRatioChart,
                "Pair Ratio",
                "Add ratio chart pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(AddWidgetKind::PairRatioChart)),
                &["pair", "ratio", "spread", "comparison", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddSessionDataPane,
                "Session Data",
                "Market session returns pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(AddWidgetKind::SessionData)),
                &[
                    "session", "data", "returns", "market", "hours", "widget", "add",
                ],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddPositionsHistoryPane,
                "Positions / History",
                "Positions, orders, balances, trade history, and funding pane",
                open_tag(positions_history_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(
                    AddWidgetKind::PositionsHistory,
                )),
                &[
                    "positions",
                    "orders",
                    "balances",
                    "trades",
                    "history",
                    "funding",
                    "account",
                    "widget",
                    "add",
                ],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddPortfolioPane,
                "Portfolio",
                "Account overview pane",
                open_tag(portfolio_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(AddWidgetKind::Portfolio)),
                &["account", "pnl", "equity", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddIncomePane,
                "Income",
                "Portfolio margin income pane",
                income_tag(income_open, can_add_income),
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(AddWidgetKind::Income)),
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
                Some(Message::BeginWidgetPlacement(AddWidgetKind::Outcomes)),
                &["prediction", "markets", "feed", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddHypeEtfsPane,
                "HYPE ETFs",
                "ETF flow pane",
                open_tag(hype_etfs_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(AddWidgetKind::HypeEtfs)),
                &["etf", "flow", "feed", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddHypeUnstakingQueuePane,
                "HYPE Unstaking Queue",
                "Upcoming HYPE unlocks pane",
                open_tag(hype_unstaking_queue_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(
                    AddWidgetKind::HypeUnstakingQueue,
                )),
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
                Some(Message::BeginWidgetPlacement(AddWidgetKind::Liquidations)),
                &["liq", "liquidation", "feed", "hydromancer", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddLiquidationsDistributionPane,
                "Liquidations Distribution",
                "HyperDash liquidation depth pane",
                open_tag(liquidations_distribution_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(
                    AddWidgetKind::LiquidationsDistribution,
                )),
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
                Some(Message::BeginWidgetPlacement(AddWidgetKind::TrackedTrades)),
                &[
                    "wallet", "tracker", "tracked", "trades", "feed", "widget", "add",
                ],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddTelegramFeedPane,
                "Telegram Feed",
                "Telegram channel feed pane",
                open_tag(telegram_feed_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(AddWidgetKind::TelegramFeed)),
                &["telegram", "news", "channel", "feed", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddXFeedPane,
                "X Feed",
                "Following and Lists feed pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(AddWidgetKind::XFeed)),
                &[
                    "x",
                    "twitter",
                    "tweets",
                    "following",
                    "lists",
                    "feed",
                    "widget",
                    "add",
                ],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddCalendarPane,
                "Calendar",
                "Economic calendar pane",
                open_tag(calendar_open, "Pane"),
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(AddWidgetKind::Calendar)),
                &["events", "macro", "economic", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddOrderBookPane,
                "Order Book",
                "Market depth pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(AddWidgetKind::OrderBook)),
                &["book", "depth", "dom", "ladder", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
            AlfredCommand::new(
                AlfredCommandId::AddLiveWatchlistPane,
                "Live Watchlist",
                "Symbol watchlist pane",
                "Pane",
                AlfredCommandKind::AddWidget,
                Some(Message::BeginWidgetPlacement(AddWidgetKind::LiveWatchlist)),
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
                Some(Message::BeginWidgetPlacement(
                    AddWidgetKind::PositioningInfo,
                )),
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
                Some(Message::BeginWidgetPlacement(AddWidgetKind::AdvancedOrders)),
                &["chase", "twap", "orders", "tools", "widget", "add"],
            )
            .disabled_if(!can_add_pane, no_pane_reason),
        ]
    }
}
