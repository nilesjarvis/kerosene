mod actions;
mod metrics;
mod symbol;

use self::metrics::push_asset_context_columns;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;
use iced::widget::{Space, column, row, text};
use iced::{Element, Fill};

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
        let change_color = self.direction_color(&theme, change);

        let sym_btn =
            self.view_chart_symbol_button(chart_id, instance, last.close, change_color, &theme);

        let chg_val = text(format!("{change:+.2} ({change_pct:+.2}%)"))
            .size(12)
            .font(iced::Font::MONOSPACE)
            .color(change_color);
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
            .push(self.view_chart_screenshot_button(chart_id, &theme));

        header_row.into()
    }
}
