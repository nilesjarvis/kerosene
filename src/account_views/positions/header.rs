use crate::account;
use crate::account_state::PositionsSortColumn;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;

use iced::widget::{button, container, row, text};
use iced::{Element, Fill, Theme, color};

impl TradingTerminal {
    pub(super) fn view_positions_header<'a>(
        &'a self,
        can_close: bool,
        positions: &[&account::AssetPosition],
        theme: &Theme,
    ) -> Element<'a, Message> {
        let has_positions = can_close
            && positions.iter().any(|ap| {
                ap.position
                    .szi
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|szi| szi.is_finite())
                    .is_some_and(|szi| szi.abs() > 1e-12)
                    && !self.is_outcome_coin(&ap.position.coin)
            });

        let nuke_cell: Element<'a, Message> = if has_positions {
            let nuke_armed = self.nuke_confirmation.is_some();
            let nuke_label = if nuke_armed { "CONFIRM" } else { "NUKE" };
            button(
                text(nuke_label)
                    .size(10)
                    .center()
                    .color(theme.palette().text)
                    .width(Fill),
            )
            .on_press(Message::NukePositions)
            .padding([2, 8])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.palette().danger,
                    _ => color!(0x5a2020),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
        } else {
            text("").size(12).into()
        };

        let sort_btn = |label: &'static str, col: PositionsSortColumn| {
            let mut row_content = row![
                text(label)
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
            ];
            if self.positions_sort_column == col {
                let icon = if self.positions_sort_direction == config::SortDirection::Ascending {
                    "\u{2191}"
                } else {
                    "\u{2193}"
                };
                row_content = row_content.push(
                    text(icon)
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                );
            }

            button(row_content.spacing(2))
                .on_press(Message::PositionsSortChanged(col))
                .style(|_theme: &Theme, _status| button::Style {
                    background: None,
                    ..Default::default()
                })
                .padding(0)
                .width(Fill)
        };

        container(
            row![
                sort_btn("Symbol", PositionsSortColumn::Symbol),
                sort_btn("Side", PositionsSortColumn::Side),
                sort_btn("Size", PositionsSortColumn::Size),
                sort_btn("Entry", PositionsSortColumn::Entry),
                sort_btn("Liq", PositionsSortColumn::Liquidation),
                sort_btn("Mark", PositionsSortColumn::Mark),
                sort_btn("Value", PositionsSortColumn::Value),
                sort_btn("uPnL", PositionsSortColumn::UnrealizedPnl),
                sort_btn("Funding", PositionsSortColumn::Funding),
                sort_btn("Total PnL", PositionsSortColumn::TotalPnl),
                sort_btn("Lev", PositionsSortColumn::Leverage),
                container(nuke_cell).width(120),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .padding([0, 8])
        .into()
    }
}
