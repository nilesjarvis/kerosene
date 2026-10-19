use crate::account::AssetContext;
use crate::api::OutcomeVolume24h;
use crate::app_time::local_datetime_from_unix_ms;
use crate::chart_state::ChartId;
use crate::message::Message;

use chrono::Timelike;
use iced::Theme;
use iced::widget::{Row, button, column, row, svg, text, tooltip};

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
const HIDE_24H_VOLUME_BELOW: f32 = 420.0;
const HIDE_24H_CHANGE_BELOW: f32 = 340.0;
const CAP_WARNING_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.3 3.5 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.5a2 2 0 0 0-3.4 0Z"/><path d="M12 9v4m0 4h.01"/></svg>"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChartHeaderMetricVisibility {
    pub(crate) show_24h_change: bool,
    pub(crate) show_24h_volume: bool,
    pub(crate) show_mark_oracle: bool,
    pub(crate) show_open_interest: bool,
}

impl ChartHeaderMetricVisibility {
    pub(crate) fn for_width(width: f32) -> Self {
        let width = if width.is_finite() { width } else { 0.0 };
        Self {
            show_24h_change: width >= HIDE_24H_CHANGE_BELOW,
            show_24h_volume: width >= HIDE_24H_VOLUME_BELOW,
            show_mark_oracle: width >= HIDE_MARK_ORACLE_BELOW,
            show_open_interest: width >= HIDE_OPEN_INTEREST_BELOW,
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
    open_interest_cap: Option<f64>,
    at_open_interest_cap: bool,
) -> Row<'a, Message> {
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

    if visibility.show_open_interest {
        header_row = header_row.push(open_interest_column(
            theme,
            chart_id,
            oi,
            chart_price,
            open_interest_as_notional,
            open_interest_cap,
            at_open_interest_cap,
        ));
    }

    header_row
}

fn open_interest_column(
    theme: &Theme,
    chart_id: ChartId,
    oi: Option<f64>,
    chart_price: f64,
    as_notional: bool,
    cap: Option<f64>,
    at_cap: bool,
) -> iced::Element<'static, Message> {
    let label = open_interest_label(as_notional);
    let value = format_open_interest(oi, chart_price, as_notional);
    let content = if at_cap {
        // Custom themes can use a neutral bearish/danger color. Cap warnings
        // stay red, using the built-in danger palette for the background.
        let danger = if theme.extended_palette().is_dark {
            Theme::Dark.palette().danger
        } else {
            Theme::Light.palette().danger
        };
        column![
            text(label).size(9).color(danger),
            row![
                svg(svg::Handle::from_memory(CAP_WARNING_SVG.as_bytes()))
                    .width(12)
                    .height(12)
                    .style(move |_: &Theme, _| svg::Style {
                        color: Some(danger)
                    }),
                text(value)
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .color(danger),
            ]
            .spacing(3)
            .align_y(iced::Alignment::Center)
        ]
        .spacing(2)
    } else {
        metric_column(label, value, theme.palette().text, theme)
    };
    let metric = clickable_metric_button(content, Message::ToggleOpenInterestNotional(chart_id));
    let mut hint = if at_cap {
        "At open interest cap".to_string()
    } else {
        String::new()
    };
    if let Some(cap) = cap.filter(|cap| cap.is_finite() && *cap >= 0.0) {
        if !hint.is_empty() {
            hint.push('\n');
        }
        hint.push_str(&format!(
            "Open interest cap: {}",
            crate::denomination::format_compact_usd(cap)
        ));
    }
    if hint.is_empty() {
        return metric;
    }
    tooltip(metric, text(hint).size(11), tooltip::Position::Bottom)
        .style(iced::widget::container::rounded_box)
        .into()
}

pub(in crate::chart_views::header) fn funding_column(
    theme: &Theme,
    ctx: Option<&AssetContext>,
    now_ms: u64,
) -> iced::widget::Column<'static, Message> {
    let raw_funding = ctx.and_then(|ctx| ctx.funding.as_deref());
    let funding = parse_ctx_f64(raw_funding);
    let color = match funding {
        Some(value) if value >= 0.0 => theme.palette().success,
        Some(_) => theme.palette().danger,
        None => theme.palette().warning,
    };
    metric_column(
        format!("Funding ({})", funding_countdown(now_ms)),
        if raw_funding.is_some() {
            format_funding_pct(funding)
        } else {
            "-".to_string()
        },
        color,
        theme,
    )
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
    clickable_metric_button(metric_column(label, value, value_color, theme), message)
}

fn clickable_metric_button(
    content: iced::widget::Column<'static, Message>,
    message: Message,
) -> iced::Element<'static, Message> {
    button(content)
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
