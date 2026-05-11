use super::model::CandlestickChart;
use super::state::ChartState;
use super::tooltips::TooltipSurface;
use crate::helpers::format_price;
use iced::widget::canvas;
use iced::{Color, Point, Theme, alignment};

mod measurement;
mod range;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Crosshair Overlay
// ---------------------------------------------------------------------------

pub(super) struct CrosshairOverlayContext<'a, PriceToY>
where
    PriceToY: Fn(f64) -> f32,
{
    pub(super) frame: &'a mut canvas::Frame,
    pub(super) state: &'a ChartState,
    pub(super) theme: &'a Theme,
    pub(super) chart_w: f32,
    pub(super) chart_h: f32,
    pub(super) funding_panel_h: f32,
    pub(super) price_h: f32,
    pub(super) price_hi: f64,
    pub(super) price_range: f64,
    pub(super) heatmap_stride: usize,
    pub(super) step: f32,
    pub(super) price_to_y: &'a PriceToY,
}

impl CandlestickChart {
    pub(super) fn draw_crosshair_overlay<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(pos) = ctx.state.cursor_position else {
            return;
        };
        let drawable_h = ctx.chart_h + ctx.funding_panel_h;
        if pos.x >= ctx.chart_w || pos.y >= drawable_h {
            return;
        }

        let h_line = canvas::Path::line(Point::new(0.0, pos.y), Point::new(ctx.chart_w, pos.y));
        let v_line = canvas::Path::line(Point::new(pos.x, 0.0), Point::new(pos.x, drawable_h));
        let stroke = canvas::Stroke::default()
            .with_color(Color {
                a: 0.25,
                ..ctx.theme.palette().text
            })
            .with_width(0.5);
        ctx.frame.stroke(&h_line, stroke);
        ctx.frame.stroke(&v_line, stroke);

        if ctx.funding_panel_h > 0.0 && pos.y >= ctx.chart_h {
            let mut tooltip_surface =
                TooltipSurface::new(ctx.frame, ctx.theme, pos, ctx.chart_w, ctx.price_h);
            tooltip_surface.draw_funding_hover(
                &self.funding_rates,
                ctx.chart_h,
                ctx.funding_panel_h,
                self.funding_annualized,
                |point| self.timestamp_to_x(point.time_ms, ctx.state, ctx.chart_w),
            );
            return;
        }

        if let Some(idx) = self.x_to_candle_index(pos.x, ctx.state, ctx.chart_w) {
            let volume = self.candles[idx].volume;
            ctx.frame.fill_text(canvas::Text {
                content: format!("Vol: {}", format_volume_compact(volume)),
                position: Point::new(6.0, ctx.price_h + 2.0),
                color: ctx.theme.palette().text,
                size: iced::Pixels(11.0),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Top,
                font: iced::Font::MONOSPACE,
                ..canvas::Text::default()
            });
        }

        if pos.y > ctx.price_h || ctx.price_range <= 0.0 {
            return;
        }

        let hover_price = self.y_to_price_with(pos.y, ctx.price_hi, ctx.price_range, ctx.price_h);

        self.draw_range_measurement(ctx, pos, hover_price);

        ctx.frame.fill_text(canvas::Text {
            content: format_price(hover_price),
            position: Point::new(ctx.chart_w + 6.0, pos.y),
            color: Color::WHITE,
            size: iced::Pixels(11.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });

        let mut tooltip_surface =
            TooltipSurface::new(ctx.frame, ctx.theme, pos, ctx.chart_w, ctx.price_h);
        tooltip_surface.draw_liquidation_hover(
            hover_price,
            ctx.price_range,
            &self.liquidation_buckets,
            ctx.price_to_y,
        );
        tooltip_surface.draw_heatmap_hover(
            &self.heatmap_rects,
            ctx.heatmap_stride,
            self.heatmap_max_usd,
            |rect| self.heatmap_x_bounds(rect, ctx.state, ctx.chart_w, ctx.step),
            ctx.price_to_y,
        );
    }
}

pub(super) fn format_volume_compact(volume: f64) -> String {
    if !volume.is_finite() || volume <= 0.0 {
        return "0".to_string();
    }
    if volume >= 1_000_000_000.0 {
        format!("{:.2}B", volume / 1_000_000_000.0)
    } else if volume >= 1_000_000.0 {
        format!("{:.2}M", volume / 1_000_000.0)
    } else if volume >= 1_000.0 {
        format!("{:.1}K", volume / 1_000.0)
    } else if volume >= 1.0 {
        format!("{volume:.2}")
    } else {
        format!("{volume:.4}")
    }
}
