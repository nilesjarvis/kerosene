mod symbol_row;
mod top_bar;

use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, button, column, container, responsive, rule, scrollable, stack, text};
use iced::{Alignment, Color, Element, Fill, Theme};

impl TradingTerminal {
    /// Render the inline symbol editor overlay for a chart pane.
    pub(crate) fn view_chart_editor<'a>(
        &'a self,
        chart_id: ChartId,
        instance: &'a ChartInstance,
    ) -> Element<'a, Message> {
        responsive(move |size| {
            self.view_chart_editor_sized(chart_id, instance, size.width, size.height)
        })
        .into()
    }

    fn view_chart_editor_sized<'a>(
        &'a self,
        chart_id: ChartId,
        instance: &'a ChartInstance,
        available_width: f32,
        available_height: f32,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let top_bar = self.view_chart_editor_top_bar(chart_id, instance);
        let menu_width = (available_width - 24.0).clamp(1.0, 360.0);
        let menu_height = (available_height - 24.0).clamp(1.0, 320.0);

        let content = if self.exchange_symbols.is_empty() {
            column![
                top_bar,
                container(
                    text("Loading symbols...")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text)
                )
                .padding([8, 0]),
            ]
            .spacing(4)
        } else {
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
                let is_keyboard_selected = instance.editor_selected_index == Some(i);
                rows = rows.push(self.view_chart_editor_symbol_row(
                    id,
                    sym,
                    is_fav,
                    is_selected,
                    is_keyboard_selected,
                    &theme,
                ));
            }

            column![
                top_bar,
                count_label,
                rule::horizontal(1),
                scrollable(rows)
                    .id(Self::chart_symbol_search_results_scroll_id(chart_id))
                    .height(Fill)
            ]
            .spacing(4)
        };

        let menu_card = container(content.padding(6).width(Fill))
            .width(menu_width)
            .max_height(menu_height)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                text_color: Some(theme.palette().text),
                border: iced::Border {
                    color: theme.extended_palette().background.weak.color,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        let bg_overlay = button("")
            .width(Fill)
            .height(Fill)
            .on_press(Message::ChartCloseEditor(chart_id))
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Color::TRANSPARENT.into()),
                ..Default::default()
            });

        stack![
            bg_overlay,
            container(menu_card)
                .width(Fill)
                .height(Fill)
                .padding([12, 12])
                .align_x(Alignment::Start)
                .align_y(Alignment::Start)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }
}
