use crate::account;
use crate::account_state::PositionsSortColumn;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;

use super::{POSITION_ACTION_WIDTH, PositionColumnVisibility};
use iced::widget::{button, container, row, text, tooltip};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_positions_header<'a>(
        &'a self,
        can_close: bool,
        positions: &[&account::AssetPosition],
        hidden_count: usize,
        theme: &Theme,
        columns: PositionColumnVisibility,
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

        let hidden_toggle: Element<'a, Message> = if hidden_count > 0 {
            let label = if self.show_hidden_positions {
                format!("\u{25C9}{hidden_count}")
            } else {
                format!("\u{2298}{hidden_count}")
            };
            let tip = if self.show_hidden_positions {
                "Hide hidden positions"
            } else {
                "Show hidden positions"
            };
            tooltip(
                button(text(label).size(10).center())
                    .on_press(Message::ToggleShowHiddenPositions)
                    .padding([2, 5])
                    .style(|theme: &Theme, status| {
                        let bg = match status {
                            button::Status::Hovered => {
                                theme.extended_palette().background.weak.color
                            }
                            _ => Color::TRANSPARENT,
                        };
                        button::Style {
                            background: Some(bg.into()),
                            text_color: theme.extended_palette().background.weak.text,
                            border: iced::Border {
                                radius: 3.0.into(),
                                width: 1.0,
                                color: Color {
                                    a: 0.32,
                                    ..theme.extended_palette().background.weak.text
                                },
                            },
                            ..Default::default()
                        }
                    }),
                text(tip).size(10),
                tooltip::Position::Top,
            )
            .into()
        } else {
            text("").size(12).into()
        };

        let nuke_cell: Element<'a, Message> = if has_positions {
            let nuke_armed = self.nuke_confirmation.is_some();
            let nuke_label = if nuke_armed {
                "\u{2622} CONFIRM"
            } else {
                "\u{2622} NUKE"
            };
            button(text(nuke_label).size(10).center().width(Fill))
                .on_press(Message::NukePositions)
                .padding([2, 8])
                .style(move |theme: &Theme, status| nuke_button_style(theme, status, nuke_armed))
                .into()
        } else {
            text("").size(12).into()
        };
        let action_cell: Element<'a, Message> = row![hidden_toggle, nuke_cell]
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .into();

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

        let mut header_row = row![
            sort_btn("Symbol", PositionsSortColumn::Symbol),
            sort_btn("Side", PositionsSortColumn::Side),
            sort_btn("Size", PositionsSortColumn::Size),
            sort_btn("Entry", PositionsSortColumn::Entry),
        ];
        if columns.liquidation {
            header_row = header_row.push(sort_btn("Liq", PositionsSortColumn::Liquidation));
        }
        header_row = header_row
            .push(sort_btn("Mark", PositionsSortColumn::Mark))
            .push(sort_btn("Value", PositionsSortColumn::Value))
            .push(sort_btn("uPnL", PositionsSortColumn::UnrealizedPnl));
        if columns.funding {
            header_row = header_row.push(sort_btn("Funding", PositionsSortColumn::Funding));
        }
        if columns.total_pnl {
            header_row = header_row.push(sort_btn("Total PnL", PositionsSortColumn::TotalPnl));
        }
        if columns.leverage {
            header_row = header_row.push(sort_btn("Lev", PositionsSortColumn::Leverage));
        }
        header_row = header_row.push(container(action_cell).width(POSITION_ACTION_WIDTH));

        container(header_row.spacing(4).align_y(iced::Alignment::Center))
            .padding([0, 8])
            .into()
    }
}

fn nuke_button_style(theme: &Theme, status: button::Status, nuke_armed: bool) -> button::Style {
    let extended = theme.extended_palette();
    let danger = &extended.danger;

    let pair = match (nuke_armed, status) {
        (_, button::Status::Pressed) | (true, button::Status::Hovered) => &danger.strong,
        (true, _) | (false, button::Status::Hovered) => &danger.base,
        (false, _) => &danger.weak,
    };

    button::Style {
        background: Some(pair.color.into()),
        text_color: pair.text,
        border: iced::Border {
            radius: 3.0.into(),
            width: 1.0,
            color: Color {
                a: 0.55,
                ..danger.base.color
            },
        },
        ..Default::default()
    }
}
