use crate::api::OrderBook;
use crate::app_state::TradingTerminal;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;
use iced::widget::pane_grid;

impl TradingTerminal {
    pub(crate) fn update_pane_interactions(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
                self.persist_config();
            }
            Message::PaneDragged(pane_grid::DragEvent::Picked { pane }) => {
                self.dragging_pane = Some(pane);
                self.close_chart_header_menus();
            }
            Message::PaneDragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.dragging_pane = None;
                self.panes.drop(pane, target);
                self.persist_config();
            }
            Message::PaneDragged(pane_grid::DragEvent::Canceled { .. }) => {
                self.dragging_pane = None;
            }
            Message::PaneClicked(pane) => {
                self.focus = Some(pane);

                self.close_chart_header_menus();

                if let Some(PaneKind::Chart(id)) = self.panes.get(pane).cloned() {
                    self.primary_chart_id = Some(id);

                    let chart_sym = self.charts.get(&id).and_then(|inst| {
                        let sym = inst.symbol.clone();
                        let display = inst.symbol_display.clone();
                        if !sym.is_empty() && sym != self.active_symbol {
                            Some((sym, display))
                        } else {
                            None
                        }
                    });

                    if let Some((sym, display)) = chart_sym {
                        self.active_symbol = sym;
                        self.active_symbol_display = display;
                        for inst in self.order_books.values_mut() {
                            if inst.mode == OrderBookSymbolMode::Active {
                                inst.set_book(OrderBook::empty());
                            }
                        }
                        self.sync_all_chart_overlays();
                        for inst in self.charts.values_mut() {
                            inst.clear_quick_order();
                        }
                        self.persist_config();

                        for inst in self.order_books.values_mut() {
                            if inst.mode == OrderBookSymbolMode::Active {
                                inst.book_loading = true;
                            }
                        }
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
            Message::ClosePane(pane) => {
                let can_close_pane = self.panes.get(pane).is_some_and(PaneKind::can_be_closed);
                if can_close_pane
                    && self.panes.iter().count() > 1
                    && let Some((closed_kind, sibling)) = self.panes.close(pane)
                {
                    self.focus = Some(sibling);

                    match closed_kind {
                        PaneKind::Chart(id) => {
                            self.charts.remove(&id);
                            if self.primary_chart_id == Some(id) {
                                self.primary_chart_id = self.charts.keys().next().copied();
                            }
                        }
                        PaneKind::SpaghettiChart(id) => {
                            self.spaghetti_charts.remove(&id);
                        }
                        PaneKind::LiveWatchlist(id) => {
                            self.live_watchlists.remove(&id);
                        }
                        PaneKind::OrderBook(id) => {
                            self.order_books.remove(&id);
                        }
                        _ => {}
                    }
                    self.persist_config();
                }
            }
            _ => {}
        }

        Task::none()
    }
}
