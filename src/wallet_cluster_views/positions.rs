use crate::app_state::TradingTerminal;
use crate::helpers::{format_usd, format_with_commas};
use crate::message::Message;
use crate::wallet_cluster_state::{WalletClusterCloseSide, WalletClusterPositionSummary};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};

impl TradingTerminal {
    pub(super) fn view_wallet_cluster_positions(&self, theme: &Theme) -> Element<'_, Message> {
        let summaries = self.wallet_cluster_position_summaries();
        let mut content = column![
            text("Cluster Positions")
                .size(13)
                .color(theme.palette().text)
        ]
        .spacing(8);
        if summaries.is_empty() {
            return content
                .push(
                    text("No loaded cluster positions.")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .into();
        }
        content = content.push(Self::wallet_cluster_position_header(theme));
        for summary in summaries {
            content = content.push(self.view_wallet_cluster_position_row(summary, theme));
        }
        content.into()
    }

    fn wallet_cluster_position_header(theme: &Theme) -> Element<'static, Message> {
        row![
            text("Symbol")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Length::FillPortion(2)),
            text("Net")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Length::FillPortion(2)),
            text("Long")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Length::FillPortion(2)),
            text("Short")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Length::FillPortion(2)),
            text("Value")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Length::FillPortion(2)),
            text("uPnL")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Length::FillPortion(2)),
            text("").width(Length::Fixed(260.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    pub(super) fn view_wallet_cluster_position_row(
        &self,
        summary: WalletClusterPositionSummary,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let symbol = &summary.symbol;
        let close_buttons = row![
            close_button(
                symbol,
                WalletClusterCloseSide::Long,
                0.25,
                false,
                summary.has_long(),
                "L25"
            ),
            close_button(
                symbol,
                WalletClusterCloseSide::Long,
                0.5,
                false,
                summary.has_long(),
                "L50"
            ),
            close_button(
                symbol,
                WalletClusterCloseSide::Long,
                1.0,
                true,
                summary.has_long(),
                "L100 M"
            ),
            close_button(
                symbol,
                WalletClusterCloseSide::Short,
                0.25,
                false,
                summary.has_short(),
                "S25"
            ),
            close_button(
                symbol,
                WalletClusterCloseSide::Short,
                0.5,
                false,
                summary.has_short(),
                "S50"
            ),
            close_button(
                symbol,
                WalletClusterCloseSide::Short,
                1.0,
                true,
                summary.has_short(),
                "S100 M"
            ),
        ]
        .spacing(4);
        let upnl_color = summary
            .unrealized_pnl
            .map(|value| {
                if value >= 0.0 {
                    theme.palette().success
                } else {
                    theme.palette().danger
                }
            })
            .unwrap_or(theme.extended_palette().background.weak.text);
        let member_detail = summary
            .members
            .iter()
            .map(|member| {
                let dex = if member.dex.is_empty() {
                    "main"
                } else {
                    &member.dex
                };
                let entry = member
                    .entry_price
                    .map(format_with_commas)
                    .unwrap_or_else(|| "-".to_string());
                let value = member
                    .value
                    .map(|value| format_usd(&value.to_string()))
                    .unwrap_or_else(|| "-".to_string());
                let upnl = member
                    .unrealized_pnl
                    .map(|value| format_usd(&value.to_string()))
                    .unwrap_or_else(|| "-".to_string());
                format!(
                    "{} {} {dex} size {} entry {entry} value {value} uPnL {upnl}",
                    member.label,
                    Self::short_address(&member.address),
                    format_with_commas(member.size),
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        let row = row![
            text(self.display_name_for_symbol(&summary.symbol))
                .size(12)
                .width(Length::FillPortion(2)),
            text(format_with_commas(summary.net_size))
                .size(12)
                .width(Length::FillPortion(2)),
            text(format_with_commas(summary.long_size))
                .size(12)
                .width(Length::FillPortion(2)),
            text(format_with_commas(summary.short_size))
                .size(12)
                .width(Length::FillPortion(2)),
            text(
                summary
                    .value
                    .map(|value| format_usd(&value.to_string()))
                    .unwrap_or_else(|| "-".to_string())
            )
            .size(12)
            .width(Length::FillPortion(2)),
            text(
                summary
                    .unrealized_pnl
                    .map(|value| format_usd(&value.to_string()))
                    .unwrap_or_else(|| "-".to_string())
            )
            .size(12)
            .color(upnl_color)
            .width(Length::FillPortion(2)),
            container(close_buttons).width(Length::Fixed(260.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        column![
            row,
            text(member_detail)
                .size(10)
                .color(theme.extended_palette().background.weak.text)
        ]
        .spacing(2)
        .into()
    }
}

fn close_button(
    symbol: &str,
    side: WalletClusterCloseSide,
    fraction: f64,
    use_market: bool,
    enabled: bool,
    label: &'static str,
) -> Element<'static, Message> {
    let mut btn = button(text(label).size(10)).padding([3, 5]);
    if enabled {
        btn = btn.on_press(Message::WalletClusterClosePosition {
            symbol: symbol.to_string(),
            side,
            fraction,
            use_market,
        });
    }
    btn.into()
}
