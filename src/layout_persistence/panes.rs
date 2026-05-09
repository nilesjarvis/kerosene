use crate::account_state::BottomTab;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::config;
use crate::pane_state::PaneKind;
use crate::spaghetti_state::SpaghettiChartId;
use iced::widget::pane_grid;

// ---------------------------------------------------------------------------
// Layout Pane Restoration
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn restore_layout_panes(&mut self, layout: &config::SavedLayout) {
        let first_chart_id = self.charts.keys().copied().min().unwrap_or(0);
        let default_pane_config = default_pane_configuration(layout, first_chart_id);
        let pane_config = layout
            .pane_layout
            .as_ref()
            .map(Self::pane_layout_to_configuration)
            .unwrap_or(default_pane_config);

        self.panes = pane_grid::State::with_configuration(pane_config);
        self.reconcile_layout_widget_panes(first_chart_id);
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

    fn reconcile_layout_widget_panes(&mut self, first_chart_id: ChartId) {
        let mut chart_ids_in_layout = std::collections::BTreeSet::new();
        let mut spaghetti_ids_in_layout = std::collections::BTreeSet::new();
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
            let anchor = self.panes.iter().next().map(|(pane, _)| *pane);
            if let Some(anchor) = anchor {
                let _ = self.panes.split(
                    pane_grid::Axis::Vertical,
                    anchor,
                    PaneKind::Chart(first_chart_id),
                );
                chart_ids_in_layout.insert(first_chart_id);
            }
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

    fn chart_anchor_pane(&self) -> Option<pane_grid::Pane> {
        self.panes
            .iter()
            .find_map(|(pane, kind)| matches!(kind, PaneKind::Chart(_)).then_some(*pane))
            .or_else(|| self.panes.iter().next().map(|(pane, _)| *pane))
    }
}

fn default_pane_configuration(
    layout: &config::SavedLayout,
    first_chart_id: ChartId,
) -> pane_grid::Configuration<PaneKind> {
    use pane_grid::{Axis, Configuration as PaneCfg};

    let ratios = &layout.layout_ratios;
    let r0 = ratios.first().copied().unwrap_or(0.06);
    let r1 = ratios.get(1).copied().unwrap_or(0.70);
    let r2 = ratios.get(2).copied().unwrap_or(0.50);
    let r3 = ratios.get(3).copied().unwrap_or(0.55);
    let r4 = ratios.get(4).copied().unwrap_or(0.65);

    PaneCfg::Split {
        axis: Axis::Horizontal,
        ratio: r0,
        a: Box::new(PaneCfg::Pane(PaneKind::AccountSummary)),
        b: Box::new(PaneCfg::Split {
            axis: Axis::Horizontal,
            ratio: r1,
            a: Box::new(PaneCfg::Split {
                axis: Axis::Vertical,
                ratio: r2,
                a: Box::new(PaneCfg::Pane(PaneKind::Chart(first_chart_id))),
                b: Box::new(PaneCfg::Split {
                    axis: Axis::Vertical,
                    ratio: r3,
                    a: Box::new(PaneCfg::Pane(PaneKind::OrderBook(0))),
                    b: Box::new(PaneCfg::Pane(PaneKind::Watchlist)),
                }),
            }),
            b: Box::new(PaneCfg::Split {
                axis: Axis::Vertical,
                ratio: r4,
                a: Box::new(PaneCfg::Pane(PaneKind::BottomTabs {
                    active_tab: BottomTab::Positions,
                })),
                b: Box::new(PaneCfg::Pane(PaneKind::OrderEntry)),
            }),
        }),
    }
}
