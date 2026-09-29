use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{column, row};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_outcome_group_probability<'a>(
        &'a self,
        sides: &[&ExchangeSymbol],
        theme: &Theme,
    ) -> Option<Element<'a, Message>> {
        if sides.len() != 2 {
            return None;
        }

        let first = sides[0];
        let second = sides[1];
        let (Some(first_info), Some(second_info)) =
            (first.outcome.as_ref(), second.outcome.as_ref())
        else {
            return None;
        };

        if first_info.contract.scalar || second_info.contract.scalar {
            return None;
        }
        let first_mid = self.resolve_mid_for_symbol_at(&first.key, self.status_bar_now_ms);
        let second_mid = self.resolve_mid_for_symbol_at(&second.key, self.status_bar_now_ms);
        let first_color =
            Self::outcome_side_accent(theme, &first_info.side_name, first_info.side_index);
        let second_color =
            Self::outcome_side_accent(theme, &second_info.side_name, second_info.side_index);
        Some(Self::view_outcome_probability_bar(
            first_mid,
            second_mid,
            first_color,
            second_color,
        ))
    }

    pub(super) fn view_outcome_group_sides<'a>(
        &'a self,
        sides: &[&'a ExchangeSymbol],
        theme: &Theme,
        available_width: f32,
    ) -> Element<'a, Message> {
        let cards = sides.iter().filter_map(|&sym| {
            let side_info = sym.outcome.as_ref()?;
            let mid = self.resolve_mid_for_symbol_at(&sym.key, self.status_bar_now_ms);
            let accent =
                Self::outcome_side_accent(theme, &side_info.side_name, side_info.side_index);
            Some(self.view_outcome_side_button(
                theme,
                sym,
                accent,
                sym.key == self.active_symbol,
                mid,
            ))
        });
        if sides.len() == 2 && available_width >= 380.0 {
            row(cards).spacing(6).width(Fill).into()
        } else {
            column(cards).spacing(4).width(Fill).into()
        }
    }
}
