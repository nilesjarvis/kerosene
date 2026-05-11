use crate::account::AssetContext;
use crate::chart_state::ChartId;
use crate::helpers;
use crate::message::Message;

use chrono::Timelike;
use iced::Theme;
use iced::widget::{Row, button, column, text};

pub(super) fn push_perp_metric_columns<'a>(
    header_row: Row<'a, Message>,
    theme: &Theme,
    chart_id: ChartId,
    ctx: &'a AssetContext,
    chart_price: f64,
    open_interest_as_notional: bool,
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

    header_row
        .push(metric_column(
            "Mark / Oracle".to_string(),
            format!(
                "{} / {}",
                format_metric_price(mark),
                format_metric_price(oracle)
            ),
            theme.palette().text,
            theme,
        ))
        .push(metric_column(
            format!("Funding ({})", funding_countdown()),
            format_funding_pct(funding),
            funding_color,
            theme,
        ))
        .push(clickable_metric_column(
            open_interest_label(open_interest_as_notional),
            format_open_interest(oi, chart_price, open_interest_as_notional),
            theme.palette().text,
            theme,
            Message::ToggleOpenInterestNotional(chart_id),
        ))
}

pub(super) fn push_spot_metric_columns<'a>(
    mut header_row: Row<'a, Message>,
    theme: &Theme,
    ctx: &'a AssetContext,
) -> Row<'a, Message> {
    let vlm = parse_ctx_f64(ctx.day_ntl_vlm.as_deref());
    header_row = header_row.push(metric_column(
        "24h Vol".to_string(),
        format_volume(vlm),
        theme.palette().text,
        theme,
    ));

    if let Some(mid) = &ctx.mid_px {
        header_row = header_row.push(metric_column(
            "Mid".to_string(),
            mid.clone(),
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

fn open_interest_label(as_notional: bool) -> String {
    if as_notional {
        "Open Interest $".to_string()
    } else {
        "Open Interest".to_string()
    }
}

fn format_open_interest(oi: Option<f64>, price: f64, as_notional: bool) -> String {
    let Some(oi) = oi else {
        return "Invalid data".to_string();
    };
    if as_notional {
        return format_open_interest_notional(oi, price);
    }
    if oi >= 1_000_000.0 {
        format!("{:.1}M", oi / 1_000_000.0)
    } else if oi >= 1_000.0 {
        format!("{:.0}K", oi / 1_000.0)
    } else {
        format!("{oi:.0}")
    }
}

fn format_open_interest_notional(oi: f64, price: f64) -> String {
    if !oi.is_finite() || !price.is_finite() || oi < 0.0 || price <= 0.0 {
        return "Invalid data".to_string();
    }
    format_compact_usd(oi * price)
}

fn format_volume(vlm: Option<f64>) -> String {
    let Some(vlm) = vlm else {
        return "Invalid data".to_string();
    };
    if vlm >= 1_000_000.0 {
        format!("${:.1}M", vlm / 1_000_000.0)
    } else if vlm >= 1_000.0 {
        format!("${:.0}K", vlm / 1_000.0)
    } else {
        format!("${vlm:.0}")
    }
}

fn format_compact_usd(value: f64) -> String {
    if !value.is_finite() || value < 0.0 {
        return "Invalid data".to_string();
    }
    if value >= 1_000_000_000.0 {
        format!("${:.2}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("${:.2}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("${:.1}K", value / 1_000.0)
    } else {
        format!("${value:.0}")
    }
}

fn format_metric_price(value: Option<f64>) -> String {
    value
        .map(helpers::format_price)
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
        format_funding_pct, format_open_interest, format_open_interest_notional, format_volume,
        open_interest_label, parse_ctx_f64,
    };

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
        assert_eq!(format_volume(None), "Invalid data");
        assert_eq!(format_open_interest(None, 100.0, false), "Invalid data");
        assert_eq!(format_funding_pct(None), "Invalid data");
        assert_eq!(format_volume(Some(1_500.0)), "$2K");
        assert_eq!(
            format_open_interest(Some(1_500_000.0), 100.0, false),
            "1.5M"
        );
        assert_eq!(format_funding_pct(Some(0.0001)), "0.0100%");
    }

    #[test]
    fn open_interest_notional_formats_from_chart_price() {
        assert_eq!(format_open_interest(Some(1_500.0), 2_000.0, true), "$3.00M");
        assert_eq!(
            format_open_interest_notional(2_000_000.0, 2_000.0),
            "$4.00B"
        );
        assert_eq!(
            format_open_interest(Some(1_500.0), 0.0, true),
            "Invalid data"
        );
        assert_eq!(open_interest_label(false), "Open Interest");
        assert_eq!(open_interest_label(true), "Open Interest $");
    }
}
