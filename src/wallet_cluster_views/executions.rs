use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::wallet_cluster_state::{
    WalletClusterExecution, WalletClusterExecutionKind, WalletClusterLegStatus,
};
use iced::widget::{Column, column, container, row, text};
use iced::{Alignment, Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(super) fn view_wallet_cluster_executions(&self, theme: &Theme) -> Element<'_, Message> {
        let mut content = column![
            text("Recent Executions")
                .size(13)
                .color(theme.palette().text)
        ]
        .spacing(8);
        if self.wallet_clusters.executions.is_empty() {
            return content
                .push(
                    text("No cluster executions yet.")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .into();
        }
        for execution in &self.wallet_clusters.executions {
            content = content.push(self.view_wallet_cluster_execution(execution, theme));
        }
        content.into()
    }

    fn view_wallet_cluster_execution<'a>(
        &'a self,
        execution: &'a WalletClusterExecution,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let kind_label = match execution.kind {
            WalletClusterExecutionKind::Order => "order",
            WalletClusterExecutionKind::Close => "close",
        };
        let header = row![
            text(format!(
                "#{} {} {} {}",
                execution.id,
                execution.cluster_name,
                kind_label,
                self.display_name_for_symbol(&execution.symbol)
            ))
            .size(12)
            .width(Fill),
            text(format!(
                "{}/{}",
                execution.completed_count(),
                execution.legs.len()
            ))
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(8);
        let mut legs = Column::new().spacing(4).push(header);
        for leg in &execution.legs {
            let color = match leg.status {
                WalletClusterLegStatus::Confirmed => theme.palette().success,
                WalletClusterLegStatus::Failed | WalletClusterLegStatus::Uncertain => {
                    theme.palette().danger
                }
                WalletClusterLegStatus::Pending | WalletClusterLegStatus::Checking => {
                    theme.extended_palette().background.weak.text
                }
            };
            legs = legs.push(
                row![
                    text(leg.label.as_str())
                        .size(11)
                        .width(Length::FillPortion(2)),
                    text(Self::short_address(&leg.address))
                        .size(11)
                        .width(Length::FillPortion(2)),
                    text(self.display_name_for_symbol(&leg.symbol))
                        .size(11)
                        .width(Length::FillPortion(2)),
                    text(if leg.is_buy { "Buy" } else { "Sell" })
                        .size(11)
                        .width(Length::Fixed(36.0)),
                    text(format!("{} @ {}", leg.size, leg.price))
                        .size(11)
                        .width(Length::FillPortion(2)),
                    text(leg.status.label())
                        .size(11)
                        .color(color)
                        .width(Length::Fixed(72.0)),
                    text(leg.message.as_str())
                        .size(11)
                        .color(theme.extended_palette().background.weak.text)
                        .width(Length::FillPortion(3)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        container(legs).padding([6, 0]).width(Fill).into()
    }
}
