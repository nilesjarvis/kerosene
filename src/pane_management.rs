use crate::app_state::TradingTerminal;
use crate::canvas_state::WorkspaceId;
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

    #[cfg(test)]
    pub(crate) fn find_pane_matching<F>(&self, predicate: F) -> Option<pane_grid::Pane>
    where
        F: Fn(&PaneKind) -> bool,
    {
        self.panes
            .iter()
            .find_map(|(pane, kind)| predicate(kind).then_some(*pane))
    }

    pub(crate) fn find_workspace_pane_matching<F>(
        &self,
        predicate: F,
    ) -> Option<(WorkspaceId, pane_grid::Pane)>
    where
        F: Fn(&PaneKind) -> bool,
    {
        self.workspace_pane_kinds()
            .find_map(|(workspace, pane, kind)| predicate(kind).then_some((workspace, pane)))
    }

    pub(crate) fn pane_is_open<F>(&self, predicate: F) -> bool
    where
        F: Fn(&PaneKind) -> bool,
    {
        self.workspace_pane_kinds()
            .any(|(_, _, kind)| predicate(kind))
    }

    pub(crate) fn add_target_pane(&self) -> Option<pane_grid::Pane> {
        self.add_target_pane_in(self.add_widget_workspace)
    }

    pub(crate) fn add_target_pane_in(&self, workspace: WorkspaceId) -> Option<pane_grid::Pane> {
        let panes = self.workspace_panes(workspace)?;
        if let Some(pane) = self.workspace_focus(workspace)
            && panes.get(pane).is_some()
        {
            return Some(pane);
        }

        if let Some(chart_id) = self.primary_chart_id
            && let Some((pane, _)) = panes
                .iter()
                .find(|(_, kind)| matches!(kind, PaneKind::Chart(id) if *id == chart_id))
        {
            return Some(*pane);
        }

        panes
            .iter()
            .find_map(|(pane, kind)| matches!(kind, PaneKind::Chart(_)).then_some(*pane))
            .or_else(|| panes.iter().next().map(|(pane, _)| *pane))
    }

    pub(crate) fn existing_pane_for_add_widget(
        &self,
        widget: AddWidgetKind,
    ) -> Option<(WorkspaceId, pane_grid::Pane)> {
        self.workspace_pane_kinds()
            .find_map(|(workspace, pane, kind)| {
                let matches = match widget {
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
                };
                matches.then_some((workspace, pane))
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
        workspace: WorkspaceId,
        axis: pane_grid::Axis,
        target: pane_grid::Pane,
        kind: PaneKind,
        label: &str,
    ) -> Option<pane_grid::Pane> {
        let placement = self.add_widget_placement;
        self.add_widget_placement = AddWidgetPlacement::Below;

        let split_result = self
            .workspace_panes_mut(workspace)
            .and_then(|panes| panes.split(axis, target, kind));
        match split_result {
            Some((pane, _split)) => {
                if placement == AddWidgetPlacement::Left
                    && axis == pane_grid::Axis::Vertical
                    && let Some(panes) = self.workspace_panes_mut(workspace)
                {
                    panes.swap(pane, target);
                }
                self.set_workspace_focus(workspace, Some(pane));
                self.last_focused_workspace = workspace;
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
        workspace: WorkspaceId,
        axis: pane_grid::Axis,
        target: pane_grid::Pane,
        kind: PaneKind,
        label: &str,
    ) -> Option<pane_grid::Pane> {
        self.split_new_pane(workspace, axis, target, kind, label)
    }

    pub(crate) fn add_pane_next_to_focus(
        &mut self,
        workspace: WorkspaceId,
        axis: pane_grid::Axis,
        kind: PaneKind,
        label: &str,
    ) -> Option<pane_grid::Pane> {
        let Some(target) = self.add_target_pane_in(workspace) else {
            self.push_toast(format!("Could not add {label}: no pane is available"), true);
            return None;
        };
        self.split_new_pane(workspace, axis, target, kind, label)
    }

    pub(crate) fn add_or_focus_singleton_pane<F>(
        &mut self,
        workspace: WorkspaceId,
        axis: pane_grid::Axis,
        kind: PaneKind,
        label: &str,
        predicate: F,
    ) -> AddPaneOutcome
    where
        F: Fn(&PaneKind) -> bool,
    {
        if let Some((existing_workspace, pane)) = self.find_workspace_pane_matching(predicate) {
            self.set_workspace_focus(existing_workspace, Some(pane));
            self.last_focused_workspace = existing_workspace;
            self.push_toast(format!("{label} is already open"), false);
            return AddPaneOutcome::Existing;
        }

        match self.add_pane_next_to_focus(workspace, axis, kind, label) {
            Some(_) => AddPaneOutcome::Added,
            None => AddPaneOutcome::Failed,
        }
    }
}
