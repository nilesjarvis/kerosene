use crate::market_state::{OrderBookId, book_inspector::Selection};
use crate::message::Message;
use iced::widget::{button, container, hover, text, tooltip};
use iced::{Element, Fill};

/// The hover layer captures its own click, preserving price selection on the
/// base row and avoiding accidental order-ticket edits when opening details.
pub(crate) fn inspectable_book_row(
    content: Element<'static, Message>,
    id: OrderBookId,
    price: f64,
    tick: f64,
    is_bid: Option<bool>,
) -> Element<'static, Message> {
    let action = tooltip(
        button(text("⋯").size(12))
            .padding([0, 5])
            .height(super::BOOK_ROW_HEIGHT)
            .style(button::secondary)
            .on_press(Message::InspectBookBlock {
                id,
                selection: Selection {
                    price,
                    tick,
                    is_bid,
                    page: 0,
                    revision: 0,
                },
            }),
        text("Inspect orders and wallets").size(11),
        tooltip::Position::Left,
    );
    hover(
        content,
        container(action)
            .width(Fill)
            .height(Fill)
            .align_right(Fill)
            .center_y(Fill),
    )
}
