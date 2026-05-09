use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;

use iced::widget::{Column, Space, button, column, container, row, text, text_input};
use iced::{Element, Fill, Theme, color};

impl TradingTerminal {
    pub(super) fn view_assistant_bottom_block<'a>(&'a self, theme: &Theme) -> Element<'a, Message> {
        let input = text_input("Ask the assistant...", &self.assistant.input)
            .style(helpers::text_input_style)
            .on_input(Message::AssistantInputChanged)
            .on_submit(Message::AssistantSend)
            .padding([6, 8])
            .size(11);

        let send_btn = button(text("Send").size(11))
            .on_press_maybe((!self.assistant.loading).then_some(Message::AssistantSend))
            .padding([6, 10]);

        let status_row: Element<'_, Message> = if self.assistant.loading {
            let status = self
                .assistant
                .status_line
                .clone()
                .unwrap_or_else(|| "Working...".to_string());
            row![
                self.view_spinner(16),
                text(status).size(10).color(color!(0x8a93b3)),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .into()
        } else {
            container(Space::new()).height(0).into()
        };

        let error_row: Element<'_, Message> = if let Some(err) = &self.assistant.last_error {
            text(err).size(10).color(theme.palette().danger).into()
        } else {
            container(Space::new()).height(0).into()
        };

        column![
            self.view_assistant_suggestions(),
            status_row,
            error_row,
            row![input.width(Fill), send_btn]
                .spacing(6)
                .align_y(iced::Alignment::Center),
        ]
        .spacing(6)
        .into()
    }

    fn view_assistant_suggestions(&self) -> Column<'_, Message> {
        let mut suggestion_rows = Column::new().spacing(2);
        if let Some((_, query)) = Self::assistant_symbol_query(&self.assistant.input) {
            let mut filtered: Vec<&ExchangeSymbol> = self
                .exchange_symbols
                .iter()
                .filter(|sym| {
                    query.is_empty()
                        || sym.key.to_lowercase().contains(&query)
                        || sym.ticker.to_lowercase().contains(&query)
                        || sym.category.to_lowercase().contains(&query)
                        || sym
                            .display_name
                            .as_ref()
                            .is_some_and(|d| d.to_lowercase().contains(&query))
                })
                .collect();
            filtered.sort_by(|a, b| a.key.cmp(&b.key));

            for sym in filtered.into_iter().take(8) {
                let key = sym.key.clone();
                let display = sym
                    .display_name
                    .clone()
                    .unwrap_or_else(|| sym.ticker.clone());
                suggestion_rows = suggestion_rows.push(
                    button(
                        row![
                            text(format!("${{{}}}", key)).size(10).width(140),
                            text(display).size(10).color(color!(0x8893b3)),
                        ]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                    )
                    .on_press(Message::AssistantInsertTicker(key))
                    .padding([3, 6])
                    .width(Fill),
                );
            }
        }

        suggestion_rows
    }
}
