use super::super::components::journal_note_block;
use crate::journal::JournalNote;
use crate::message::Message;
use iced::Theme;
use iced::widget::Column;

pub(in crate::journal_views::trade_card) fn push_journal_trade_notes<'a>(
    card: Column<'a, Message>,
    note: &'a JournalNote,
    theme: &Theme,
) -> Column<'a, Message> {
    let mut notes_col = Column::new().spacing(4);

    if !note.open.trim().is_empty() {
        notes_col = notes_col.push(journal_note_block(
            "O",
            &note.open,
            theme.palette().primary,
            theme.extended_palette().background.weak.text,
            theme.palette().text,
        ));
    }

    if !note.close.trim().is_empty() {
        notes_col = notes_col.push(journal_note_block(
            "C",
            &note.close,
            theme.palette().danger,
            theme.extended_palette().background.weak.text,
            theme.palette().text,
        ));
    }

    card.push(notes_col)
}
