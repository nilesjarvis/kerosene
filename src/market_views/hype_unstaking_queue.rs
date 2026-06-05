use crate::app_state::TradingTerminal;
use crate::config::SortDirection;
use crate::denomination::{DISPLAY_DENOMINATION_RATE_STALE_MS, DisplayDenominationContext};
use crate::helpers::format_decimal_with_commas;
use crate::hype_unstaking_state::{
    HYPE_CORE_WEI_PER_TOKEN, HypeUnstakingAmountFilter, HypeUnstakingEvent, HypeUnstakingFilter,
    HypeUnstakingQueueState, HypeUnstakingSortField, HypeUnstakingWindowFilter, format_countdown,
    format_hype_wei, sort_unstaking_events, summarize_unstaking_events,
};
use crate::message::Message;
use crate::wallet_state::address_book::WalletDisplay;
use crate::wallet_views::{WalletAddressActionCell, wallet_address_action_cell};

use iced::alignment::Horizontal;
use iced::widget::text;
use iced::widget::{
    Column, Row, Space, button, column, container, responsive, row, rule, scrollable,
};
use iced::{Color, Element, Fill, Theme, color};

const HYPE_UNSTAKING_ROW_LIMIT: usize = 250;
const HYPE_UNSTAKING_WALLET_ACTION_WIDTH: f32 = 150.0;

#[derive(Clone, Copy)]
struct HypeUnstakingRowContext<'a> {
    now_ms: u64,
    compact: bool,
    denomination: &'a DisplayDenominationContext,
    hype_mid: Option<f64>,
    amount_scale: HypeUnstakingAmountScale,
    theme: &'a Theme,
}

#[derive(Debug, Clone, Copy)]
struct HypeUnstakingAmountScale {
    min_ln: f64,
    max_ln: f64,
}

#[derive(Debug, Clone, Copy)]
struct HypeUnstakingAmountHeat {
    fill_pct: f32,
    alpha: f32,
}

