use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use super::OutcomeMarketSet;
use iced::widget::container as container_style;
use iced::widget::{Column, button, column, container, row, rule, text};
use iced::{Color, Element, Fill, Theme};
use volume::{outcome_group_volume, outcome_market_set_volume, view_outcome_volume};

mod sides;
mod volume;

impl TradingTerminal {
    pub(in crate::market_views::outcomes) fn view_outcome_market_set<'a>(
        &'a self,
        theme: &Theme,
        group: OutcomeMarketSet<'a>,
        available_width: f32,
    ) -> Element<'a, Message> {
        let now_ms = self.status_bar_now_ms;
        let collapsed = self.outcome_collapsed_market_groups.contains(&group.key);
        let summary = outcome_market_set_summary(&group);
        let toggle_label = if collapsed { "+" } else { "-" };
        let toggle_button = button(text(toggle_label).size(12).center())
            .on_press(Message::OutcomeMarketGroupToggled(group.key))
            .padding([2, 0])
            .width(24.0)
            .style(outcome_collapse_button_style);

        let mut header = row![
            toggle_button,
            column![
                text(group.title)
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

        if let Some(volume) = view_outcome_volume(
            outcome_market_set_volume(&group.outcomes, &self.outcome_volumes_24h),
            self.outcome_volumes_loading,
            theme,
        ) {
            header = header.push(volume);
        }

        let mut content = column![header].spacing(8).width(Fill);
        if !collapsed {
            let nested = group.is_question_group;
            let mut outcomes = Column::new().spacing(6).width(Fill);
            let mut is_first = true;
            for sides in group.outcomes.into_values() {
                if !is_first {
                    outcomes = outcomes.push(rule::horizontal(1));
                }
                is_first = false;
                if let Some(outcome) =
                    self.view_outcome_market_group(theme, sides, now_ms, available_width, nested)
                {
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
        if let Some(volume) = view_outcome_volume(
            outcome_group_volume(&sides, &self.outcome_volumes_24h),
            self.outcome_volumes_loading,
            theme,
        ) {
            market_header = market_header.push(volume);
        }

        let probability_bar = self.view_outcome_group_probability(&sides, theme);
        let side_selector = self.view_outcome_group_sides(&sides, theme, available_width);

        let mut group_content = column![market_header].spacing(6).width(Fill);
        if let Some(reason) = info.trading_block_reason(now_ms) {
            group_content = group_content.push(text(reason).size(11).color(theme.palette().danger));
        }
        if let Some(deadline) = info.contract_deadline_label() {
            group_content = group_content.push(
                text(deadline)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }
        if let Some(source) = info.settlement_source_label() {
            group_content = group_content.push(
                text(source)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text)
                    .width(Fill),
            );
        }
        if let Some(probability_bar) = probability_bar {
            group_content = group_content.push(probability_bar);
        }
        group_content = group_content.push(side_selector);
        if let Some(rules) = &info.contract.rules {
            let expanded = self.outcome_expanded_rules.contains(&info.outcome_id);
            group_content = group_content.push(
                button(
                    text(if expanded {
                        "Hide contract rules"
                    } else {
                        "Contract rules"
                    })
                    .size(11),
                )
                .on_press(Message::OutcomeRulesToggled(info.outcome_id))
                .style(button::text)
                .padding([2, 0]),
            );
            if expanded {
                group_content = group_content.push(text(rules.as_str()).size(11).width(Fill));
                group_content =
                    group_content.push(text(info.fee_terms_label()).size(10).width(Fill));
            }
        }

        Some(container(group_content).width(Fill).padding([2, 0]).into())
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
    let summary = format!(
        "{} {} | {} {} | {}",
        group.outcome_count, outcome_label, group.trade_coin_count, coin_label, group.quote_symbol
    );
    match group
        .outcomes
        .values()
        .flatten()
        .find_map(|symbol| symbol.outcome.as_ref()?.venue_label())
    {
        Some(venue) => format!("{venue} | {summary}"),
        None => summary,
    }
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
