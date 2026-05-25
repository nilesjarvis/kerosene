use super::super::super::CandleLayerContext;
use super::super::format_funding_rate_percent;
use crate::chart::model::{CandlestickChart, FUNDING_MODE_BUTTON_WIDTH, FUNDING_MODE_BUTTON_X};

use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// Funding Labels
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(in crate::chart::candle_layer::funding) fn draw_funding_status<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let (label, is_error) = self
            .funding_status
            .as_ref()
            .map(|(label, is_error)| (label.as_str(), *is_error))
            .unwrap_or(("Funding waiting for data", false));
        self.draw_funding_message(ctx, frame, panel_y, label, is_error);
    }

    pub(in crate::chart::candle_layer::funding) fn draw_funding_message<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
        label: &str,
        is_error: bool,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let color = if is_error {
            ctx.theme.palette().danger
        } else {
            Color {
                a: 0.45,
                ..ctx.theme.palette().text
            }
        };
        frame.fill_text(canvas::Text {
            content: label.to_string(),
            position: Point::new(
                FUNDING_MODE_BUTTON_X + FUNDING_MODE_BUTTON_WIDTH + 8.0,
                panel_y + ctx.funding_panel_h * 0.5,
            ),
            color,
            size: iced::Pixels(10.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });
    }

    pub(in crate::chart::candle_layer::funding) fn draw_funding_axis_label<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        y: f32,
        rate: f64,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        frame.fill_text(canvas::Text {
            content: format_funding_rate_percent(rate, self.funding_annualized),
            position: Point::new(ctx.chart_w + 6.0, y),
            color: Color {
                a: 0.42,
                ..ctx.theme.palette().text
            },
            size: iced::Pixels(9.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });
    }
}
