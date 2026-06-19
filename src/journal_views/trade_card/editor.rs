use crate::app_state::TradingTerminal;
use crate::journal::AggregatedTrade;
use crate::journal_views::style::{journal_control_style, journal_text_input_style};
use crate::message::Message;
use iced::Fill;
use iced::widget::{Column, Space, button, row, text, text_input};

// ---------------------------------------------------------------------------
// Trade Card Editor
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_journal_trade_editor<'a>(
        &'a self,
        card: Column<'a, Message>,
        trade: &'a AggregatedTrade,
    ) -> Column<'a, Message> {
        let theme = self.theme();
        let default_note = crate::journal::JournalNote::default();
        let note = self
            .journal
            .edit_buffers
            .get(&trade.id)
            .unwrap_or(&default_note);

        let input_open = text_input("Entry reflection...", &note.open)
            .style(journal_text_input_style)
            .on_input({
                let id = trade.id.clone();
                move |txt| Message::JournalBufferChanged(id.clone(), true, txt)
            })
            .on_submit(Message::JournalEditSave(trade.id.clone()))
            .size(12)
            .padding(8);

        let input_close = text_input("Exit reflection...", &note.close)
            .style(journal_text_input_style)
            .on_input({
                let id = trade.id.clone();
                move |txt| Message::JournalBufferChanged(id.clone(), false, txt)
            })
            .on_submit(Message::JournalEditSave(trade.id.clone()))
            .size(12)
            .padding(8);

        let actions = row![
            Space::new().width(Fill),
            button(text("Cancel").size(11))
                .on_press(Message::JournalEditCancel(trade.id.clone()))
                .style(journal_control_style(false)),
            Space::new().width(8.0),
            button(text("Save").size(11))
                .on_press(Message::JournalEditSave(trade.id.clone()))
                .style(journal_control_style(true)),
        ];

        card.push(Space::new().height(8.0))
            .push(
                row![
                    text("O")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                    input_open,
                ]
                .align_y(iced::Alignment::Center)
                .spacing(8),
            )
            .push(
                row![
                    text("C")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                    input_close,
                ]
                .align_y(iced::Alignment::Center)
                .spacing(8),
            )
            .push(actions)
    }
}
