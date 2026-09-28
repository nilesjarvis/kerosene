use crate::app_state::TradingTerminal;
use crate::config::SortDirection;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::format_decimal_with_commas;
use crate::hype_unstaking_state::{HypeUnstakingEvent, HypeUnstakingSortField, format_countdown};
use crate::message::Message;
use crate::wallet_state::address_book::WalletDisplay;
use crate::wallet_views::{WalletAddressActionCell, wallet_address_action_cell};

use iced::alignment::Horizontal;
use iced::widget::{Column, Row, Space, button, column, container, row, scrollable, text};
use iced::{Color, Element, Fill, Theme};

mod formatting;
mod heat;

use formatting::{
    format_hype_amount_with_notional, format_local_time_ms, hype_unstaking_wallet_label,
    hype_unstaking_wallet_tooltip,
};
use heat::{HypeUnstakingAmountScale, hype_unstaking_amount_scale, hype_unstaking_row_style};

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

// ---------------------------------------------------------------------------
// Event Rows
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_hype_unstaking_event_list<'a>(
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
            &event.user,
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

fn hype_unstaking_wallet_cell(
    address: &str,
    display: WalletDisplay,
    hovered_wallet_action_key: Option<&str>,
    theme: &Theme,
) -> Element<'static, Message> {
    let label = hype_unstaking_wallet_label(&display);
    let tooltip_label = hype_unstaking_wallet_tooltip(&display, address);

    wallet_address_action_cell(WalletAddressActionCell {
        address: address.to_string(),
        label,
        tooltip_label,
        hover_key: format!("hype-unstaking:{address}"),
        hovered_key: hovered_wallet_action_key,
        width: HYPE_UNSTAKING_WALLET_ACTION_WIDTH,
        text_size: 11,
        text_color: theme.palette().text,
    })
}
