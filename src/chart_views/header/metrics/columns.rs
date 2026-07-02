use crate::account::AssetContext;
use crate::api::OutcomeVolume24h;
use crate::app_time::local_datetime_from_unix_ms;
use crate::chart_state::ChartId;
use crate::message::Message;

use chrono::Timelike;
use iced::Theme;
use iced::widget::{Row, button, column, text};

mod formatting;

#[cfg(test)]
use formatting::format_open_interest_notional;
#[cfg(test)]
use formatting::format_volume;
use formatting::{
    asset_volume_label, format_asset_volume, format_funding_pct, format_metric_price,
    format_open_interest, format_outcome_asset_volume, format_outcome_volume, open_interest_label,
    outcome_volume_label, parse_ctx_f64, spot_base_ticker,
};

const HIDE_MARK_ORACLE_BELOW: f32 = 720.0;
const HIDE_OPEN_INTEREST_BELOW: f32 = 560.0;
const HIDE_FUNDING_BELOW: f32 = 460.0;
const HIDE_24H_VOLUME_BELOW: f32 = 420.0;
const HIDE_24H_CHANGE_BELOW: f32 = 340.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChartHeaderMetricVisibility {
    pub(crate) show_24h_change: bool,
    pub(crate) show_24h_volume: bool,
    pub(crate) show_mark_oracle: bool,
    pub(crate) show_open_interest: bool,
    pub(crate) show_funding: bool,
}

impl ChartHeaderMetricVisibility {
    pub(crate) fn for_width(width: f32) -> Self {
        let width = if width.is_finite() { width } else { 0.0 };
        Self {
            show_24h_change: width >= HIDE_24H_CHANGE_BELOW,
            show_24h_volume: width >= HIDE_24H_VOLUME_BELOW,
            show_mark_oracle: width >= HIDE_MARK_ORACLE_BELOW,
            show_open_interest: width >= HIDE_OPEN_INTEREST_BELOW,
            show_funding: width >= HIDE_FUNDING_BELOW,
        }
    }
}

pub(in crate::chart_views::header) fn push_outcome_volume_column<'a>(
    mut header_row: Row<'a, Message>,
    theme: &Theme,
    chart_id: ChartId,
    volume: OutcomeVolume24h,
    time_left: Option<String>,
    as_notional: bool,
    visibility: ChartHeaderMetricVisibility,
) -> Row<'a, Message> {
    if visibility.show_24h_volume {
        header_row = header_row.push(clickable_metric_column(
            outcome_volume_label(as_notional),
            format_outcome_volume(volume, as_notional),
            theme.palette().text,
            theme,
            Message::ToggleOutcomeVolumeNotional(chart_id),
        ));
        if let Some(time_left) = time_left {
            header_row = header_row.push(metric_column(
                "Time Left".to_string(),
                time_left,
                theme.palette().text,
                theme,
            ));
        }
    }
    header_row
}

