use crate::account::AssetContext;
use crate::chart_state::ChartId;
use crate::denomination::DisplayDenominationContext;
use crate::message::Message;

use chrono::Timelike;
use iced::Theme;
use iced::widget::{Row, button, column, text};

const HIDE_MARK_ORACLE_BELOW: f32 = 720.0;
const HIDE_OPEN_INTEREST_BELOW: f32 = 560.0;
const HIDE_FUNDING_BELOW: f32 = 460.0;
const HIDE_24H_CHANGE_BELOW: f32 = 340.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChartHeaderMetricVisibility {
    pub(crate) show_24h_change: bool,
    pub(crate) show_mark_oracle: bool,
    pub(crate) show_open_interest: bool,
    pub(crate) show_funding: bool,
}

impl ChartHeaderMetricVisibility {
    pub(crate) fn for_width(width: f32) -> Self {
        let width = if width.is_finite() { width } else { 0.0 };
        Self {
            show_24h_change: width >= HIDE_24H_CHANGE_BELOW,
            show_mark_oracle: width >= HIDE_MARK_ORACLE_BELOW,
            show_open_interest: width >= HIDE_OPEN_INTEREST_BELOW,
            show_funding: width >= HIDE_FUNDING_BELOW,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_perp_metric_columns<'a>(
    mut header_row: Row<'a, Message>,
    theme: &Theme,
    chart_id: ChartId,
    ctx: &'a AssetContext,
    chart_price: f64,
    open_interest_as_notional: bool,
    visibility: ChartHeaderMetricVisibility,
    denomination: &DisplayDenominationContext,
) -> Row<'a, Message> {
    let funding = parse_ctx_f64(ctx.funding.as_deref());
    let funding_color = match funding {
        Some(value) if value >= 0.0 => theme.palette().success,
        Some(_) => theme.palette().danger,
        None => theme.palette().warning,
    };
    let mark = parse_ctx_f64(ctx.mark_px.as_deref());
    let oracle = parse_ctx_f64(ctx.oracle_px.as_deref());
    let oi = parse_ctx_f64(ctx.open_interest.as_deref());

    if visibility.show_mark_oracle {
        header_row = header_row.push(metric_column(
            "Mark / Oracle".to_string(),
            format!(
                "{} / {}",
                format_metric_price(mark, denomination),
                format_metric_price(oracle, denomination)
            ),
            theme.palette().text,
            theme,
        ));
    }

    if visibility.show_funding {
        header_row = header_row.push(metric_column(
            format!("Funding ({})", funding_countdown()),
            format_funding_pct(funding),
            funding_color,
            theme,
        ));
    }

    if visibility.show_open_interest {
        header_row = header_row.push(clickable_metric_column(
            open_interest_label(open_interest_as_notional, denomination),
            format_open_interest(oi, chart_price, open_interest_as_notional, denomination),
            theme.palette().text,
            theme,
            Message::ToggleOpenInterestNotional(chart_id),
        ));
    }

    header_row
}

pub(super) fn push_spot_metric_columns<'a>(
    mut header_row: Row<'a, Message>,
    theme: &Theme,
    ctx: &'a AssetContext,
    denomination: &DisplayDenominationContext,
) -> Row<'a, Message> {
    let vlm = parse_ctx_f64(ctx.day_ntl_vlm.as_deref());
    header_row = header_row.push(metric_column(
        "24h Vol".to_string(),
        format_volume(vlm, denomination),
        theme.palette().text,
        theme,
    ));

    if let Some(mid) = &ctx.mid_px {
        header_row = header_row.push(metric_column(
            "Mid".to_string(),
            format_metric_price(parse_ctx_f64(Some(mid.as_str())), denomination),
            theme.palette().text,
            theme,
        ));
    }

    header_row
}

fn metric_column(
    label: String,
    value: String,
    value_color: iced::Color,
    theme: &Theme,
) -> iced::widget::Column<'static, Message> {
    column![
        text(label)
            .size(9)
            .color(theme.extended_palette().background.weak.text),
        text(value)
            .size(12)
            .font(iced::Font::MONOSPACE)
            .color(value_color)
    ]
    .spacing(2)
}

