use crate::api::{OrderBook, fetch_order_book};
use crate::app_state::TradingTerminal;
use crate::helpers;
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
            Message::PaneDragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.panes.drop(pane, target);
                self.persist_config();
            }
            Message::PaneDragged(_) => {}
            Message::PaneClicked(pane) => {
                self.focus = Some(pane);

                for inst in self.charts.values_mut() {
                    inst.macro_menu_open = false;
                }
                for inst in self.spaghetti_charts.values_mut() {
                    inst.style_menu_open = false;
                }
                self.account_picker_open = false;

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
                        let chase_task = if self.active_chase.is_some() {
                            self.stop_chase_with_reason(
                                "Chase stopped: focused chart changed the active symbol",
                                false,
                            )
                        } else {
                            Task::none()
                        };

                        self.active_symbol = sym;
                        self.active_symbol_display = display;
                        for inst in self.order_books.values_mut() {
                            if inst.mode == OrderBookSymbolMode::Active {
                                inst.book = OrderBook::empty();
                            }
                        }
                        self.sync_all_chart_overlays();
                        for inst in self.charts.values_mut() {
                            inst.quick_order = None;
                        }
                        self.persist_config();

                        for inst in self.order_books.values_mut() {
                            if inst.mode == OrderBookSymbolMode::Active {
                                inst.book_loading = true;
                            }
                        }
                        let sym = self.active_symbol.clone();
                        let book_task = Task::batch(
                            self.order_books
                                .values()
                                .filter(|book| book.mode == OrderBookSymbolMode::Active)
                                .map(|book| {
                                    let id = book.id;
                                    let sigfigs = helpers::compute_sigfigs(
                                        book.tick_size,
                                        book.book.mid_price(),
                                    );
                                    Task::perform(
                                        fetch_order_book(sym.clone(), sigfigs),
                                        move |res| Message::BookLoaded(id, res),
                                    )
                                }),
                        );
                        return Task::batch([chase_task, book_task]);
                    }
                }
            }
            Message::ClosePane(pane) => {
                if self.panes.iter().count() > 1
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
