use crate::annotations::DrawingTool;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::pane_state::PaneKind;

use iced::window;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Detached Chart Surface State
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn detached_chart_window_for(&self, chart_id: ChartId) -> Option<window::Id> {
        self.detached_chart_windows
            .iter()
            .find_map(|(window_id, state)| (state.chart_id == chart_id).then_some(*window_id))
    }

    pub(crate) fn chart_is_docked(&self, chart_id: ChartId) -> bool {
        self.panes
            .iter()
            .any(|(_, kind)| matches!(kind, PaneKind::Chart(id) if *id == chart_id))
    }

    pub(crate) fn active_chart_surface_tool(
        &self,
        chart_id: ChartId,
        surface_id: ChartSurfaceId,
    ) -> Option<DrawingTool> {
        self.chart_surface_active_tools
            .get(&surface_id)
            .copied()
            .or_else(|| {
                self.charts
                    .get(&chart_id)
                    .and_then(|inst| (inst.chart.surface_id() == surface_id).then_some(inst))
                    .and_then(|inst| inst.chart.active_tool)
            })
    }

    pub(crate) fn chart_surface_has_quick_order(
        &self,
        chart_id: ChartId,
        surface_id: ChartSurfaceId,
    ) -> bool {
        let Some(instance) = self.charts.get(&chart_id) else {
            return false;
        };
        if instance.quick_order.is_none() {
            return false;
        }
        self.chart_quick_order_surface
            .get(&chart_id)
            .copied()
            .unwrap_or_else(|| instance.chart.surface_id())
            == surface_id
    }

    pub(crate) fn clear_chart_surface_state(
        &mut self,
        chart_id: ChartId,
        surface_id: ChartSurfaceId,
    ) {
        self.chart_surface_active_tools.remove(&surface_id);
        self.chart_surface_viewports.remove(&surface_id);
        if self.chart_screenshot_menu_open == Some(surface_id) {
            self.chart_screenshot_menu_open = None;
        }
        if let Some(instance) = self.charts.get_mut(&chart_id)
            && instance.chart.surface_id() == surface_id
        {
            instance.chart.active_tool = None;
        }

        let quick_order_surface = self
            .chart_quick_order_surface
            .get(&chart_id)
            .copied()
            .or_else(|| {
                self.charts
                    .get(&chart_id)
                    .map(|inst| inst.chart.surface_id())
            });
        if quick_order_surface == Some(surface_id) {
            if let Some(instance) = self.charts.get_mut(&chart_id) {
                instance.clear_quick_order();
            }
            self.chart_quick_order_surface.remove(&chart_id);
        }
    }

    pub(crate) fn clear_all_chart_surface_state(&mut self, chart_id: ChartId) {
        self.clear_chart_surface_state(chart_id, ChartSurfaceId::Docked(chart_id));
        if let Some(window_id) = self.detached_chart_window_for(chart_id) {
            self.clear_chart_surface_state(chart_id, ChartSurfaceId::Detached(window_id));
        }
        if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.chart.active_tool = None;
        }
    }

    pub(crate) fn clear_chart_pending_request_state(&mut self, chart_id: ChartId) {
        self.clear_chart_heatmap_pending_request_state(chart_id);
        self.clear_chart_liquidation_pending_request_state(chart_id);
        self.clear_chart_earnings_pending_request_state(chart_id);
    }

    pub(crate) fn clear_chart_heatmap_pending_request_state(&mut self, chart_id: ChartId) {
        retain_pending_chart_ids(&mut self.heatmap_pending_charts, chart_id);
    }

    pub(crate) fn clear_chart_liquidation_pending_request_state(&mut self, chart_id: ChartId) {
        retain_pending_chart_ids(&mut self.liquidation_pending_charts, chart_id);
    }

    pub(crate) fn clear_chart_earnings_pending_request_state(&mut self, chart_id: ChartId) {
        let empty_earnings_tickers =
            remove_pending_chart_id(&mut self.sec_earnings_pending_charts, chart_id);
        for ticker in empty_earnings_tickers {
            self.sec_earnings_pending_charts.remove(&ticker);
            self.sec_earnings_pending_request_ids.remove(&ticker);
        }
        let empty_filing_summary_keys =
            remove_pending_chart_id(&mut self.sec_filing_summary_pending_charts, chart_id);
        for key in empty_filing_summary_keys {
            self.sec_filing_summary_pending_charts.remove(&key);
            self.sec_filing_summary_pending_request_ids.remove(&key);
        }
    }

    pub(crate) fn clear_all_chart_pending_request_state(&mut self) {
        self.heatmap_pending_charts.clear();
        self.liquidation_pending_charts.clear();
        self.sec_earnings_pending_charts.clear();
        self.sec_earnings_pending_request_ids.clear();
        self.sec_filing_summary_pending_charts.clear();
        self.sec_filing_summary_pending_request_ids.clear();
    }

    pub(crate) fn remove_detached_chart_window_state(&mut self, window_id: window::Id) -> bool {
        let Some(state) = self.detached_chart_windows.remove(&window_id) else {
            return false;
        };
        self.clear_chart_surface_state(state.chart_id, ChartSurfaceId::Detached(window_id));
        if !self.chart_is_docked(state.chart_id) {
            self.clear_chart_pending_request_state(state.chart_id);
            self.charts.remove(&state.chart_id);
        }
        if self
            .primary_chart_id
            .is_some_and(|chart_id| !self.charts.contains_key(&chart_id))
        {
            self.sync_primary_chart_id_from_panes();
        }
        true
    }

    pub(crate) fn prune_chart_surface_state(&mut self) {
        let valid_chart_ids: HashSet<_> = self.charts.keys().copied().collect();
        let valid_window_ids: HashSet<_> = self.detached_chart_windows.keys().copied().collect();

        self.chart_surface_active_tools.retain(|surface_id, _| {
            surface_is_valid(*surface_id, &valid_chart_ids, &valid_window_ids)
        });
        self.chart_surface_viewports.retain(|surface_id, _| {
            surface_is_valid(*surface_id, &valid_chart_ids, &valid_window_ids)
        });

        let stale_quick_order_charts: Vec<_> = self
            .chart_quick_order_surface
            .iter()
            .filter_map(|(chart_id, surface_id)| {
                (!valid_chart_ids.contains(chart_id)
                    || !surface_is_valid(*surface_id, &valid_chart_ids, &valid_window_ids))
                .then_some(*chart_id)
            })
            .collect();
        for chart_id in stale_quick_order_charts {
            if let Some(instance) = self.charts.get_mut(&chart_id) {
                instance.clear_quick_order();
            }
            self.chart_quick_order_surface.remove(&chart_id);
        }
    }
}

fn retain_pending_chart_ids(pending: &mut HashMap<String, Vec<ChartId>>, chart_id: ChartId) {
    pending.retain(|_, waiting_charts| {
        waiting_charts.retain(|pending_id| *pending_id != chart_id);
        !waiting_charts.is_empty()
    });
}

fn remove_pending_chart_id(
    pending: &mut HashMap<String, Vec<ChartId>>,
    chart_id: ChartId,
) -> Vec<String> {
    pending
        .iter_mut()
        .filter_map(|(key, waiting_charts)| {
            waiting_charts.retain(|pending_id| *pending_id != chart_id);
            waiting_charts.is_empty().then_some(key.clone())
        })
        .collect()
}

fn surface_is_valid(
    surface_id: ChartSurfaceId,
    valid_chart_ids: &HashSet<ChartId>,
    valid_window_ids: &HashSet<window::Id>,
) -> bool {
    match surface_id {
        ChartSurfaceId::Docked(chart_id) => valid_chart_ids.contains(&chart_id),
        ChartSurfaceId::Detached(window_id) => valid_window_ids.contains(&window_id),
    }
}
