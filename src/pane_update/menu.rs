use crate::app_state::TradingTerminal;
use crate::canvas_state::WorkspaceId;
use crate::message::Message;
use crate::pane_management::AddWidgetKind;
use crate::pane_state::PaneKind;
use iced::Task;
use iced::widget::pane_grid;

impl TradingTerminal {
    pub(super) fn update_pane_menu(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SwitchBottomTab(workspace, pane, tab) => {
                if let Some(PaneKind::BottomTabs { active_tab }) = self
                    .workspace_panes_mut(workspace)
                    .and_then(|panes| panes.get_mut(pane))
                {
                    *active_tab = tab;
                }
                self.set_workspace_focus(workspace, Some(pane));
                self.last_focused_workspace = workspace;
                self.persist_config();
            }
            Message::CloseAllMenus => {
                self.close_chart_header_menus();
                self.alfred.close();
                self.cancel_widget_placement();
            }
            Message::ToggleAddWidgetMenu(workspace) => {
                let opening = !self.add_widget_menu_open || self.add_widget_workspace != workspace;
                if opening {
                    self.close_chart_header_menus();
                    self.alfred.close();
                    self.cancel_widget_placement();
                }
                self.add_widget_workspace = workspace;
                self.last_focused_workspace = workspace;
                self.add_widget_menu_open = opening;
            }
            Message::ToggleLayoutMenu => {
                let opening = !self.layout_menu_open;
                if opening {
                    self.close_chart_header_menus();
                    self.alfred.close();
                    self.cancel_widget_placement();
                }
                self.layout_menu_open = opening;
            }
            Message::ToggleTickerTape => {
                self.add_widget_menu_open = false;
                self.ticker_tape_enabled = !self.ticker_tape_enabled;
                self.ticker_tape_scroll_px = 0.0;
                self.persist_config();
                return Task::batch([
                    self.request_ticker_tape_refresh(true),
                    self.sync_main_window_min_size(),
                ]);
            }
            Message::BeginWidgetPlacement(widget) => {
                self.add_widget_menu_open = false;
                self.layout_menu_open = false;
                self.add_widget_placement = crate::pane_management::AddWidgetPlacement::Below;
                self.close_chart_header_menus();
                self.alfred.close();

                if let Some((workspace, pane)) = self.existing_pane_for_add_widget(widget) {
                    self.set_workspace_focus(workspace, Some(pane));
                    self.push_toast(format!("{} is already open", widget.label()), false);
                    return self.focus_workspace_window(workspace);
                }

                self.set_workspace_dragging_pane(self.add_widget_workspace, None);
                self.placing_widget = Some(widget);
                self.widget_placement_hover = None;
            }
            Message::WidgetPlacementHovered(workspace, pane, placement)
                if self.placing_widget.is_some()
                    && self.add_widget_workspace == workspace
                    && self
                        .workspace_panes(workspace)
                        .is_some_and(|panes| panes.get(pane).is_some())
                    && self.widget_placement_hover != Some((pane, placement)) =>
            {
                self.widget_placement_hover = Some((pane, placement));
            }
            Message::WidgetPlacementExited(workspace, pane)
                if self.add_widget_workspace == workspace
                    && self
                        .widget_placement_hover
                        .is_some_and(|(hovered, _)| hovered == pane) =>
            {
                self.widget_placement_hover = None;
            }
            Message::PlaceWidget(workspace, pane, placement) => {
                let Some(widget) = self.placing_widget else {
                    return Task::none();
                };
                if self.add_widget_workspace != workspace
                    || self
                        .workspace_panes(workspace)
                        .is_none_or(|panes| panes.get(pane).is_none())
                {
                    self.cancel_widget_placement();
                    self.push_toast(
                        "Could not place widget: pane is unavailable".to_string(),
                        true,
                    );
                    return Task::none();
                }

                self.set_workspace_focus(workspace, Some(pane));
                self.last_focused_workspace = workspace;
                self.add_widget_workspace = workspace;
                self.add_widget_placement = placement;
                self.cancel_widget_placement();
                return Task::done(add_widget_message(widget, workspace, pane));
            }
            Message::CancelWidgetPlacement => self.cancel_widget_placement(),
            _ => {}
        }

        Task::none()
    }

    pub(crate) fn cancel_widget_placement(&mut self) {
        self.placing_widget = None;
        self.widget_placement_hover = None;
    }
}

pub(super) fn add_widget_message(
    widget: AddWidgetKind,
    workspace: WorkspaceId,
    pane: pane_grid::Pane,
) -> Message {
    match widget {
        AddWidgetKind::CandlestickChart => Message::AddChart(workspace, pane),
        AddWidgetKind::ComparisonChart => Message::AddComparisonChart,
        AddWidgetKind::PairRatioChart => Message::AddPairRatioChart,
        AddWidgetKind::SessionData => Message::AddSessionDataPane,
        AddWidgetKind::PositionsHistory => Message::AddPositionsHistoryPane,
        AddWidgetKind::Portfolio => Message::AddPortfolioPane,
        AddWidgetKind::Income => Message::AddIncomePane,
        AddWidgetKind::Outcomes => Message::AddOutcomesPane,
        AddWidgetKind::HypeEtfs => Message::AddHypeEtfsPane,
        AddWidgetKind::HypeUnstakingQueue => Message::AddHypeUnstakingQueuePane,
        AddWidgetKind::Liquidations => Message::AddLiquidationsPane,
        AddWidgetKind::LiquidationsDistribution => Message::AddLiquidationsDistributionPane,
        AddWidgetKind::TrackedTrades => Message::AddTrackedTradesPane,
        AddWidgetKind::TelegramFeed => Message::AddTelegramFeedPane,
        AddWidgetKind::XFeed => Message::AddXFeedPane,
        AddWidgetKind::Calendar => Message::AddCalendarPane,
        AddWidgetKind::OrderBook => Message::AddOrderBookPane,
        AddWidgetKind::LiveWatchlist => Message::AddLiveWatchlistPane,
        AddWidgetKind::PositioningInfo => Message::AddPositioningInfoPane,
        AddWidgetKind::AdvancedOrders => Message::AddAdvancedOrdersPane,
    }
}
