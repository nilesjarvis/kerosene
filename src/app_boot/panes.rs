use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::pane_state::PaneKind;
use crate::spaghetti_state::SpaghettiChartId;

use iced::widget::pane_grid;
use std::collections::BTreeSet;

impl TradingTerminal {
    pub(super) fn ensure_boot_layout_chart_panes(
        &mut self,
        first_chart_id: ChartId,
        detached_chart_ids: &BTreeSet<ChartId>,
    ) {
        let mut chart_ids_in_layout = BTreeSet::new();
        let mut spaghetti_ids_in_layout = BTreeSet::new();
        for (_, kind) in self.panes.iter() {
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

        self.ensure_loaded_chart_panes(&mut chart_ids_in_layout, detached_chart_ids);
        self.ensure_loaded_spaghetti_panes(&mut spaghetti_ids_in_layout);

        self.sync_primary_chart_id_from_panes();
    }

    fn ensure_loaded_chart_panes(
        &mut self,
        chart_ids_in_layout: &mut BTreeSet<ChartId>,
        detached_chart_ids: &BTreeSet<ChartId>,
    ) {
        let mut all_chart_ids: Vec<ChartId> = self.charts.keys().copied().collect();
        all_chart_ids.sort_unstable();

        for id in all_chart_ids {
            if chart_ids_in_layout.contains(&id) || detached_chart_ids.contains(&id) {
                continue;
            }

            if let Some(anchor) = self.chart_anchor_pane()
                && let Some((new_pane, _)) =
                    self.panes
                        .split(pane_grid::Axis::Vertical, anchor, PaneKind::Chart(id))
            {
                chart_ids_in_layout.insert(id);
                self.focus = Some(new_pane);
            }
        }
    }

    fn ensure_loaded_spaghetti_panes(
        &mut self,
        spaghetti_ids_in_layout: &mut BTreeSet<SpaghettiChartId>,
    ) {
        let mut all_spaghetti_ids: Vec<SpaghettiChartId> =
            self.spaghetti_charts.keys().copied().collect();
        all_spaghetti_ids.sort_unstable();

        for sid in all_spaghetti_ids {
            if spaghetti_ids_in_layout.contains(&sid) {
                continue;
            }

            if let Some(anchor) = self.chart_anchor_pane()
                && let Some((new_pane, _)) = self.panes.split(
                    pane_grid::Axis::Vertical,
                    anchor,
                    PaneKind::SpaghettiChart(sid),
                )
            {
                spaghetti_ids_in_layout.insert(sid);
                self.focus = Some(new_pane);
            }
        }
    }
}
