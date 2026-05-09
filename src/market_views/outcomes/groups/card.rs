use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{Column, column, container, row, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::market_views::outcomes) fn view_outcome_market_group<'a>(
        &'a self,
        theme: &Theme,
        mut sides: Vec<&'a ExchangeSymbol>,
        available_width: f32,
    ) -> Option<Element<'a, Message>> {
        sides.sort_by_key(|sym| {
            sym.outcome
                .as_ref()
                .map(|info| info.side_index)
                .unwrap_or(u32::MAX)
        });

        let info = sides.iter().find_map(|sym| sym.outcome.as_ref())?;
        let market = Self::outcome_market_label(info);

        let market_header = row![
            text(market)
                .size(12)
                .color(theme.palette().text)
                .width(Fill),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .width(Fill);

        let probability_bar = self.view_outcome_group_probability(&sides, theme);
        let side_selector = self.view_outcome_group_sides(&sides, theme, available_width);

        let mut group_content = column![market_header].spacing(6).width(Fill);
        if let Some(probability_bar) = probability_bar {
            group_content = group_content.push(probability_bar);
        }
        group_content = group_content.push(side_selector);

        Some(container(group_content).width(Fill).padding([2, 0]).into())
    }

    fn view_outcome_group_probability<'a>(
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

        let first_mid = self.resolve_mid_for_symbol(&first.key);
        let second_mid = self.resolve_mid_for_symbol(&second.key);
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

    fn view_outcome_group_sides<'a>(
        &'a self,
        sides: &[&'a ExchangeSymbol],
        theme: &Theme,
        available_width: f32,
    ) -> Element<'a, Message> {
        if sides.len() == 2 && available_width >= 380.0 {
            let mut cards = row![].spacing(6).width(Fill);
            for &sym in sides {
                let Some(side_info) = &sym.outcome else {
                    continue;
                };
                let mid = self.resolve_mid_for_symbol(&sym.key);
                let accent =
                    Self::outcome_side_accent(theme, &side_info.side_name, side_info.side_index);
                cards = cards.push(self.view_outcome_side_button(
                    theme,
                    sym,
                    accent,
                    sym.key == self.active_symbol,
                    mid,
                ));
            }
            cards.into()
        } else {
            let mut cards = Column::new().spacing(4).width(Fill);
            for &sym in sides {
                let Some(side_info) = &sym.outcome else {
                    continue;
                };
                let mid = self.resolve_mid_for_symbol(&sym.key);
                let accent =
                    Self::outcome_side_accent(theme, &side_info.side_name, side_info.side_index);
                cards = cards.push(self.view_outcome_side_button(
                    theme,
                    sym,
                    accent,
                    sym.key == self.active_symbol,
                    mid,
                ));
            }
            cards.into()
        }
    }
}
