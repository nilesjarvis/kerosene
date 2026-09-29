mod executions;
mod members;
mod positions;
mod ticket;

use crate::app_state::TradingTerminal;
use crate::helpers::text_input_style;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, button, column, container, row, rule, scrollable, text, text_input};
use iced::{Alignment, Element, Fill, Length, Theme};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Wallet Clusters Window
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_wallet_clusters(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let mut content = column![
            self.view_wallet_cluster_header(&theme),
            self.view_wallet_cluster_create_row(),
            rule::horizontal(1),
            self.view_wallet_cluster_selector(&theme),
        ]
        .spacing(10);

        if let Some((status, is_error)) = self.wallet_clusters.status.as_ref() {
            let color = if *is_error {
                theme.palette().danger
            } else {
                theme.palette().success
            };
            content = content.push(
                container(text(status.as_str()).size(12).color(color))
                    .padding([6, 8])
                    .width(Fill),
            );
        }

        if let Some(cluster) = self.wallet_clusters.selected_cluster() {
            content = content
                .push(rule::horizontal(1))
                .push(self.view_wallet_cluster_members(cluster, &theme))
                .push(rule::horizontal(1))
                .push(self.view_wallet_cluster_ticket(&theme))
                .push(rule::horizontal(1))
                .push(self.view_wallet_cluster_positions(&theme))
                .push(rule::horizontal(1))
                .push(self.view_wallet_cluster_executions(&theme));
        } else {
            content = content.push(
                container(text("Create a cluster to begin.").size(12))
                    .width(Fill)
                    .height(Length::Fixed(160.0))
                    .center_x(Fill)
                    .center_y(Length::Fixed(160.0)),
            );
        }

        container(scrollable(content).height(Fill))
            .padding(12)
            .width(Fill)
            .height(Fill)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                ..Default::default()
            })
            .into()
    }

    fn view_wallet_cluster_header(&self, theme: &Theme) -> Element<'_, Message> {
        row![
            text("Wallet Clusters").size(14).color(theme.palette().text),
            Space::new().width(Fill),
            button(text("Refresh").size(11))
                .padding([5, 8])
                .on_press(Message::WalletClusterRefresh)
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_wallet_cluster_create_row(&self) -> Element<'_, Message> {
        row![
            text_input(
                "New cluster name",
                &self.wallet_clusters.new_cluster_name_input
            )
            .style(text_input_style)
            .on_input(Message::WalletClusterNameInputChanged)
            .on_submit(Message::WalletClusterCreate)
            .size(12)
            .padding(6)
            .width(Length::Fill),
            button(text("Create").size(11))
                .padding([5, 10])
                .on_press(Message::WalletClusterCreate)
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_wallet_cluster_selector(&self, theme: &Theme) -> Element<'_, Message> {
        let mut clusters = row![text("Clusters").size(12).color(theme.palette().text)]
            .spacing(6)
            .align_y(Alignment::Center);
        for cluster in &self.wallet_clusters.clusters {
            let selected = self.wallet_clusters.selected_cluster_id.as_deref() == Some(&cluster.id);
            let label = if selected {
                format!("{} *", cluster.display_name())
            } else {
                cluster.display_name()
            };
            clusters = clusters.push(
                button(text(label).size(11))
                    .padding([4, 8])
                    .on_press(Message::WalletClusterSelected(cluster.id.clone())),
            );
        }
        clusters.into()
    }
}
