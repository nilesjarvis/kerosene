use super::CandleLayerContext;
use crate::chart::model::CandlestickChart;
use crate::helpers::format_timestamp;
use crate::timeframe::Timeframe;
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// Axis and Grid Rendering
// ---------------------------------------------------------------------------

const MONTH_AXIS_SPAN_SECS: u64 = 90 * 24 * 60 * 60;
const YEAR_AXIS_SPAN_SECS: u64 = 366 * 24 * 60 * 60;

impl CandlestickChart {
    pub(super) fn draw_price_grid<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let grid_steps = 5usize;
        for i in 0..=grid_steps {
            let frac = i as f32 / grid_steps as f32;
            let y = frac * ctx.price_h;
            let line = canvas::Path::line(Point::new(0.0, y), Point::new(ctx.chart_w, y));
            frame.stroke(
                &line,
                canvas::Stroke::default()
                    .with_color(Color {
                        a: 0.06,
                        ..ctx.theme.palette().text
                    })
                    .with_width(1.0),
            );
        }

        let sep = canvas::Path::line(
            Point::new(0.0, ctx.price_h),
            Point::new(ctx.chart_w, ctx.price_h),
        );
        frame.stroke(
            &sep,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.10,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
    }

    pub(super) fn draw_price_axis_labels<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let grid_steps = 5usize;
        for i in 0..=grid_steps {
            let frac = i as f32 / grid_steps as f32;
            let y = frac * ctx.price_h;
            let price_val = if self.inverted {
                ctx.price_lo + (frac as f64) * ctx.price_range
            } else {
                ctx.price_hi - (frac as f64) * ctx.price_range
            };
            frame.fill_text(canvas::Text {
                content: self.display_denomination.format_chart_price(price_val),
                position: Point::new(ctx.chart_w + 6.0, y),
                color: Color {
                    a: 0.45,
                    ..ctx.theme.palette().text
                },
                size: iced::Pixels(11.0),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Center,
                font: iced::Font::MONOSPACE,
                ..canvas::Text::default()
            });
        }
    }

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
                font: iced::Font::MONOSPACE,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimeAxisLabelMode {
    Time,
    DateTime,
    Month,
    MonthYear,
}

impl TimeAxisLabelMode {
    fn for_timeframe_and_span(timeframe: Timeframe, span_secs: u64) -> Self {
        if span_secs >= YEAR_AXIS_SPAN_SECS {
            Self::MonthYear
        } else if span_secs >= MONTH_AXIS_SPAN_SECS {
            Self::Month
        } else if uses_time_only_axis(timeframe) {
            Self::Time
        } else {
            Self::DateTime
        }
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

fn format_time_axis_label(unix_secs: u64, mode: TimeAxisLabelMode) -> String {
    match mode {
        TimeAxisLabelMode::Time => format_time_of_day(unix_secs),
        TimeAxisLabelMode::DateTime => format_timestamp(unix_secs),
        TimeAxisLabelMode::Month => {
            let (_, month, _, _) = timestamp_parts(unix_secs);
            month_name(month).to_string()
        }
        TimeAxisLabelMode::MonthYear => {
            let (year, month, _, _) = timestamp_parts(unix_secs);
            format!("{} {:02}", month_name(month), year % 100)
        }
    }
}

fn uses_time_only_axis(timeframe: Timeframe) -> bool {
    timeframe.duration_ms() <= Timeframe::H1.duration_ms()
}

fn format_time_of_day(unix_secs: u64) -> String {
    let secs_per_day: u64 = 86400;
    let secs_per_hour: u64 = 3600;
    let secs_per_minute: u64 = 60;

    let remaining = unix_secs % secs_per_day;
    let hours = remaining / secs_per_hour;
    let minutes = (remaining % secs_per_hour) / secs_per_minute;

    format!("{hours:02}:{minutes:02}")
}

fn month_name(month: usize) -> &'static str {
    const MONTH_NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    MONTH_NAMES
        .get(month.saturating_sub(1))
        .copied()
        .unwrap_or("Jan")
}

fn timestamp_parts(unix_secs: u64) -> (u64, usize, u64, u64) {
    let secs_per_day: u64 = 86400;
    let secs_per_hour: u64 = 3600;

    let total_days = unix_secs / secs_per_day;
    let remaining = unix_secs % secs_per_day;
    let hours = remaining / secs_per_hour;

    let mut year: u64 = 1970;
    let mut days_left = total_days;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days_left < days_in_year {
            break;
        }
        days_left -= days_in_year;
        year += 1;
    }

    let month_days = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month: usize = 1;
    for (index, &days_in_month) in month_days.iter().enumerate() {
        if days_left < days_in_month {
            month = index + 1;
            break;
        }
        days_left -= days_in_month;
    }

    (year, month, days_left + 1, hours)
}

fn is_leap_year(year: u64) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

#[cfg(test)]
mod tests {
    use super::{TimeAxisLabelMode, format_time_axis_label};
    use crate::timeframe::Timeframe;

    #[test]
    fn low_timeframe_axis_labels_use_time_only() {
        assert_eq!(
            format_time_axis_label(1_714_566_840, TimeAxisLabelMode::Time),
            "12:34"
        );
    }

    #[test]
    fn monthly_axis_labels_use_month_names() {
        assert_eq!(
            format_time_axis_label(1_714_521_600, TimeAxisLabelMode::Month),
            "May"
        );
        assert_eq!(
            format_time_axis_label(1_714_521_600, TimeAxisLabelMode::MonthYear),
            "May 24"
        );
    }

    #[test]
    fn axis_label_mode_switches_to_months_for_wide_views() {
        assert_eq!(
            TimeAxisLabelMode::for_timeframe_and_span(Timeframe::M15, 30 * 24 * 60 * 60),
            TimeAxisLabelMode::Time
        );
        assert_eq!(
            TimeAxisLabelMode::for_timeframe_and_span(Timeframe::H4, 30 * 24 * 60 * 60),
            TimeAxisLabelMode::DateTime
        );
        assert_eq!(
            TimeAxisLabelMode::for_timeframe_and_span(Timeframe::M15, 120 * 24 * 60 * 60),
            TimeAxisLabelMode::Month
        );
        assert_eq!(
            TimeAxisLabelMode::for_timeframe_and_span(Timeframe::M15, 400 * 24 * 60 * 60),
            TimeAxisLabelMode::MonthYear
        );
    }
}
