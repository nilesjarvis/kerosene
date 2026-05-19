use crate::account;
use crate::app_state::TradingTerminal;
use crate::helpers::{format_decimal_with_commas, format_usd};
use crate::message::Message;

use super::{PositionColumnVisibility, PositionNumberMode};
use iced::Theme;
use iced::widget::Column;

mod close_cell;
mod position_row;
mod sort;
mod summary;

impl TradingTerminal {
    pub(super) fn view_position_rows<'a>(
        &'a self,
        positions: &[&'a account::AssetPosition],
        can_close: bool,
        theme: &Theme,
        columns: PositionColumnVisibility,
        number_mode: PositionNumberMode,
    ) -> Column<'a, Message> {
        self.sorted_position_rows(positions).into_iter().fold(
            Column::new().spacing(2),
            |col, data| {
                col.push(self.view_position_row(data, can_close, theme, columns, number_mode))
            },
        )
    }
}

pub(super) fn format_position_usd_value(value: f64, number_mode: PositionNumberMode) -> String {
    if !number_mode.is_compact() {
        return format_usd(&format!("{value:.2}"));
    }

    let compact_value = format_position_compact_number(value.abs());
    let sign = if value.is_sign_negative() && compact_value != "0" {
        "-"
    } else {
        ""
    };
    format!("{sign}${compact_value}")
}

pub(super) fn format_position_compact_number(value: f64) -> String {
    let rounded_abs = value.abs().round();
    if rounded_abs < 10_000.0 {
        return format_decimal_with_commas(rounded_abs, 0);
    }

    let bucket = if rounded_abs < 100_000.0 {
        1_000.0
    } else if rounded_abs < 1_000_000.0 {
        100_000.0
    } else {
        1.0
    };
    let compact_abs = (rounded_abs / bucket).round() * bucket;
    format_position_compact_bucket(compact_abs)
}

fn format_position_compact_bucket(value: f64) -> String {
    if value >= 1_000_000.0 {
        return format!(
            "{}M",
            trim_decimal_zeros(format!("{:.1}", value / 1_000_000.0))
        );
    }

    if value >= 10_000.0 {
        return format!("{}k", format_decimal_with_commas(value / 1_000.0, 0));
    }

    format_decimal_with_commas(value, 0)
}

fn trim_decimal_zeros(value: String) -> String {
    let Some((whole, fraction)) = value.split_once('.') else {
        return value;
    };
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{fraction}")
    }
}
