use crate::app_state::TradingTerminal;
use crate::canvas_state::WorkspaceId;
use crate::chart_state::ChartSurfaceId;
use crate::pane_state::PaneKind;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn toggle_close_menu(&mut self, coin: String) {
        if self.close_menu_coin.as_deref() == Some(&coin) {
            self.close_menu_coin = None;
        } else {
            self.close_menu_coin = Some(coin);
        }
    }

    pub(crate) fn clear_workspace_transient_order_ui(&mut self, workspace: WorkspaceId) {
        let chart_ids = self
            .workspace_panes(workspace)
            .into_iter()
            .flat_map(|panes| panes.iter())
            .filter_map(|(_, kind)| match kind {
                PaneKind::Chart(id) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>();

        for chart_id in chart_ids {
            self.clear_chart_surface_state(chart_id, ChartSurfaceId::Docked(chart_id));
            if let Some(instance) = self.charts.get_mut(&chart_id) {
                instance.editor_open = false;
                instance.editor_search_query.clear();
                instance.editor_selected_index = None;
                instance.secondary_editor_open = false;
                instance.secondary_editor_search_query.clear();
                instance.secondary_editor_selected_index = None;
            }
        }
    }
}
