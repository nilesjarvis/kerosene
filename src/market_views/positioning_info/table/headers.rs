use super::super::columns::*;
use super::cells::{header_cell, header_cell_aligned, sort_header_cell};
use crate::config;
use crate::message::Message;
use crate::positioning_state::{PositioningInfoId, PositioningInfoSortField};

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
