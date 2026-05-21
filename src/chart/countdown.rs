use super::model::CandlestickChart;
use super::tooltips::{TooltipLine, TooltipSurface};
use crate::timeframe::Timeframe;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Theme, alignment};

// ---------------------------------------------------------------------------
// Next Candle Countdown
// ---------------------------------------------------------------------------

const COUNTDOWN_BADGE_TEXT_SIZE: f32 = 10.0;
const COUNTDOWN_AXIS_PADDING_X: f32 = 6.0;
const COUNTDOWN_TOOLTIP_W: f32 = 156.0;
const COUNTDOWN_TOOLTIP_H: f32 = 28.0;
const COUNTDOWN_TOOLTIP_GAP: f32 = 6.0;

impl CandlestickChart {
    pub(super) fn draw_next_candle_countdown(
        &self,
        frame: &mut canvas::Frame,
        theme: &Theme,
        chart_w: f32,
        drawable_h: f32,
        bounds: Rectangle,
        cursor_position: Option<Point>,
    ) {
        let Some(last_open_ms) = self.candles.last().map(|candle| candle.open_time) else {
            return;
        };
        let Some(label) =
            next_candle_countdown_label(last_open_ms, self.timeframe, self.clock_now_ms)
        else {
            return;
        };

        let axis_corner_w = bounds.width - chart_w;
        let axis_corner_h = bounds.height - drawable_h;
        if chart_w <= 0.0 || drawable_h <= 0.0 || axis_corner_w <= 0.0 || axis_corner_h <= 0.0 {
            return;
        }

        frame.fill_text(canvas::Text {
            content: label,
            position: Point::new(
                bounds.width - COUNTDOWN_AXIS_PADDING_X,
                drawable_h + axis_corner_h * 0.5,
            ),
            color: Color {
                a: 0.55,
                ..theme.palette().text
            },
            size: iced::Pixels(COUNTDOWN_BADGE_TEXT_SIZE),
            align_x: alignment::Horizontal::Right.into(),
            align_y: alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });

        if !cursor_position.is_some_and(|pos| {
            point_in_axis_corner(pos, chart_w, drawable_h, axis_corner_w, axis_corner_h)
        }) {
            return;
        }

        let origin = Point::new(
            (chart_w - COUNTDOWN_TOOLTIP_W - COUNTDOWN_TOOLTIP_GAP).max(0.0),
            (drawable_h - COUNTDOWN_TOOLTIP_H - COUNTDOWN_TOOLTIP_GAP).max(0.0),
        );
        let mut tooltip = TooltipSurface::new(frame, theme, origin, chart_w, drawable_h.max(0.0));
        tooltip.draw_card(
            origin,
            iced::Size::new(COUNTDOWN_TOOLTIP_W, COUNTDOWN_TOOLTIP_H),
            &[TooltipLine {
                content: format!("Next {} candle", self.timeframe.label()),
                color: theme.palette().text,
            }],
        );
    }
}

fn point_in_axis_corner(
    point: Point,
    chart_w: f32,
    drawable_h: f32,
    axis_corner_w: f32,
    axis_corner_h: f32,
) -> bool {
    point.x >= chart_w
        && point.x <= chart_w + axis_corner_w
        && point.y >= drawable_h
        && point.y <= drawable_h + axis_corner_h
}

fn next_candle_countdown_label(
    last_open_ms: u64,
    timeframe: Timeframe,
    now_ms: u64,
) -> Option<String> {
    remaining_ms_until_next_candle(last_open_ms, timeframe.duration_ms(), now_ms)
        .map(format_candle_countdown)
}

fn remaining_ms_until_next_candle(last_open_ms: u64, interval_ms: u64, now_ms: u64) -> Option<u64> {
    if interval_ms == 0 {
        return None;
    }

    if now_ms < last_open_ms {
        return Some(last_open_ms - now_ms);
    }

    let elapsed = now_ms - last_open_ms;
    if elapsed == 0 {
        return Some(interval_ms);
    }

    let remainder = elapsed % interval_ms;
    if remainder == 0 {
        Some(0)
    } else {
        Some(interval_ms - remainder)
    }
}

fn format_candle_countdown(remaining_ms: u64) -> String {
    let seconds = remaining_ms.div_ceil(1_000);
    if seconds < 60 {
        return format!("{seconds}s");
    }

    let minutes = seconds / 60;
    let seconds_part = seconds % 60;
    if minutes < 60 {
        return format!("{minutes}m {seconds_part}s");
    }

    let hours = minutes / 60;
    let minutes_part = minutes % 60;
    if hours < 24 {
        return format!("{hours}h {minutes_part}m");
    }

    let days = hours / 24;
    let hours_part = hours % 24;
    format!("{days}d {hours_part}h")
}

#[cfg(test)]
mod tests {
    use super::{
        format_candle_countdown, next_candle_countdown_label, point_in_axis_corner,
        remaining_ms_until_next_candle,
    };
    use crate::timeframe::Timeframe;
    use iced::Point;

    #[test]
    fn countdown_uses_last_candle_open_as_anchor() {
        let last_open = 1_000_000;
        let interval = Timeframe::M1.duration_ms();

        assert_eq!(
            remaining_ms_until_next_candle(last_open, interval, last_open),
            Some(60_000)
        );
        assert_eq!(
            remaining_ms_until_next_candle(last_open, interval, last_open + 15_000),
            Some(45_000)
        );
        assert_eq!(
            remaining_ms_until_next_candle(last_open, interval, last_open + 60_000),
            Some(0)
        );
        assert_eq!(
            remaining_ms_until_next_candle(last_open, interval, last_open + 60_001),
            Some(59_999)
        );
    }

    #[test]
    fn candle_countdown_label_is_compact() {
        assert_eq!(
            next_candle_countdown_label(1_000_000, Timeframe::M5, 1_060_000),
            Some("4m 0s".to_string())
        );
        assert_eq!(format_candle_countdown(42_000), "42s");
        assert_eq!(format_candle_countdown(90_000), "1m 30s");
        assert_eq!(format_candle_countdown(3_660_000), "1h 1m");
        assert_eq!(format_candle_countdown(90_000_000), "1d 1h");
    }

    #[test]
    fn hover_target_is_the_axis_corner_only() {
        assert!(point_in_axis_corner(
            Point::new(420.0, 276.0),
            400.0,
            260.0,
            70.0,
            24.0,
        ));
        assert!(!point_in_axis_corner(
            Point::new(399.0, 276.0),
            400.0,
            260.0,
            70.0,
            24.0,
        ));
        assert!(!point_in_axis_corner(
            Point::new(420.0, 259.0),
            400.0,
            260.0,
            70.0,
            24.0,
        ));
    }
}
