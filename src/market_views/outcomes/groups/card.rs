use crate::api::{ExchangeSymbol, OutcomeVolume24h};
use crate::app_state::TradingTerminal;
use crate::message::Message;

use super::OutcomeMarketSet;
use iced::widget::container as container_style;
use iced::widget::{Column, button, column, container, row, rule, text};
use iced::{Color, Element, Fill, Theme};
use std::collections::{BTreeMap, HashMap};

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(in crate::market_views::outcomes) fn view_outcome_market_set<'a>(
        &'a self,
        theme: &Theme,
        group: OutcomeMarketSet<'a>,
        available_width: f32,
    ) -> Element<'a, Message> {
        let now_ms = self.status_bar_now_ms;
        let collapsed = self.outcome_collapsed_market_groups.contains(&group.key);
        let toggle_label = if collapsed { "+" } else { "-" };
        let toggle_button = button(text(toggle_label).size(12).center())
            .on_press(Message::OutcomeMarketGroupToggled(group.key.clone()))
            .padding([2, 0])
            .width(24.0)
            .style(outcome_collapse_button_style);

        let summary = outcome_market_set_summary(&group);
        let mut header = row![
            toggle_button,
            column![
                text(group.title.clone())
                    .size(13)
                    .color(theme.palette().text)
                    .width(Fill),
                text(summary)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text)
                    .width(Fill),
            ]
            .spacing(1)
            .width(Fill),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .width(Fill);

        if let Some(volume) = outcome_market_set_volume(&group.outcomes, &self.outcome_volumes_24h)
        {
            header = header.push(
                text(format!(
                    "24h Vol {}",
                    format_outcome_contract_volume(volume)
                ))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text),
            );
        } else if self.outcome_volumes_loading {
            header = header.push(
                text("24h Vol ...")
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        let mut content = column![header].spacing(8).width(Fill);
        if !collapsed {
            let nested = group.is_question_group;
            let mut outcomes = Column::new().spacing(6).width(Fill);
            let mut is_first = true;
            for sides in group.outcomes.values() {
                if !is_first {
                    outcomes = outcomes.push(rule::horizontal(1));
                }
                is_first = false;
                if let Some(outcome) = self.view_outcome_market_group(
                    theme,
                    sides.clone(),
                    now_ms,
                    available_width,
                    nested,
                ) {
                    outcomes = outcomes.push(outcome);
                }
            }
            content = content.push(outcomes);
        }

        container(content)
            .width(Fill)
            .padding([7, 8])
            .style(outcome_market_set_style)
            .into()
    }

    pub(in crate::market_views::outcomes) fn view_outcome_market_group<'a>(
        &'a self,
        theme: &Theme,
        mut sides: Vec<&'a ExchangeSymbol>,
        now_ms: u64,
        available_width: f32,
        nested: bool,
    ) -> Option<Element<'a, Message>> {
        sides.sort_by_key(|sym| {
            sym.outcome
                .as_ref()
                .map(|info| info.side_index)
                .unwrap_or(u32::MAX)
        });

        let info = sides.iter().find_map(|sym| sym.outcome.as_ref())?;
        let market = outcome_market_title(info, nested, now_ms);

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
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text),
            );
        } else if self.outcome_volumes_loading {
            market_header = market_header.push(
                text("24h Vol ...")
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
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

fn outcome_market_set_summary(group: &OutcomeMarketSet<'_>) -> String {
    let outcome_label = if group.outcome_count == 1 {
        "outcome"
    } else {
        "outcomes"
    };
    let coin_label = if group.trade_coin_count == 1 {
        "trade coin"
    } else {
        "trade coins"
    };
    format!(
        "{} {} | {} {} | {}",
        group.outcome_count, outcome_label, group.trade_coin_count, coin_label, group.quote_symbol
    )
}

fn outcome_market_title(info: &crate::api::OutcomeSymbolInfo, nested: bool, now_ms: u64) -> String {
    if nested {
        let label = info.side_condition_short_label();
        if !label.trim().is_empty() {
            return label;
        }
    }

    info.market_label_with_countdown(now_ms)
}

fn outcome_market_set_volume(
    outcomes: &BTreeMap<u32, Vec<&ExchangeSymbol>>,
    volumes: &HashMap<String, OutcomeVolume24h>,
) -> Option<f64> {
    let mut total = 0.0;
    let mut found = false;
    for sides in outcomes.values() {
        if let Some(volume) = outcome_group_volume(sides, volumes) {
            total += volume;
            found = true;
        }
    }
    found.then_some(total)
}

fn outcome_group_volume(
    sides: &[&ExchangeSymbol],
    volumes: &HashMap<String, OutcomeVolume24h>,
) -> Option<f64> {
    sides
        .iter()
        .filter_map(|symbol| volumes.get(&symbol.key).map(|volume| volume.contract))
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

fn outcome_market_set_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            radius: 5.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}

fn outcome_collapse_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => theme.extended_palette().background.strong.color,
        _ => theme.extended_palette().background.base.color,
    };

    button::Style {
        background: Some(background.into()),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: Color {
                a: 0.50,
                ..theme.extended_palette().background.strong.text
            },
        },
        ..Default::default()
    }
}
