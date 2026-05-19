use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{Column, column, container, row, text};
use iced::{Element, Fill, Theme};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

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
        let market = info.market_label_with_countdown(Self::now_ms());

        let mut market_header = row![
            text(market)
                .size(12)
                .color(theme.palette().text)
                .width(Fill),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .width(Fill);
        if let Some(volume) = outcome_group_volume(&sides, &self.outcome_volumes_24h) {
            market_header = market_header.push(
                text(format!(
                    "24h Vol {}",
                    format_outcome_contract_volume(volume)
                ))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(theme.extended_palette().background.weak.text),
            );
        } else if self.outcome_volumes_loading {
            market_header = market_header.push(
                text("24h Vol ...")
                    .size(11)
                    .font(iced::Font::MONOSPACE)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

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

fn outcome_group_volume(sides: &[&ExchangeSymbol], volumes: &HashMap<String, f64>) -> Option<f64> {
    sides
        .iter()
        .filter_map(|symbol| volumes.get(&symbol.key).copied())
        .filter(|volume| volume.is_finite() && *volume >= 0.0)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

fn format_outcome_contract_volume(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1_000_000_000.0 {
        format!("{:.1}B contracts", value / 1_000_000_000.0)
    } else if abs >= 1_000_000.0 {
        format!("{:.1}M contracts", value / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.1}K contracts", value / 1_000.0)
    } else if abs >= 1.0 {
        format!("{value:.0} contracts")
    } else {
        format!("{value:.2} contracts")
    }
}
