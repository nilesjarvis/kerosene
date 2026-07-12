use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::pane_state::PaneKind;

use iced::widget::pane_grid;

// ---------------------------------------------------------------------------
// Pane insertion helpers
// ---------------------------------------------------------------------------

pub(crate) enum AddPaneOutcome {
    Added,
    Existing,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AddWidgetKind {
    CandlestickChart,
    ComparisonChart,
    PairRatioChart,
    SessionData,
    PositionsHistory,
    Portfolio,
    Income,
    Outcomes,
    HypeEtfs,
    HypeUnstakingQueue,
    Liquidations,
    LiquidationsDistribution,
    TrackedTrades,
    TelegramFeed,
    XFeed,
    Calendar,
    OrderBook,
    LiveWatchlist,
    PositioningInfo,
    AdvancedOrders,
}

impl AddWidgetKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::CandlestickChart => "Candlestick Chart",
            Self::ComparisonChart => "Comparison Chart",
            Self::PairRatioChart => "Pair Ratio",
            Self::SessionData => "Session Data",
            Self::PositionsHistory => "Positions / History",
            Self::Portfolio => "Portfolio",
            Self::Income => "Income",
            Self::Outcomes => "Outcomes",
            Self::HypeEtfs => "HYPE ETFs",
            Self::HypeUnstakingQueue => "HYPE Unstaking Queue",
            Self::Liquidations => "Liquidations Feed",
            Self::LiquidationsDistribution => "Liquidations Distribution",
            Self::TrackedTrades => "Wallet Tracker",
            Self::TelegramFeed => "Telegram Feed",
            Self::XFeed => "X Feed",
            Self::Calendar => "Calendar",
            Self::OrderBook => "Order Book",
            Self::LiveWatchlist => "Live Watchlist",
            Self::PositioningInfo => "Positioning Information",
            Self::AdvancedOrders => "Advanced Orders",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AddWidgetPlacement {
    Left,
    Below,
    Right,
}

impl TradingTerminal {
    pub(crate) fn first_chart_pane(&self) -> Option<(pane_grid::Pane, ChartId)> {
        self.panes.iter().find_map(|(pane, kind)| match kind {
            PaneKind::Chart(id) => Some((*pane, *id)),
            _ => None,
        })
    }

    pub(crate) fn chart_anchor_pane(&self) -> Option<pane_grid::Pane> {
        self.first_chart_pane()
            .map(|(pane, _)| pane)
            .or_else(|| self.panes.iter().next().map(|(pane, _)| *pane))
    }

    pub(crate) fn sync_primary_chart_id_from_panes(&mut self) {
        self.primary_chart_id = self
            .first_chart_pane()
            .map(|(_, id)| id)
            .or_else(|| self.charts.keys().copied().min());
    }

    pub(crate) fn find_pane_matching<F>(&self, predicate: F) -> Option<pane_grid::Pane>
    where
        F: Fn(&PaneKind) -> bool,
    {
        self.panes
            .iter()
            .find_map(|(pane, kind)| predicate(kind).then_some(*pane))
    }

    pub(crate) fn pane_is_open<F>(&self, predicate: F) -> bool
    where
        F: Fn(&PaneKind) -> bool,
    {
        self.find_pane_matching(predicate).is_some()
    }

    pub(crate) fn add_target_pane(&self) -> Option<pane_grid::Pane> {
        if let Some(pane) = self.focus
            && self.panes.get(pane).is_some()
        {
            return Some(pane);
        }

        if let Some(chart_id) = self.primary_chart_id
            && let Some((pane, _)) = self
                .panes
                .iter()
                .find(|(_, kind)| matches!(kind, PaneKind::Chart(id) if *id == chart_id))
        {
            return Some(*pane);
        }

        self.find_pane_matching(|kind| matches!(kind, PaneKind::Chart(_)))
            .or_else(|| self.panes.iter().next().map(|(pane, _)| *pane))
    }

