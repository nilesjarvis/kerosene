#[path = "funding/row.rs"]
mod funding_row;
mod summary;

use crate::account::{self, AccountDataSection};
use crate::account_views::history_tables::numbers::parse_history_number;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, column, container, row, rule, scrollable, text};
use iced::{Element, Fill};
use std::cmp::Reverse;

// ---------------------------------------------------------------------------
// Funding History Table
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_funding_history(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let header_txt = |s: &'static str| {
            text(s)
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Fill)
        };
        let header = row![
            header_txt("Time"),
            header_txt("Symbol"),
            header_txt("Rate"),
            header_txt("Position"),
            header_txt("Amount"),
        ]
        .spacing(4);

        let entries: Vec<_> = self
            .account_data
            .as_ref()
            .map(|d| {
                d.funding_history
                    .iter()
                    .filter(|entry| !self.is_ticker_muted(&entry.delta.coin))
                    .collect()
            })
            .unwrap_or_default();
        let warning = self.account_data.as_ref().and_then(|data| {
            data.completeness
                .section_warning(AccountDataSection::Funding)
        });

        if entries.is_empty() {
            let msg = if let Some(warning) = warning {
                warning
            } else if self.connected_address.is_some() {
                "No funding payments in the last 7 days".to_string()
            } else {
                "Connect wallet to view funding history".to_string()
            };
            let content = column![
                header,
                rule::horizontal(1),
                container(
                    text(msg)
                        .size(12)
                        .color(theme.extended_palette().background.weak.text)
                )
                .padding([8, 0]),
            ]
            .spacing(4);
            return scrollable(content)
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .margin(0)
                        .scroller_width(4),
                ))
                .width(Fill)
                .height(Fill)
                .into();
        }

        let mut sorted: Vec<&account::FundingEntry> = entries;
        sorted.sort_by_key(|entry| Reverse(entry.time));

        let total_funding = funding_total(sorted.iter().map(|entry| entry.delta.usdc.as_str()));
        let total_label = self.view_funding_total_label(total_funding, &theme);

        let rows = sorted
            .iter()
            .take(200)
            .fold(Column::new().spacing(2), |col, entry| {
                col.push(self.view_funding_history_row(entry, &theme))
            });

        let mut content = column![header, total_label].spacing(4);
        if let Some(warning) = warning {
            content = content.push(text(warning).size(11).color(theme.palette().warning));
        }
        let content = content.push(rule::horizontal(1)).push(rows);
        scrollable(content)
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .margin(0)
                    .scroller_width(4),
            ))
            .width(Fill)
            .height(Fill)
            .into()
    }
}

fn funding_total<'a>(amounts: impl IntoIterator<Item = &'a str>) -> Option<f64> {
    let mut total = 0.0;
    for amount in amounts {
        total += parse_history_number(amount)?;
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn funding_total_marks_any_invalid_amount_unknown() {
        assert_eq!(funding_total(["1", "-0.25"]), Some(0.75));
        assert_eq!(funding_total(["1", "bad"]), None);
        assert_eq!(funding_total(["1", "NaN"]), None);
    }
}
