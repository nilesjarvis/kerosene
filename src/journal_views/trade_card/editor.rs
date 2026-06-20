use super::{journal_note_block, journal_tag_chips, push_opt};
use crate::app_state::TradingTerminal;
use crate::journal::{self, AggregatedTrade, JournalNote};
use crate::journal_views::style::{
    journal_accent_soft, journal_ghost_button_style, journal_muted, journal_primary_button_style,
    journal_text_input_style,
};
use crate::message::Message;
use iced::widget::{Space, button, column, row, text, text_input};
use iced::{Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Reflection editor
//
// Structured entry-thesis / exit-reflection per trade, plus `#tag` chips and a
// primary Save button. Reuses the journal edit buffers so the inspector and the
// persistence path share one flow.
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::journal_views) fn view_journal_reflection<'a>(
        &'a self,
        trade: &'a AggregatedTrade,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let is_editing = self
            .journal
            .edit_modes
            .get(&trade.id)
            .copied()
            .unwrap_or(false);

        if is_editing {
            self.view_journal_reflection_editor(trade, &theme)
        } else {
            self.view_journal_reflection_display(trade, &theme)
        }
    }

    fn view_journal_reflection_editor<'a>(
        &'a self,
        trade: &'a AggregatedTrade,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let default_note = JournalNote::default();
        let note = self
            .journal
            .edit_buffers
            .get(&trade.id)
            .unwrap_or(&default_note);
        let tag_raw = self
            .journal
            .edit_tag_raw
            .get(&trade.id)
            .cloned()
            .unwrap_or_default();

        let open_input = text_input("What was the setup and thesis?", &note.open)
            .style(journal_text_input_style)
            .on_input({
                let id = trade.id.clone();
                move |text| Message::JournalBufferChanged(id.clone(), true, text)
            })
            .size(13)
            .padding(8);

        let close_input = text_input("How did it play out? What did you learn?", &note.close)
            .style(journal_text_input_style)
            .on_input({
                let id = trade.id.clone();
                move |text| Message::JournalBufferChanged(id.clone(), false, text)
            })
            .size(13)
            .padding(8);

        let cause_of_error_input = text_input("What caused the error?", &note.cause_of_error)
            .style(journal_text_input_style)
            .on_input({
                let id = trade.id.clone();
                move |text| Message::JournalCauseOfErrorChanged(id.clone(), text)
            })
            .size(13)
            .padding(8);

        let tag_input = text_input("breakout momentum trend ...", &tag_raw)
            .style(journal_text_input_style)
            .on_input({
                let id = trade.id.clone();
                move |text| Message::JournalTagsChanged(id.clone(), text)
            })
            .on_submit(Message::JournalEditSave(trade.id.clone()))
            .size(12)
            .padding(8)
            .font(crate::app_fonts::monospace_font());

        let actions = row![
            Space::new().width(Fill),
            button(text("Cancel").size(11))
                .on_press(Message::JournalEditCancel(trade.id.clone()))
                .padding([6, 12])
                .style(journal_ghost_button_style),
            button(text("Save reflection").size(11))
                .on_press(Message::JournalEditSave(trade.id.clone()))
                .padding([6, 14])
                .style(journal_primary_button_style),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let mut content = column![
            reflection_label("ENTRY THESIS", theme),
            open_input,
            reflection_label("EXIT REFLECTION", theme),
            close_input,
            reflection_label("CAUSE OF ERROR", theme),
            cause_of_error_input,
            reflection_label("TAGS", theme),
            tag_input,
        ]
        .spacing(6);
        content = push_opt(content, journal_tag_chips(&note.tags, theme));
        content = content.push(Space::new().height(2.0));
        content = content.push(actions);

        content.into()
    }

    fn view_journal_reflection_display<'a>(
        &'a self,
        trade: &'a AggregatedTrade,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let note_key = journal::note_key_for_trade(&self.journal.entries, trade);
        let note = journal::note_for_trade(&self.journal.entries, trade);
        let has_note = note.is_some_and(|note| !note.is_empty());

        let mut content = column![].spacing(8);
        if let Some(note) = note.filter(|note| !note.is_empty()) {
            content = content.push(journal_note_block("ENTRY THESIS", &note.open, theme));
            content = content.push(journal_note_block("EXIT REFLECTION", &note.close, theme));
            content = content.push(journal_note_block(
                "CAUSE OF ERROR",
                &note.cause_of_error,
                theme,
            ));
            content = push_opt(content, journal_tag_chips(&note.tags, theme));
        } else {
            content = content.push(
                text("No reflection recorded for this trade yet.")
                    .size(12)
                    .color(journal_muted(theme)),
            );
        }

        let label = if has_note {
            "Edit reflection"
        } else {
            "Add reflection"
        };
        content = content.push(
            row![
                Space::new().width(Fill),
                button(text(label).size(11))
                    .on_press(Message::JournalEditStart(trade.id.clone(), note_key))
                    .padding([6, 12])
                    .style(journal_ghost_button_style),
            ]
            .align_y(iced::Alignment::Center),
        );

        content.into()
    }
}

fn reflection_label(label: &'static str, theme: &Theme) -> Element<'static, Message> {
    text(label)
        .size(10)
        .font(crate::app_fonts::monospace_font())
        .color(journal_accent_soft(theme))
        .into()
}
