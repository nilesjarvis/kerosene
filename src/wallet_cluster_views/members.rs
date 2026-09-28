use crate::app_state::TradingTerminal;
use crate::helpers::text_input_style;
use crate::message::Message;
use crate::wallet_cluster_state::WalletCluster;
use iced::widget::{Space, button, column, row, text, text_input};
use iced::{Alignment, Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(super) fn view_wallet_cluster_members(
        &self,
        cluster: &WalletCluster,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let rename_id = cluster.id.clone();
        let mut members = column![
            row![
                text("Members").size(13).color(theme.palette().text),
                text(format!(
                    "{} / {}",
                    cluster.members.len(),
                    crate::wallet_cluster_state::MAX_WALLET_CLUSTER_MEMBERS
                ))
                .size(11)
                .color(theme.extended_palette().background.weak.text),
                Space::new().width(Fill),
                button(text("Delete Cluster").size(11))
                    .padding([4, 8])
                    .on_press(Message::WalletClusterDeleted(cluster.id.clone()))
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text_input("Cluster name", &cluster.name)
                .style(text_input_style)
                .on_input(move |value| Message::WalletClusterRenamed(rename_id.clone(), value))
                .size(12)
                .padding(6)
                .width(Fill),
        ]
        .spacing(8);

        if cluster.members.is_empty() {
            members = members.push(
                text("No wallets in this cluster.")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            );
        } else {
            members = members.push(Self::wallet_cluster_member_header(theme));
            for member in &cluster.members {
                members = members.push(self.view_wallet_cluster_member_row(
                    cluster.id.clone(),
                    member,
                    theme,
                ));
            }
        }

        let member_ids: std::collections::HashSet<&str> = cluster
            .members
            .iter()
            .map(|member| member.profile_secret_id.as_str())
            .collect();
        let mut add_row = row![text("Add").size(12).color(theme.palette().text)]
            .spacing(6)
            .align_y(Alignment::Center);
        let mut available_count = 0usize;
        for profile in &self.accounts {
            // Only offer profiles that can actually sign a cluster leg: not
            // already a member, not watch-only, and with a committed agent key.
            if member_ids.contains(profile.secret_id.as_str())
                || self.ghost_account_secret_ids.contains(&profile.secret_id)
                || profile.agent_key.trim().is_empty()
            {
                continue;
            }
            available_count += 1;
            let address = Self::normalize_wallet_address(&profile.wallet_address)
                .map(|address| Self::short_address(&address))
                .unwrap_or_else(|| "missing address".to_string());
            let label = if profile.name.trim().is_empty() {
                address
            } else {
                format!("{} ({address})", profile.name.trim())
            };
            add_row = add_row.push(
                button(text(label).size(11))
                    .padding([4, 8])
                    .on_press(Message::WalletClusterAddMember(profile.secret_id.clone())),
            );
        }
        if available_count == 0 {
            add_row = add_row.push(
                text("No saved trading profiles available.")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        column![members, add_row].spacing(10).into()
    }

    fn wallet_cluster_member_header(theme: &Theme) -> Element<'static, Message> {
        row![
            text("Account")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Length::FillPortion(3)),
            text("Address")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Length::FillPortion(2)),
            text("Weight")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Length::Fixed(90.0)),
            text("Snapshot")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Length::FillPortion(2)),
            text("").width(Length::Fixed(72.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_wallet_cluster_member_row(
        &self,
        cluster_id: String,
        member: &crate::wallet_cluster_state::WalletClusterMember,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let profile = self
            .accounts
            .iter()
            .find(|profile| profile.secret_id == member.profile_secret_id);
        let profile_name = profile
            .map(|profile| profile.name.trim())
            .filter(|name| !name.is_empty())
            .unwrap_or("Unnamed");
        let address = profile
            .and_then(|profile| Self::normalize_wallet_address(&profile.wallet_address))
            .unwrap_or_default();
        let display_address = if address.is_empty() {
            "missing".to_string()
        } else {
            Self::short_address(&address)
        };
        let snapshot = self
            .wallet_clusters
            .member_data
            .get(&member.profile_secret_id)
            .map(|state| {
                if state.loading {
                    "loading"
                } else if state.stale {
                    "stale"
                } else if let Some(error) = state.error.as_ref() {
                    error.as_str()
                } else if state.data.is_some() {
                    "ready"
                } else {
                    "not loaded"
                }
            })
            .unwrap_or("not loaded");
        let weight_cluster_id = cluster_id.clone();
        let weight_member_id = member.profile_secret_id.clone();
        row![
            text(profile_name).size(12).width(Length::FillPortion(3)),
            text(display_address).size(12).width(Length::FillPortion(2)),
            text_input("0", &member.weight_input)
                .style(text_input_style)
                .on_input(move |value| {
                    Message::WalletClusterMemberWeightChanged(
                        weight_cluster_id.clone(),
                        Some(weight_member_id.clone()).into(),
                        value.into(),
                    )
                })
                .size(12)
                .padding(5)
                .width(Length::Fixed(90.0)),
            text(snapshot)
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Length::FillPortion(2)),
            button(text("Remove").size(11))
                .padding([4, 8])
                .on_press(Message::WalletClusterRemoveMember(
                    cluster_id,
                    Some(member.profile_secret_id.clone()).into(),
                ))
                .width(Length::Fixed(72.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }
}