// ---------------------------------------------------------------------------
// HYPE Unstaking Queue View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_hype_unstaking_queue(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_hype_unstaking_queue_sized(size.width)).into()
    }

    fn view_hype_unstaking_queue_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let compact = available_width < 680.0;
        let now_ms = Self::now_ms();
        let denomination = self.display_denomination_context();
        let hype_mid = self.hype_unstaking_notional_mid(now_ms);

        let status_text = hype_unstaking_status_text(&self.hype_unstaking_queue);
        let status_color = if self.hype_unstaking_queue.error.is_some() {
            theme.palette().danger
        } else {
            theme.extended_palette().background.weak.text
        };

        let mut content = column![
            self.view_hype_unstaking_header(compact),
            self.view_hype_unstaking_filters(),
            text(status_text).size(10).color(status_color).width(Fill),
            rule::horizontal(1),
        ]
        .spacing(8)
        .width(Fill);

        if let Some(data) = &self.hype_unstaking_queue.data {
            let mine_address = if self.hype_unstaking_queue.mine_only {
                self.connected_address.as_deref()
            } else {
                None
            };
            let mut filtered = data.filtered_events(HypeUnstakingFilter {
                now_ms,
                window: self.hype_unstaking_queue.window_filter,
                amount: self.hype_unstaking_queue.amount_filter,
                mine_address,
            });
            sort_unstaking_events(
                filtered.as_mut_slice(),
                self.hype_unstaking_queue.sort_field,
                self.hype_unstaking_queue.sort_direction,
            );
            let summary = summarize_unstaking_events(&filtered);
            content = content.push(hype_unstaking_summary_grid(
                &summary,
                now_ms,
                available_width,
                &theme,
            ));
            content = content.push(self.view_hype_unstaking_event_list(
                &filtered,
                now_ms,
                compact,
                &denomination,
                hype_mid,
                &theme,
            ));
        } else if self.hype_unstaking_queue.loading {
            content = content.push(
                row![
                    self.view_spinner(18),
                    text("Loading unstaking queue")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            );
        } else {
            content = content.push(
                text("No unstaking queue data loaded")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn hype_unstaking_notional_mid(&self, now_ms: u64) -> Option<f64> {
        let updated_at_ms = self.all_mids_updated_at_ms.get("HYPE").copied()?;
        if now_ms.saturating_sub(updated_at_ms) > DISPLAY_DENOMINATION_RATE_STALE_MS {
            return None;
        }

        self.all_mids
            .get("HYPE")
            .copied()
            .filter(|mid| mid.is_finite() && *mid > 0.0)
    }

    fn view_hype_unstaking_header(&self, compact: bool) -> Element<'_, Message> {
        let theme = self.theme();
        let refresh_label = if compact { "Refresh" } else { "Refresh Queue" };
        row![
            text("HYPE Unstaking Queue")
                .size(13)
                .color(theme.palette().text)
                .width(Fill),
            button(text(refresh_label).size(11).center())
                .padding([3, 8])
                .on_press(Message::RefreshHypeUnstakingQueue)
                .style(button::text),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn view_hype_unstaking_filters(&self) -> Element<'_, Message> {
        let connected_address = self.connected_address.as_deref();
        let mine_enabled = connected_address.is_some();
        let mine_active = self.hype_unstaking_queue.mine_only && mine_enabled;

        let mut controls: Row<'_, Message> = row![].spacing(0).align_y(iced::Alignment::Center);
        let mut has_controls = false;

        controls = push_hype_unstaking_filter_item(
            controls,
            hype_unstaking_filter_label("Unlock"),
            &mut has_controls,
        );
        for filter in HypeUnstakingWindowFilter::ALL {
            controls = push_hype_unstaking_filter_item(
                controls,
                filter_button(
                    filter.label(),
                    self.hype_unstaking_queue.window_filter == filter,
                    Message::HypeUnstakingWindowChanged(filter),
                ),
                &mut has_controls,
            );
        }

        controls = push_hype_unstaking_filter_item(
            controls,
            hype_unstaking_filter_label("Min HYPE"),
            &mut has_controls,
        );
        for filter in HypeUnstakingAmountFilter::ALL {
            controls = push_hype_unstaking_filter_item(
                controls,
                filter_button(
                    filter.label(),
                    self.hype_unstaking_queue.amount_filter == filter,
                    Message::HypeUnstakingAmountFilterChanged(filter),
                ),
                &mut has_controls,
            );
        }

        controls = push_hype_unstaking_filter_item(
            controls,
            optional_filter_button(
                "Mine",
                mine_active,
                mine_enabled,
                Message::ToggleHypeUnstakingMineOnly,
            ),
            &mut has_controls,
        );
        controls = push_hype_unstaking_filter_item(
            controls,
            filter_button("Clear", false, Message::ClearHypeUnstakingFilters),
            &mut has_controls,
        );

        column![
            hype_unstaking_filter_strip(controls),
            hype_unstaking_filter_bottom_separator(),
        ]
        .spacing(0)
        .width(Fill)
        .into()
    }

    fn view_hype_unstaking_event_list<'a>(
        &'a self,
        events: &[&'a HypeUnstakingEvent],
        now_ms: u64,
        compact: bool,
        denomination: &DisplayDenominationContext,
        hype_mid: Option<f64>,
        theme: &Theme,
    ) -> Element<'a, Message> {
        if events.is_empty() {
            return container(
                text("No upcoming unstaking events for the selected filters")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .padding([8, 0])
            .into();
        }

        let mut rows = Column::new().spacing(3).width(Fill);
        if !compact {
            rows = rows.push(hype_unstaking_table_header(
                theme,
                self.hype_unstaking_queue.sort_field,
                self.hype_unstaking_queue.sort_direction,
            ));
        }

        let row_context = HypeUnstakingRowContext {
            now_ms,
            compact,
            denomination,
            hype_mid,
            amount_scale: hype_unstaking_amount_scale(events),
            theme,
        };
        for (index, event) in events.iter().take(HYPE_UNSTAKING_ROW_LIMIT).enumerate() {
            rows = rows.push(self.hype_unstaking_event_row(event, index, row_context));
        }

        if events.len() > HYPE_UNSTAKING_ROW_LIMIT {
            rows = rows.push(
                text(format!(
                    "Showing {} of {}",
                    HYPE_UNSTAKING_ROW_LIMIT,
                    format_decimal_with_commas(events.len() as f64, 0)
                ))
                .size(10)
                .color(theme.extended_palette().background.weak.text),
            );
        }

        scrollable(rows)
            .id(iced::widget::Id::new("hype_unstaking_queue_scroll"))
            .direction(hype_unstaking_scroll_direction())
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn hype_unstaking_event_row<'a>(
        &self,
        event: &'a HypeUnstakingEvent,
        index: usize,
        context: HypeUnstakingRowContext<'_>,
    ) -> Element<'a, Message> {
        let theme = context.theme;
        let secondary = theme.extended_palette().background.weak.text;
        let text_color = theme.palette().text;
        let countdown = format_countdown(event.unlock_time_ms, context.now_ms);
        let unlock_time = format_local_time_ms(event.unlock_time_ms);
        let amount = format_hype_amount_with_notional(
            event.amount_wei,
            context.hype_mid,
            context.denomination,
        );
        let wallet_cell = hype_unstaking_wallet_cell(
            event.user.clone(),
            self.wallet_display(&event.user),
            self.hovered_wallet_address_actions.as_deref(),
            theme,
        );

        let content: Element<'a, Message> = if context.compact {
            column![
                row![
                    text(countdown)
                        .font(crate::app_fonts::monospace_font())
                        .size(12)
                        .color(theme.palette().primary),
                    Space::new().width(Fill),
                    text(amount)
                        .font(crate::app_fonts::monospace_font())
                        .size(12)
                        .color(text_color),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
                row![
                    wallet_cell,
                    Space::new().width(Fill),
                    text(unlock_time).size(11).color(secondary),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(3)
            .into()
        } else {
            row![
                text(countdown)
                    .font(crate::app_fonts::monospace_font())
                    .size(12)
                    .color(theme.palette().primary)
                    .width(88),
                text(unlock_time)
                    .font(crate::app_fonts::monospace_font())
                    .size(11)
                    .color(secondary)
                    .width(132),
                container(wallet_cell).width(Fill),
                text(amount)
                    .font(crate::app_fonts::monospace_font())
                    .size(11)
                    .color(text_color)
                    .width(220),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
        };

        container(content)
            .width(Fill)
            .padding([5, 6])
            .style(move |theme: &Theme| {
                hype_unstaking_row_style(theme, event.amount_wei, context.amount_scale, index)
            })
            .into()
    }
}

fn hype_unstaking_scroll_direction() -> iced::widget::scrollable::Direction {
    iced::widget::scrollable::Direction::Vertical(
        iced::widget::scrollable::Scrollbar::new()
            .width(4)
            .margin(0)
            .scroller_width(4)
            .spacing(8),
    )
}

fn hype_unstaking_status_text(state: &HypeUnstakingQueueState) -> String {
    if state.loading && state.data.is_none() {
        "Loading queue...".to_string()
    } else if state.loading {
        "Refreshing...".to_string()
    } else if let Some(error) = &state.error {
        if state.data.is_none() {
            format!("Load failed: {error}")
        } else {
            format!("Showing last good data; refresh failed: {error}")
        }
    } else if let Some(last_fetch) = state.last_fetch {
        let age = last_fetch.elapsed().as_secs();
        if age < 60 {
            "Updated just now".to_string()
        } else {
            format!("Updated {}m ago", age / 60)
        }
    } else {
        "Not loaded".to_string()
    }
}

fn hype_unstaking_summary_grid(
    summary: &crate::hype_unstaking_state::HypeUnstakingSummary,
    now_ms: u64,
    available_width: f32,
    theme: &Theme,
) -> Element<'static, Message> {
    let next_unlock = summary
        .next_unlock_time_ms
        .map(|unlock_time_ms| format_countdown(unlock_time_ms, now_ms))
        .unwrap_or_else(|| "-".to_string());
    let largest_unlock = summary
        .largest_amount_wei
        .map(|amount| format_hype_wei(amount as u128))
        .unwrap_or_else(|| "-".to_string());

    let metrics = vec![
        metric(
            "Events",
            format_decimal_with_commas(summary.event_count as f64, 0),
        ),
        metric(
            "Wallets",
            format_decimal_with_commas(summary.unique_wallet_count as f64, 0),
        ),
        metric("Total", format_hype_wei(summary.total_wei)),
        metric("Next", next_unlock),
        metric("Largest", largest_unlock),
    ];

    metric_grid(metrics, available_width, theme)
}

#[derive(Debug, Clone)]
struct Metric {
    label: &'static str,
    value: String,
}

fn metric(label: &'static str, value: String) -> Metric {
    Metric { label, value }
}

fn metric_grid(
    metrics: Vec<Metric>,
    available_width: f32,
    theme: &Theme,
) -> Element<'static, Message> {
    let columns = if available_width >= 680.0 {
        5
    } else if available_width >= 480.0 {
        3
    } else if available_width >= 320.0 {
        2
    } else {
        1
    };

    let mut grid = Column::new().spacing(6);
    for chunk in metrics.chunks(columns) {
        let mut line = row![].spacing(6).width(Fill);
        for item in chunk {
            line = line.push(metric_card(item.clone(), theme));
        }
        grid = grid.push(line);
    }
    grid.into()
}

fn metric_card(metric: Metric, theme: &Theme) -> Element<'static, Message> {
    let text_color = theme.palette().text;
    let border_color = theme.extended_palette().background.strong.color;
    let background = theme.extended_palette().background.base.color;

    container(
        column![
            text(metric.label)
                .size(10)
                .color(color!(0x888888))
                .width(Fill),
            text(metric.value)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(text_color)
                .width(Fill),
        ]
        .spacing(2),
    )
    .width(Fill)
    .padding([6, 8])
    .style(move |_theme: &Theme| container::Style {
        background: Some(background.into()),
        border: iced::Border {
            radius: 3.0.into(),
            width: 1.0,
            color: border_color,
        },
        ..Default::default()
    })
    .into()
}

fn hype_unstaking_table_header(
    theme: &Theme,
    sort_field: HypeUnstakingSortField,
    sort_direction: SortDirection,
) -> Element<'static, Message> {
    let color = theme.extended_palette().background.weak.text;
    row![
        hype_unstaking_sort_header_cell(
            "ETA",
            HypeUnstakingSortField::UnlockTime,
            sort_field,
            sort_direction,
            88.0,
            color,
            Horizontal::Left,
        ),
        hype_unstaking_sort_header_cell(
            "Unlock",
            HypeUnstakingSortField::UnlockTime,
            sort_field,
            sort_direction,
            132.0,
            color,
            Horizontal::Left,
        ),
        text("Address").size(10).color(color).width(Fill),
        hype_unstaking_sort_header_cell(
            "Amount (Notional)",
            HypeUnstakingSortField::Amount,
            sort_field,
            sort_direction,
            220.0,
            color,
            Horizontal::Right,
        ),
    ]
    .spacing(8)
    .padding([0, 6])
    .into()
}

