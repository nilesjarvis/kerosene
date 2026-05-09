use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartInstance;

use iced::widget::container as container_style;
use iced::widget::{button, container, text};
use iced::{Color, Element, Fill, Theme, color};

impl TradingTerminal {
    pub(in crate::spaghetti_views::pair) fn can_execute_spaghetti_pair_trade(
        &self,
        inst: &SpaghettiChartInstance,
    ) -> bool {
        inst.canvas.series.len() >= 2
            && !inst.pair_pending
            && self.connected_address.is_some()
            && !self.wallet_key_input.trim().is_empty()
            && inst
                .canvas
                .series
                .first()
                .is_some_and(|s| self.market_type_for_symbol(&s.symbol) == Some(MarketType::Perp))
            && inst
                .canvas
                .series
                .get(1)
                .is_some_and(|s| self.market_type_for_symbol(&s.symbol) == Some(MarketType::Perp))
            && inst
                .canvas
                .series
                .first()
                .and_then(|s| self.resolve_mid_for_symbol(&s.symbol))
                .is_some_and(|m| m.is_finite() && m > 0.0)
            && inst
                .canvas
                .series
                .get(1)
                .and_then(|s| self.resolve_mid_for_symbol(&s.symbol))
                .is_some_and(|m| m.is_finite() && m > 0.0)
    }

    pub(in crate::spaghetti_views::pair) fn view_pair_execute_button(
        &self,
        label: &'static str,
        message: Message,
        can_trade: bool,
        pair_pending: bool,
        long_a_short_b: bool,
    ) -> Element<'_, Message> {
        if can_trade {
            return button(text(label).size(11).center().width(Fill))
                .on_press(message)
                .padding([4, 8])
                .style(move |_theme: &Theme, status| {
                    let bg = match (long_a_short_b, status) {
                        (true, button::Status::Hovered) => color!(0x2f5f3f),
                        (true, _) => color!(0x265236),
                        (false, button::Status::Hovered) => color!(0x5f3232),
                        (false, _) => color!(0x522a2a),
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: Color::WHITE,
                        border: iced::Border {
                            radius: 3.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .into();
        }

        let idle_bg = if long_a_short_b {
            color!(0x223229)
        } else {
            color!(0x322222)
        };

        container(if pair_pending {
            self.view_spinner(16)
        } else {
            text(label).size(11).into()
        })
        .padding([4, 8])
        .width(Fill)
        .center_x(Fill)
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(idle_bg.into()),
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    }
}
