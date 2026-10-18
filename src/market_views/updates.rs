use crate::app_state::TradingTerminal;
use crate::config::updates::{MarginTier, ParameterChange, ParameterEvent, ParameterKind};
use crate::market_state::updates::{UPDATES_STALE_MS, UpdatesFilter, UpdatesVenue};
use crate::message::Message;
use iced::widget::{
    button, checkbox, column, container, pick_list, row, rule, scrollable, text, text_input,
    tooltip,
};
use iced::{Alignment, Element, Fill};
use std::collections::BTreeSet;

impl TradingTerminal {
    pub(crate) fn view_updates(&self) -> Element<'_, Message> {
        let state = &self.updates;
        let theme = self.theme();
        let palette = theme.extended_palette();
        let muted = palette.background.weak.text;
        let filters: Vec<_> = std::iter::once(UpdatesFilter::All)
            .chain(ParameterKind::ALL.into_iter().map(UpdatesFilter::Kind))
            .collect();
        let dexes: BTreeSet<_> = state
            .history
            .parameters
            .keys()
            .chain(state.history.caps.keys())
            .chain(state.history.events.iter().map(|event| &event.key))
            .filter_map(|key| key.split_once(':').map(|(dex, _)| dex.to_string()))
            .collect();
        let venues: Vec<_> = [UpdatesVenue::All, UpdatesVenue::Native]
            .into_iter()
            .chain(dexes.into_iter().map(UpdatesVenue::Dex))
            .collect();
        let header = row![
            text("Updates").size(13).width(Fill),
            button(text("Refresh").size(11))
                .padding([3, 6])
                .on_press_maybe((!state.loading).then_some(Message::RefreshUpdates))
                .style(button::text),
        ]
        .align_y(Alignment::Center);
        let controls = row![
            pick_list(filters, Some(state.filter), Message::UpdatesFilterChanged)
                .text_size(11)
                .width(Fill),
            pick_list(
                venues,
                Some(state.venue.clone()),
                Message::UpdatesVenueChanged
            )
            .text_size(11)
            .width(Fill),
        ]
        .spacing(6);
        let search = row![
            text_input("Asset…", &state.search)
                .on_input(Message::UpdatesSearchChanged)
                .size(11)
                .width(Fill),
            checkbox(state.favourites_only)
                .label("Favourites")
                .text_size(11)
                .on_toggle(Message::UpdatesFavouritesChanged),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let mut rows = column![].spacing(6).width(Fill);
        let mut count = 0;
        for event in &state.history.events {
            if self.symbol_key_is_hidden(&event.key)
                || !state.matches(event, self.favourite_symbols.contains(&event.key))
            {
                continue;
            }
            count += 1;
            rows = rows.push(self.view_parameter_event(event));
        }
        let body: Element<'_, Message> = if count == 0 {
            let label = if state.error.is_some() && state.history.events.is_empty() {
                "Waiting for market data"
            } else if state.history.parameters_observed_at_ms.is_none()
                || state.history.caps_observed_at_ms.is_none()
            {
                "Establishing baseline…"
            } else if state.history.events.is_empty() {
                "No parameter changes detected"
            } else {
                "No matching updates"
            };
            container(text(label).size(12).color(muted))
                .center(Fill)
                .into()
        } else {
            scrollable(rows).height(Fill).into()
        };
        let last_success = state
            .history
            .parameters_observed_at_ms
            .zip(state.history.caps_observed_at_ms)
            .map(|(parameters, caps)| parameters.min(caps));
        let status = if let Some(error) = state.error {
            error
        } else if state.loading {
            "Checking…"
        } else if state.last_attempt.is_none() || last_success.is_none() {
            "Connecting…"
        } else if last_success.is_some_and(|at| state.now_ms.saturating_sub(at) > UPDATES_STALE_MS)
        {
            "Feed delayed"
        } else {
            "Monitoring · 30s"
        };
        let mut content = column![
            header,
            controls,
            search,
            rule::horizontal(1),
            body,
            row![
                text(status)
                    .size(11)
                    .color(if state.error.is_some() {
                        palette.danger.base.color
                    } else {
                        muted
                    })
                    .width(Fill),
                tooltip(
                    text("Detected time").size(11).color(muted),
                    text("Changes are detected by polling. Numeric OI caps cover HIP-3 markets.")
                        .size(11),
                    tooltip::Position::Top
                )
            ]
        ]
        .spacing(8)
        .height(Fill);
        if state.storage_error {
            content = content.push(
                text("History storage unavailable · retrying")
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

    fn view_parameter_event<'a>(&'a self, event: &'a ParameterEvent) -> Element<'a, Message> {
        let state = &self.updates;
        let theme = self.theme();
        let muted = theme.extended_palette().background.weak.text;
        let expanded = state
            .expanded
            .contains(&(event.detected_at_ms, event.key.clone()));
        let available = self
            .exchange_symbols
            .iter()
            .any(|symbol| symbol.key == event.key);
        let age_ms = state.now_ms.saturating_sub(event.detected_at_ms);
        let age = match age_ms / 1000 {
            0..60 => "Just now".to_string(),
            60..3600 => format!("{}m ago", age_ms / 60_000),
            3600..86400 => format!("{}h ago", age_ms / 3_600_000),
            _ => format!("{}d ago", age_ms / 86_400_000),
        };
        let date = |ms| {
            chrono::DateTime::from_timestamp_millis(i64::try_from(ms).unwrap_or(i64::MAX))
                .map(|at| at.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_default()
        };
        let timing = format!(
            "Detected {}\nPrevious observation {}",
            date(event.detected_at_ms),
            date(event.previous_observed_at_ms)
        );
        let mut content = column![
            row![
                button(text(&event.key).size(13))
                    .padding(0)
                    .style(button::text)
                    .on_press_maybe(available.then(|| Message::SymbolSelected(event.key.clone()))),
                iced::widget::space().width(Fill),
                tooltip(
                    text(age).size(11).color(muted),
                    text(timing).size(11),
                    tooltip::Position::Top
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        ]
        .spacing(6);
        for change in event.changes.iter().filter(|change| state.includes(change)) {
            let label = match change {
                ParameterChange::OiCap { old, new } => format!(
                    "OI cap {}: {} → {}",
                    if new > old { "increased" } else { "decreased" },
                    crate::helpers::format_usd(old.as_str()),
                    crate::helpers::format_usd(new.as_str())
                ),
                ParameterChange::MaxLeverage { old, new } => {
                    format!("Maximum leverage: {old}× → {new}×")
                }
                ParameterChange::MarginMode { old, new } => {
                    format!("Margin mode: {} → {}", old.label(), new.label())
                }
                ParameterChange::MarginTiers { .. } => "Margin tiers changed".into(),
            };
            content = content.push(text(label).size(12));
            if expanded {
                match change {
                    ParameterChange::MarginTiers { old, new } => {
                        content = content.push(
                            text(format!(
                                "Before: {}\nAfter: {}",
                                format_tiers(old),
                                format_tiers(new)
                            ))
                            .size(11)
                            .color(muted),
                        );
                    }
                    ParameterChange::OiCap { old, new } => {
                        content = content.push(
                            text(format!("{} → {} USD notional", old.as_str(), new.as_str()))
                                .size(11)
                                .color(muted),
                        );
                    }
                    _ => {}
                }
            }
        }
        if expanded {
            content = content.push(
                text(format!(
                    "Detected {}\nPrevious observation {}",
                    date(event.detected_at_ms),
                    date(event.previous_observed_at_ms)
                ))
                .size(11)
                .color(muted),
            );
        }
        let detail = button(text(if expanded { "Less" } else { "Details" }).size(11))
            .padding(0)
            .style(button::text)
            .on_press(Message::UpdatesToggleDetails(
                event.detected_at_ms,
                event.key.clone(),
            ));
        let mut footer = row![detail].spacing(8).align_y(Alignment::Center);
        if event.after_gap {
            footer = footer.push(text("Observed after a gap").size(11).color(muted));
        }
        content = content.push(footer);
        container(content)
            .padding(10)
            .width(Fill)
            .style(container::rounded_box)
            .into()
    }
}

fn format_tiers(tiers: &[MarginTier]) -> String {
    tiers
        .iter()
        .map(|tier| {
            format!(
                "≥ {}: {}×",
                crate::helpers::format_usd(tier.lower_bound.as_str()),
                tier.max_leverage
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}
