use crate::account;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pnl_card::{PnlCardTarget, pnl_card_icon_button};

use iced::widget::{container, row, text};
use iced::{Alignment, Color, Element, Fill, Font, Theme};

use super::super::PositionNumberMode;
use super::format_position_display_value;
#[cfg(test)]
use super::format_position_usd_value;
use super::sort::PositionRowData;

impl TradingTerminal {
    pub(in crate::account_views::positions) fn view_position_summary_bar<'a>(
        &'a self,
        positions: &[&'a account::AssetPosition],
        theme: &Theme,
        number_mode: PositionNumberMode,
    ) -> Element<'a, Message> {
        let totals =
            PositionSummaryTotals::from_rows(positions.iter().map(|ap| self.position_row_data(ap)));
        let weak_text = theme.extended_palette().background.weak.text;
        let neutral_text = theme.palette().text;
        let long_color = theme.palette().success;
        let short_color = theme.palette().danger;
        let account_balance = self
            .account_data
            .as_ref()
            .and_then(|data| self.position_summary_account_value(data));
        let total_pnl_pct = position_total_pnl_percent(totals.total_pnl, account_balance);
        let denomination = self.display_denomination_context();

        let summary = row![
            summary_cell(
                "Funding",
                format_optional_unsigned_display(
                    &denomination,
                    totals.funding_gross,
                    self.hide_pnl,
                    number_mode,
                ),
                weak_text,
                neutral_text,
            ),
            summary_cell(
                "Long Ntl",
                format_unsigned_display(
                    &denomination,
                    totals.long_notional,
                    self.hide_pnl,
                    number_mode,
                ),
                weak_text,
                long_color,
            ),
            summary_cell(
                "Short Ntl",
                format_unsigned_display(
                    &denomination,
                    totals.short_notional,
                    self.hide_pnl,
                    number_mode,
                ),
                weak_text,
                short_color,
            ),
            summary_cell(
                "Net Fund",
                format_optional_signed_display(
                    &denomination,
                    totals.net_funding,
                    self.hide_pnl,
                    number_mode,
                ),
                weak_text,
                totals
                    .net_funding
                    .value()
                    .map(|value| self.direction_color(theme, value))
                    .unwrap_or(weak_text),
            ),
            summary_cell_with_action(
                "uPnL",
                format_optional_signed_display(
                    &denomination,
                    totals.upnl,
                    self.hide_pnl,
                    number_mode,
                ),
                weak_text,
                totals
                    .upnl
                    .value()
                    .map(|value| self.direction_color(theme, value))
                    .unwrap_or(weak_text),
                totals
                    .upnl
                    .value()
                    .map(|_| Message::OpenPnlCard(PnlCardTarget::Summary)),
            ),
            summary_cell(
                "Total PnL",
                format_optional_total_pnl_display(
                    &denomination,
                    totals.total_pnl,
                    total_pnl_pct,
                    self.hide_pnl,
                    number_mode,
                ),
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

    fn position_summary_account_value(&self, data: &account::AccountData) -> Option<f64> {
        let clearinghouse = self.visible_clearinghouse_state(data);
        let include_spot = self.account_view_includes_spot_balances(data);
        let live_upnl = sum_required(
            clearinghouse
                .asset_positions
                .iter()
                .filter(|ap| !self.symbol_key_is_hidden(&ap.position.coin))
                .map(|ap| {
                    position_summary_position_upnl_value(
                        &ap.position.szi,
                        &ap.position.entry_px,
                        &ap.position.unrealized_pnl,
                        self.resolve_mid_for_symbol(&ap.position.coin),
                    )
                }),
        );
        let stale_upnl = sum_required(
            clearinghouse
                .asset_positions
                .iter()
                .filter(|ap| !self.symbol_key_is_hidden(&ap.position.coin))
                .map(|ap| parse_summary_number(&ap.position.unrealized_pnl)),
        );
        let spot_value = if include_spot {
            sum_required(
                data.spot
                    .balances
                    .iter()
                    .filter(|balance| !self.account_spot_balance_is_hidden(data, &balance.coin))
                    .map(|balance| {
                        position_summary_spot_balance_value(
                            &balance.coin,
                            &balance.total,
                            &balance.entry_ntl,
                            self.resolve_mid_for_symbol(&balance.coin),
                        )
                    }),
            )
        } else {
            Some(0.0)
        };
        let perp_equity = if include_spot && data.is_portfolio_margin() {
            Some(0.0)
        } else {
            parse_summary_number(&clearinghouse.margin_summary.account_value)
        };

        match (perp_equity, spot_value, live_upnl, stale_upnl) {
            (Some(perp_equity), Some(spot_value), Some(live_upnl), Some(stale_upnl)) => {
                Some(perp_equity + spot_value + (live_upnl - stale_upnl))
            }
            _ => None,
        }
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

fn summary_cell_with_action(
    label: &'static str,
    value: String,
    label_color: Color,
    value_color: Color,
    action: Option<Message>,
) -> Element<'static, Message> {
    container(
        row![
            text(label).size(10).color(label_color),
            text(value)
                .size(11)
                .font(Font::MONOSPACE)
                .color(value_color),
            pnl_card_icon_button(action, "Open summary PnL card"),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .into()
}

#[cfg(test)]
fn format_unsigned_usd(value: f64, hide_pnl: bool, number_mode: PositionNumberMode) -> String {
    if hide_pnl {
        "$***".to_string()
    } else {
        format_position_usd_value(value, number_mode)
    }
}

fn format_unsigned_display(
    context: &crate::denomination::DisplayDenominationContext,
    value: f64,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    if hide_pnl {
        context.hidden_mask()
    } else {
        format_position_display_value(context, value, number_mode)
    }
}

#[cfg(test)]
fn format_optional_unsigned_usd(
    total: OptionalTotal,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total.value() {
        Some(value) => format_unsigned_usd(value, hide_pnl, number_mode),
        None => "--".to_string(),
    }
}

fn format_optional_unsigned_display(
    context: &crate::denomination::DisplayDenominationContext,
    total: OptionalTotal,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total.value() {
        Some(value) => format_unsigned_display(context, value, hide_pnl, number_mode),
        None => "--".to_string(),
    }
}

#[cfg(test)]
fn format_optional_signed_usd(
    total: OptionalTotal,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total.value() {
        Some(_) if hide_pnl => "$***".to_string(),
        Some(value) => format_signed_usd(value, number_mode),
        None => "--".to_string(),
    }
}

fn format_optional_signed_display(
    context: &crate::denomination::DisplayDenominationContext,
    total: OptionalTotal,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total.value() {
        Some(_) if hide_pnl => context.hidden_mask(),
        Some(value) => format_signed_display(context, value, number_mode),
        None => "--".to_string(),
    }
}

#[cfg(test)]
fn format_optional_total_pnl(
    total: OptionalTotal,
    percent: Option<f64>,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total.value() {
        Some(_) if hide_pnl => {
            let percent = percent
                .map(|percent| format_signed_percent(percent, number_mode))
                .unwrap_or_else(|| "--%".to_string());
            format!("$*** ({percent})")
        }
        Some(value) => {
            let percent = percent
                .map(|percent| format_signed_percent(percent, number_mode))
                .unwrap_or_else(|| "--%".to_string());
            format!("{} ({percent})", format_signed_usd(value, number_mode))
        }
        None => "--".to_string(),
    }
}

fn format_optional_total_pnl_display(
    context: &crate::denomination::DisplayDenominationContext,
    total: OptionalTotal,
    percent: Option<f64>,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total.value() {
        Some(_) if hide_pnl => {
            let percent = percent
                .map(|percent| format_signed_percent(percent, number_mode))
                .unwrap_or_else(|| "--%".to_string());
            format!("{} ({percent})", context.hidden_mask())
        }
        Some(value) => {
            let percent = percent
                .map(|percent| format_signed_percent(percent, number_mode))
                .unwrap_or_else(|| "--%".to_string());
            format!(
                "{} ({percent})",
                format_signed_display(context, value, number_mode)
            )
        }
        None => "--".to_string(),
    }
}

#[cfg(test)]
fn format_signed_usd(value: f64, number_mode: PositionNumberMode) -> String {
    let min_display = if number_mode.is_compact() { 0.5 } else { 0.005 };
    let display_value = if value.abs() < min_display {
        0.0
    } else {
        value
    };
    let formatted = format_position_usd_value(display_value, number_mode);
    if display_value > 0.0 {
        format!("+{formatted}")
    } else {
        formatted
    }
}

fn format_signed_display(
    context: &crate::denomination::DisplayDenominationContext,
    value: f64,
    number_mode: PositionNumberMode,
) -> String {
    let min_display = if number_mode.is_compact() { 0.5 } else { 0.005 };
    let display_value = if value.abs() < min_display {
        0.0
    } else {
        value
    };
    let formatted = format_position_display_value(context, display_value, number_mode);
    if display_value > 0.0 {
        format!("+{formatted}")
    } else {
        formatted
    }
}

fn format_signed_percent(value: f64, number_mode: PositionNumberMode) -> String {
    let decimals = if number_mode.is_compact() { 1 } else { 2 };
    let min_display = if number_mode.is_compact() {
        0.05
    } else {
        0.005
    };
    let display_value = if value.abs() < min_display {
        0.0
    } else {
        value
    };
    if display_value > 0.0 {
        format!("+{display_value:.decimals$}%")
    } else {
        format!("{display_value:.decimals$}%")
    }
}

fn position_total_pnl_percent(
    total_pnl: OptionalTotal,
    account_balance: Option<f64>,
) -> Option<f64> {
    match (total_pnl.value(), account_balance) {
        (Some(total_pnl), Some(account_balance)) if account_balance.abs() > f64::EPSILON => {
            Some(total_pnl / account_balance * 100.0)
        }
        _ => None,
    }
}

fn parse_summary_number(raw: &str) -> Option<f64> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn position_summary_position_upnl_value(
    szi_raw: &str,
    entry_raw: &str,
    wire_upnl_raw: &str,
    live_mid: Option<f64>,
) -> Option<f64> {
    match (
        live_mid,
        parse_summary_number(szi_raw),
        parse_summary_number(entry_raw),
    ) {
        (Some(mid), Some(szi), Some(entry)) => Some(szi * (mid - entry)),
        _ => parse_summary_number(wire_upnl_raw),
    }
}

fn position_summary_spot_balance_value(
    coin: &str,
    total_raw: &str,
    entry_ntl_raw: &str,
    live_mid: Option<f64>,
) -> Option<f64> {
    let total = parse_summary_number(total_raw)?;
    if total.abs() < 1e-12 {
        return Some(0.0);
    }
    if matches!(coin, "USDC" | "USDE" | "USDT0" | "USDH") {
        Some(total)
    } else if let Some(mid) = live_mid {
        Some(total * mid)
    } else {
        parse_summary_number(entry_ntl_raw)
    }
}

fn sum_required(values: impl IntoIterator<Item = Option<f64>>) -> Option<f64> {
    let mut total = 0.0;
    for value in values {
        total += value?;
    }
    Some(total)
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

        assert_eq!(
            format_optional_signed_usd(total, true, PositionNumberMode::Full),
            "--"
        );

        total.add(Some(-12.34));

        assert_eq!(
            format_optional_signed_usd(total, true, PositionNumberMode::Full),
            "$***"
        );
        assert_eq!(
            format_optional_signed_usd(total, false, PositionNumberMode::Full),
            "-$12.34"
        );
    }

    #[test]
    fn total_pnl_percent_uses_overall_account_balance() {
        let mut total = OptionalTotal::default();
        total.add(Some(50.0));

        assert_eq!(position_total_pnl_percent(total, Some(1_000.0)), Some(5.0));
        assert_eq!(position_total_pnl_percent(total, Some(0.0)), None);
        assert_eq!(position_total_pnl_percent(total, None), None);
    }

    #[test]
    fn total_pnl_display_includes_percent_and_masks_when_hidden() {
        let mut total = OptionalTotal::default();
        total.add(Some(12.5));

        assert_eq!(
            format_optional_total_pnl(total, Some(1.25), false, PositionNumberMode::Full),
            "+$12.50 (+1.25%)"
        );
        assert_eq!(
            format_optional_total_pnl(total, None, false, PositionNumberMode::Full),
            "+$12.50 (--%)"
        );
        assert_eq!(
            format_optional_total_pnl(total, Some(1.25), true, PositionNumberMode::Full),
            "$*** (+1.25%)"
        );
        assert_eq!(
            format_optional_total_pnl(total, None, true, PositionNumberMode::Full),
            "$*** (--%)"
        );
    }

    #[test]
    fn compact_summary_formatting_rounds_money_and_percent() {
        let mut total = OptionalTotal::default();
        total.add(Some(1234.56));

        assert_eq!(
            format_optional_unsigned_usd(total, false, PositionNumberMode::Compact),
            "$1,235"
        );
        assert_eq!(
            format_optional_signed_usd(total, false, PositionNumberMode::Compact),
            "+$1,235"
        );
        assert_eq!(
            format_optional_total_pnl(total, Some(1.25), false, PositionNumberMode::Compact),
            "+$1,235 (+1.2%)"
        );

        let mut large_total = OptionalTotal::default();
        large_total.add(Some(532_023.0));

        assert_eq!(
            format_optional_unsigned_usd(large_total, false, PositionNumberMode::Compact),
            "$500k"
        );
        assert_eq!(
            format_optional_total_pnl(large_total, Some(1.25), false, PositionNumberMode::Compact),
            "+$500k (+1.2%)"
        );
    }

    #[test]
    fn account_balance_helpers_use_live_position_and_spot_values() {
        assert_eq!(
            position_summary_position_upnl_value("2", "100", "1", Some(110.0)),
            Some(20.0)
        );
        assert_eq!(
            position_summary_position_upnl_value("bad", "100", "1", Some(110.0)),
            Some(1.0)
        );

        assert_eq!(
            position_summary_spot_balance_value("USDC", "10", "0", None),
            Some(10.0)
        );
        assert_eq!(
            position_summary_spot_balance_value("PURR", "2", "3", Some(4.0)),
            Some(8.0)
        );
        assert_eq!(
            position_summary_spot_balance_value("PURR", "2", "3", None),
            Some(3.0)
        );
    }
}