fn hype_unstaking_sort_header_cell(
    label: &'static str,
    field: HypeUnstakingSortField,
    sort_field: HypeUnstakingSortField,
    sort_direction: SortDirection,
    width: f32,
    color: Color,
    alignment: Horizontal,
) -> Element<'static, Message> {
    let is_active = sort_field == field;
    let mut content = Row::new().spacing(2).align_y(iced::Alignment::Center).push(
        text(label)
            .size(10)
            .color(color)
            .width(Fill)
            .align_x(alignment),
    );

    if is_active {
        let icon = if sort_direction == SortDirection::Ascending {
            "\u{2191}"
        } else {
            "\u{2193}"
        };
        content = content.push(text(icon).size(10).color(color));
    }

    button(content)
        .on_press(Message::HypeUnstakingSortChanged(field))
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            ..Default::default()
        })
        .padding(0)
        .width(width)
        .into()
}

fn hype_unstaking_row_style(
    theme: &Theme,
    amount_wei: u64,
    scale: HypeUnstakingAmountScale,
    index: usize,
) -> container::Style {
    use iced::gradient;

    let heat = scale.heat(amount_wei);
    let mut start = theme.palette().primary;
    start.a = heat.alpha;
    let mut end = theme.palette().primary;
    end.a = heat.alpha * 0.55;
    let base = hype_unstaking_row_base_background(theme, index);

    let fill = heat.fill_pct.clamp(0.0, 1.0);
    let background = if fill >= 0.999 {
        gradient::Linear::new(iced::Degrees(90.0))
            .add_stop(0.0, start)
            .add_stop(1.0, end)
    } else {
        gradient::Linear::new(iced::Degrees(90.0))
            .add_stop(0.0, start)
            .add_stop(fill, end)
            .add_stop((fill + 0.0001).min(1.0), base)
            .add_stop(1.0, base)
    };

    container::Style {
        background: Some(background.into()),
        ..Default::default()
    }
}

