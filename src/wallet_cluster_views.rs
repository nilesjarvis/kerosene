use crate::app_state::TradingTerminal;
use crate::helpers::{format_usd, format_with_commas, text_input_style};
use crate::message::Message;
use crate::signing::OrderKind;
use crate::wallet_cluster_state::{
    WalletClusterCloseSide, WalletClusterExecution, WalletClusterLegStatus,
    WalletClusterPositionSummary, cluster_button_label, cluster_order_kind_options,
};

use iced::widget::container as container_style;
use iced::widget::{
    Column, Space, button, checkbox, column, container, row, rule, scrollable, text, text_input,
};
use iced::{Alignment, Element, Fill, Length, Theme};

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
                container(text(status.clone()).size(12).color(color))
                    .padding([6, 8])
                    .width(Fill),
            );
        }

        if let Some(cluster) = self.wallet_clusters.selected_cluster() {
            content = content
                .push(rule::horizontal(1))
                .push(self.view_wallet_cluster_members(cluster.id.clone(), &theme))
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

    fn view_wallet_cluster_members(
        &self,
        cluster_id: String,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let Some(cluster) = self
            .wallet_clusters
            .clusters
            .iter()
            .find(|cluster| cluster.id == cluster_id)
        else {
            return text("").into();
        };
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
            if member_ids.contains(profile.secret_id.as_str())
                || self.ghost_account_secret_ids.contains(&profile.secret_id)
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
                    "loading".to_string()
                } else if state.stale {
                    "stale".to_string()
                } else if let Some(error) = state.error.as_ref() {
                    error.clone()
                } else if state.data.is_some() {
                    "ready".to_string()
                } else {
                    "not loaded".to_string()
                }
            })
            .unwrap_or_else(|| "not loaded".to_string());
        let weight_cluster_id = cluster_id.clone();
        let weight_member_id = member.profile_secret_id.clone();
        row![
            text(profile_name.to_string())
                .size(12)
                .width(Length::FillPortion(3)),
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

    fn view_wallet_cluster_ticket(&self, theme: &Theme) -> Element<'_, Message> {
        let order_kind = self.wallet_clusters.order_kind;
        let kind_row = cluster_order_kind_options().into_iter().fold(
            row![text("Order").size(13).color(theme.palette().text)]
                .spacing(6)
                .align_y(Alignment::Center),
            |row, kind| {
                let label = if kind == order_kind {
                    format!("{} *", cluster_button_label(kind))
                } else {
                    cluster_button_label(kind).to_string()
                };
                row.push(
                    button(text(label).size(11))
                        .padding([4, 8])
                        .on_press(Message::WalletClusterSetOrderKind(kind)),
                )
            },
        );
        let show_price = !matches!(order_kind, OrderKind::Market);
        let price_input = text_input("Price", &self.wallet_clusters.order_price)
            .style(text_input_style)
            .on_input(|value| Message::WalletClusterOrderPriceChanged(value.into()))
            .size(12)
            .padding(6)
            .width(Length::Fixed(130.0));
        let mut price_row = row![
            text(self.display_name_for_symbol(&self.active_symbol))
                .size(12)
                .width(Length::Fill),
            button(text("Mid").size(11))
                .padding([4, 8])
                .on_press(Message::WalletClusterSetMidPrice)
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if show_price {
            price_row = price_row.push(price_input);
        }

        column![
            kind_row,
            price_row,
            row![
                text_input(
                    if self.wallet_clusters.order_quantity_is_usd {
                        "USDC size"
                    } else {
                        "Coin size"
                    },
                    &self.wallet_clusters.order_quantity,
                )
                .style(text_input_style)
                .on_input(|value| Message::WalletClusterOrderQuantityChanged(value.into()))
                .size(12)
                .padding(6)
                .width(Length::Fixed(150.0)),
                button(
                    text(if self.wallet_clusters.order_quantity_is_usd {
                        "USDC"
                    } else {
                        "Coin"
                    })
                    .size(11)
                )
                .padding([4, 8])
                .on_press(Message::WalletClusterToggleOrderDenomination),
                checkbox(self.wallet_clusters.reduce_only)
                    .label("Reduce only")
                    .on_toggle(|_| Message::WalletClusterToggleReduceOnly)
                    .size(12)
                    .spacing(6)
                    .text_size(12),
                Space::new().width(Fill),
                button(text("Buy Cluster").size(12))
                    .padding([6, 12])
                    .on_press(Message::WalletClusterSubmitOrder { is_buy: true }),
                button(text("Sell Cluster").size(12))
                    .padding([6, 12])
                    .on_press(Message::WalletClusterSubmitOrder { is_buy: false }),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(8)
        .into()
    }

    fn view_wallet_cluster_positions(&self, theme: &Theme) -> Element<'_, Message> {
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

    fn view_wallet_cluster_position_row(
        &self,
        summary: WalletClusterPositionSummary,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let symbol = summary.symbol.clone();
        let close_buttons = row![
            close_button(
                symbol.clone(),
                WalletClusterCloseSide::Long,
                0.25,
                false,
                summary.has_long(),
                "L25"
            ),
            close_button(
                symbol.clone(),
                WalletClusterCloseSide::Long,
                0.5,
                false,
                summary.has_long(),
                "L50"
            ),
            close_button(
                symbol.clone(),
                WalletClusterCloseSide::Long,
                1.0,
                true,
                summary.has_long(),
                "L100 M"
            ),
            close_button(
                symbol.clone(),
                WalletClusterCloseSide::Short,
                0.25,
                false,
                summary.has_short(),
                "S25"
            ),
            close_button(
                symbol.clone(),
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

    fn view_wallet_cluster_executions(&self, theme: &Theme) -> Element<'_, Message> {
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

    fn view_wallet_cluster_execution(
        &self,
        execution: &WalletClusterExecution,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let header = row![
            text(format!(
                "#{} {} {} {}",
                execution.id,
                execution.cluster_name,
                execution.kind_label(),
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
                    text(leg.label.clone())
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
                    text(leg.message.clone())
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

fn close_button(
    symbol: String,
    side: WalletClusterCloseSide,
    fraction: f64,
    use_market: bool,
    enabled: bool,
    label: &'static str,
) -> Element<'static, Message> {
    let mut btn = button(text(label).size(10)).padding([3, 5]);
    if enabled {
        btn = btn.on_press(Message::WalletClusterClosePosition {
            symbol,
            side,
            fraction,
            use_market,
        });
    }
    btn.into()
}

trait WalletClusterExecutionView {
    fn kind_label(&self) -> &'static str;
}

impl WalletClusterExecutionView for WalletClusterExecution {
    fn kind_label(&self) -> &'static str {
        match self.kind {
            crate::wallet_cluster_state::WalletClusterExecutionKind::Order => "order",
            crate::wallet_cluster_state::WalletClusterExecutionKind::Close => "close",
        }
    }
}
