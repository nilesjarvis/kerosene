use super::*;
use crate::pnl_card::{PnlCardDisplayMode, PnlCardPercentMode, PnlCardTarget};
use crate::portfolio_state::PnlValueDisplayMode;

mod account;
mod annotations;
mod chrome_layout;
mod feature_groups;
mod markets;

fn assert_route(message: Message, expected: UpdateRoute) {
    assert_eq!(message_route(&message), expected);
}

fn window_id() -> iced::window::Id {
    iced::window::Id::unique()
}
