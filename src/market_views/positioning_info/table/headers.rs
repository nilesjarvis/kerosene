use super::super::columns::*;
use super::cells::{change_sort_header_cell, header_cell, header_cell_aligned, sort_header_cell};
use crate::config;
use crate::denomination::DisplayDenominationContext;
use crate::message::Message;
use crate::positioning_state::{
    PositioningInfoChangeSortField, PositioningInfoId, PositioningInfoSortField,
};

use iced::alignment::Horizontal;
use iced::widget::Row;
use iced::{Element, Length, Theme};

// ---------------------------------------------------------------------------
// Positioning Table Headers
// ---------------------------------------------------------------------------

pub(in crate::market_views::positioning_info) fn positioning_table_header(
    id: PositioningInfoId,
    sort_field: PositioningInfoSortField,
    sort_direction: config::SortDirection,
    columns: PositioningInfoColumns,
    theme: &Theme,
) -> Element<'static, Message> {
    let muted = theme.extended_palette().background.weak.text;
    let mut header = Row::new()
        .spacing(POSITIONING_TABLE_COLUMN_SPACING)
        .padding([0, 8])
        .push(header_cell_aligned(
            "Trader",
            Length::Fixed(columns.trader_width),
            muted,
            Horizontal::Left,
        ))
        .push(header_cell_aligned(
            "Side",
            Length::Fixed(columns.side_width),
            muted,
            Horizontal::Left,
        ));

    if columns.show_size {
        header = header.push(sort_header_cell(
            "Size",
            PositioningInfoSortField::Size,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.size_width),
            muted,
        ));
    }

    header = header
        .push(sort_header_cell(
            "Notional",
            PositioningInfoSortField::NotionalSize,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.notional_width),
            muted,
        ))
        .push(sort_header_cell(
            "uPnL",
            PositioningInfoSortField::UnrealizedPnl,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.upnl_width),
            muted,
        ));

    if columns.show_entry {
        header = header.push(header_cell(
            "Entry",
            Length::Fixed(columns.entry_width),
            muted,
        ));
    }
    if columns.show_liq {
        header = header.push(header_cell("Liq", Length::Fixed(columns.liq_width), muted));
    }
    if columns.show_funding {
        header = header.push(header_cell(
            "Funding",
            Length::Fixed(columns.funding_width),
            muted,
        ));
    }
    if columns.show_account {
        header = header.push(sort_header_cell(
            "Account",
            PositioningInfoSortField::AccountValue,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.account_width),
            muted,
        ));
    }

    header.into()
}

pub(in crate::market_views::positioning_info) fn positioning_change_table_header(
    id: PositioningInfoId,
    sort_field: PositioningInfoChangeSortField,
    sort_direction: config::SortDirection,
    columns: PositioningChangeColumns,
    theme: &Theme,
    denomination: &DisplayDenominationContext,
) -> Element<'static, Message> {
    let muted = theme.extended_palette().background.weak.text;
    Row::new()
        .spacing(POSITIONING_TABLE_COLUMN_SPACING)
        .padding([0, 8])
        .push(change_sort_header_cell(
            "Trader",
            PositioningInfoChangeSortField::Trader,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.trader_width),
            muted,
            Horizontal::Left,
        ))
        .push(change_sort_header_cell(
            "Previous",
            PositioningInfoChangeSortField::Previous,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.previous_width),
            muted,
            Horizontal::Right,
        ))
        .push(change_sort_header_cell(
            "Current",
            PositioningInfoChangeSortField::Current,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.current_width),
            muted,
            Horizontal::Right,
        ))
        .push(change_sort_header_cell(
            "\u{0394} Change",
            PositioningInfoChangeSortField::Change,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.delta_width),
            muted,
            Horizontal::Right,
        ))
        .push(change_sort_header_cell(
            format!("Current {}", denomination.active_symbol()),
            PositioningInfoChangeSortField::CurrentUsd,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.current_usd_width),
            muted,
            Horizontal::Right,
        ))
        .push(change_sort_header_cell(
            format!("Change {}", denomination.active_symbol()),
            PositioningInfoChangeSortField::ChangeUsd,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.delta_usd_width),
            muted,
            Horizontal::Right,
        ))
        .into()
}
