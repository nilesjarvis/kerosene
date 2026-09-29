use crate::app_state::TradingTerminal;
use crate::helpers::format_decimal_with_commas;
use crate::hype_unstaking_state::{HypeUnstakingAmountFilter, HypeUnstakingWindowFilter};
use crate::message::Message;

use iced::widget::{Row, button, column, container, row, rule, text};
use iced::{Color, Element, Fill, Theme, color};
use std::borrow::Cow;

// ---------------------------------------------------------------------------
// Header and Filters
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_hype_unstaking_header(&self, compact: bool) -> Element<'_, Message> {
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

    pub(super) fn view_hype_unstaking_filters(
        &self,
        filtered_count: usize,
        has_data: bool,
    ) -> Element<'_, Message> {
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
        if has_data {
            let label = format!(
                "\u{25C9} {}",
                format_decimal_with_commas(filtered_count as f64, 0)
            );
            controls = push_hype_unstaking_filter_item(
                controls,
                hype_unstaking_filter_label(label),
                &mut has_controls,
            );
        }

        column![
            hype_unstaking_filter_strip(controls),
            hype_unstaking_filter_bottom_separator(),
        ]
        .spacing(0)
        .width(Fill)
        .into()
    }
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

fn hype_unstaking_filter_label(label: impl Into<Cow<'static, str>>) -> Element<'static, Message> {
    container(
        text(label.into())
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
