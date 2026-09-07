use crate::account::transfers::{TransferDirection, TransferEntry, TransferProvider};
use crate::account_state::transfers::{TRANSFER_PAGE_SIZE, TransferSourceState};
use crate::account_views::{
    history::format_history_time_millis, table_helpers::account_table_scroll,
};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, button, column, container, row, rule, text};
use iced::{Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(crate) fn view_transfer_history(&self) -> Element<'_, Message> {
        if self.connected_address.is_none() {
            return text("Connect wallet to view deposits and withdrawals")
                .size(12)
                .into();
        }
        let history = &self.transfer_history;
        let theme = self.theme();
        let muted = theme.extended_palette().background.weak.text;
        let current = history.address.as_deref() == self.connected_address.as_deref();
        let entries = if current {
            history.entries.as_slice()
        } else {
            &[]
        };
        let page = if current { history.page } else { 0 };
        let pages = entries.len().max(1).div_ceil(TRANSFER_PAGE_SIZE);
        let loading = current && history.loading();
        let controls = row![
            text(if loading {
                "Refreshing…".to_string()
            } else {
                format!("{} transfers", entries.len())
            })
            .size(11)
            .color(muted)
            .width(Fill),
            button(text("Refresh").size(11))
                .on_press_maybe((!loading).then_some(Message::RefreshTransferHistory)),
            button(text("Previous").size(11))
                .on_press_maybe((page > 0).then_some(Message::TransferHistoryPage(false))),
            text(format!("{} / {pages}", page + 1)).size(11),
            button(text("Next").size(11))
                .on_press_maybe((page + 1 < pages).then_some(Message::TransferHistoryPage(true))),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);
        let header_cell = |label, portion| {
            text(label)
                .size(11)
                .color(muted)
                .width(Length::FillPortion(portion))
        };
        let header = row![
            header_cell("Time (UTC)", 2),
            header_cell("Type", 2),
            header_cell("Asset", 1),
            header_cell("Amount sent", 2),
            header_cell("Route", 3),
            header_cell("Status", 2),
        ]
        .spacing(6)
        .padding([4, 6]);
        let mut content = column![controls].spacing(6);
        if current {
            for (provider, source) in [
                (TransferProvider::Hyperliquid, &history.native),
                (TransferProvider::Unit, &history.unit),
            ] {
                if let Some(warning) = source_warning(provider, source) {
                    content = content.push(text(warning).size(11).color(theme.palette().warning));
                }
            }
        }
        content = content.push(header).push(rule::horizontal(1));
        if entries.is_empty() {
            let label = if !current || loading {
                "Loading deposits and withdrawals…"
            } else if history.native.loaded
                && history.unit.loaded
                && history.native.error.is_none()
                && history.unit.error.is_none()
                && history.native.warning.is_none()
                && history.unit.warning.is_none()
            {
                "No deposits or withdrawals"
            } else {
                "History is incomplete. Refresh to retry."
            };
            content = content.push(text(label).size(12).color(muted));
        }
        for (index, entry) in entries
            .iter()
            .enumerate()
            .skip(page * TRANSFER_PAGE_SIZE)
            .take(TRANSFER_PAGE_SIZE)
        {
            let expanded = history.expanded.as_ref() == Some(&entry.id);
            content = content.push(view_transfer_row(entry, index, expanded, &theme));
        }
        account_table_scroll(content)
    }
}

fn source_warning(provider: TransferProvider, source: &TransferSourceState) -> Option<String> {
    if let Some(error) = &source.error {
        Some(format!(
            "{}: {error}{}",
            provider.label(),
            if source.loaded {
                ". Showing previous data."
            } else {
                ""
            }
        ))
    } else {
        source
            .warning
            .as_ref()
            .map(|warning| format!("{}: {warning}", provider.label()))
    }
}

