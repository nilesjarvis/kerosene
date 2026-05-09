use crate::helpers::format_usd;
use crate::message::Message;
use iced::widget::{Space, button, row, text};
use iced::{Element, Fill, Theme};

pub(in crate::journal_views::trade_card) fn journal_trade_card_details(
    trade_id: String,
    note_key: Option<String>,
    max_position_label: String,
    fill_count: usize,
    fee: f64,
    duration_str: String,
    theme: &Theme,
) -> Element<'static, Message> {
    row![
        text("Max Pos:")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        Space::new().width(4.0),
        text(max_position_label)
            .font(iced::Font::MONOSPACE)
            .size(11)
            .color(theme.palette().text),
        Space::new().width(16.0),
        text("Fills:")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        Space::new().width(4.0),
        text(format!("{}", fill_count))
            .font(iced::Font::MONOSPACE)
            .size(11)
            .color(theme.palette().text),
        Space::new().width(16.0),
        text("Fees:")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        Space::new().width(4.0),
        text(format_usd(&fee.to_string()))
            .font(iced::Font::MONOSPACE)
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
