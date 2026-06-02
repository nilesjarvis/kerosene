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
    duration_str: String,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    row![
        text("Max Pos:")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        Space::new().width(4.0),
        text(max_position_label)
            .font(crate::app_fonts::monospace_font())
            .size(11)
            .color(theme.palette().text),
        Space::new().width(16.0),
        text("Fills:")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        Space::new().width(4.0),
        text(format!("{}", fill_count))
            .font(crate::app_fonts::monospace_font())
            .size(11)
            .color(theme.palette().text),
        Space::new().width(16.0),
        text("Fees:")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        Space::new().width(4.0),
        text(denomination.format_value(fee, 2))
            .font(crate::app_fonts::monospace_font())
            .size(11)
            .color(theme.palette().text),
        Space::new().width(16.0),
        text("Duration:")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        Space::new().width(4.0),
        text(duration_str).size(11).color(theme.palette().text),
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
