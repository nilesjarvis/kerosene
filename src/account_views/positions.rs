mod header;
mod table;

use crate::account::{self, AccountDataSection};
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{column, container, responsive, rule, scrollable, text};
use iced::{Element, Fill};

pub(super) const POSITION_ACTION_WIDTH: f32 = 152.0;

const HIDE_TOTAL_PNL_BELOW: f32 = 1_080.0;
const HIDE_LEVERAGE_BELOW: f32 = 990.0;
const HIDE_FUNDING_BELOW: f32 = 900.0;
const HIDE_LIQUIDATION_BELOW: f32 = 810.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PositionColumnVisibility {
    pub(super) liquidation: bool,
    pub(super) funding: bool,
    pub(super) total_pnl: bool,
    pub(super) leverage: bool,
}

impl PositionColumnVisibility {
    fn for_width(width: f32) -> Self {
        Self {
            liquidation: width >= HIDE_LIQUIDATION_BELOW,
            funding: width >= HIDE_FUNDING_BELOW,
            total_pnl: width >= HIDE_TOTAL_PNL_BELOW,
            leverage: width >= HIDE_LEVERAGE_BELOW,
        }
    }
}

impl TradingTerminal {
    pub(crate) fn view_positions(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_positions_for_width(size.width))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn view_positions_for_width(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let columns = PositionColumnVisibility::for_width(available_width);
        let can_close =
            self.connected_address.is_some() && !self.wallet_key_input.trim().is_empty();

        let all_positions: Vec<&account::AssetPosition> = self
            .account_data
            .as_ref()
            .map(|d| {
                d.clearinghouse
                    .asset_positions
                    .iter()
                    .filter(|ap| !self.symbol_key_is_hidden(&ap.position.coin))
                    .collect()
            })
            .unwrap_or_default();
        let hidden_count = all_positions
            .iter()
            .filter(|ap| self.position_is_hidden(&ap.position.coin))
            .count();
        let positions: Vec<&account::AssetPosition> = all_positions
            .into_iter()
            .filter(|ap| self.show_hidden_positions || !self.position_is_hidden(&ap.position.coin))
            .collect();
        let warning = self.account_data.as_ref().and_then(|data| {
            data.completeness
                .section_warning(AccountDataSection::Positions)
        });

        let header =
            self.view_positions_header(can_close, &positions, hidden_count, &theme, columns);

        if positions.is_empty() {
            let msg = if let Some(warning) = warning {
                warning
            } else if hidden_count > 0 {
                "All open positions are hidden".to_string()
            } else if self.connected_address.is_some() {
                "No open positions".to_string()
            } else {
                "Connect wallet to view positions".to_string()
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
            return positions_scrollable(content);
        }

        let rows = self.view_position_rows(&positions, can_close, &theme, columns);
        let mut content = column![header].spacing(4);
        if let Some(warning) = warning {
            content = content.push(text(warning).size(11).color(theme.palette().warning));
        }
        let content = content.push(rule::horizontal(1)).push(rows);
        column![
            positions_scrollable(content),
            self.view_position_summary_bar(&positions, &theme),
        ]
        .spacing(0)
        .width(Fill)
        .height(Fill)
        .into()
    }

    pub(crate) fn position_is_hidden(&self, coin: &str) -> bool {
        self.accounts
            .get(self.active_account_index)
            .and_then(|profile| self.hidden_positions_by_account.get(&profile.secret_id))
            .is_some_and(|hidden| hidden.contains(coin))
    }
}

fn positions_scrollable<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_columns_hide_one_group_at_a_time_as_width_shrinks() {
        assert_eq!(
            PositionColumnVisibility::for_width(HIDE_TOTAL_PNL_BELOW),
            PositionColumnVisibility {
                liquidation: true,
                funding: true,
                total_pnl: true,
                leverage: true,
            }
        );
        assert_eq!(
            PositionColumnVisibility::for_width(HIDE_TOTAL_PNL_BELOW - 1.0),
            PositionColumnVisibility {
                liquidation: true,
                funding: true,
                total_pnl: false,
                leverage: true,
            }
        );
        assert_eq!(
            PositionColumnVisibility::for_width(HIDE_LEVERAGE_BELOW - 1.0),
            PositionColumnVisibility {
                liquidation: true,
                funding: true,
                total_pnl: false,
                leverage: false,
            }
        );
        assert_eq!(
            PositionColumnVisibility::for_width(HIDE_FUNDING_BELOW - 1.0),
            PositionColumnVisibility {
                liquidation: true,
                funding: false,
                total_pnl: false,
                leverage: false,
            }
        );
        assert_eq!(
            PositionColumnVisibility::for_width(HIDE_LIQUIDATION_BELOW - 1.0),
            PositionColumnVisibility {
                liquidation: false,
                funding: false,
                total_pnl: false,
                leverage: false,
            }
        );
    }
}
