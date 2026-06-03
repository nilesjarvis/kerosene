#[path = "assets/rows.rs"]
mod rows;

use super::stats::JournalAssetStats;
use crate::app_state::TradingTerminal;
use crate::journal_views::style::{JOURNAL_PANEL_PADDING, journal_panel_style};
use crate::message::Message;
use iced::widget::{Column, Space, button, container, row, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(super) fn view_journal_top_assets_box<'a>(
        &'a self,
        sorted_assets: JournalAssetStats,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let top_assets = if self.journal.show_all_assets {
            sorted_assets.into_iter().collect::<Vec<_>>()
        } else {
            sorted_assets.into_iter().take(3).collect::<Vec<_>>()
        };

        let mut top_assets_col = Column::new().spacing(4).push(
            row![
                text(if self.journal.show_all_assets {
                    "All Assets"
                } else {
                    "Most Traded"
                })
                .size(14)
                .color(theme.palette().text),
                Space::new().width(Fill),
                button(
                    text(if self.journal.show_all_assets {
                        "Collapse"
                    } else {
                        "See All"
                    })
                    .size(10)
                    .color(theme.palette().primary)
                )
                .padding([0, 4])
                .on_press(Message::JournalToggleAllAssets)
                .style(button::text)
            ]
            .align_y(iced::Alignment::Center),
        );

        if top_assets.is_empty() {
            top_assets_col = top_assets_col.push(
                text("None")
                    .size(14)
                    .color(theme.extended_palette().background.weak.text),
            );
        } else {
            let mut asset_list = Column::new().spacing(6);

            if self.journal.show_all_assets {
                asset_list = asset_list.push(rows::journal_asset_table_header(&theme));
                asset_list = asset_list.push(iced::widget::rule::horizontal(1));
            }

            for (coin, (count, pnl, fees)) in top_assets {
                if self.journal.show_all_assets {
                    asset_list = asset_list.push(self.view_journal_asset_table_row(
                        coin.as_str(),
                        count,
                        pnl,
                        fees,
                        &theme,
                    ));
                } else {
                    asset_list = asset_list.push(self.view_journal_asset_compact_row(
                        coin.as_str(),
                        count,
                        pnl,
                        &theme,
                    ));
                }
            }

            if self.journal.show_all_assets {
                top_assets_col = top_assets_col.push(scrollable(asset_list).height(150.0));
            } else {
                top_assets_col = top_assets_col.push(asset_list);
            }
        }

        container(top_assets_col)
            .padding(JOURNAL_PANEL_PADDING)
            .width(Fill)
            .style(journal_panel_style)
            .into()
    }
}
