use super::CandleLayerContext;
use crate::chart::model::{
    CandlestickChart, FUNDING_PLOT_BOTTOM_PADDING, FUNDING_PLOT_TOP_PADDING,
};
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Size};

mod chrome;
mod range;

#[cfg(test)]
use range::FundingDisplayRange;
pub(in crate::chart) use range::format_funding_rate_percent;

// ---------------------------------------------------------------------------
// Funding Rate Panel Rendering
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_funding_panel<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.funding_panel_h <= 0.0 {
            return;
        }

        let panel_y = ctx.chart_h;
        ctx.fisheye.fill_projected_rect_without_edge_blur(
            frame,
            Point::new(0.0, panel_y),
            Size::new(ctx.chart_w, ctx.funding_panel_h),
            Color {
                a: 0.025,
                ..ctx.theme.palette().text
            },
        );

        ctx.fisheye.stroke_projected_line(
            frame,
            Point::new(0.0, panel_y),
            Point::new(ctx.chart_w, panel_y),
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.14,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
        let plot_top = panel_y + FUNDING_PLOT_TOP_PADDING;
        let plot_bottom = panel_y + ctx.funding_panel_h - FUNDING_PLOT_BOTTOM_PADDING;
        if plot_bottom <= plot_top {
            self.draw_funding_panel_chrome(ctx, frame, panel_y);
            return;
        }

        let visible: Vec<_> = self
            .funding_rates
            .iter()
            .filter_map(|point| {
                let x = self.timestamp_to_x(point.time_ms, ctx.state, ctx.chart_w)?;
                (x >= -ctx.step && x <= ctx.chart_w + ctx.step).then_some((x, point))
            })
            .collect();

        if visible.is_empty() {
            if self.funding_rates.is_empty() {
                self.draw_funding_status(ctx, frame, panel_y);
            } else {
                self.draw_funding_message(ctx, frame, panel_y, "No funding in view", false);
            }
            self.draw_funding_panel_chrome(ctx, frame, panel_y);
            return;
        }

        let max_abs = visible
            .iter()
            .map(|(_, point)| self.display_funding_rate(point.rate).abs())
            .fold(0.0_f64, f64::max);
        if max_abs <= 0.0 {
            self.draw_funding_message(ctx, frame, panel_y, "Funding flat", false);
            self.draw_funding_panel_chrome(ctx, frame, panel_y);
            return;
        }

        let Some(display_range) = self.funding_display_range_from_max_abs(max_abs, ctx.state)
        else {
            self.draw_funding_message(ctx, frame, panel_y, "Funding flat", false);
            self.draw_funding_panel_chrome(ctx, frame, panel_y);
            return;
        };
        let baseline_y = display_range.rate_to_y(0.0, plot_top, plot_bottom);
        if baseline_y >= plot_top && baseline_y <= plot_bottom {
            ctx.fisheye.stroke_projected_line(
                frame,
                Point::new(0.0, baseline_y),
                Point::new(ctx.chart_w, baseline_y),
                canvas::Stroke::default()
                    .with_color(Color {
                        a: 0.10,
                        ..ctx.theme.palette().text
                    })
                    .with_width(1.0),
            );
        }

        let bar_w = (ctx.step * 0.34).clamp(1.0, 4.0);
        let stride = (visible.len() / 2_000).max(1);

        frame.with_clip(
            Rectangle {
                x: 0.0,
                y: plot_top,
                width: ctx.chart_w,
                height: plot_bottom - plot_top,
            },
            |frame| {
                for (x, point) in visible.iter().step_by(stride).copied() {
                    let display_rate = self.display_funding_rate(point.rate);
                    let y = display_range.rate_to_y(display_rate, plot_top, plot_bottom);
                    let top = y.min(baseline_y);
                    let height = (baseline_y - y).abs().max(1.0);
                    let color = if display_rate >= 0.0 {
                        ctx.candle_bull_color
                    } else {
                        ctx.candle_bear_color
                    };
                    ctx.fisheye.fill_projected_rect_without_edge_blur(
                        frame,
                        Point::new(x - bar_w * 0.5, top),
                        Size::new(bar_w, height),
                        Color { a: 0.78, ..color },
                    );
                }
            },
        );

        self.draw_funding_axis_label(ctx, frame, plot_top, display_range.hi);
        if baseline_y >= plot_top + 8.0 && baseline_y <= plot_bottom - 8.0 {
            self.draw_funding_axis_label(ctx, frame, baseline_y, 0.0);
        }
        self.draw_funding_axis_label(ctx, frame, plot_bottom, display_range.lo);
        self.draw_funding_panel_chrome(ctx, frame, panel_y);
    }
}

#[cfg(test)]
mod tests;