fn hype_unstaking_row_base_background(theme: &Theme, index: usize) -> Color {
    if index.is_multiple_of(2) {
        Color {
            a: 0.12,
            ..theme.extended_palette().background.strong.color
        }
    } else {
        Color {
            a: 0.02,
            ..theme.extended_palette().background.weak.color
        }
    }
}

fn hype_unstaking_amount_scale(events: &[&HypeUnstakingEvent]) -> HypeUnstakingAmountScale {
    let (min_ln, max_ln) = events
        .iter()
        .map(|event| hype_amount_ln(event.amount_wei))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
            (min.min(value), max.max(value))
        });

    if !min_ln.is_finite() || !max_ln.is_finite() {
        return HypeUnstakingAmountScale {
            min_ln: 0.0,
            max_ln: 0.0,
        };
    }

    HypeUnstakingAmountScale { min_ln, max_ln }
}

impl HypeUnstakingAmountScale {
    fn heat(self, amount_wei: u64) -> HypeUnstakingAmountHeat {
        let amount_ln = hype_amount_ln(amount_wei);
        let span = self.max_ln - self.min_ln;
        let heat = if span.is_finite() && span > f64::EPSILON {
            ((amount_ln - self.min_ln) / span).clamp(0.0, 1.0)
        } else {
            hype_amount_absolute_heat(amount_wei)
        } as f32;

        HypeUnstakingAmountHeat {
            fill_pct: 0.10 + heat * 0.90,
            alpha: 0.035 + heat * 0.245,
        }
    }
}

