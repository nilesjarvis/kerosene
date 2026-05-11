use crate::account;
use crate::app_state::TradingTerminal;
use crate::helpers::format_usd;
use crate::message::Message;

use iced::widget::{container, row, text};
use iced::{Alignment, Color, Element, Fill, Font, Theme};

use super::sort::PositionRowData;

impl TradingTerminal {
    pub(in crate::account_views::positions) fn view_position_summary_bar<'a>(
        &'a self,
        positions: &[&'a account::AssetPosition],
        theme: &Theme,
    ) -> Element<'a, Message> {
        let totals =
            PositionSummaryTotals::from_rows(positions.iter().map(|ap| self.position_row_data(ap)));
        let weak_text = theme.extended_palette().background.weak.text;
        let neutral_text = theme.palette().text;
        let long_color = theme.palette().success;
        let short_color = theme.palette().danger;

        let summary = row![
            summary_cell(
                "Funding",
                format_optional_unsigned_usd(totals.funding_gross, self.hide_pnl),
                weak_text,
                neutral_text,
            ),
            summary_cell(
                "Long Ntl",
                format_unsigned_usd(totals.long_notional, self.hide_pnl),
                weak_text,
                long_color,
            ),
            summary_cell(
                "Short Ntl",
                format_unsigned_usd(totals.short_notional, self.hide_pnl),
                weak_text,
                short_color,
            ),
            summary_cell(
                "Net Fund",
                format_optional_signed_usd(totals.net_funding, self.hide_pnl),
                weak_text,
                totals
                    .net_funding
                    .value()
                    .map(|value| self.direction_color(theme, value))
                    .unwrap_or(weak_text),
            ),
            summary_cell(
                "uPnL",
                format_optional_signed_usd(totals.upnl, self.hide_pnl),
                weak_text,
                totals
                    .upnl
                    .value()
                    .map(|value| self.direction_color(theme, value))
                    .unwrap_or(weak_text),
            ),
            summary_cell(
                "Total PnL",
                format_optional_signed_usd(totals.total_pnl, self.hide_pnl),
                weak_text,
                totals
                    .total_pnl
                    .value()
                    .map(|value| self.direction_color(theme, value))
                    .unwrap_or(weak_text),
            ),
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        container(summary)
            .width(Fill)
            .padding([4, 8])
            .style(|theme: &Theme| {
                let mut background = theme.extended_palette().background.weak.color;
                background.a = 0.20;
                iced::widget::container::Style {
                    background: Some(background.into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: 1.0,
                        color: theme.extended_palette().background.strong.color,
                    },
                    ..Default::default()
                }
            })
            .into()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct PositionSummaryTotals {
    funding_gross: OptionalTotal,
    long_notional: f64,
    short_notional: f64,
    net_funding: OptionalTotal,
    upnl: OptionalTotal,
    total_pnl: OptionalTotal,
}

impl PositionSummaryTotals {
    fn from_rows<'a>(rows: impl IntoIterator<Item = PositionRowData<'a>>) -> Self {
        rows.into_iter().fold(Self::default(), |mut totals, row| {
            totals.add_row(row);
            totals
        })
    }

    fn add_row(&mut self, row: PositionRowData<'_>) {
        self.add_position(
            row.is_long,
            row.position_value,
            row.funding_since_open,
            row.upnl,
            row.total_pnl,
        );
    }

    fn add_position(
        &mut self,
        is_long: Option<bool>,
        position_value: Option<f64>,
        funding_since_open: Option<f64>,
        upnl: Option<f64>,
        total_pnl: Option<f64>,
    ) {
        if let (Some(is_long), Some(position_value)) = (is_long, position_value) {
            if is_long {
                self.long_notional += position_value.abs();
            } else {
                self.short_notional += position_value.abs();
            }
        }

        self.funding_gross.add(funding_since_open.map(f64::abs));
        self.net_funding.add(funding_since_open);
        self.upnl.add(upnl);
        self.total_pnl.add(total_pnl);
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct OptionalTotal {
    value: f64,
    count: usize,
}

impl OptionalTotal {
    fn add(&mut self, value: Option<f64>) {
        if let Some(value) = value.filter(|value| value.is_finite()) {
            self.value += value;
            self.count += 1;
        }
    }

    fn value(self) -> Option<f64> {
        (self.count > 0).then_some(self.value)
    }
}

fn summary_cell(
    label: &'static str,
    value: String,
    label_color: Color,
    value_color: Color,
) -> Element<'static, Message> {
    container(
        row![
            text(label).size(10).color(label_color),
            text(value)
                .size(11)
                .font(Font::MONOSPACE)
                .color(value_color),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .into()
}

fn format_unsigned_usd(value: f64, hide_pnl: bool) -> String {
    if hide_pnl {
        "$***".to_string()
    } else {
        format_usd(&format!("{value:.2}"))
    }
}

fn format_optional_unsigned_usd(total: OptionalTotal, hide_pnl: bool) -> String {
    match total.value() {
        Some(value) => format_unsigned_usd(value, hide_pnl),
        None => "--".to_string(),
    }
}

fn format_optional_signed_usd(total: OptionalTotal, hide_pnl: bool) -> String {
    match total.value() {
        Some(_) if hide_pnl => "$***".to_string(),
        Some(value) => format_signed_usd(value),
        None => "--".to_string(),
    }
}

fn format_signed_usd(value: f64) -> String {
    let display_value = if value.abs() < 0.005 { 0.0 } else { value };
    let formatted = format_usd(&format!("{display_value:.2}"));
    if display_value > 0.0 {
        format!("+{formatted}")
    } else {
        formatted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_splits_exposure_and_sums_signed_totals() {
        let mut totals = PositionSummaryTotals::default();

        totals.add_position(Some(true), Some(100.0), Some(2.5), Some(10.0), Some(12.5));
        totals.add_position(Some(false), Some(80.0), Some(-1.0), Some(-3.0), Some(-4.0));

        assert_eq!(totals.long_notional, 100.0);
        assert_eq!(totals.short_notional, 80.0);
        assert_eq!(totals.funding_gross.value(), Some(3.5));
        assert_eq!(totals.net_funding.value(), Some(1.5));
        assert_eq!(totals.upnl.value(), Some(7.0));
        assert_eq!(totals.total_pnl.value(), Some(8.5));
    }

    #[test]
    fn summary_formatting_masks_only_present_values_when_pnl_is_hidden() {
        let mut total = OptionalTotal::default();

        assert_eq!(format_optional_signed_usd(total, true), "--");

        total.add(Some(-12.34));

        assert_eq!(format_optional_signed_usd(total, true), "$***");
        assert_eq!(format_optional_signed_usd(total, false), "-$12.34");
    }
}
