use crate::account::{WalletDetailsData, WalletPositionDetail};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Element;
use iced::widget::{Column, row, rule, text};
use std::borrow::Cow;

use super::numbers::wallet_has_visible_nonzero;
use super::style::wallet_detail_table;

mod position_row;

// ---------------------------------------------------------------------------
// Wallet Detail Positions
// ---------------------------------------------------------------------------

impl TradingTerminal {
    /// Borrow stored rows and append owned spot rows, synthesizing them before
    /// callers iterate.
    pub(super) fn wallet_position_details_with_spot<'a>(
        &self,
        data: &'a WalletDetailsData,
    ) -> Vec<Cow<'a, WalletPositionDetail>> {
        let mut positions: Vec<_> = data.positions.iter().map(Cow::Borrowed).collect();
        positions.extend(data.spot.balances.iter().filter_map(|balance| {
            self.spot_asset_position_for_balance(balance, &data.fills)
                .map(|asset_position| {
                    Cow::Owned(WalletPositionDetail {
                        dex: String::new(),
                        asset_position,
                    })
                })
        }));
        positions
    }

    /// Outcome and spot position coins resolve to their human labels; perp and
    /// HIP-3 coins keep the dex-qualified key.
    fn wallet_position_symbol_label(&self, dex: &str, coin: &str) -> String {
        if self.is_outcome_coin(coin) || self.is_spot_coin(coin) {
            self.display_name_for_symbol(coin)
        } else {
            Self::wallet_detail_symbol(dex, coin)
        }
    }

    pub(super) fn view_wallet_positions_table<'a>(
        &'a self,
        data: &'a WalletDetailsData,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let mut position_rows = self.wallet_position_details_with_spot(data);
        position_rows.retain(|detail| {
            let pos = &detail.asset_position.position;
            wallet_has_visible_nonzero(&pos.szi)
                && self
                    .visible_wallet_detail_symbol(&detail.dex, &pos.coin)
                    .is_some()
        });
        position_rows.sort_by_cached_key(|detail| {
            Self::wallet_detail_symbol(&detail.dex, &detail.asset_position.position.coin)
        });

        let positions_header = row![
            text("Coin").size(10).width(95),
            text("Dex").size(10).width(60),
            text("Side").size(10).width(44),
            text("Size").size(10).width(84),
            text("Entry").size(10).width(78),
            text("Mark").size(10).width(78),
            text("Liq").size(10).width(78),
            text("Value").size(10).width(84),
            text("uPnL").size(10).width(84),
            text("Funding").size(10).width(84),
            text("Lev").size(10).width(44),
        ]
        .spacing(8);
        let mut positions_table = Column::new()
            .spacing(4)
            .push(text("Positions").size(13).color(theme.palette().text))
            .push(positions_header)
            .push(rule::horizontal(1));

        if position_rows.is_empty() {
            positions_table = positions_table.push(
                text("No open positions")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            );
        } else {
            for detail in &position_rows {
                let symbol_label = self.wallet_position_symbol_label(
                    &detail.dex,
                    &detail.asset_position.position.coin,
                );
                positions_table =
                    positions_table.push(self.view_wallet_position_row(detail, symbol_label));
            }
        }

        wallet_detail_table(positions_table)
    }
}