fn view_transfer_row<'a>(
    entry: &'a TransferEntry,
    index: usize,
    expanded: bool,
    theme: &Theme,
) -> Element<'a, Message> {
    let direction_color = match entry.direction {
        TransferDirection::Deposit => theme.palette().success,
        TransferDirection::Withdrawal => theme.palette().danger,
    };
    let cell = |value: String, portion| text(value).size(12).width(Length::FillPortion(portion));
    let status_color = if entry.failed {
        theme.palette().danger
    } else {
        theme.palette().text
    };
    let summary = row![
        cell(format_history_time_millis(entry.time), 2),
        cell(
            format!(
                "{} {}",
                if expanded { "▾" } else { "▸" },
                entry.direction.label()
            ),
            2
        )
        .color(direction_color),
        cell(entry.asset.clone(), 1),
        cell(entry.amount.clone(), 2),
        cell(
            format!("{} → {}", entry.source_chain, entry.destination_chain),
            3
        ),
        cell(entry.status.clone(), 2).color(status_color),
    ]
    .spacing(6);
    let mut content = column![
        button(summary)
            .width(Fill)
            .padding([6, 6])
            .style(button::text)
            .on_press(Message::ToggleTransferDetails(index))
    ];
    if expanded {
        let mut details = Column::new()
            .spacing(5)
            .push(
                text(entry.provider.label())
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            )
            .push(detail_value(
                "Source wallet",
                &entry.source_chain,
                entry.source_address.as_deref(),
            ))
            .push(detail_value(
                "Destination wallet",
                &entry.destination_chain,
                entry.destination_address.as_deref(),
            ));
        if entry.protocol_address.is_some() {
            details = details.push(detail_value(
                "Unit bridge address",
                &entry.source_chain,
                entry.protocol_address.as_deref(),
            ));
        }
        if entry.provider == TransferProvider::Hyperliquid {
            details = details
                .push(detail_value("Ledger transaction", "Hyperliquid", entry.ledger_tx.as_deref()))
                .push(text("External wallet and Arbitrum transaction are not provided by the Hyperliquid ledger.").size(11));
            if entry.direction == TransferDirection::Withdrawal {
                details = details.push(text("Debited records the Hyperliquid withdrawal; Arbitrum delivery is unverified.").size(11));
            }
        } else {
            details = details
                .push(detail_value(
                    "Source transaction reference",
                    &entry.source_chain,
                    entry.source_tx.as_deref(),
                ))
                .push(detail_value(
                    "Destination transaction reference",
                    &entry.destination_chain,
                    entry.destination_tx.as_deref(),
                ));
        }
        if let Some(fee) = &entry.fee {
            details = details.push(
                text(format!(
                    "{}: {fee} {}",
                    if entry.provider == TransferProvider::Unit {
                        "Destination fee (source asset units)"
                    } else {
                        "Withdrawal fee"
                    },
                    entry.asset
                ))
                .size(11),
            );
        }
        if let Some(fee) = &entry.sweep_fee {
            details =
                details.push(text(format!("Sweep fee estimate: {fee} {}", entry.asset)).size(11));
        }
        if let Some(confirmations) = entry.source_confirmations {
            details = details.push(text(format!("Source confirmations: {confirmations}")).size(11));
        }
        if let Some(confirmations) = entry.destination_confirmations {
            details =
                details.push(text(format!("Destination confirmations: {confirmations}")).size(11));
        }
        content = content.push(container(details).padding([6, 12]).width(Fill));
    }
    content.push(rule::horizontal(1)).into()
}

fn detail_value<'a>(label: &str, chain: &str, value: Option<&'a str>) -> Element<'a, Message> {
    let copy = value.map(|value| Message::CopyToClipboard(value.into()));
    row![
        text(format!("{label} ({chain})"))
            .size(11)
            .width(Length::FillPortion(2)),
        text(value.unwrap_or("Not provided"))
            .size(11)
            .wrapping(text::Wrapping::Glyph)
            .width(Length::FillPortion(5)),
        button(text("Copy").size(11))
            .style(button::text)
            .on_press_maybe(copy),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}