    pub(crate) fn existing_pane_for_add_widget(
        &self,
        widget: AddWidgetKind,
    ) -> Option<pane_grid::Pane> {
        self.find_pane_matching(|kind| match widget {
            AddWidgetKind::PositionsHistory => matches!(kind, PaneKind::BottomTabs { .. }),
            AddWidgetKind::Portfolio => matches!(kind, PaneKind::Portfolio),
            AddWidgetKind::Income => matches!(kind, PaneKind::Income),
            AddWidgetKind::Outcomes => matches!(kind, PaneKind::Outcomes),
            AddWidgetKind::HypeEtfs => matches!(kind, PaneKind::HypeEtfs),
            AddWidgetKind::HypeUnstakingQueue => {
                matches!(kind, PaneKind::HypeUnstakingQueue)
            }
            AddWidgetKind::Liquidations => matches!(kind, PaneKind::Liquidations),
            AddWidgetKind::LiquidationsDistribution => {
                matches!(kind, PaneKind::LiquidationsDistribution)
            }
            AddWidgetKind::TrackedTrades => matches!(kind, PaneKind::TrackedTrades),
            AddWidgetKind::TelegramFeed => matches!(kind, PaneKind::TelegramFeed),
            AddWidgetKind::Calendar => matches!(kind, PaneKind::Calendar),
            AddWidgetKind::AdvancedOrders => matches!(kind, PaneKind::AdvancedOrders),
            AddWidgetKind::CandlestickChart
            | AddWidgetKind::ComparisonChart
            | AddWidgetKind::PairRatioChart
            | AddWidgetKind::SessionData
            | AddWidgetKind::XFeed
            | AddWidgetKind::OrderBook
            | AddWidgetKind::LiveWatchlist
            | AddWidgetKind::PositioningInfo => false,
        })
    }

    pub(crate) fn add_widget_axis(&self) -> pane_grid::Axis {
        match self.add_widget_placement {
            AddWidgetPlacement::Below => pane_grid::Axis::Horizontal,
            AddWidgetPlacement::Left | AddWidgetPlacement::Right => pane_grid::Axis::Vertical,
        }
    }

    fn split_new_pane(
        &mut self,
        axis: pane_grid::Axis,
        target: pane_grid::Pane,
        kind: PaneKind,
        label: &str,
    ) -> Option<pane_grid::Pane> {
        let placement = self.add_widget_placement;
        self.add_widget_placement = AddWidgetPlacement::Below;

        match self.panes.split(axis, target, kind) {
            Some((pane, _split)) => {
                if placement == AddWidgetPlacement::Left && axis == pane_grid::Axis::Vertical {
                    self.panes.swap(pane, target);
                }
                self.focus = Some(pane);
                self.persist_config();
                Some(pane)
            }
            None => {
                self.push_toast(
                    format!("Could not add {label}: target pane is unavailable"),
                    true,
                );
                None
            }
        }
    }

    pub(crate) fn add_pane_to_target(
        &mut self,
        axis: pane_grid::Axis,
        target: pane_grid::Pane,
        kind: PaneKind,
        label: &str,
    ) -> Option<pane_grid::Pane> {
        self.split_new_pane(axis, target, kind, label)
    }

    pub(crate) fn add_pane_next_to_focus(
        &mut self,
        axis: pane_grid::Axis,
        kind: PaneKind,
        label: &str,
    ) -> Option<pane_grid::Pane> {
        let Some(target) = self.add_target_pane() else {
            self.push_toast(format!("Could not add {label}: no pane is available"), true);
            return None;
        };
        self.split_new_pane(axis, target, kind, label)
    }

    pub(crate) fn add_or_focus_singleton_pane<F>(
        &mut self,
        axis: pane_grid::Axis,
        kind: PaneKind,
        label: &str,
        predicate: F,
    ) -> AddPaneOutcome
    where
        F: Fn(&PaneKind) -> bool,
    {
        if let Some(pane) = self.find_pane_matching(predicate) {
            self.focus = Some(pane);
            self.push_toast(format!("{label} is already open"), false);
            return AddPaneOutcome::Existing;
        }

        match self.add_pane_next_to_focus(axis, kind, label) {
            Some(_) => AddPaneOutcome::Added,
            None => AddPaneOutcome::Failed,
        }
    }
}
