use crate::api::OrderBook;
use crate::app_state::TradingTerminal;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::widget::pane_grid;
use iced::{Size, Task};

const MAIN_STATUS_BAR_RESERVED_HEIGHT: f32 = 28.0;
const ORDER_ENTRY_MIN_WIDTH: f32 = 300.0;
const ORDER_ENTRY_MIN_HEIGHT: f32 = 360.0;

impl TradingTerminal {
    pub(crate) fn update_pane_interactions(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                let ratio = self.clamp_order_entry_resize_ratio(split, ratio);
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
                return self.sync_main_window_min_size();
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
                    return self.sync_main_window_min_size();
                }
            }
            _ => {}
        }

        Task::none()
    }

    fn clamp_order_entry_resize_ratio(&self, split: pane_grid::Split, ratio: f32) -> f32 {
        let base_min_size = self.account_summary_pane_min_size();
        let size = self.main_pane_grid_size();
        let pane_border_thickness = self.pane_border_thickness;
        let split_regions =
            self.panes
                .layout()
                .split_regions(pane_border_thickness, base_min_size, size);
        let Some((axis, region, _current_ratio)) = split_regions.get(&split).copied() else {
            return ratio;
        };
        let Some((_, a, b)) = split_node(self.panes.layout(), split) else {
            return ratio;
        };

        let order_entry_in_a = subtree_contains_order_entry(a, &self.panes);
        let order_entry_in_b = subtree_contains_order_entry(b, &self.panes);
        if !order_entry_in_a && !order_entry_in_b {
            return ratio;
        }

        let min_a =
            subtree_min_length(a, axis, &self.panes, base_min_size, pane_border_thickness);
        let min_b =
            subtree_min_length(b, axis, &self.panes, base_min_size, pane_border_thickness);
        let axis_length = match axis {
            pane_grid::Axis::Horizontal => region.height,
            pane_grid::Axis::Vertical => region.width,
        };

        clamp_split_ratio(
            ratio,
            axis_length,
            min_a,
            min_b,
            order_entry_in_a,
            order_entry_in_b,
            pane_border_thickness,
        )
    }

    fn main_pane_grid_size(&self) -> Size {
        let size = self.main_window_size.unwrap_or(Size::new(1600.0, 960.0));
        Size::new(
            size.width,
            (size.height - MAIN_STATUS_BAR_RESERVED_HEIGHT).max(1.0),
        )
    }

    pub(crate) fn main_window_min_size(&self) -> Size {
        let base_min_size = self.account_summary_pane_min_size();
        let layout = self.panes.layout();

        Size::new(
            subtree_min_length(
                layout,
                pane_grid::Axis::Vertical,
                &self.panes,
                base_min_size,
                self.pane_border_thickness,
            ),
            subtree_min_length(
                layout,
                pane_grid::Axis::Horizontal,
                &self.panes,
                base_min_size,
                self.pane_border_thickness,
            ) + MAIN_STATUS_BAR_RESERVED_HEIGHT,
        )
    }

    pub(crate) fn sync_main_window_min_size(&self) -> Task<Message> {
        self.main_window_id
            .map(|id| iced::window::set_min_size(id, Some(self.main_window_min_size())))
            .unwrap_or_else(Task::none)
    }
}

fn split_node(
    node: &pane_grid::Node,
    split: pane_grid::Split,
) -> Option<(pane_grid::Axis, &pane_grid::Node, &pane_grid::Node)> {
    match node {
        pane_grid::Node::Split { id, axis, a, b, .. } => {
            if *id == split {
                Some((*axis, a, b))
            } else {
                split_node(a, split).or_else(|| split_node(b, split))
            }
        }
        pane_grid::Node::Pane(_) => None,
    }
}

fn subtree_contains_order_entry(
    node: &pane_grid::Node,
    panes: &pane_grid::State<PaneKind>,
) -> bool {
    match node {
        pane_grid::Node::Split { a, b, .. } => {
            subtree_contains_order_entry(a, panes) || subtree_contains_order_entry(b, panes)
        }
        pane_grid::Node::Pane(pane) => {
            matches!(panes.get(*pane), Some(PaneKind::OrderEntry))
        }
    }
}

fn subtree_min_length(
    node: &pane_grid::Node,
    measured_axis: pane_grid::Axis,
    panes: &pane_grid::State<PaneKind>,
    base_min_size: f32,
    pane_border_thickness: f32,
) -> f32 {
    match node {
        pane_grid::Node::Split { axis, a, b, .. } => {
            let min_a =
                subtree_min_length(a, measured_axis, panes, base_min_size, pane_border_thickness);
            let min_b =
                subtree_min_length(b, measured_axis, panes, base_min_size, pane_border_thickness);

            if *axis == measured_axis {
                min_a + min_b + pane_border_thickness
            } else {
                min_a.max(min_b)
            }
        }
        pane_grid::Node::Pane(pane) => panes
            .get(*pane)
            .map(|kind| pane_min_length(kind, measured_axis, base_min_size))
            .unwrap_or(base_min_size),
    }
}

fn pane_min_length(kind: &PaneKind, axis: pane_grid::Axis, base_min_size: f32) -> f32 {
    match (kind, axis) {
        (PaneKind::OrderEntry, pane_grid::Axis::Horizontal) => ORDER_ENTRY_MIN_HEIGHT,
        (PaneKind::OrderEntry, pane_grid::Axis::Vertical) => ORDER_ENTRY_MIN_WIDTH,
        _ => base_min_size,
    }
}

fn clamp_split_ratio(
    ratio: f32,
    axis_length: f32,
    min_a: f32,
    min_b: f32,
    order_entry_in_a: bool,
    order_entry_in_b: bool,
    pane_border_thickness: f32,
) -> f32 {
    if !ratio.is_finite() || axis_length <= 0.0 {
        return ratio;
    }

    let raw_a = (axis_length * ratio - pane_border_thickness / 2.0).round();
    let max_a = axis_length - min_b - pane_border_thickness;
    let target_a = if max_a >= min_a {
        raw_a.clamp(min_a, max_a)
    } else if order_entry_in_a && !order_entry_in_b {
        min_a.min((axis_length - pane_border_thickness).max(0.0))
    } else if order_entry_in_b && !order_entry_in_a {
        (axis_length - min_b - pane_border_thickness).max(0.0)
    } else {
        raw_a.clamp(0.0, (axis_length - pane_border_thickness).max(0.0))
    };

    ((target_a + pane_border_thickness / 2.0) / axis_length).clamp(0.0, 1.0)
}
