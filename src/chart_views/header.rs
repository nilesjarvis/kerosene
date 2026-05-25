mod actions;
mod feedback;
mod metrics;
mod symbol;

use self::feedback::format_signed_usd_change;
use self::metrics::{
    ChartHeaderMetricVisibility, push_asset_context_columns, push_outcome_volume_column,
};
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId};
use crate::helpers::parse_finite_number;
use crate::message::Message;
use iced::widget::{Space, column, responsive, row, text};
use iced::{Element, Fill, Length};

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

        let ref_price = chart_reference_price(
            instance
                .asset_ctx
                .as_ref()
                .and_then(|ctx| ctx.prev_day_px.as_deref()),
            first.open,
        );
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

        if self.is_outcome_coin(&instance.symbol)
            && let Some(volume) = self.outcome_volumes_24h.get(&instance.symbol)
        {
            header_row = push_outcome_volume_column(
                header_row,
                &theme,
                chart_id,
                *volume,
                instance.outcome_volume_as_notional,
                metric_visibility,
            );
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

fn chart_reference_price(prev_day_px: Option<&str>, fallback: f64) -> f64 {
    prev_day_px
        .and_then(parse_finite_number)
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests;
