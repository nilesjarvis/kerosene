mod search;

use super::ChartId;
use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;

use self::search::{chart_editor_symbol_matches, compare_chart_editor_symbols};

impl TradingTerminal {
    pub(crate) fn chart_symbol_search_input_id(chart_id: ChartId) -> iced::widget::Id {
        iced::widget::Id::from(format!("chart_symbol_search_{chart_id}"))
    }

    pub(crate) fn chart_symbol_search_results_scroll_id(chart_id: ChartId) -> iced::widget::Id {
        iced::widget::Id::from(format!("chart_symbol_search_results_{chart_id}"))
    }

    pub(crate) fn chart_secondary_symbol_search_input_id(chart_id: ChartId) -> iced::widget::Id {
        iced::widget::Id::from(format!("chart_secondary_symbol_search_{chart_id}"))
    }

    pub(crate) fn chart_secondary_symbol_search_results_scroll_id(
        chart_id: ChartId,
    ) -> iced::widget::Id {
        iced::widget::Id::from(format!("chart_secondary_symbol_search_results_{chart_id}"))
    }

    pub(crate) fn focus_chart_symbol_search_input(chart_id: ChartId) -> Task<Message> {
        iced::widget::operation::focus(Self::chart_symbol_search_input_id(chart_id))
    }

    pub(crate) fn focus_chart_secondary_symbol_search_input(chart_id: ChartId) -> Task<Message> {
        iced::widget::operation::focus(Self::chart_secondary_symbol_search_input_id(chart_id))
    }

    pub(crate) fn scroll_chart_symbol_search_results_to(
        chart_id: ChartId,
        offset_y: f32,
    ) -> Task<Message> {
        iced::widget::operation::scroll_to(
            Self::chart_symbol_search_results_scroll_id(chart_id),
            iced::widget::scrollable::AbsoluteOffset {
                x: None,
                y: Some(offset_y),
            },
        )
        .map(|_: ()| Message::NoOp)
    }

    pub(crate) fn scroll_chart_secondary_symbol_search_results_to(
        chart_id: ChartId,
        offset_y: f32,
    ) -> Task<Message> {
        iced::widget::operation::scroll_to(
            Self::chart_secondary_symbol_search_results_scroll_id(chart_id),
            iced::widget::scrollable::AbsoluteOffset {
                x: None,
                y: Some(offset_y),
            },
        )
        .map(|_: ()| Message::NoOp)
    }

    pub(crate) fn active_candlestick_chart_id(&self) -> Option<ChartId> {
        if let Some(pane) = self.focus
            && let Some(PaneKind::Chart(id)) = self.panes.get(pane)
            && self.charts.contains_key(id)
        {
            return Some(*id);
        }

        if let Some(id) = self.primary_chart_id
            && self.charts.contains_key(&id)
        {
            return Some(id);
        }

        self.panes.iter().find_map(|(_, kind)| match kind {
            PaneKind::Chart(id) if self.charts.contains_key(id) => Some(*id),
            _ => None,
        })
    }

    pub(crate) fn active_chart_editor_id(&self) -> Option<ChartId> {
        if let Some(pane) = self.focus
            && let Some(PaneKind::Chart(id)) = self.panes.get(pane)
            && self
                .charts
                .get(id)
                .is_some_and(|instance| instance.editor_open)
        {
            return Some(*id);
        }

        if let Some(id) = self.primary_chart_id
            && self
                .charts
                .get(&id)
                .is_some_and(|instance| instance.editor_open)
        {
            return Some(id);
        }

        self.charts
            .iter()
            .find_map(|(id, instance)| instance.editor_open.then_some(*id))
    }

    pub(crate) fn active_chart_secondary_editor_id(&self) -> Option<ChartId> {
        if let Some(pane) = self.focus
            && let Some(PaneKind::Chart(id)) = self.panes.get(pane)
            && self
                .charts
                .get(id)
                .is_some_and(|instance| instance.secondary_editor_open)
        {
            return Some(*id);
        }

        if let Some(id) = self.primary_chart_id
            && self
                .charts
                .get(&id)
                .is_some_and(|instance| instance.secondary_editor_open)
        {
            return Some(id);
        }

        self.charts
            .iter()
            .find_map(|(id, instance)| instance.secondary_editor_open.then_some(*id))
    }

    pub(crate) fn open_quick_symbol_search(&mut self) -> Task<Message> {
        let Some(chart_id) = self.active_candlestick_chart_id() else {
            self.push_toast(
                "No candlestick chart available for quick search".to_string(),
                true,
            );
            return Task::none();
        };

        self.primary_chart_id = Some(chart_id);
        for inst in self.charts.values_mut() {
            inst.macro_menu_open = false;
        }
        if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.editor_open = true;
            instance.editor_search_query.clear();
            instance.editor_selected_index = None;
            instance.secondary_editor_open = false;
            instance.secondary_editor_search_query.clear();
            instance.secondary_editor_selected_index = None;
            instance.clear_quick_order();
            instance.chart.active_tool = None;
        }
        self.chart_quick_order_surface.remove(&chart_id);
        self.chart_surface_active_tools
            .remove(&crate::chart_state::ChartSurfaceId::Docked(chart_id));

        Task::batch([
            Self::focus_chart_symbol_search_input(chart_id),
            Self::scroll_chart_symbol_search_results_to(chart_id, 0.0),
        ])
    }

    pub(crate) fn chart_editor_symbol_matches(symbol: &ExchangeSymbol, query: &str) -> bool {
        chart_editor_symbol_matches(symbol, query)
    }

    pub(crate) fn chart_editor_filtered_symbols<'a>(
        &'a self,
        query: &str,
    ) -> Vec<&'a ExchangeSymbol> {
        let normalized_query = query.trim().to_lowercase();
        let favs = &self.favourite_symbols;
        let mut filtered: Vec<&ExchangeSymbol> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .filter(|symbol| Self::chart_editor_symbol_matches(symbol, &normalized_query))
            .collect();

        filtered.sort_by(|a, b| compare_chart_editor_symbols(a, b, &normalized_query, favs));

        filtered
    }
}