fn clickable_metric_column(
    label: String,
    value: String,
    value_color: iced::Color,
    theme: &Theme,
    message: Message,
) -> iced::Element<'static, Message> {
    button(metric_column(label, value, value_color, theme))
        .on_press(message)
        .padding(0)
        .style(|theme: &Theme, status| {
            let background = match status {
                button::Status::Hovered => Some(theme.extended_palette().background.weak.color),
                _ => None,
            };
            button::Style {
                background: background.map(Into::into),
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn funding_countdown() -> String {
    let now = chrono::Local::now();
    let next_hour = (now + chrono::Duration::hours(1))
        .with_minute(0)
        .and_then(|dt| dt.with_second(0))
        .unwrap_or(now + chrono::Duration::hours(1));
    let diff = next_hour.signed_duration_since(now);
    format!(
        "{:02}:{:02}",
        diff.num_minutes().max(0),
        (diff.num_seconds() % 60).max(0)
    )
}

fn open_interest_label(as_notional: bool, denomination: &DisplayDenominationContext) -> String {
    if as_notional {
        format!("Open Interest {}", denomination.active_symbol())
    } else {
        "Open Interest".to_string()
    }
}

fn format_open_interest(
    oi: Option<f64>,
    price: f64,
    as_notional: bool,
    denomination: &DisplayDenominationContext,
) -> String {
    let Some(oi) = oi else {
        return "Invalid data".to_string();
    };
    if as_notional {
        return format_open_interest_notional(oi, price, denomination);
    }
    if oi >= 1_000_000.0 {
        format!("{:.1}M", oi / 1_000_000.0)
    } else if oi >= 1_000.0 {
        format!("{:.0}K", oi / 1_000.0)
    } else {
        format!("{oi:.0}")
    }
}

fn format_open_interest_notional(
    oi: f64,
    price: f64,
    denomination: &DisplayDenominationContext,
) -> String {
    if !oi.is_finite() || !price.is_finite() || oi < 0.0 || price <= 0.0 {
        return "Invalid data".to_string();
    }
    denomination.format_compact_value(oi * price)
}

fn format_volume(vlm: Option<f64>, denomination: &DisplayDenominationContext) -> String {
    let Some(vlm) = vlm else {
        return "Invalid data".to_string();
    };
    if !vlm.is_finite() || vlm < 0.0 {
        return "Invalid data".to_string();
    }
    denomination.format_compact_value(vlm)
}

fn format_metric_price(value: Option<f64>, denomination: &DisplayDenominationContext) -> String {
    value
        .map(|value| denomination.format_chart_price(value))
        .unwrap_or_else(|| "Invalid data".to_string())
}

fn format_funding_pct(funding: Option<f64>) -> String {
    funding
        .map(|funding| format!("{:.4}%", funding * 100.0))
        .unwrap_or_else(|| "Invalid data".to_string())
}

fn parse_ctx_f64(value: Option<&str>) -> Option<f64> {
    let parsed = value?.trim().parse::<f64>().ok()?;
    parsed.is_finite().then_some(parsed)
}

#[cfg(test)]
mod tests {
    use super::{
        ChartHeaderMetricVisibility, format_funding_pct, format_open_interest,
        format_open_interest_notional, format_volume, open_interest_label, parse_ctx_f64,
    };
    use crate::denomination::DisplayDenominationContext;

    #[test]
    fn context_number_parser_rejects_missing_malformed_or_nonfinite_values() {
        assert_eq!(parse_ctx_f64(Some("12.5")), Some(12.5));
        assert_eq!(parse_ctx_f64(None), None);
        assert_eq!(parse_ctx_f64(Some("bad")), None);
        assert_eq!(parse_ctx_f64(Some("NaN")), None);
        assert_eq!(parse_ctx_f64(Some("inf")), None);
    }

    #[test]
    fn header_metric_formatters_mark_invalid_values() {
        let denomination = DisplayDenominationContext::default();
        assert_eq!(format_volume(None, &denomination), "Invalid data");
        assert_eq!(
            format_open_interest(None, 100.0, false, &denomination),
            "Invalid data"
        );
        assert_eq!(format_funding_pct(None), "Invalid data");
        assert_eq!(format_volume(Some(1_500.0), &denomination), "$1.5K");
        assert_eq!(
            format_open_interest(Some(1_500_000.0), 100.0, false, &denomination),
            "1.5M"
        );
        assert_eq!(format_funding_pct(Some(0.0001)), "0.0100%");
    }

    #[test]
    fn open_interest_notional_formats_from_chart_price() {
        let denomination = DisplayDenominationContext::default();
        assert_eq!(
            format_open_interest(Some(1_500.0), 2_000.0, true, &denomination),
            "$3.00M"
        );
        assert_eq!(
            format_open_interest_notional(2_000_000.0, 2_000.0, &denomination),
            "$4.00B"
        );
        assert_eq!(
            format_open_interest(Some(1_500.0), 0.0, true, &denomination),
            "Invalid data"
        );
        assert_eq!(open_interest_label(false, &denomination), "Open Interest");
        assert_eq!(open_interest_label(true, &denomination), "Open Interest $");
    }

    #[test]
    fn metric_visibility_collapses_in_priority_order() {
        assert_eq!(
            ChartHeaderMetricVisibility::for_width(760.0),
            ChartHeaderMetricVisibility {
                show_24h_change: true,
                show_mark_oracle: true,
                show_open_interest: true,
                show_funding: true,
            }
        );
        assert_eq!(
            ChartHeaderMetricVisibility::for_width(680.0),
            ChartHeaderMetricVisibility {
                show_24h_change: true,
                show_mark_oracle: false,
                show_open_interest: true,
                show_funding: true,
            }
        );
        assert_eq!(
            ChartHeaderMetricVisibility::for_width(520.0),
            ChartHeaderMetricVisibility {
                show_24h_change: true,
                show_mark_oracle: false,
                show_open_interest: false,
                show_funding: true,
            }
        );
        assert_eq!(
            ChartHeaderMetricVisibility::for_width(420.0),
            ChartHeaderMetricVisibility {
                show_24h_change: true,
                show_mark_oracle: false,
                show_open_interest: false,
                show_funding: false,
            }
        );
        assert_eq!(
            ChartHeaderMetricVisibility::for_width(320.0),
            ChartHeaderMetricVisibility {
                show_24h_change: false,
                show_mark_oracle: false,
                show_open_interest: false,
                show_funding: false,
            }
        );
    }
}
