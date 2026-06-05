use super::super::super::metrics::{position_identity, trader_text_limit, truncate_ascii};
use crate::message::Message;
use crate::wallet_state::address_book::WalletDisplay;
use crate::wallet_views::{WalletAddressActionCell, wallet_address_action_cell};

use iced::widget::container;
use iced::{Element, Length, Theme};

// ---------------------------------------------------------------------------
// Trader Identity Cell
// ---------------------------------------------------------------------------

const POSITIONING_TRADER_ACTIONS_MIN_WIDTH: f32 = 120.0;

pub(in crate::market_views::positioning_info::table) fn positioning_trader_cell(
    address: &str,
    wallet_display: WalletDisplay,
    width: f32,
    hover_key: String,
    hovered_key: Option<&str>,
    theme: &Theme,
) -> Element<'static, Message> {
    let identity_label = position_identity(wallet_display);
    let address = address.to_string();
    let label_limit = trader_text_limit(width, 34);
    let tooltip_label = format!("Copy {address}");
    let content = wallet_address_action_cell(WalletAddressActionCell {
        address,
        label: truncate_ascii(&identity_label, label_limit),
        tooltip_label,
        hover_key,
        hovered_key: positioning_trader_actions_enabled(width)
            .then_some(hovered_key)
            .flatten(),
        width,
        text_size: 11,
        text_color: theme.palette().text,
    });

    container(content)
        .width(Length::Fixed(width))
        .padding([1, 0])
        .into()
}

pub(in crate::market_views::positioning_info) fn positioning_trader_actions_enabled(
    width: f32,
) -> bool {
    width >= POSITIONING_TRADER_ACTIONS_MIN_WIDTH
}
