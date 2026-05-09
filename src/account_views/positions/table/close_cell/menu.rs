use crate::message::Message;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Theme, color};

pub(super) fn view_position_close_menu(
    coin_for_close: String,
    theme: &Theme,
) -> Element<'static, Message> {
    let market_row = position_close_pct_row("Mkt", coin_for_close.clone(), true, theme);
    let limit_row = position_close_pct_row("Lmt", coin_for_close.clone(), false, theme);

    let menu = column![
        row![
            text("Close").size(9).color(theme.palette().text),
            position_close_dismiss_button(coin_for_close)
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center),
        market_row,
        limit_row,
    ]
    .spacing(2);

    container(menu).width(120).into()
}

fn position_close_pct_row(
    label: &'static str,
    coin_for_close: String,
    market: bool,
    theme: &Theme,
) -> Element<'static, Message> {
    row![
        text(label)
            .size(9)
            .color(theme.extended_palette().background.weak.text),
        position_close_pct_button("25%", coin_for_close.clone(), 0.25, market),
        position_close_pct_button("50%", coin_for_close.clone(), 0.50, market),
        position_close_pct_button("100%", coin_for_close, 1.0, market),
    ]
    .spacing(2)
    .align_y(iced::Alignment::Center)
    .into()
}

fn position_close_pct_button(
    label: &'static str,
    coin: String,
    fraction: f64,
    market: bool,
) -> Element<'static, Message> {
    button(text(label).size(9).center())
        .on_press(Message::ClosePosition {
            coin,
            fraction,
            use_market: market,
        })
        .padding([1, 3])
        .style(move |theme: &Theme, status| {
            let bg = if market {
                match status {
                    button::Status::Hovered => theme.palette().danger,
                    _ => color!(0x5a2020),
                }
            } else {
                match status {
                    button::Status::Hovered => color!(0x4a4a7a),
                    _ => theme.extended_palette().background.strong.color,
                }
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn position_close_dismiss_button(coin_for_close: String) -> Element<'static, Message> {
    button(text("X").size(9).center())
        .on_press(Message::ToggleCloseMenu(coin_for_close))
        .padding([1, 3])
        .style(|_theme: &Theme, _status| button::Style {
            background: Some(color!(0x3a3a3a).into()),
            text_color: color!(0xaaaaaa),
            border: iced::Border {
                radius: 2.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
