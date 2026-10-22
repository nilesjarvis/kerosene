use crate::app_state::TradingTerminal;
use crate::config::effective_radius;
use crate::config::updates::{MarginTier, ParameterChange, ParameterEvent, ParameterKind};
use crate::helpers;
use crate::helpers::design::{UiColors, compact_button_style, compact_input_style};
use crate::market_state::updates::{UPDATES_STALE_MS, UpdatesFilter, UpdatesVenue};
use crate::message::Message;
use iced::widget::text::LineHeight;
use iced::widget::{
    button, checkbox, column, container, pick_list, row, rule, scrollable, space, text, text_input,
    tooltip,
};
use iced::{Alignment, Border, Element, Fill, Theme};
use std::collections::BTreeSet;

/// Resolved status foreground for the footer line.
#[derive(Clone, Copy)]
enum StatusTone {
    Muted,
    Warning,
    Negative,
}

impl StatusTone {
    fn resolve(self, theme: &Theme) -> iced::Color {
        let colors = UiColors::new(theme);
        match self {
            StatusTone::Muted => colors.muted,
            StatusTone::Warning => colors.warning,
            StatusTone::Negative => colors.negative,
        }
    }
}

impl TradingTerminal {
    pub(crate) fn view_updates(&self) -> Element<'_, Message> {
        let state = &self.updates;
        let pane_corner_radius = self.pane_corner_radius;
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

        // Pane chrome already titles this pane and owns its close control;
        // the body toolbar carries only the pane-specific refresh action as
        // a compact ghost control.
        let toolbar = row![
            space().width(Fill),
            button(text("Refresh").size(12))
                .height(24)
                .padding([0, 8])
                .on_press_maybe((!state.loading).then_some(Message::RefreshUpdates))
                .style(move |theme: &Theme, status| {
                    compact_button_style(theme, status, false, pane_corner_radius)
                }),
        ]
        .align_y(Alignment::Center);

