use crate::account_state::BottomTab;
use crate::app_state::TradingTerminal;
use crate::canvas_state::{CanvasState, WorkspaceId};
use crate::chart_state::ChartId;
use crate::config;
use crate::pane_state::PaneKind;
use crate::spaghetti_state::SpaghettiChartId;
use iced::widget::pane_grid;

// ---------------------------------------------------------------------------
// Layout Pane Restoration
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn restore_layout_canvases(
        &mut self,
        canvas_configs: &[config::CanvasConfig],
    ) -> Vec<iced::Task<crate::message::Message>> {
        let old_window_ids = self
            .canvases
            .values()
            .filter_map(|canvas| canvas.window_id)
            .collect::<Vec<_>>();
        self.canvases.clear();
        self.preserved_unavailable_canvases.clear();
        self.next_canvas_id = 0;

        let mut open_ids = Vec::new();
        for canvas_config in canvas_configs {
            self.next_canvas_id = self.next_canvas_id.max(canvas_config.id.saturating_add(1));
            let Some(configuration) = canvas_config
                .pane_layout
                .as_ref()
                .and_then(Self::pane_layout_to_configuration)
            else {
                self.preserved_unavailable_canvases
                    .push(canvas_config.clone());
                continue;
            };
            if self.canvases.contains_key(&canvas_config.id) {
                continue;
            }
            self.canvases.insert(
                canvas_config.id,
                CanvasState::from_config(
                    canvas_config,
                    pane_grid::State::with_configuration(configuration),
                ),
            );
            if canvas_config.open {
                open_ids.push(canvas_config.id);
            }
        }

        self.last_focused_workspace = WorkspaceId::Main;
        self.add_widget_workspace = WorkspaceId::Main;
        self.add_widget_menu_open = false;
        self.placing_widget = None;
        self.widget_placement_hover = None;

        let mut tasks = old_window_ids
            .into_iter()
            .map(iced::window::close)
            .collect::<Vec<_>>();
        tasks.extend(
            open_ids
                .into_iter()
                .map(|canvas_id| self.open_canvas_window(canvas_id)),
        );
        tasks
    }

    pub(super) fn restore_layout_panes(
        &mut self,
        layout: &config::SavedLayout,
        default_main_chart_id: ChartId,
    ) {
        let default_pane_config = default_pane_configuration(layout, default_main_chart_id);
        let pane_config = layout
            .pane_layout
            .as_ref()
            .and_then(Self::pane_layout_to_configuration)
            .unwrap_or(default_pane_config);

        self.panes = pane_grid::State::with_configuration(pane_config);
        self.reconcile_layout_widget_panes(default_main_chart_id);
        self.sync_primary_chart_id_from_panes();
    }

    fn reconcile_layout_widget_panes(&mut self, first_chart_id: ChartId) {
        let mut chart_ids_in_layout = std::collections::BTreeSet::new();
        let mut spaghetti_ids_in_layout = std::collections::BTreeSet::new();
        for (_, _, kind) in self.workspace_pane_kinds() {
            match kind {
                PaneKind::Chart(id) => {
                    chart_ids_in_layout.insert(*id);
                }
                PaneKind::SpaghettiChart(id) => {
                    spaghetti_ids_in_layout.insert(*id);
                }
                _ => {}
            }
        }

        if chart_ids_in_layout.is_empty()
            && let Some(anchor) = self.chart_anchor_pane()
        {
            let _ = self.panes.split(
                pane_grid::Axis::Vertical,
                anchor,
                PaneKind::Chart(first_chart_id),
            );
            chart_ids_in_layout.insert(first_chart_id);
        }

        let mut all_chart_ids: Vec<ChartId> = self.charts.keys().copied().collect();
        all_chart_ids.sort_unstable();
        for id in all_chart_ids {
            if !chart_ids_in_layout.contains(&id) {
                let anchor = self.chart_anchor_pane();
                if let Some(anchor) = anchor
                    && let Some((new_pane, _)) =
                        self.panes
                            .split(pane_grid::Axis::Vertical, anchor, PaneKind::Chart(id))
                {
                    chart_ids_in_layout.insert(id);
                    self.focus = Some(new_pane);
                }
            }
        }

        let mut all_spaghetti_ids: Vec<SpaghettiChartId> =
            self.spaghetti_charts.keys().copied().collect();
        all_spaghetti_ids.sort_unstable();
        for id in all_spaghetti_ids {
            if !spaghetti_ids_in_layout.contains(&id) {
                let anchor = self.chart_anchor_pane();
                if let Some(anchor) = anchor
                    && let Some((new_pane, _)) = self.panes.split(
                        pane_grid::Axis::Vertical,
                        anchor,
                        PaneKind::SpaghettiChart(id),
                    )
                {
                    spaghetti_ids_in_layout.insert(id);
                    self.focus = Some(new_pane);
                }
            }
        }
    }
}

fn default_pane_configuration(
    layout: &config::SavedLayout,
    first_chart_id: ChartId,
) -> pane_grid::Configuration<PaneKind> {
    use pane_grid::{Axis, Configuration as PaneCfg};

    let ratios = &layout.layout_ratios;
    let ratios = movable_pane_layout_ratios(ratios);
    let r0 = layout_ratio_or_default(ratios, 0, 0.70);
    let r1 = layout_ratio_or_default(ratios, 1, 0.50);
    let r2 = layout_ratio_or_default(ratios, 2, 0.55);
    let r3 = layout_ratio_or_default(ratios, 3, 0.65);

    PaneCfg::Split {
        axis: Axis::Horizontal,
        ratio: r0,
        a: Box::new(PaneCfg::Split {
            axis: Axis::Vertical,
            ratio: r1,
            a: Box::new(PaneCfg::Pane(PaneKind::Chart(first_chart_id))),
            b: Box::new(PaneCfg::Split {
                axis: Axis::Vertical,
                ratio: r2,
                a: Box::new(PaneCfg::Pane(PaneKind::OrderBook(0))),
                b: Box::new(PaneCfg::Pane(PaneKind::Watchlist)),
            }),
        }),
        b: Box::new(PaneCfg::Split {
            axis: Axis::Vertical,
            ratio: r3,
            a: Box::new(PaneCfg::Pane(PaneKind::BottomTabs {
                active_tab: BottomTab::Positions,
            })),
            b: Box::new(PaneCfg::Pane(PaneKind::OrderEntry)),
        }),
    }
}

fn layout_ratio_or_default(ratios: &[f32], index: usize, default: f32) -> f32 {
    ratios
        .get(index)
        .copied()
        .map(config::normalize_pane_split_ratio)
        .unwrap_or(default)
}

fn movable_pane_layout_ratios(ratios: &[f32]) -> &[f32] {
    if ratios.len() >= 5 {
        &ratios[1..]
    } else {
        ratios
    }
}
