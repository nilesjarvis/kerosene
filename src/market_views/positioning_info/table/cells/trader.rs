use super::super::super::columns::{
    POSITIONING_TRADER_COMPACT_ACTIONS_WIDTH, POSITIONING_TRADER_FULL_ACTIONS_MIN_WIDTH,
    POSITIONING_TRADER_FULL_ACTIONS_WIDTH,
};
use super::super::super::metrics::{position_identity, trader_text_limit, truncate_ascii};
use crate::message::Message;
use crate::wallet_state::address_book::WalletDisplay;

use iced::widget::{button, container, row, text, tooltip};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// Trader Identity Cell
// ---------------------------------------------------------------------------

pub(in crate::market_views::positioning_info::table) fn positioning_trader_cell(
    address: &str,
    wallet_display: WalletDisplay,
    width: f32,
    compact_actions_min_width: f32,
    theme: &Theme,
) -> Element<'static, Message> {
    let identity_label = position_identity(wallet_display);
    let address = address.to_string();
    let (show_actions, show_full_actions) =
        positioning_trader_action_visibility(width, compact_actions_min_width);
    let action_width = if show_actions {
        if show_full_actions {
            POSITIONING_TRADER_FULL_ACTIONS_WIDTH
        } else {
            POSITIONING_TRADER_COMPACT_ACTIONS_WIDTH
        }
    } else {
        0.0
    };
    let identity_width = (width - action_width).max(0.0);
    let label_limit = trader_text_limit(identity_width, 34);

    let identity_content = text(truncate_ascii(&identity_label, label_limit))
        .size(11)
        .color(theme.palette().text)
        .width(Fill);

    let identity_button = button(identity_content)
        .on_press(Message::CopyToClipboard(address.clone()))
        .padding(0)
        .style(|theme: &Theme, status| {
            let background = match status {
                button::Status::Hovered => Some(
                    Color {
                        a: 0.18,
                        ..theme.extended_palette().background.weak.color
                    }
                    .into(),
                ),
                _ => None,
            };
            button::Style {
                background,
                ..Default::default()
            }
        })
        .width(Fill);
    let identity: Element<'static, Message> = tooltip(
        identity_button,
        text(format!("Copy {address}"))
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Top,
    )
    .into();

    let mut content = row![identity]
        .spacing(3)
        .align_y(Alignment::Center)
        .width(Fill);
    if show_actions {
        content = content
            .push(positioning_trader_action_button(
                if show_full_actions {
                    "Details"
                } else {
                    "\u{2197}"
                },
                "Open detachable wallet details",
                Message::OpenWalletDetailsWindow(address.clone()),
                show_full_actions,
            ))
            .push(positioning_trader_action_button(
                if show_full_actions { "Ghost" } else { "G" },
                "Open in ghost mode",
                Message::GhostWallet(address),
                show_full_actions,
            ));
    }

    container(content)
        .width(Length::Fixed(width))
        .padding([1, 0])
        .into()
}

pub(in crate::market_views::positioning_info) fn positioning_trader_action_visibility(
    width: f32,
    compact_actions_min_width: f32,
) -> (bool, bool) {
    (
        width >= compact_actions_min_width,
        width >= POSITIONING_TRADER_FULL_ACTIONS_MIN_WIDTH,
    )
}

fn positioning_trader_action_button(
    label: &'static str,
    tooltip_label: &'static str,
    msg: Message,
    full: bool,
) -> Element<'static, Message> {
    let button_width = if full { 50.0 } else { 18.0 };
    tooltip(
        button(
            text(label)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .center(),
        )
        .on_press(msg)
        .padding([0, 4])
        .width(Length::Fixed(button_width))
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().primary,
                border: iced::Border {
                    radius: 3.0.into(),
                    width: 1.0,
                    color: Color {
                        a: 0.45,
                        ..theme.palette().primary
                    },
                },
                ..Default::default()
            }
        }),
        text(tooltip_label).size(10),
        tooltip::Position::Top,
    )
    .into()
}
