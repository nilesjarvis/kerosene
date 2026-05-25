use crate::account::WalletTrackerSnapshot;
use crate::app_state::TradingTerminal;
use crate::helpers::sum_optional_f64;
use crate::message::Message;
use crate::wallet_views::numbers::invalid_wallet_data;
use iced::widget::{Space, button, column, row, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::wallet_views::tracker) fn view_wallet_tracker_header(
        &self,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let wallet_count = self.wallet_tracker.tracked_addresses.len();
        let snapshots: Vec<&WalletTrackerSnapshot> = self
            .wallet_tracker
            .tracked_addresses
            .iter()
            .filter_map(|address| {
                self.wallet_tracker
                    .rows
                    .get(address)
                    .and_then(|row| row.snapshot.as_ref())
            })
            .collect();
        let total_equity = sum_optional_f64(snapshots.iter().map(|snapshot| snapshot.equity));
        let total_withdrawable =
            sum_optional_f64(snapshots.iter().map(|snapshot| snapshot.withdrawable));
        let total_upnl = sum_optional_f64(snapshots.iter().map(|snapshot| snapshot.unrealized_pnl));
        let error_count = self
            .wallet_tracker
            .tracked_addresses
            .iter()
            .filter(|address| {
                self.wallet_tracker
                    .rows
                    .get(*address)
                    .is_some_and(|row| row.error.is_some() || row.order_error.is_some())
            })
            .count();
        let has_invalid_snapshot =
            total_equity.is_none() || total_withdrawable.is_none() || total_upnl.is_none();
        let denomination = self.display_denomination_context();

        row![
            column![
                text("Wallet Tracker").size(16).color(theme.palette().text),
                text(format!(
                    "{} wallets | Equity {} | Available {} | uPnL {}",
                    wallet_count,
                    money_total_display(&denomination, total_equity),
                    money_total_display(&denomination, total_withdrawable),
                    signed_money_total_display(&denomination, total_upnl)
                ))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(if error_count > 0 {
                    theme.palette().danger
                } else if has_invalid_snapshot {
                    theme.palette().warning
                } else {
                    theme.extended_palette().background.weak.text
                }),
            ]
            .spacing(2),
            Space::new().width(Fill),
            button(text("Import Labels").size(11))
                .on_press(Message::ImportWalletLabels)
                .padding([4, 8]),
            button(text("Export Labels").size(11))
                .on_press(Message::ExportWalletLabels)
                .padding([4, 8]),
            button(text("Queue Refresh").size(11))
                .on_press(Message::WalletTrackerRefresh)
                .padding([4, 8]),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }
}

fn money_total_display(
    denomination: &crate::denomination::DisplayDenominationContext,
    value: Option<f64>,
) -> String {
    value
        .map(|value| denomination.format_value(value, 2))
        .unwrap_or_else(invalid_wallet_data)
}

fn signed_money_total_display(
    denomination: &crate::denomination::DisplayDenominationContext,
    value: Option<f64>,
) -> String {
    value
        .map(|value| denomination.format_signed_value(value, 2))
        .unwrap_or_else(invalid_wallet_data)
}

#[cfg(test)]
mod tests;
