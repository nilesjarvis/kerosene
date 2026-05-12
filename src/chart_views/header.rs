mod actions;
mod metrics;
mod symbol;

use self::metrics::push_asset_context_columns;
use crate::app_state::TradingTerminal;
use crate::chart_state::{
    CHART_PRICE_FLASH_MS, ChartId, ChartInstance, PriceFlash, PriceFlashDirection,
};
use crate::message::Message;
use iced::widget::{Space, column, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(crate) fn view_chart_header<'a>(
        &'a self,
        chart_id: ChartId,
        instance: &'a ChartInstance,
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
        let number_color =
            chart_header_price_flash_color(instance.last_price_flash, Self::now_ms(), &theme)
                .unwrap_or(theme.palette().text);

        let sym_btn =
            self.view_chart_symbol_button(chart_id, instance, last.close, number_color, &theme);

        let chg_val = text(format!("{change:+.2} ({change_pct:+.2}%)"))
            .size(12)
            .font(iced::Font::MONOSPACE)
            .color(number_color);
        let col_chg = column![
            text("24h Chg")
                .size(9)
                .color(theme.extended_palette().background.weak.text),
            chg_val
        ]
        .spacing(2);

        let mut header_row = row![sym_btn, Space::new().width(8), col_chg,]
            .spacing(16)
            .align_y(iced::Alignment::Center);

        if let Some(ctx) = &instance.asset_ctx {
            header_row = push_asset_context_columns(
                header_row,
                &theme,
                chart_id,
                ctx,
                last.close,
                instance.open_interest_as_notional,
            );
        }

        header_row = header_row
            .push(Space::new().width(Fill))
            .push(self.view_chart_screenshot_button(chart_id));

        header_row.into()
    }
}

fn chart_header_price_flash_color(
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
