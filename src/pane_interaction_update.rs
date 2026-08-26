use crate::app_state::TradingTerminal;
use crate::canvas_state::WorkspaceId;
use crate::chart_state::ChartSurfaceId;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;
use iced::widget::pane_grid;

mod min_size;

impl TradingTerminal {
    pub(crate) fn update_pane_interactions(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PaneResized(workspace, pane_grid::ResizeEvent { split, ratio }) => {
                let ratio = if workspace == WorkspaceId::Main {
                    self.clamp_order_entry_resize_ratio(split, ratio)
                } else {
                    ratio
                };
                if let Some(panes) = self.workspace_panes_mut(workspace) {
                    panes.resize(split, ratio);
                }
                self.persist_config();
            }
            Message::PaneDragged(workspace, pane_grid::DragEvent::Picked { pane }) => {
                self.set_workspace_dragging_pane(workspace, Some(pane));
                self.last_focused_workspace = workspace;
                self.close_chart_header_menus();
            }
            Message::PaneDragged(workspace, pane_grid::DragEvent::Dropped { pane, target }) => {
                self.set_workspace_dragging_pane(workspace, None);
                if let Some(panes) = self.workspace_panes_mut(workspace) {
                    panes.drop(pane, target);
                }
                self.persist_config();
                return if workspace == WorkspaceId::Main {
                    self.sync_main_window_min_size()
                } else {
                    Task::none()
                };
            }
            Message::PaneDragged(workspace, pane_grid::DragEvent::Canceled { .. }) => {
                self.set_workspace_dragging_pane(workspace, None);
            }
            Message::PaneClicked(workspace, pane) => {
                self.set_workspace_focus(workspace, Some(pane));
                self.last_focused_workspace = workspace;
                self.add_widget_workspace = workspace;

                self.close_chart_header_menus();

                if let Some(PaneKind::Chart(id)) = self
                    .workspace_panes(workspace)
                    .and_then(|panes| panes.get(pane))
                    .cloned()
                {
                    self.primary_chart_id = Some(id);

                    let chart_sym = self.charts.get(&id).and_then(|inst| {
                        let sym = inst.symbol.clone();
                        if !sym.is_empty() && sym != self.active_symbol {
                            Some(sym)
                        } else {
                            None
                        }
                    });

                    if let Some(sym) = chart_sym {
                        if let Some(symbol) = self.resolve_exchange_symbol_by_key_or_ticker(&sym)
                            && let Err(message) =
                                self.validate_exchange_symbol_orderable(symbol, "Chart")
                        {
                            self.order_status = Some((message, true));
                            return Task::none();
                        }
                        let symbol_key = sym.clone();
                        // Resolve fresh instead of copying the chart's cached
                        // label, so a stale placeholder never becomes the
                        // global active-symbol display.
                        let display = self.display_name_for_symbol(&sym);
                        self.apply_active_symbol_selection(sym, display);
                        self.reset_active_order_books_for_symbol(&symbol_key);
                        self.sync_all_chart_overlays();
                        for inst in self.charts.values_mut() {
                            inst.clear_quick_order();
                        }
                        self.chart_quick_order_surface.clear();
                        self.persist_config();

                        let book_task = Task::batch(
                            self.order_books
                                .values()
                                .filter(|book| book.mode == OrderBookSymbolMode::Active)
                                .map(|book| book.id)
                                .collect::<Vec<_>>()
                                .into_iter()
                                .map(|id| self.order_book_fetch_task_for_id(id)),
                        );
                        return book_task;
                    }
                }
            }
            Message::ClosePane(workspace, pane) => {
                let can_close_pane = self
                    .workspace_panes(workspace)
                    .and_then(|panes| panes.get(pane))
                    .is_some_and(PaneKind::can_be_closed);
                let pane_count = self
                    .workspace_panes(workspace)
                    .map(|panes| panes.iter().count())
                    .unwrap_or_default();
                if can_close_pane
                    && pane_count > 1
                    && let Some((closed_kind, sibling)) = self
                        .workspace_panes_mut(workspace)
                        .and_then(|panes| panes.close(pane))
                {
                    self.set_workspace_focus(workspace, Some(sibling));
                    self.last_focused_workspace = workspace;
                    let mut detached_window_to_close = None;
                    let mut quick_trade_window_to_close = None;
                    let closed_target =
                        crate::config::WidgetPaddingTargetConfig::from_pane_kind(&closed_kind);
                    let instance_still_open = self.workspace_pane_kinds().any(|(_, _, kind)| {
                        crate::config::WidgetPaddingTargetConfig::from_pane_kind(kind)
                            == closed_target
                    });

                    if !instance_still_open {
                        self.remove_widget_padding_override_for_kind(&closed_kind);

                        match closed_kind {
                            PaneKind::Chart(id) => {
                                if self
                                    .quick_trade_editor
                                    .as_ref()
                                    .is_some_and(|editor| editor.chart_id == id)
                                {
                                    quick_trade_window_to_close = self
                                        .quick_trade_editor
                                        .take()
                                        .map(|editor| editor.window_id);
                                }
                                self.clear_chart_surface_state(id, ChartSurfaceId::Docked(id));
                                detached_window_to_close = self.detached_chart_window_for(id);
                                if let Some(window_id) = detached_window_to_close {
                                    self.remove_detached_chart_window_state(window_id);
                                }
                                self.clear_chart_pending_request_state(id);
                                self.charts.remove(&id);
                                if self.primary_chart_id == Some(id) {
                                    self.primary_chart_id = self.charts.keys().next().copied();
                                }
                            }
                            PaneKind::SpaghettiChart(id) => {
                                detached_window_to_close = self.detached_spaghetti_window_for(id);
                                if let Some(window_id) = detached_window_to_close {
                                    self.remove_detached_spaghetti_window_state(window_id);
                                }
                                self.spaghetti_charts.remove(&id);
                            }
                            PaneKind::LiveWatchlist(id) => {
                                self.live_watchlists.remove(&id);
                                if self.live_watchlist_settings_menu_open == Some(id) {
                                    self.live_watchlist_settings_menu_open = None;
                                }
                            }
                            PaneKind::PositioningInfo(id) => {
                                self.positioning_infos.remove(&id);
                                for pending in self.positioning_info_pending.values_mut() {
                                    pending.retain(|pending_id| *pending_id != id);
                                }
                                self.positioning_info_pending
                                    .retain(|_, pending| !pending.is_empty());
                            }
                            PaneKind::OrderBook(id) => {
                                self.order_books.remove(&id);
                            }
                            PaneKind::SessionData(id) => {
                                self.session_data.remove(&id);
                            }
                            PaneKind::XFeed(id) => {
                                self.x_feed.instances.remove(&id);
                            }
                            _ => {}
                        }
                    }
                    self.persist_config();
                    let mut tasks = Vec::new();
                    if workspace == WorkspaceId::Main {
                        tasks.push(self.sync_main_window_min_size());
                    }
                    if let Some(window_id) = detached_window_to_close {
                        tasks.push(iced::window::close(window_id));
                    }
                    if let Some(window_id) = quick_trade_window_to_close {
                        tasks.push(iced::window::close(window_id));
                    }
                    return Task::batch(tasks);
                }
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests;
