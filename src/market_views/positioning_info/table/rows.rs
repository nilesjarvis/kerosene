use super::super::columns::*;
use super::super::metrics::*;
use super::cells::{positioning_trader_cell, value_cell};
use crate::denomination::DisplayDenominationContext;
use crate::helpers;
use crate::hyperdash_api::{PerpDeltaEntry, TickerPositionEntry};
use crate::message::Message;
use crate::positioning_state::PositioningInfoId;
use crate::wallet_state::address_book::WalletDisplay;

use iced::widget::{Row, container};
use iced::{Alignment, Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// Positioning Table Rows
// ---------------------------------------------------------------------------

pub(in crate::market_views::positioning_info) fn positioning_position_row(
    instance_id: PositioningInfoId,
    position: &TickerPositionEntry,
    wallet_display: WalletDisplay,
    columns: PositioningInfoColumns,
    hovered_wallet_action_key: Option<&str>,
    theme: &Theme,
    live_mark: Option<f64>,
    denomination: &DisplayDenominationContext,
) -> Element<'static, Message> {
    let side = position_side_label(position.size);
    let side_color = position_side_color(position.size, theme);
    let notional = positioning_live_notional(position, live_mark).unwrap_or(position.notional_size);
    let unrealized_pnl =
        positioning_live_unrealized_pnl(position, live_mark).unwrap_or(position.unrealized_pnl);
    let pnl_color = signed_value_color(unrealized_pnl, theme);
    let funding_color = signed_value_color(position.funding_pnl, theme);
    let pnl_text = if columns.compact_money {
        format_signed_usd_compact(unrealized_pnl, denomination)
    } else {
        format_signed_usd(unrealized_pnl, denomination)
    };
    let funding_text = if columns.compact_money {
        format_signed_usd_compact(position.funding_pnl, denomination)
    } else {
        format_signed_usd(position.funding_pnl, denomination)
    };

    let mut row = Row::new()
        .spacing(POSITIONING_TABLE_COLUMN_SPACING)
        .padding([4, 8])
        .align_y(Alignment::Center)
        .push(positioning_trader_cell(
            &position.address,
            wallet_display,
            columns.trader_width,
            POSITIONING_TRADER_COMPACT_ACTIONS_MIN_WIDTH,
            format!(
                "positioning-info:{instance_id}:positions:{}",
                position.address
            ),
            hovered_wallet_action_key,
            theme,
        ))
        .push(value_cell(
            side,
            Length::Fixed(columns.side_width),
            side_color,
            false,
        ));

    if columns.show_size {
        row = row.push(value_cell(
            helpers::format_size(position.size.abs()),
            Length::Fixed(columns.size_width),
            theme.palette().text,
            true,
        ));
    }

    row = row
        .push(value_cell(
            format_usd_number(notional.abs(), denomination),
            Length::Fixed(columns.notional_width),
            theme.palette().text,
            true,
        ))
        .push(value_cell(
            pnl_text,
            Length::Fixed(columns.upnl_width),
            pnl_color,
            true,
        ));

    if columns.show_entry {
        row = row.push(value_cell(
            format_price_number(position.entry_price, denomination),
            Length::Fixed(columns.entry_width),
            theme.palette().text,
            true,
        ));
    }
    if columns.show_liq {
        row = row.push(value_cell(
            position
                .liquidation_price
                .map(|value| format_price_number(value, denomination))
                .unwrap_or_else(|| "-".to_string()),
            Length::Fixed(columns.liq_width),
            theme.palette().text,
            true,
        ));
    }
    if columns.show_funding {
        row = row.push(value_cell(
            funding_text,
            Length::Fixed(columns.funding_width),
            funding_color,
            true,
        ));
    }
    if columns.show_account {
        row = row.push(value_cell(
            format_usd_number(position.account_value, denomination),
            Length::Fixed(columns.account_width),
            theme.palette().text,
            true,
        ));
    }

    container(row)
        .width(Fill)
        .style(row_accent_style(side_color, 0.15))
        .into()
}

pub(in crate::market_views::positioning_info) fn positioning_change_row(
    instance_id: PositioningInfoId,
    entry: &PerpDeltaEntry,
    wallet_display: WalletDisplay,
    columns: PositioningChangeColumns,
    hovered_wallet_action_key: Option<&str>,
    theme: &Theme,
    live_mark: Option<f64>,
    denomination: &DisplayDenominationContext,
) -> Element<'static, Message> {
    let previous = positioning_previous_change_size(entry);
    let previous_color = previous
        .map(|value| signed_value_color(value, theme))
        .unwrap_or_else(|| theme.extended_palette().background.weak.text);
    let current_color = signed_value_color(entry.current, theme);
    let delta_color = signed_value_color(entry.delta, theme);
    let current_usd = positioning_live_change_usd(entry.current, live_mark)
        .map(|value| format_signed_usd(value, denomination))
        .unwrap_or_else(|| "-".to_string());
    let delta_usd = positioning_live_change_usd(entry.delta, live_mark)
        .map(|value| format_signed_usd(value, denomination))
        .unwrap_or_else(|| "-".to_string());

    let row = Row::new()
        .spacing(POSITIONING_TABLE_COLUMN_SPACING)
        .padding([4, 8])
        .align_y(Alignment::Center)
        .push(positioning_trader_cell(
            &entry.address,
            wallet_display,
            columns.trader_width,
            POSITIONING_CHANGE_TRADER_COMPACT_ACTIONS_MIN_WIDTH,
            format!("positioning-info:{instance_id}:changes:{}", entry.address),
            hovered_wallet_action_key,
            theme,
        ))
        .push(value_cell(
            previous
                .map(|value| format_signed_size(value, false))
                .unwrap_or_else(|| "-".to_string()),
            Length::Fixed(columns.previous_width),
            previous_color,
            true,
        ))
        .push(value_cell(
            format_signed_size(entry.current, false),
            Length::Fixed(columns.current_width),
            current_color,
            true,
        ))
        .push(value_cell(
            format_signed_size(entry.delta, true),
            Length::Fixed(columns.delta_width),
            delta_color,
            true,
        ))
        .push(value_cell(
            current_usd,
            Length::Fixed(columns.current_usd_width),
            current_color,
            true,
        ))
        .push(value_cell(
            delta_usd,
            Length::Fixed(columns.delta_usd_width),
            delta_color,
            true,
        ));

    container(row)
        .width(Fill)
        .style(row_accent_style(delta_color, 0.12))
        .into()
}

fn row_accent_style(
    accent_color: iced::Color,
    alpha: f32,
) -> impl Fn(&Theme) -> iced::widget::container::Style {
    move |_theme: &Theme| {
        use iced::gradient;
        let mut base_color = accent_color;
        base_color.a = alpha;
        iced::widget::container::Style {
            background: Some(
                gradient::Linear::new(iced::Degrees(90.0))
                    .add_stop(0.0, base_color)
                    .add_stop(0.20, iced::Color::TRANSPARENT)
                    .add_stop(1.0, iced::Color::TRANSPARENT)
                    .into(),
            ),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
