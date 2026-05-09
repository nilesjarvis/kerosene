use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::pane_state::PaneKind;
use crate::spaghetti_state::SpaghettiChartId;

use iced::widget::pane_grid;
use std::collections::BTreeSet;

impl TradingTerminal {
    pub(super) fn ensure_boot_layout_chart_panes(&mut self, first_chart_id: ChartId) {
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

        if chart_ids_in_layout.is_empty() {
            let anchor = self.panes.iter().next().map(|(p, _)| *p);
            if let Some(anchor) = anchor {
                let _ = self.panes.split(
                    pane_grid::Axis::Vertical,
                    anchor,
                    PaneKind::Chart(first_chart_id),
                );
                chart_ids_in_layout.insert(first_chart_id);
            }
        }

        self.ensure_loaded_chart_panes(&mut chart_ids_in_layout);
        self.ensure_loaded_spaghetti_panes(&mut spaghetti_ids_in_layout);

        self.primary_chart_id = self
            .panes
            .iter()
            .find_map(|(_, kind)| {
                if let PaneKind::Chart(id) = kind {
                    Some(*id)
                } else {
                    None
                }
            })
            .or_else(|| self.charts.keys().copied().min());
    }

    fn ensure_loaded_chart_panes(&mut self, chart_ids_in_layout: &mut BTreeSet<ChartId>) {
        let mut all_chart_ids: Vec<ChartId> = self.charts.keys().copied().collect();
        all_chart_ids.sort_unstable();

        for id in all_chart_ids {
            if chart_ids_in_layout.contains(&id) {
                continue;
            }

            let anchor = self
                .panes
                .iter()
                .find_map(|(p, kind)| matches!(kind, PaneKind::Chart(_)).then_some(*p))
                .or_else(|| self.panes.iter().next().map(|(p, _)| *p));
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

            let anchor = self
                .panes
                .iter()
                .find_map(|(p, kind)| matches!(kind, PaneKind::Chart(_)).then_some(*p))
                .or_else(|| self.panes.iter().next().map(|(p, _)| *p));
            if let Some(anchor) = anchor
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
