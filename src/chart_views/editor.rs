mod symbol_row;
mod top_bar;

use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;
use iced::widget::{Column, column, container, rule, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    /// Render the inline symbol editor overlay for a chart pane.
    pub(crate) fn view_chart_editor(
        &self,
        chart_id: ChartId,
        instance: &ChartInstance,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let top_bar = self.view_chart_editor_top_bar(chart_id, instance);

        if self.exchange_symbols.is_empty() {
            let content = column![
                top_bar,
                container(
                    text("Loading symbols...")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text)
                )
                .padding([8, 0]),
            ]
            .spacing(4);

            return container(content)
                .width(Fill)
                .height(Fill)
                .padding(10)
                .into();
        }

        let filtered = self.chart_editor_filtered_symbols(&instance.editor_search_query);
        let favs = &self.favourite_symbols;

        let count_label = text(format!("{} symbols", filtered.len()))
            .size(11)
            .color(theme.extended_palette().background.weak.text);

        let current_sym = &instance.symbol;
        let id = chart_id;
        let mut rows = Column::new().spacing(2);
        let mut past_favs = false;

        for (i, sym) in filtered.iter().enumerate() {
            let is_fav = favs.contains(&sym.key);

            if !past_favs && !is_fav && i > 0 {
                past_favs = true;
                rows = rows.push(rule::horizontal(1));
            }

            let is_selected = sym.key == *current_sym;
            let is_keyboard_selected = instance.editor_keyboard_selected && i == 0;
            rows = rows.push(self.view_chart_editor_symbol_row(
                id,
                sym,
                is_fav,
                is_selected,
                is_keyboard_selected,
                &theme,
            ));
        }

        let content =
            column![top_bar, count_label, rule::horizontal(1), scrollable(rows)].spacing(4);

        container(content).width(Fill).height(Fill).into()
    }
}