fn hype_amount_ln(amount_wei: u64) -> f64 {
    let amount_hype = amount_wei as f64 / HYPE_CORE_WEI_PER_TOKEN as f64;
    amount_hype.max(0.0).ln_1p()
}

fn hype_amount_absolute_heat(amount_wei: u64) -> f64 {
    let amount_hype = amount_wei as f64 / HYPE_CORE_WEI_PER_TOKEN as f64;
    let max_reference = 100_000.0_f64.ln_1p();
    (amount_hype.max(0.0).ln_1p() / max_reference).clamp(0.0, 1.0)
}

fn format_hype_amount_with_notional(
    amount_wei: u64,
    hype_mid: Option<f64>,
    denomination: &DisplayDenominationContext,
) -> String {
    let amount = format_hype_wei(amount_wei as u128);
    let notional = hype_mid
        .and_then(|mid| {
            let usd_value = amount_wei as f64 / HYPE_CORE_WEI_PER_TOKEN as f64 * mid;
            usd_value
                .is_finite()
                .then(|| denomination.format_value(usd_value, 2))
        })
        .unwrap_or_else(|| "n/a".to_string());

    format!("{amount} ({notional})")
}

fn hype_unstaking_wallet_cell(
    address: String,
    display: WalletDisplay,
    hovered_wallet_action_key: Option<&str>,
    theme: &Theme,
) -> Element<'static, Message> {
    let label = hype_unstaking_wallet_label(&display, 26);
    let tooltip_label = hype_unstaking_wallet_tooltip(&display, &address);

    wallet_address_action_cell(WalletAddressActionCell {
        address: address.clone(),
        label,
        tooltip_label,
        hover_key: format!("hype-unstaking:{address}"),
        hovered_key: hovered_wallet_action_key,
        width: HYPE_UNSTAKING_WALLET_ACTION_WIDTH,
        text_size: 11,
        text_color: theme.palette().text,
    })
}