#[allow(clippy::too_many_arguments)]
pub(in crate::chart_views::header) fn push_outcome_asset_context_columns<'a>(
    mut header_row: Row<'a, Message>,
    theme: &Theme,
    chart_id: ChartId,
    ctx: &'a AssetContext,
    fallback_volume: Option<OutcomeVolume24h>,
    time_left: Option<String>,
    chart_price: f64,
    volume_as_notional: bool,
    open_interest_as_notional: bool,
    visibility: ChartHeaderMetricVisibility,
) -> Row<'a, Message> {
    let base_volume = parse_ctx_f64(ctx.day_base_vlm.as_deref());
    let notional_volume = parse_ctx_f64(ctx.day_ntl_vlm.as_deref());
    let oi = parse_ctx_f64(ctx.open_interest.as_deref());

    if visibility.show_24h_volume {
        header_row = header_row.push(clickable_metric_column(
            outcome_volume_label(volume_as_notional),
            format_outcome_asset_volume(
                base_volume,
                notional_volume,
                fallback_volume,
                volume_as_notional,
            ),
            theme.palette().text,
            theme,
            Message::ToggleOutcomeVolumeNotional(chart_id),
        ));
        if let Some(time_left) = time_left {
            header_row = header_row.push(metric_column(
                "Time Left".to_string(),
                time_left,
                theme.palette().text,
                theme,
            ));
        }
    }

    if visibility.show_open_interest {
        header_row = header_row.push(clickable_metric_column(
            open_interest_label(open_interest_as_notional),
            format_open_interest(oi, chart_price, open_interest_as_notional),
            theme.palette().text,
            theme,
            Message::ToggleOpenInterestNotional(chart_id),
        ));
    }

    header_row
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_perp_metric_columns<'a>(
    mut header_row: Row<'a, Message>,
    theme: &Theme,
    chart_id: ChartId,
    ctx: &'a AssetContext,
    symbol_display: &str,
    chart_price: f64,
    asset_volume_as_notional: bool,
    open_interest_as_notional: bool,
    visibility: ChartHeaderMetricVisibility,
    now_ms: u64,
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
    let base_volume = parse_ctx_f64(ctx.day_base_vlm.as_deref());
    let notional_volume = parse_ctx_f64(ctx.day_ntl_vlm.as_deref());

    if visibility.show_24h_volume {
        header_row = header_row.push(clickable_metric_column(
            asset_volume_label(asset_volume_as_notional),
            format_asset_volume(
                base_volume,
                notional_volume,
                asset_volume_as_notional,
                symbol_display,
            ),
            theme.palette().text,
            theme,
            Message::ToggleAssetVolumeNotional(chart_id),
        ));
    }

    if visibility.show_mark_oracle {
        header_row = header_row.push(metric_column(
            "Mark / Oracle".to_string(),
            format!(
                "{} / {}",
                format_metric_price(mark),
                format_metric_price(oracle)
            ),
            theme.palette().text,
            theme,
        ));
    }

    if visibility.show_funding {
        header_row = header_row.push(metric_column(
            format!("Funding ({})", funding_countdown(now_ms)),
            format_funding_pct(funding),
            funding_color,
            theme,
        ));
    }

    if visibility.show_open_interest {
        header_row = header_row.push(clickable_metric_column(
            open_interest_label(open_interest_as_notional),
            format_open_interest(oi, chart_price, open_interest_as_notional),
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
    chart_id: ChartId,
    ctx: &'a AssetContext,
    symbol_display: &str,
    asset_volume_as_notional: bool,
) -> Row<'a, Message> {
    let base_volume = parse_ctx_f64(ctx.day_base_vlm.as_deref());
    let notional_volume = parse_ctx_f64(ctx.day_ntl_vlm.as_deref());
    header_row = header_row.push(clickable_metric_column(
        asset_volume_label(asset_volume_as_notional),
        format_asset_volume(
            base_volume,
            notional_volume,
            asset_volume_as_notional,
            spot_base_ticker(symbol_display),
        ),
        theme.palette().text,
        theme,
        Message::ToggleAssetVolumeNotional(chart_id),
    ));

    if let Some(mid) = &ctx.mid_px {
        header_row = header_row.push(metric_column(
            "Mid".to_string(),
            format_metric_price(parse_ctx_f64(Some(mid.as_str()))),
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
            .font(crate::app_fonts::monospace_font())
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

fn funding_countdown(now_ms: u64) -> String {
    let now = local_datetime_from_unix_ms(now_ms);
    funding_countdown_from_minute_second(now.minute(), now.second())
}

fn funding_countdown_from_minute_second(minute: u32, second: u32) -> String {
    let seconds_until_next_hour = (60_u32.saturating_sub(minute))
        .saturating_mul(60)
        .saturating_sub(second);
    format!(
        "{:02}:{:02}",
        seconds_until_next_hour / 60,
        seconds_until_next_hour % 60
    )
}

#[cfg(test)]
mod tests;
