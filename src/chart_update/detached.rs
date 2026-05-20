use crate::annotations::DrawingTool;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId, DetachedChartWindowState};
use crate::message::Message;
use iced::{Task, window};
use std::collections::HashSet;

impl TradingTerminal {
    pub(crate) fn detached_chart_window_for(&self, chart_id: ChartId) -> Option<window::Id> {
        self.detached_chart_windows
            .iter()
            .find_map(|(window_id, state)| (state.chart_id == chart_id).then_some(*window_id))
    }

    pub(crate) fn chart_has_detached_window(&self, chart_id: ChartId) -> bool {
        self.detached_chart_window_for(chart_id).is_some()
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
                (surface_id == ChartSurfaceId::Docked(chart_id))
                    .then(|| {
                        self.charts
                            .get(&chart_id)
                            .and_then(|inst| inst.chart.active_tool)
                    })
                    .flatten()
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
            .unwrap_or(ChartSurfaceId::Docked(chart_id))
            == surface_id
    }

    pub(crate) fn clear_chart_surface_state(
        &mut self,
        chart_id: ChartId,
        surface_id: ChartSurfaceId,
    ) {
        self.chart_surface_reset_epochs.remove(&surface_id);
        self.chart_surface_active_tools.remove(&surface_id);
        self.chart_surface_viewports.remove(&surface_id);
        if self.chart_screenshot_menu_open == Some(surface_id) {
            self.chart_screenshot_menu_open = None;
        }
        if self.chart_quick_order_surface.get(&chart_id) == Some(&surface_id) {
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

    pub(crate) fn remove_detached_chart_window_state(&mut self, window_id: window::Id) -> bool {
        let Some(state) = self.detached_chart_windows.remove(&window_id) else {
            return false;
        };
        self.clear_chart_surface_state(state.chart_id, ChartSurfaceId::Detached(window_id));
        true
    }

    pub(crate) fn prune_chart_surface_state(&mut self) {
        let valid_chart_ids: HashSet<_> = self.charts.keys().copied().collect();
        let valid_window_ids: HashSet<_> = self.detached_chart_windows.keys().copied().collect();

        self.chart_surface_reset_epochs.retain(|surface_id, _| {
            surface_is_valid(*surface_id, &valid_chart_ids, &valid_window_ids)
        });
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

    pub(super) fn open_detached_chart_window(&mut self, chart_id: ChartId) -> Task<Message> {
        self.close_chart_header_menus();
        self.add_widget_menu_open = false;

        if !self.charts.contains_key(&chart_id) {
            self.push_toast(
                "Chart window unavailable: chart not found".to_string(),
                true,
            );
            return Task::none();
        }

        if let Some(window_id) = self.detached_chart_window_for(chart_id) {
            return window::gain_focus(window_id);
        }

        let state = DetachedChartWindowState::new(chart_id);
        let settings = window::Settings {
            size: state.size(),
            position: state.position(),
            ..crate::window_chrome::settings()
        };
        let (window_id, task) = window::open(settings);
        self.detached_chart_windows.insert(window_id, state);
        self.persist_config();

        task.map(Message::WindowOpened)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::order_execution::QuickOrderForm;
    use crate::timeframe::Timeframe;

    fn terminal_with_chart(chart_id: ChartId) -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal.detached_chart_windows.clear();
        terminal.chart_surface_reset_epochs.clear();
        terminal.chart_surface_active_tools.clear();
        terminal.chart_surface_viewports.clear();
        terminal.chart_quick_order_surface.clear();
        terminal.charts.insert(
            chart_id,
            ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1),
        );
        terminal
    }

    #[test]
    fn open_detached_chart_window_reuses_existing_window_for_chart() {
        let chart_id = 7;
        let mut terminal = terminal_with_chart(chart_id);

        let _task = terminal.open_detached_chart_window(chart_id);
        let first_window_id = terminal
            .detached_chart_window_for(chart_id)
            .expect("detached chart window");

        let _task = terminal.open_detached_chart_window(chart_id);

        assert_eq!(terminal.detached_chart_windows.len(), 1);
        assert_eq!(
            terminal.detached_chart_window_for(chart_id),
            Some(first_window_id)
        );
    }

    #[test]
    fn clear_detached_surface_state_removes_only_detached_quick_order_owner() {
        let chart_id = 7;
        let mut terminal = terminal_with_chart(chart_id);
        let _task = terminal.open_detached_chart_window(chart_id);
        let window_id = terminal
            .detached_chart_window_for(chart_id)
            .expect("detached chart window");
        let surface_id = ChartSurfaceId::Detached(window_id);
        let instance = terminal.charts.get_mut(&chart_id).expect("chart instance");
        instance.set_quick_order(QuickOrderForm {
            price: 100.0,
            quantity: "1".to_string(),
            quantity_is_usd: false,
            percentage: 0.0,
            is_limit: true,
            click_x: 10.0,
            click_y: 20.0,
            chart_w: 300.0,
            chart_h: 200.0,
        });
        terminal
            .chart_quick_order_surface
            .insert(chart_id, surface_id);

        assert!(terminal.chart_surface_has_quick_order(chart_id, surface_id));

        terminal.clear_chart_surface_state(chart_id, surface_id);

        assert!(!terminal.chart_surface_has_quick_order(chart_id, surface_id));
        assert!(
            !terminal
                .charts
                .get(&chart_id)
                .expect("chart instance")
                .chart
                .quick_order_open
        );
    }
}
