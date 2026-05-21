mod actions;
mod metrics;
mod symbol;

use self::metrics::{ChartHeaderMetricVisibility, push_asset_context_columns};
use crate::app_state::TradingTerminal;
use crate::chart_state::{
    CHART_PRICE_FLASH_MS, ChartId, ChartInstance, ChartSurfaceId, PriceFlash, PriceFlashDirection,
};
use crate::helpers::format_decimal_with_commas;
use crate::message::Message;
use iced::widget::{Space, column, responsive, row, text};
use iced::{Color, Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(crate) fn view_chart_header<'a>(
        &'a self,
        chart_id: ChartId,
        instance: &'a ChartInstance,
        surface_id: ChartSurfaceId,
    ) -> Element<'a, Message> {
        responsive(move |size| {
            self.view_chart_header_sized(chart_id, instance, surface_id, size.width)
        })
        .height(Length::Shrink)
        .into()
    }

    fn view_chart_header_sized<'a>(
        &'a self,
        chart_id: ChartId,
        instance: &'a ChartInstance,
        surface_id: ChartSurfaceId,
        available_width: f32,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let (Some(last), Some(first)) = (
            instance.chart.candles.last(),
            instance.chart.candles.first(),
        ) else {
            return self.view_chart_placeholder_header(chart_id, instance, &theme);
        };

        let ref_price = instance
            .asset_ctx
            .as_ref()
            .and_then(|ctx| ctx.prev_day_px.as_deref())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(first.open);
        let change = last.close - ref_price;
        let change_pct = if ref_price != 0.0 {
            (change / ref_price) * 100.0
        } else {
            0.0
        };
        let now_ms = Self::now_ms();
        let sym_btn = self.view_chart_symbol_button(
            chart_id,
            instance,
            last.close,
            instance.last_price_flash,
            now_ms,
            &theme,
        );

        let metric_visibility = ChartHeaderMetricVisibility::for_width(available_width);
        let mut header_row = row![sym_btn].spacing(16).align_y(iced::Alignment::Center);

        if metric_visibility.show_24h_change {
            let chg_val = text(format!(
                "{} ({change_pct:+.2}%)",
                format_signed_usd_change(change)
            ))
            .size(12)
            .font(crate::app_fonts::monospace_font())
            .color(theme.palette().text);
            let col_chg = column![
                text("24h Chg")
                    .size(9)
                    .color(theme.extended_palette().background.weak.text),
                chg_val
            ]
            .spacing(2);
            header_row = header_row.push(Space::new().width(8)).push(col_chg);
        }

        if let Some(ctx) = &instance.asset_ctx {
            header_row = push_asset_context_columns(
                header_row,
                &theme,
                chart_id,
                ctx,
                last.close,
                instance.open_interest_as_notional,
                metric_visibility,
            );
        }

        header_row = header_row
            .push(Space::new().width(Fill))
            .push(self.view_chart_screenshot_button(chart_id, surface_id));

        header_row.into()
    }
}

fn format_signed_usd_change(value: f64) -> String {
    if !value.is_finite() {
        return "Invalid data".to_string();
    }
    let sign = if value.is_sign_negative() { "-" } else { "+" };
    format!("{sign}${}", format_decimal_with_commas(value.abs(), 2))
}

pub(super) fn chart_header_price_flash_color(
    flash: Option<PriceFlash>,
    now_ms: u64,
    theme: &Theme,
) -> Option<Color> {
    let flash = flash?;
    let elapsed = now_ms.saturating_sub(flash.started_at_ms);
    if elapsed >= CHART_PRICE_FLASH_MS {
        return None;
    }

    let base = match flash.direction {
        PriceFlashDirection::Up => theme.palette().success,
        PriceFlashDirection::Down => theme.palette().danger,
    };
    let target = theme.palette().text;
    let factor = (elapsed as f32 / CHART_PRICE_FLASH_MS as f32).clamp(0.0, 1.0);

    Some(Color::from_rgba(
        base.r + (target.r - base.r) * factor,
        base.g + (target.g - base.g) * factor,
        base.b + (target.b - base.b) * factor,
        1.0,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChartHeaderChangedText {
    pub(super) before: String,
    pub(super) changed: String,
    pub(super) after: String,
}

pub(super) fn chart_header_changed_text(
    previous: &str,
    current: &str,
) -> Option<ChartHeaderChangedText> {
    if previous == current {
        return None;
    }

    let previous_chars = previous.chars().collect::<Vec<_>>();
    let current_chars = current.chars().collect::<Vec<_>>();

    let prefix_len = previous_chars
        .iter()
        .zip(current_chars.iter())
        .take_while(|(previous, current)| previous == current)
        .count();

    let max_suffix_len = previous_chars
        .len()
        .min(current_chars.len())
        .saturating_sub(prefix_len);
    let suffix_len = previous_chars
        .iter()
        .rev()
        .zip(current_chars.iter().rev())
        .take(max_suffix_len)
        .take_while(|(previous, current)| previous == current)
        .count();

    let changed_end = current_chars.len().saturating_sub(suffix_len);
    let changed = current_chars[prefix_len..changed_end]
        .iter()
        .collect::<String>();
    if changed.is_empty() {
        return None;
    }

    Some(ChartHeaderChangedText {
        before: current_chars[..prefix_len].iter().collect(),
        changed,
        after: current_chars[changed_end..].iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::chart_header_changed_text;

    #[test]
    fn changed_text_highlights_only_changed_decimal_digit() {
        let parts = chart_header_changed_text("82,543.2", "82,543.3").expect("changed text");

        assert_eq!(parts.before, "82,543.");
        assert_eq!(parts.changed, "3");
        assert_eq!(parts.after, "");
    }

    #[test]
    fn changed_text_keeps_shared_suffix_when_middle_digits_change() {
        let parts = chart_header_changed_text("82,543.2", "82,613.2").expect("changed text");

        assert_eq!(parts.before, "82,");
        assert_eq!(parts.changed, "61");
        assert_eq!(parts.after, "3.2");
    }

    #[test]
    fn changed_text_ignores_equal_formatted_prices() {
        assert_eq!(chart_header_changed_text("82,543.2", "82,543.2"), None);
    }
}
