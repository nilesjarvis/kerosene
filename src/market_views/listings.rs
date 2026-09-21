use crate::app_state::TradingTerminal;
use crate::config::listings::ListingKind;
use crate::market_state::listings::ListingsFilter;
use crate::message::Message;
use iced::widget::{button, column, container, row, rule, scrollable, text, tooltip};
use iced::{Alignment, Element, Fill, Theme};

impl TradingTerminal {
    pub(crate) fn view_new_listings(&self) -> Element<'_, Message> {
        let state = &self.listings;
        let theme = self.theme();
        let palette = theme.extended_palette();
        let muted = palette.background.weak.text;
        let status = if let Some(error) = state.error {
            error
        } else if state.loading {
            "Checking…"
        } else if state.last_success_ms.is_none() {
            "Connecting…"
        } else if state
            .last_success_ms
            .is_some_and(|at| state.now_ms.saturating_sub(at) > 90_000)
        {
            "Feed delayed"
        } else {
            "Monitoring · 30s"
        };
        let status_color = if state.error.is_some() {
            palette.danger.base.color
        } else {
            muted
        };
        let tabs = ListingsFilter::ALL
            .into_iter()
            .fold(row![].spacing(4), |tabs, filter| {
                tabs.push(
                    button(text(filter.label()).size(11))
                        .padding([4, 9])
                        .on_press(Message::ListingsFilterChanged(filter))
                        .style(if filter == state.filter {
                            button::secondary
                        } else {
                            button::text
                        }),
                )
            });
        let header = row![
            text("New Listings").size(13).width(Fill),
            button(text("Refresh").size(11))
                .padding([3, 6])
                .on_press_maybe(
                    (!state.loading && !state.saving).then_some(Message::RefreshListings)
                )
                .style(button::text),
        ]
        .align_y(Alignment::Center);
        let mut rows = column![].spacing(4).width(Fill);
        let mut count = 0;
        for event in &state.history.events {
            if !state.filter.includes(event.kind) || self.symbol_key_is_hidden(&event.key) {
                continue;
            }
            count += 1;
            let (badge, venue) = match event.kind {
                ListingKind::Spot => ("SPOT", "Hyperliquid"),
                ListingKind::Perp => match event.key.split_once(':') {
                    Some((dex, _)) => ("HIP-3", dex),
                    None => ("PERP", "Hyperliquid"),
                },
            };
            let badge = container(text(badge).size(10))
                .padding([2, 5])
                .style(|theme: &Theme| container::Style {
                    background: Some(theme.extended_palette().primary.weak.color.into()),
                    text_color: Some(theme.extended_palette().primary.weak.text),
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            let age = detected_age(state.now_ms.saturating_sub(event.detected_at_ms));
            let date = chrono::DateTime::from_timestamp_millis(event.detected_at_ms as i64)
                .map(|date| format!("First detected {} UTC", date.format("%Y-%m-%d %H:%M:%S")))
                .unwrap_or_else(|| "First detected".into());
            let available = self
                .exchange_symbols
                .iter()
                .any(|symbol| symbol.key == event.key);
            let content = column![
                row![text(&event.label).size(13).width(Fill), badge]
                    .spacing(8)
                    .align_y(Alignment::Center),
                row![
                    text(venue).size(11).color(muted).width(Fill),
                    tooltip(
                        text(age).size(11).color(muted),
                        text(date).size(11),
                        tooltip::Position::Top
                    )
                ]
                .spacing(8),
            ]
            .spacing(5);
            rows = rows.push(
                button(content)
                    .width(Fill)
                    .padding(10)
                    .on_press_maybe(available.then(|| Message::SymbolSelected(event.key.clone())))
                    .style(|theme: &Theme, status| {
                        let mut style = button::text(theme, status);
                        style.background = Some(
                            if matches!(status, button::Status::Hovered) {
                                theme.extended_palette().background.strong.color
                            } else {
                                theme.extended_palette().background.weak.color
                            }
                            .into(),
                        );
                        style.border.radius = 5.0.into();
                        style
                    }),
            );
        }
        let body: Element<'_, Message> = if count == 0 {
            let detail = if state.error.is_some() {
                "Waiting for market data"
            } else if !state.history.perps.initialized || !state.history.spot.initialized {
                "Establishing market baseline…"
            } else {
                "New markets will appear here"
            };
            container(
                column![
                    text("No new listings yet").size(13),
                    text(detail).size(11).color(muted)
                ]
                .spacing(6)
                .align_x(Alignment::Center),
            )
            .center(Fill)
            .into()
        } else {
            scrollable(rows).height(Fill).into()
        };
        let mut content = column![
            header,
            tabs,
            rule::horizontal(1),
            body,
            row![
                text(status).size(11).color(status_color).width(Fill),
                text("First detected").size(11).color(muted)
            ]
        ]
        .spacing(8)
        .height(Fill);
        if state.storage_error {
            content = content.push(
                text("History unavailable · session only")
                    .size(11)
                    .color(palette.danger.base.color),
            );
        }
        container(content)
            .padding(10)
            .width(Fill)
            .height(Fill)
            .into()
    }
}

fn detected_age(age_ms: u64) -> String {
    match age_ms / 1000 {
        0..60 => "Just now".into(),
        60..3600 => format!("{}m ago", age_ms / 60_000),
        3600..86400 => format!("{}h ago", age_ms / 3_600_000),
        _ => format!("{}d ago", age_ms / 86_400_000),
    }
}