fn hype_unstaking_wallet_label(display: &WalletDisplay, max_chars: usize) -> String {
    truncate_ascii(&display.primary, max_chars)
}

fn hype_unstaking_wallet_tooltip(display: &WalletDisplay, address: &str) -> String {
    if display.has_label {
        format!("{} ({address})", display.primary)
    } else {
        format!("Copy {address}")
    }
}

fn truncate_ascii(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut truncated: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    truncated.push_str("...");
    truncated
}

fn push_hype_unstaking_filter_item<'a>(
    mut toolbar: Row<'a, Message>,
    item: Element<'a, Message>,
    has_items: &mut bool,
) -> Row<'a, Message> {
    if *has_items {
        toolbar = toolbar.push(hype_unstaking_filter_separator());
    }
    *has_items = true;
    toolbar.push(item)
}

fn hype_unstaking_filter_strip<'a>(content: Row<'a, Message>) -> Element<'a, Message> {
    container(content.width(Fill).wrap().vertical_spacing(0))
        .width(Fill)
        .style(|theme: &Theme| {
            let background = Color {
                a: 0.04,
                ..theme.extended_palette().background.weak.color
            };
            container::Style {
                background: Some(background.into()),
                ..Default::default()
            }
        })
        .into()
}

fn hype_unstaking_filter_separator() -> Element<'static, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.12,
            ..theme.extended_palette().background.weak.text
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .height(14)
    .width(1)
    .into()
}

fn hype_unstaking_filter_bottom_separator() -> Element<'static, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.12,
                ..theme.extended_palette().background.weak.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}

