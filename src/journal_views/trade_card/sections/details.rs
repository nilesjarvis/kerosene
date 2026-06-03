use crate::denomination::DisplayDenominationContext;
use crate::message::Message;
use iced::widget::{Space, button, row, text};
use iced::{Element, Fill, Theme};

#[allow(clippy::too_many_arguments)]
pub(in crate::journal_views::trade_card) fn journal_trade_card_details(
    trade_id: String,
    note_key: Option<String>,
    snapshot_expanded: bool,
    max_position_label: String,
    fill_count: usize,
    fee: f64,
    opened_time_str: String,
    duration_str: String,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let muted = theme.extended_palette().background.weak.text;
    let dot = || text(" \u{2022} ").size(11).color(muted);

    row![
        text(max_position_label)
            .font(crate::app_fonts::monospace_font())
            .size(11)
            .color(theme.palette().text),
        Space::new().width(6.0),
        dot(),
        Space::new().width(6.0),
        text(opened_time_str).size(11).color(muted),
        Space::new().width(6.0),
        dot(),
        Space::new().width(6.0),
        text(format!("{} duration", duration_str))
            .size(11)
            .color(muted),
        Space::new().width(6.0),
        dot(),
        Space::new().width(6.0),
        text(format!("{} fills", fill_count)).size(11).color(muted),
        Space::new().width(6.0),
        dot(),
        Space::new().width(6.0),
        text(format!("{} fees", denomination.format_value(fee, 2)))
            .size(11)
            .color(muted),
        Space::new().width(Fill),
        button(
            text(if snapshot_expanded {
                "Hide Chart"
            } else {
                "Chart"
            })
            .size(11)
        )
        .on_press(Message::JournalSnapshotToggle(trade_id.clone()))
        .padding([4, 8])
        .style(button::text),
        button(
            text(if note_key.is_some() {
                "\u{270e} Note"
            } else {
                "+ Note"
            })
            .size(11)
        )
        .on_press(Message::JournalEditStart(trade_id, note_key))
        .padding([4, 8])
        .style(button::text),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}