        // Filter and venue are one balanced picker pair at control size;
        // iced's default picker chrome already matches the field frame.
        let controls = row![
            pick_list(filters, Some(state.filter), Message::UpdatesFilterChanged)
                .text_size(12)
                .width(Fill),
            pick_list(
                venues,
                Some(state.venue.clone()),
                Message::UpdatesVenueChanged,
            )
            .text_size(12)
            .width(Fill),
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        let search = row![
            text_input("Asset…", &state.search)
                .size(12)
                .line_height(LineHeight::Absolute(iced::Pixels(16.0)))
                .padding([6, 8])
                .on_input(Message::UpdatesSearchChanged)
                .width(Fill)
                .style(move |theme, status| {
                    compact_input_style(theme, status, false, pane_corner_radius)
                }),
            checkbox(state.favourites_only)
                .label("Favourites")
                .text_size(12)
                .on_toggle(Message::UpdatesFavouritesChanged),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut rows = column![].spacing(8).width(Fill);
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
            container(
                text(label)
                    .size(12)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).muted),
                    }),
            )
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
        let (status, tone) = if let Some(error) = state.error {
            (error, StatusTone::Negative)
        } else if state.loading {
            ("Checking…", StatusTone::Muted)
        } else if state.last_attempt.is_none() || last_success.is_none() {
            ("Connecting…", StatusTone::Muted)
        } else if last_success.is_some_and(|at| state.now_ms.saturating_sub(at) > UPDATES_STALE_MS)
        {
            ("Feed delayed", StatusTone::Warning)
        } else {
            ("Monitoring · 30s", StatusTone::Muted)
        };

        let mut content = column![
            toolbar,
            controls,
            search,
            rule::horizontal(1),
            body,
            row![
                text(status)
                    .size(11)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(tone.resolve(theme)),
                    })
                    .width(Fill),
                tooltip(
                    text("Detected time")
                        .size(11)
                        .style(move |theme: &Theme| text::Style {
                            color: Some(UiColors::new(theme).muted),
                        }),
                    container(
                        text(
                            "Changes are detected by polling. Numeric OI caps cover HIP-3 markets."
                        )
                        .size(11),
                    )
                    .padding([6, 8])
                    .max_width(280)
                    .style(move |theme: &Theme| UiColors::new(theme).overlay(pane_corner_radius)),
                    tooltip::Position::Top,
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(8)
        .height(Fill);
        if state.storage_error {
            content = content.push(
                text("History storage unavailable · retrying")
                    .size(11)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).negative),
                    }),
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
        let colors = UiColors::new(&theme);
        let pane_corner_radius = self.pane_corner_radius;
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

        // Identity order: logo, symbol, then metadata. The key stays a
        // ghost navigation action into the chart.
        let mut key_row = row![].spacing(4).align_y(Alignment::Center);
        if let Some(icon) = helpers::symbol_icon(&event.key, 14, colors.text) {
            key_row = key_row.push(icon);
        }
        let key_button = button(text(event.key.as_str()).size(12))
            .padding(0)
            .on_press_maybe(available.then(|| Message::SymbolSelected(event.key.clone())))
            .style(move |theme: &Theme, status| {
                compact_button_style(theme, status, false, pane_corner_radius)
            });
        key_row = key_row.push(key_button);

        let mut content = column![
            row![
                key_row,
                space().width(Fill),
                tooltip(
                    text(age).size(11).style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).muted),
                    }),
                    container(text(timing).size(11))
                        .padding([6, 8])
                        .max_width(280)
                        .style(
                            move |theme: &Theme| UiColors::new(theme).overlay(pane_corner_radius)
                        ),
                    tooltip::Position::Top,
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
            content = content.push(
                text(label)
                    .size(12)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).text),
                    }),
            );
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
                            .style(move |theme: &Theme| text::Style {
                                color: Some(UiColors::new(theme).muted),
                            }),
                        );
                    }
                    ParameterChange::OiCap { old, new } => {
                        content = content.push(
                            text(format!("{} → {} USD notional", old.as_str(), new.as_str()))
                                .size(11)
                                .style(move |theme: &Theme| text::Style {
                                    color: Some(UiColors::new(theme).muted),
                                }),
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
                .style(move |theme: &Theme| text::Style {
                    color: Some(UiColors::new(theme).muted),
                }),
            );
        }
        let detail = button(text(if expanded { "Less" } else { "Details" }).size(12))
            .height(24)
            .padding([0, 8])
            .on_press(Message::UpdatesToggleDetails(
                event.detected_at_ms,
                event.key.clone(),
            ))
            .style(move |theme: &Theme, status| {
                compact_button_style(theme, status, false, pane_corner_radius)
            });
        let mut footer = row![detail].spacing(8).align_y(Alignment::Center);
        if event.after_gap {
            // Data-quality limitation next to the affected event.
            footer = footer.push(text("Observed after a gap").size(11).style(
                move |theme: &Theme| text::Style {
                    color: Some(UiColors::new(theme).warning),
                },
            ));
        }
        content = content.push(footer);
        container(content)
            .padding(8)
            .width(Fill)
            .style(move |theme: &Theme| container::Style {
                background: Some(UiColors::new(theme).inset.into()),
                border: Border {
                    radius: effective_radius(pane_corner_radius, 6.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }
}

fn format_tiers(tiers: &[MarginTier]) -> String {
    tiers
        .iter()
        .map(|tier| {
            format!(
                "{}× up to {}",
                tier.max_leverage,
                crate::helpers::format_usd(tier.lower_bound.as_str())
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType};
    use crate::config::KeroseneConfig;
    use crate::config::updates::ParameterDecimal;
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
    use iced::{Event, Font, Pixels, Point, Rectangle, Size};

    fn symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "crypto".to_string(),
            display_name: Some(key.to_string()),
            keywords: Vec::new(),
            asset_index: 1,
            collateral_token: None,
            sz_decimals: 2,
            max_leverage: 10,
            only_isolated: false,
            growth_mode: false,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    fn decimal(value: &str) -> ParameterDecimal {
        ParameterDecimal::try_from(value.to_string()).expect("decimal")
    }

    fn event(key: &str, detected_at_ms: u64, changes: Vec<ParameterChange>) -> ParameterEvent {
        ParameterEvent {
            key: key.to_string(),
            detected_at_ms,
            previous_observed_at_ms: detected_at_ms.saturating_sub(30_000),
            after_gap: false,
            changes,
        }
    }

    fn render(
        view: &mut Element<'_, Message>,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        size: Size,
        name: &str,
    ) -> (Tree, layout::Node) {
        let bounds = Rectangle::with_size(size);
        let mut tree = Tree::new(view.as_widget());
        let node =
            view.as_widget_mut()
                .layout(&mut tree, renderer, &layout::Limits::new(size, size));
        iced::advanced::Renderer::reset(renderer, bounds);
        view.as_widget().draw(
            &tree,
            renderer,
            theme,
            &renderer::Style {
                text_color: theme.palette().text,
            },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &bounds,
        );
        let pixels = renderer.screenshot(
            Size::new(size.width as u32, size.height as u32),
            1.0,
            theme.palette().background,
        );
        assert_eq!(
            pixels.len(),
            size.width as usize * size.height as usize * 4,
            "{name}"
        );
        assert!(
            pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]),
            "{name}"
        );
        (tree, node)
    }

    fn click(
        view: &mut Element<'_, Message>,
        tree: &mut Tree,
        node: &layout::Node,
        renderer: &mut iced::Renderer,
        size: Size,
        point: Point,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            view.as_widget_mut().update(
                tree,
                &Event::Mouse(event),
                Layout::new(node),
                mouse::Cursor::Available(point),
                renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &Rectangle::with_size(size),
            );
        }
        messages
    }

    #[tokio::test]
    async fn updates_view_preserves_actions_across_refresh_and_state_sweep() {
        let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
            .await
            .expect("software renderer");
        let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.exchange_symbols = vec![symbol("BTC"), symbol("UBTC")];
        let theme = terminal.theme();
        let size = Size::new(400.0, 400.0);

        let populate = |terminal: &mut TradingTerminal| {
            terminal.updates.now_ms = 90_000;
            terminal.updates.history.parameters_observed_at_ms = Some(30_000);
            terminal.updates.history.caps_observed_at_ms = Some(30_000);
            terminal.updates.history.events = vec![
                event(
                    "BTC",
                    30_000,
                    vec![ParameterChange::MaxLeverage { old: 10, new: 20 }],
                ),
                event(
                    "UBTC",
                    30_000,
                    vec![ParameterChange::OiCap {
                        old: decimal("1000"),
                        new: decimal("2000"),
                    }],
                ),
            ];
        };

        // Idle and populated: refresh, details toggles, and symbol navigation
        // all remain reachable through the restyled controls.
        populate(&mut terminal);
        let mut view = terminal.view_updates();
        let (mut tree, node) = render(&mut view, &mut renderer, &theme, size, "updates-populated");
        let mut saw_refresh = false;
        let mut saw_details = false;
        let mut saw_symbol = false;
        for y in (10..390).step_by(10) {
            for x in (10..390).step_by(10) {
                for message in click(
                    &mut view,
                    &mut tree,
                    &node,
                    &mut renderer,
                    size,
                    Point::new(x as f32, y as f32),
                ) {
                    match message {
                        Message::RefreshUpdates => saw_refresh = true,
                        Message::UpdatesToggleDetails(..) => saw_details = true,
                        Message::SymbolSelected(_) => saw_symbol = true,
                        _ => {}
                    }
                }
            }
        }
        assert!(saw_refresh, "refresh action reachable while idle");
        assert!(saw_details, "details toggle reachable");
        assert!(saw_symbol, "symbol navigation reachable");
        drop(view);

        // Loading: the refresh action is unavailable without moving geometry.
        terminal.updates.loading = true;
        let mut view = terminal.view_updates();
        let (mut tree, node) = render(&mut view, &mut renderer, &theme, size, "updates-loading");
        let mut refresh_while_loading = false;
        for y in (10..390).step_by(10) {
            for x in (10..390).step_by(10) {
                for message in click(
                    &mut view,
                    &mut tree,
                    &node,
                    &mut renderer,
                    size,
                    Point::new(x as f32, y as f32),
                ) {
                    if matches!(message, Message::RefreshUpdates) {
                        refresh_while_loading = true;
                    }
                }
            }
        }
        assert!(!refresh_while_loading, "refresh disabled while loading");
        drop(view);
        terminal.updates.loading = false;

        // Distinct domain states all render through the shared recipes.
        terminal.updates.error = Some("Updates source unavailable");
        let mut view = terminal.view_updates();
        let _ = render(&mut view, &mut renderer, &theme, size, "updates-error");
        drop(view);
        terminal.updates.error = None;

        terminal.updates.now_ms = 500_000;
        let mut view = terminal.view_updates();
        let _ = render(&mut view, &mut renderer, &theme, size, "updates-stale");
        drop(view);

        terminal.updates.storage_error = true;
        let mut view = terminal.view_updates();
        let _ = render(
            &mut view,
            &mut renderer,
            &theme,
            size,
            "updates-storage-error",
        );
        drop(view);
        terminal.updates.storage_error = false;

        terminal
            .updates
            .expanded
            .insert((30_000, "BTC".to_string()));
        let mut view = terminal.view_updates();
        let _ = render(&mut view, &mut renderer, &theme, size, "updates-expanded");
        drop(view);

        terminal.updates.history.events.clear();
        terminal.updates.history.parameters_observed_at_ms = None;
        terminal.updates.history.caps_observed_at_ms = None;
        let mut view = terminal.view_updates();
        let _ = render(&mut view, &mut renderer, &theme, size, "updates-empty");
    }
}