fn hype_unstaking_filter_label(label: &'static str) -> Element<'static, Message> {
    container(
        text(label)
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(color!(0x888888))
            .center(),
    )
    .padding([3, 8])
    .into()
}

fn filter_button(label: &'static str, active: bool, msg: Message) -> Element<'static, Message> {
    optional_filter_button(label, active, true, msg)
}

fn optional_filter_button(
    label: &'static str,
    active: bool,
    enabled: bool,
    msg: Message,
) -> Element<'static, Message> {
    let button = button(text(label).size(11).center()).padding([3, 8]).style(
        move |theme: &Theme, status| {
            let bg = hype_unstaking_filter_button_background(theme, status, active, enabled);
            button::Style {
                background: Some(bg.into()),
                text_color: if !enabled {
                    Color {
                        a: 0.45,
                        ..theme.extended_palette().background.weak.text
                    }
                } else if active {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 0.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        },
    );

    if enabled {
        button.on_press(msg).into()
    } else {
        button.into()
    }
}

fn hype_unstaking_filter_button_background(
    theme: &Theme,
    status: button::Status,
    active: bool,
    enabled: bool,
) -> Color {
    if active {
        return Color {
            a: 0.10,
            ..theme.palette().primary
        };
    }

    match status {
        button::Status::Hovered if enabled => Color {
            a: 0.55,
            ..theme.extended_palette().background.strong.color
        },
        _ => Color::TRANSPARENT,
    }
}

fn format_local_time_ms(time_ms: u64) -> String {
    let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(time_ms as i64) else {
        return "-".to_string();
    };
    dt.with_timezone(&chrono::Local)
        .format("%m/%d %H:%M:%S")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_hype_amount_with_selected_denomination_notional() {
        let denomination = DisplayDenominationContext::usd();

        assert_eq!(
            format_hype_amount_with_notional(150_000_000, Some(40.0), &denomination),
            "1.5 HYPE ($60.00)"
        );
    }

    #[test]
    fn formats_hype_amount_with_missing_notional_price() {
        let denomination = DisplayDenominationContext::usd();

        assert_eq!(
            format_hype_amount_with_notional(150_000_000, None, &denomination),
            "1.5 HYPE (n/a)"
        );
    }

    #[test]
    fn wallet_identity_uses_label_and_tooltip_keeps_address() {
        let display = WalletDisplay {
            primary: "Market Maker Alpha".to_string(),
            secondary: "0x1234...abcd".to_string(),
            has_label: true,
        };

        assert_eq!(
            hype_unstaking_wallet_label(&display, 26),
            "Market Maker Alpha"
        );
        assert_eq!(
            hype_unstaking_wallet_tooltip(&display, "0x1234567890abcdef1234567890abcdef12345678"),
            "Market Maker Alpha (0x1234567890abcdef1234567890abcdef12345678)"
        );
    }

    #[test]
    fn amount_scale_makes_large_unlocks_more_prominent() {
        let small = HypeUnstakingEvent {
            unlock_time_ms: 1_000,
            user: "0xsmall".to_string(),
            amount_wei: 100 * HYPE_CORE_WEI_PER_TOKEN as u64,
        };
        let large = HypeUnstakingEvent {
            unlock_time_ms: 2_000,
            user: "0xlarge".to_string(),
            amount_wei: 10_000 * HYPE_CORE_WEI_PER_TOKEN as u64,
        };
        let scale = hype_unstaking_amount_scale(&[&small, &large]);
        let small_heat = scale.heat(small.amount_wei);
        let large_heat = scale.heat(large.amount_wei);

        assert!(large_heat.fill_pct > small_heat.fill_pct);
        assert!(large_heat.alpha > small_heat.alpha);
    }

    #[test]
    fn amount_scale_uses_rows_beyond_render_limit() {
        let mut events: Vec<_> = (0..HYPE_UNSTAKING_ROW_LIMIT)
            .map(|index| HypeUnstakingEvent {
                unlock_time_ms: index as u64,
                user: format!("0xsmall{index}"),
                amount_wei: HYPE_CORE_WEI_PER_TOKEN as u64,
            })
            .collect();
        events.push(HypeUnstakingEvent {
            unlock_time_ms: HYPE_UNSTAKING_ROW_LIMIT as u64,
            user: "0xlarge".to_string(),
            amount_wei: 100_000 * HYPE_CORE_WEI_PER_TOKEN as u64,
        });
        let refs: Vec<_> = events.iter().collect();

        let scale = hype_unstaking_amount_scale(&refs);
        let small_heat = scale.heat(events[0].amount_wei);
        let large_heat = scale.heat(events[HYPE_UNSTAKING_ROW_LIMIT].amount_wei);

        assert!(large_heat.fill_pct > small_heat.fill_pct);
        assert!(large_heat.alpha > small_heat.alpha);
    }
}
