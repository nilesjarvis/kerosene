use super::CandleLayerContext;
use crate::chart::model::CandlestickChart;
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point};

mod price;
mod time_labels;

use time_labels::{TimeAxisLabelMode, format_time_axis_label};

// ---------------------------------------------------------------------------
// Axis and Grid Rendering
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_time_grid<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.step <= 0.0 || !ctx.step.is_finite() {
            return;
        }

        let visible_slots = (ctx.chart_w / ctx.step).ceil() as isize + 1;
        if visible_slots <= 1 {
            return;
        }

        let label_count = 6usize.min(visible_slots as usize);
        if label_count <= 1 {
            return;
        }

        let step_i = (visible_slots / label_count as isize).max(1);
        let left_idx = ctx.right_idx - visible_slots;
        let first_idx = left_idx + first_grid_offset(left_idx, step_i);
        let mut idx = first_idx;
        while idx <= ctx.right_idx {
            let slots_from_right = ctx.right_idx - idx;
            let x = ctx.chart_w - slots_from_right as f32 * ctx.step - ctx.step * 0.5;

            if x <= 0.0 || x >= ctx.chart_w {
                idx += step_i;
                continue;
            }

            let line = canvas::Path::line(Point::new(x, 0.0), Point::new(x, ctx.chart_h));
            frame.stroke(
                &line,
                canvas::Stroke::default()
                    .with_color(Color {
                        a: 0.06,
                        ..ctx.theme.palette().text
                    })
                    .with_width(1.0),
            );
            idx += step_i;
        }
    }

    pub(super) fn draw_time_axis_labels<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.step <= 0.0 || !ctx.step.is_finite() {
            return;
        }

        let visible_slots = (ctx.chart_w / ctx.step).ceil() as isize + 1;
        if visible_slots <= 1 {
            return;
        }

        let label_count = 6usize.min(visible_slots as usize);
        if label_count <= 1 {
            return;
        }

        let label_mode = self
            .visible_time_axis_span(ctx)
            .map(|span| TimeAxisLabelMode::for_timeframe_and_span(self.timeframe, span))
            .unwrap_or(TimeAxisLabelMode::DateTime);
        let step_i = (visible_slots / label_count as isize).max(1);
        let left_idx = ctx.right_idx - visible_slots;
        let first_idx = left_idx + first_grid_offset(left_idx, step_i);
        let mut last_label: Option<String> = None;
        let mut idx = first_idx;
        while idx <= ctx.right_idx {
            let slots_from_right = ctx.right_idx - idx;
            let x = ctx.chart_w - slots_from_right as f32 * ctx.step - ctx.step * 0.5;

            if x <= 0.0 || x >= ctx.chart_w {
                idx += step_i;
                continue;
            }

            let Some(ts_ms) = self.x_to_timestamp(x, ctx.state, ctx.chart_w) else {
                idx += step_i;
                continue;
            };
            let label = format_time_axis_label(ts_ms / 1000, label_mode);
            if label_mode != TimeAxisLabelMode::DateTime
                && last_label.as_deref() == Some(label.as_str())
            {
                idx += step_i;
                continue;
            }

            frame.fill_text(canvas::Text {
                content: label.clone(),
                position: Point::new(x, ctx.chart_h + ctx.funding_panel_h + 4.0),
                color: Color {
                    a: 0.45,
                    ..ctx.theme.palette().text
                },
                size: iced::Pixels(10.0),
                align_x: alignment::Horizontal::Center.into(),
                align_y: alignment::Vertical::Top,
                font: crate::app_fonts::monospace_font(),
                ..canvas::Text::default()
            });

            last_label = Some(label);
            idx += step_i;
        }
    }

    pub(super) fn draw_axis_border<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let axis_border = canvas::Path::line(
            Point::new(ctx.chart_w, 0.0),
            Point::new(ctx.chart_w, ctx.chart_h + ctx.funding_panel_h),
        );
        frame.stroke(
            &axis_border,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.10,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
    }
}

impl CandlestickChart {
    fn visible_time_axis_span<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
    ) -> Option<u64>
    where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let left = self.x_to_timestamp(0.0, ctx.state, ctx.chart_w)?;
        let right = self.x_to_timestamp(ctx.chart_w, ctx.state, ctx.chart_w)?;
        Some(right.abs_diff(left) / 1000)
    }
}

fn first_grid_offset(left_idx: isize, step_i: isize) -> isize {
    let remainder = left_idx.rem_euclid(step_i);
    if remainder == 0 {
        0
    } else {
        step_i - remainder
    }
}

#[cfg(test)]
mod tests;
