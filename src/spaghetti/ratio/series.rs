use super::{PairRatioRenderContext, RatioCandle};
use iced::widget::canvas;
use iced::{Color, Point, Size, Theme};

// ---------------------------------------------------------------------------
// Pair Ratio Series Drawing
// ---------------------------------------------------------------------------

pub(super) fn draw_ratio_candles(
    frame: &mut canvas::Frame,
    ctx: &PairRatioRenderContext<'_>,
    ratio_candles: &[RatioCandle],
    ratio_to_y: &impl Fn(f64) -> f32,
    theme: &Theme,
) {
    let step_px = if ratio_candles.len() >= 2 {
        let mut sum = 0.0_f32;
        let mut n = 0usize;
        for window in ratio_candles.windows(2) {
            sum += (window[1].x - window[0].x).abs();
            n += 1;
        }
        if n > 0 { sum / n as f32 } else { 6.0 }
    } else {
        6.0
    };
    let body_w = (step_px * 0.62).clamp(2.0, 14.0);

    for candle in ratio_candles {
        let px = candle.x.clamp(-10.0, ctx.chart_w + 10.0);
        let y_open = ratio_to_y(candle.open).clamp(-50.0, ctx.chart_h + 50.0);
        let y_high = ratio_to_y(candle.high).clamp(-50.0, ctx.chart_h + 50.0);
        let y_low = ratio_to_y(candle.low).clamp(-50.0, ctx.chart_h + 50.0);
        let y_close = ratio_to_y(candle.close).clamp(-50.0, ctx.chart_h + 50.0);
        let bullish = candle.close >= candle.open;
        let color = if bullish {
            theme.palette().success
        } else {
            theme.palette().danger
        };

        let wick = canvas::Path::line(Point::new(px, y_high), Point::new(px, y_low));
        frame.stroke(
            &wick,
            canvas::Stroke::default().with_color(color).with_width(1.0),
        );

        let top = y_open.min(y_close);
        let bottom = y_open.max(y_close);
        let body_h = (bottom - top).max(1.0);
        frame.fill_rectangle(
            Point::new(px - body_w * 0.5, top),
            Size::new(body_w, body_h),
            color,
        );
    }
}

pub(super) fn draw_ratio_line(
    frame: &mut canvas::Frame,
    ctx: &PairRatioRenderContext<'_>,
    ratio_candles: &[RatioCandle],
    ratio_to_y: &impl Fn(f64) -> f32,
    color: Color,
) {
    let mut path = canvas::path::Builder::new();
    for (i, candle) in ratio_candles.iter().enumerate() {
        let px = candle.x.clamp(-10.0, ctx.chart_w + 10.0);
        let py = ratio_to_y(candle.close).clamp(-50.0, ctx.chart_h + 50.0);
        if i == 0 {
            path.move_to(Point::new(px, py));
        } else {
            path.line_to(Point::new(px, py));
        }
    }
    frame.stroke(
        &path.build(),
        canvas::Stroke::default().with_color(color).with_width(1.8),
    );
}
