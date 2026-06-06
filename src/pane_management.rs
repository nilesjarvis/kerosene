use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::pane_state::PaneKind;

use crate::helpers::pane_title;
use iced::widget::pane_grid;

// ---------------------------------------------------------------------------
// Pane insertion helpers
// ---------------------------------------------------------------------------

pub(crate) enum AddPaneOutcome {
    Added,
    Existing,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AddWidgetPlacement {
    Below,
    Right,
}

impl TradingTerminal {
    pub(crate) fn first_chart_pane(&self) -> Option<(pane_grid::Pane, ChartId)> {
        self.panes.iter().find_map(|(pane, kind)| match kind {
            PaneKind::Chart(id) => Some((*pane, *id)),
            _ => None,
        })
    }

    pub(crate) fn chart_anchor_pane(&self) -> Option<pane_grid::Pane> {
        self.first_chart_pane()
            .map(|(pane, _)| pane)
            .or_else(|| self.panes.iter().next().map(|(pane, _)| *pane))
    }

    pub(crate) fn sync_primary_chart_id_from_panes(&mut self) {
        self.primary_chart_id = self
            .first_chart_pane()
            .map(|(_, id)| id)
            .or_else(|| self.charts.keys().copied().min());
    }

    pub(crate) fn find_pane_matching<F>(&self, predicate: F) -> Option<pane_grid::Pane>
    where
        F: Fn(&PaneKind) -> bool,
    {
        self.panes
            .iter()
            .find_map(|(pane, kind)| predicate(kind).then_some(*pane))
    }

    pub(crate) fn pane_is_open<F>(&self, predicate: F) -> bool
    where
        F: Fn(&PaneKind) -> bool,
    {
        self.find_pane_matching(predicate).is_some()
    }

    pub(crate) fn add_target_pane(&self) -> Option<pane_grid::Pane> {
        if let Some(pane) = self.focus
            && self.panes.get(pane).is_some()
        {
            return Some(pane);
        }

        if let Some(chart_id) = self.primary_chart_id
            && let Some((pane, _)) = self
                .panes
                .iter()
                .find(|(_, kind)| matches!(kind, PaneKind::Chart(id) if *id == chart_id))
        {
            return Some(*pane);
        }

        self.find_pane_matching(|kind| matches!(kind, PaneKind::Chart(_)))
            .or_else(|| self.panes.iter().next().map(|(pane, _)| *pane))
    }

    pub(crate) fn add_target_title(&self) -> String {
        self.add_target_pane()
            .and_then(|pane| self.panes.get(pane))
            .map(pane_title)
            .unwrap_or_else(|| "No pane selected".to_string())
    }

    pub(crate) fn add_widget_axis(&self) -> pane_grid::Axis {
        match self.add_widget_placement {
            AddWidgetPlacement::Below => pane_grid::Axis::Horizontal,
            AddWidgetPlacement::Right => pane_grid::Axis::Vertical,
        }
    }

    fn split_new_pane(
        &mut self,
        axis: pane_grid::Axis,
        target: pane_grid::Pane,
        kind: PaneKind,
        label: &str,
    ) -> Option<pane_grid::Pane> {
        match self.panes.split(axis, target, kind) {
            Some((pane, _split)) => {
                self.focus = Some(pane);
                self.persist_config();
                Some(pane)
            }
            None => {
                self.push_toast(
                    format!("Could not add {label}: target pane is unavailable"),
                    true,
                );
                None
            }
        }
    }

    pub(crate) fn add_pane_to_target(
        &mut self,
        axis: pane_grid::Axis,
        target: pane_grid::Pane,
        kind: PaneKind,
        label: &str,
    ) -> Option<pane_grid::Pane> {
        self.split_new_pane(axis, target, kind, label)
    }

    pub(crate) fn add_pane_next_to_focus(
        &mut self,
        axis: pane_grid::Axis,
        kind: PaneKind,
        label: &str,
    ) -> Option<pane_grid::Pane> {
        let Some(target) = self.add_target_pane() else {
            self.push_toast(format!("Could not add {label}: no pane is available"), true);
            return None;
        };
        self.split_new_pane(axis, target, kind, label)
    }

    pub(crate) fn add_or_focus_singleton_pane<F>(
        &mut self,
        axis: pane_grid::Axis,
        kind: PaneKind,
        label: &str,
        predicate: F,
    ) -> AddPaneOutcome
    where
        F: Fn(&PaneKind) -> bool,
    {
        if let Some(pane) = self.find_pane_matching(predicate) {
            self.focus = Some(pane);
            self.push_toast(format!("{label} is already open"), false);
            return AddPaneOutcome::Existing;
        }

        match self.add_pane_next_to_focus(axis, kind, label) {
            Some(_) => AddPaneOutcome::Added,
            None => AddPaneOutcome::Failed,
        }
    }
}
