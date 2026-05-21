use crate::chart::{CandlestickChart, TIME_AXIS_HEIGHT};
use crate::message::Message;
use iced::widget::canvas;
use iced::widget::container as container_style;
use iced::widget::container;
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme};

// ---------------------------------------------------------------------------
// Chart Skeleton Loader
// ---------------------------------------------------------------------------

const GRID_LINE_COUNT: usize = 5;
const PRICE_AXIS_LABEL_COUNT: usize = 6;
const TIME_AXIS_TICK_COUNT: usize = 5;
const SKELETON_CANDLE_COUNT: usize = 64;
const MIN_CANDLE_SPACING: f32 = 8.0;
const FUNDING_PANEL_MIN_HEIGHT: f32 = 44.0;
const FUNDING_PANEL_MAX_HEIGHT: f32 = 160.0;
const SHIMMER_MIN_WIDTH: f32 = 80.0;
const SHIMMER_MAX_WIDTH: f32 = 220.0;

#[derive(Debug, Clone, Copy)]
struct SkeletonCandle {
    open: f32,
    high: f32,
    low: f32,
    close: f32,
    volume: f32,
}

impl SkeletonCandle {
    const fn new(open: f32, high: f32, low: f32, close: f32, volume: f32) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume,
        }
    }
}

// Shape-only data normalized from a real BTC 1h Hyperliquid candleSnapshot.
const API_SAMPLE_CANDLES: [SkeletonCandle; 64] = [
    SkeletonCandle::new(0.3670, 0.5056, 0.3114, 0.4671, 0.4684),
    SkeletonCandle::new(0.4671, 0.5310, 0.3885, 0.4553, 0.4327),
    SkeletonCandle::new(0.4553, 0.4944, 0.3885, 0.4192, 0.2574),
    SkeletonCandle::new(0.4192, 0.6218, 0.3943, 0.4871, 0.5044),
    SkeletonCandle::new(0.4871, 0.4939, 0.2484, 0.2489, 0.6007),
    SkeletonCandle::new(0.2489, 0.4143, 0.1552, 0.2289, 0.4659),
    SkeletonCandle::new(0.2289, 0.3494, 0.2089, 0.3080, 0.2692),
    SkeletonCandle::new(0.3084, 0.3914, 0.2689, 0.3807, 0.3944),
    SkeletonCandle::new(0.3807, 0.4285, 0.2416, 0.3714, 0.2962),
    SkeletonCandle::new(0.3719, 0.4685, 0.3592, 0.4290, 0.2225),
    SkeletonCandle::new(0.4295, 0.5559, 0.3997, 0.5242, 0.3286),
    SkeletonCandle::new(0.5242, 0.5691, 0.3982, 0.4368, 0.3833),
    SkeletonCandle::new(0.4368, 0.4480, 0.2655, 0.3226, 0.4907),
    SkeletonCandle::new(0.3221, 0.4685, 0.2704, 0.4353, 0.4559),
    SkeletonCandle::new(0.4353, 0.4568, 0.2304, 0.2777, 0.4372),
    SkeletonCandle::new(0.2777, 0.4095, 0.2421, 0.3860, 0.3003),
    SkeletonCandle::new(0.3865, 0.4456, 0.2143, 0.3519, 0.6571),
    SkeletonCandle::new(0.3519, 0.4192, 0.0000, 0.1230, 0.7945),
    SkeletonCandle::new(0.1235, 0.2616, 0.0937, 0.1776, 0.5052),
    SkeletonCandle::new(0.1781, 0.3812, 0.1274, 0.3641, 0.5301),
    SkeletonCandle::new(0.3646, 0.4251, 0.2967, 0.3426, 0.4457),
    SkeletonCandle::new(0.3426, 0.3904, 0.2879, 0.3182, 0.3788),
    SkeletonCandle::new(0.3187, 0.4539, 0.2997, 0.3255, 0.3199),
    SkeletonCandle::new(0.3255, 0.4344, 0.3255, 0.4241, 0.3165),
    SkeletonCandle::new(0.4246, 0.4353, 0.3992, 0.4056, 0.2717),
    SkeletonCandle::new(0.4061, 0.4061, 0.2377, 0.2753, 0.4787),
    SkeletonCandle::new(0.2753, 0.4046, 0.2533, 0.3324, 0.2747),
    SkeletonCandle::new(0.3328, 0.3651, 0.1781, 0.3099, 0.4228),
    SkeletonCandle::new(0.3104, 0.3714, 0.1855, 0.3216, 0.4413),
    SkeletonCandle::new(0.3221, 0.3685, 0.2172, 0.2596, 0.3006),
    SkeletonCandle::new(0.2601, 0.3309, 0.2191, 0.2767, 0.2703),
    SkeletonCandle::new(0.2772, 0.3294, 0.2626, 0.3045, 0.4399),
    SkeletonCandle::new(0.3050, 0.5730, 0.3050, 0.5115, 0.4767),
    SkeletonCandle::new(0.5115, 0.6179, 0.5076, 0.6144, 0.3555),
    SkeletonCandle::new(0.6149, 0.6423, 0.5281, 0.5481, 0.2790),
    SkeletonCandle::new(0.5486, 0.7277, 0.5144, 0.6408, 0.4587),
    SkeletonCandle::new(0.6408, 0.6808, 0.5930, 0.6496, 0.3123),
    SkeletonCandle::new(0.6496, 0.7374, 0.6491, 0.6652, 0.3106),
    SkeletonCandle::new(0.6657, 0.6916, 0.5671, 0.6022, 0.3153),
    SkeletonCandle::new(0.6022, 0.7794, 0.5905, 0.7189, 0.4833),
    SkeletonCandle::new(0.7189, 0.7374, 0.3943, 0.5173, 0.6661),
    SkeletonCandle::new(0.5178, 0.6940, 0.3543, 0.6457, 1.0000),
    SkeletonCandle::new(0.6442, 0.8365, 0.5735, 0.6413, 0.8970),
    SkeletonCandle::new(0.6413, 0.6925, 0.4334, 0.5857, 0.5142),
    SkeletonCandle::new(0.5857, 0.7706, 0.5354, 0.6979, 0.4651),
    SkeletonCandle::new(0.6984, 0.7321, 0.6101, 0.6208, 0.3340),
    SkeletonCandle::new(0.6213, 0.7716, 0.5876, 0.7423, 0.3833),
    SkeletonCandle::new(0.7423, 0.8009, 0.6486, 0.7716, 0.6801),
    SkeletonCandle::new(0.7721, 0.8019, 0.6657, 0.6657, 0.3557),
    SkeletonCandle::new(0.6657, 0.6657, 0.5271, 0.6633, 0.5064),
    SkeletonCandle::new(0.6628, 0.6906, 0.6052, 0.6872, 0.2847),
    SkeletonCandle::new(0.6872, 0.9419, 0.6711, 0.8931, 0.6117),
    SkeletonCandle::new(0.8931, 1.0000, 0.8106, 0.8404, 0.6040),
    SkeletonCandle::new(0.8399, 0.9614, 0.8219, 0.8853, 0.3800),
    SkeletonCandle::new(0.8853, 0.9566, 0.8555, 0.9517, 0.3943),
    SkeletonCandle::new(0.9512, 0.9922, 0.8838, 0.9200, 0.5081),
    SkeletonCandle::new(0.9200, 0.9722, 0.7687, 0.8180, 0.4308),
    SkeletonCandle::new(0.8175, 0.8424, 0.6613, 0.7125, 0.4997),
    SkeletonCandle::new(0.7125, 0.8536, 0.6593, 0.8433, 0.3939),
    SkeletonCandle::new(0.8433, 0.9917, 0.8429, 0.8604, 0.5842),
    SkeletonCandle::new(0.8599, 0.8907, 0.7213, 0.7218, 0.5668),
    SkeletonCandle::new(0.7213, 0.7438, 0.4929, 0.5041, 0.7446),
    SkeletonCandle::new(0.5041, 0.5959, 0.4788, 0.4988, 0.5069),
    SkeletonCandle::new(0.4993, 0.5939, 0.4441, 0.5544, 0.4585),
];

pub(super) fn chart_skeleton_overlay(
    chart: &CandlestickChart,
    phase: f32,
) -> Element<'static, Message> {
    let price_axis_width = chart.price_axis_width();
    let funding_panel_height = chart
        .macro_indicators
        .show_funding_rate
        .then_some(chart.funding_panel_height);

    container(
        iced::widget::canvas(ChartSkeleton {
            phase,
            price_axis_width,
            funding_panel_height,
        })
        .width(Fill)
        .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(|theme: &Theme| container_style::Style {
        background: Some(
            Color {
                a: 0.86,
                ..theme.extended_palette().background.strong.color
            }
            .into(),
        ),
        ..Default::default()
    })
    .into()
}

struct ChartSkeleton {
    phase: f32,
    price_axis_width: f32,
    funding_panel_height: Option<f32>,
}

impl canvas::Program<Message> for ChartSkeleton {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let width = bounds.width.max(0.0);
        let height = bounds.height.max(0.0);
        if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
            return vec![frame.into_geometry()];
        }

        let palette = SkeletonPalette::new(theme);
        frame.fill_rectangle(Point::ORIGIN, Size::new(width, height), palette.background);

        let price_axis_w = if width >= 52.0 {
            self.price_axis_width.clamp(52.0, width.min(96.0))
        } else {
            width
        };
        let chart_w = (width - price_axis_w).max(0.0);
        let available_chart_h = (height - TIME_AXIS_HEIGHT).max(0.0);
        let funding_h = self
            .funding_panel_height
            .map(|height| {
                height
                    .clamp(FUNDING_PANEL_MIN_HEIGHT, FUNDING_PANEL_MAX_HEIGHT)
                    .min((available_chart_h * 0.35).max(0.0))
            })
            .unwrap_or(0.0);
        let main_h = (available_chart_h - funding_h).max(0.0);

        if chart_w <= 0.0 || main_h <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let shimmer = Shimmer::new(width, self.phase, &palette);
        draw_chart_grid(&mut frame, chart_w, main_h, &palette);
        draw_skeleton_candles(&mut frame, chart_w, main_h, &palette);
        draw_price_axis(&mut frame, width, price_axis_w, main_h, &palette);

        if funding_h > 0.0 {
            draw_funding_panel(&mut frame, chart_w, main_h, funding_h, &palette);
        }

        draw_time_axis(
            &mut frame,
            chart_w,
            main_h + funding_h,
            TIME_AXIS_HEIGHT,
            &palette,
        );
        draw_axis_borders(&mut frame, chart_w, main_h, funding_h, height, &palette);
        draw_skeleton_candles_shimmer(&mut frame, chart_w, main_h, &shimmer);
        draw_price_axis_shimmer(&mut frame, width, price_axis_w, main_h, &shimmer);
        if funding_h > 0.0 {
            draw_funding_panel_shimmer(&mut frame, chart_w, main_h, funding_h, &shimmer);
        }
        draw_time_axis_shimmer(
            &mut frame,
            chart_w,
            main_h + funding_h,
            TIME_AXIS_HEIGHT,
            &shimmer,
        );

        vec![frame.into_geometry()]
    }
}

struct SkeletonPalette {
    background: Color,
    grid: Color,
    axis: Color,
    axis_label: Color,
    candle: Color,
    volume: Color,
    funding: Color,
    shimmer: Color,
}

impl SkeletonPalette {
    fn new(theme: &Theme) -> Self {
        let extended = theme.extended_palette();
        let neutral = extended.background.weak.text;

        Self {
            background: Color {
                a: 0.58,
                ..extended.background.strong.color
            },
            grid: Color { a: 0.11, ..neutral },
            axis: Color { a: 0.20, ..neutral },
            axis_label: Color { a: 0.20, ..neutral },
            candle: Color { a: 0.26, ..neutral },
            volume: Color { a: 0.15, ..neutral },
            funding: Color { a: 0.17, ..neutral },
            shimmer: Color { a: 0.18, ..neutral },
        }
    }
}

struct Shimmer {
    center_x: f32,
    half_width: f32,
    color: Color,
}

impl Shimmer {
    fn new(width: f32, phase: f32, palette: &SkeletonPalette) -> Self {
        let band_w = (width * 0.24).clamp(SHIMMER_MIN_WIDTH, SHIMMER_MAX_WIDTH);
        let progress = phase.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
        let travel_w = width + band_w * 2.0;

        Self {
            center_x: progress * travel_w - band_w,
            half_width: band_w * 0.5,
            color: palette.shimmer,
        }
    }

    fn color_at(&self, x: f32) -> Option<Color> {
        let distance = (x - self.center_x).abs();
        if distance >= self.half_width {
            return None;
        }

        let strength = 1.0 - distance / self.half_width;
        Some(Color {
            a: self.color.a * strength,
            ..self.color
        })
    }
}

fn draw_chart_grid(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    palette: &SkeletonPalette,
) {
    let stroke = canvas::Stroke::default()
        .with_color(palette.grid)
        .with_width(1.0);

    for idx in 1..=GRID_LINE_COUNT {
        let y = chart_h * idx as f32 / (GRID_LINE_COUNT + 1) as f32;
        frame.stroke(
            &canvas::Path::line(Point::new(0.0, y), Point::new(chart_w, y)),
            stroke,
        );
    }

    for idx in 1..=TIME_AXIS_TICK_COUNT {
        let x = chart_w * idx as f32 / (TIME_AXIS_TICK_COUNT + 1) as f32;
        frame.stroke(
            &canvas::Path::line(Point::new(x, 0.0), Point::new(x, chart_h)),
            stroke,
        );
    }
}

fn draw_skeleton_candles(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    palette: &SkeletonPalette,
) {
    draw_skeleton_candle_marks(
        frame,
        chart_w,
        chart_h,
        palette.candle,
        palette.volume,
        None,
    );
}

fn draw_skeleton_candles_shimmer(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    shimmer: &Shimmer,
) {
    draw_skeleton_candle_marks(
        frame,
        chart_w,
        chart_h,
        shimmer.color,
        shimmer.color,
        Some(shimmer),
    );
}

fn draw_skeleton_candle_marks(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    candle_color: Color,
    volume_color: Color,
    shimmer: Option<&Shimmer>,
) {
    let volume_h = chart_h * 0.18;
    let price_h = (chart_h - volume_h).max(0.0);
    let visible_count = ((chart_w / MIN_CANDLE_SPACING).floor() as usize)
        .clamp(12, SKELETON_CANDLE_COUNT)
        .min(API_SAMPLE_CANDLES.len());
    let sample_start = API_SAMPLE_CANDLES.len().saturating_sub(visible_count);
    let step = chart_w / visible_count as f32;
    let candle_w = (step * 0.58).clamp(1.0, 9.0);
    let price_pad = (price_h * 0.06).min(12.0);
    let price_draw_h = (price_h - price_pad * 2.0).max(1.0);
    let price_to_y =
        |value: f32| -> f32 { price_pad + (1.0 - value.clamp(0.0, 1.0)) * price_draw_h };

    for idx in 0..visible_count {
        let candle = API_SAMPLE_CANDLES[sample_start + idx];
        let cx = chart_w - step * (visible_count.saturating_sub(idx) as f32 + 0.35);
        let open_y = price_to_y(candle.open);
        let close_y = price_to_y(candle.close);
        let high_y = price_to_y(candle.high);
        let low_y = price_to_y(candle.low);
        let body_top = open_y.min(close_y);
        let body_h = (open_y - close_y).abs().max(2.0);
        let mark_candle_color = shimmer
            .and_then(|shimmer| shimmer.color_at(cx))
            .unwrap_or(candle_color);
        if mark_candle_color.a <= 0.0 {
            continue;
        }

        frame.stroke(
            &canvas::Path::line(Point::new(cx, high_y), Point::new(cx, low_y)),
            canvas::Stroke::default()
                .with_color(mark_candle_color)
                .with_width(1.0),
        );
        frame.fill_rectangle(
            Point::new(cx - candle_w * 0.5, body_top),
            Size::new(candle_w, body_h),
            mark_candle_color,
        );

        let volume_height = (volume_h * (0.12 + candle.volume * 0.76)).max(2.0);
        let mark_volume_color = shimmer
            .and_then(|shimmer| shimmer.color_at(cx))
            .unwrap_or(volume_color);
        frame.fill_rectangle(
            Point::new(cx - candle_w * 0.5, chart_h - volume_height),
            Size::new(candle_w, volume_height),
            mark_volume_color,
        );
    }
}

fn draw_price_axis(
    frame: &mut canvas::Frame,
    width: f32,
    price_axis_w: f32,
    chart_h: f32,
    palette: &SkeletonPalette,
) {
    draw_price_axis_marks(
        frame,
        width,
        price_axis_w,
        chart_h,
        palette.axis_label,
        None,
    );
}

fn draw_price_axis_shimmer(
    frame: &mut canvas::Frame,
    width: f32,
    price_axis_w: f32,
    chart_h: f32,
    shimmer: &Shimmer,
) {
    draw_price_axis_marks(
        frame,
        width,
        price_axis_w,
        chart_h,
        shimmer.color,
        Some(shimmer),
    );
}

fn draw_price_axis_marks(
    frame: &mut canvas::Frame,
    width: f32,
    price_axis_w: f32,
    chart_h: f32,
    color: Color,
    shimmer: Option<&Shimmer>,
) {
    let axis_x = (width - price_axis_w).max(0.0);
    let label_w = (price_axis_w - 18.0).max(22.0);
    let label_h = 5.0;

    for idx in 0..PRICE_AXIS_LABEL_COUNT {
        let y = chart_h * (idx as f32 + 0.5) / PRICE_AXIS_LABEL_COUNT as f32;
        let label_x = axis_x + 10.0;
        let mark_w = label_w * (0.64 + (idx % 3) as f32 * 0.12);
        let mark_color = shimmer
            .and_then(|shimmer| shimmer.color_at(label_x + mark_w * 0.5))
            .unwrap_or(color);
        if mark_color.a <= 0.0 {
            continue;
        }
        frame.fill_rectangle(
            Point::new(label_x, (y - label_h * 0.5).max(0.0)),
            Size::new(mark_w, label_h),
            mark_color,
        );
    }
}

fn draw_funding_panel(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    funding_h: f32,
    palette: &SkeletonPalette,
) {
    let baseline_y = chart_h + funding_h * 0.52;
    frame.stroke(
        &canvas::Path::line(Point::new(0.0, baseline_y), Point::new(chart_w, baseline_y)),
        canvas::Stroke::default()
            .with_color(palette.grid)
            .with_width(1.0),
    );
    draw_funding_panel_marks(frame, chart_w, chart_h, funding_h, palette.funding, None);
}

fn draw_funding_panel_shimmer(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    funding_h: f32,
    shimmer: &Shimmer,
) {
    draw_funding_panel_marks(
        frame,
        chart_w,
        chart_h,
        funding_h,
        shimmer.color,
        Some(shimmer),
    );
}

fn draw_funding_panel_marks(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    funding_h: f32,
    color: Color,
    shimmer: Option<&Shimmer>,
) {
    let panel_y = chart_h;
    let baseline_y = panel_y + funding_h * 0.52;

    let segments = 24;
    let step = chart_w / segments as f32;
    for idx in 0..segments {
        let x = idx as f32 * step + step * 0.28;
        let height = funding_h * (0.12 + ((idx as f32 * 0.7).sin().abs() * 0.24));
        let bar_w = (step * 0.38).max(2.0);
        let mark_color = shimmer
            .and_then(|shimmer| shimmer.color_at(x + bar_w * 0.5))
            .unwrap_or(color);
        if mark_color.a <= 0.0 {
            continue;
        }
        frame.fill_rectangle(
            Point::new(x, baseline_y - height * 0.5),
            Size::new(bar_w, height),
            mark_color,
        );
    }
}

fn draw_time_axis(
    frame: &mut canvas::Frame,
    chart_w: f32,
    axis_y: f32,
    axis_h: f32,
    palette: &SkeletonPalette,
) {
    draw_time_axis_marks(frame, chart_w, axis_y, axis_h, palette.axis_label, None);
}

fn draw_time_axis_shimmer(
    frame: &mut canvas::Frame,
    chart_w: f32,
    axis_y: f32,
    axis_h: f32,
    shimmer: &Shimmer,
) {
    draw_time_axis_marks(frame, chart_w, axis_y, axis_h, shimmer.color, Some(shimmer));
}

fn draw_time_axis_marks(
    frame: &mut canvas::Frame,
    chart_w: f32,
    axis_y: f32,
    axis_h: f32,
    color: Color,
    shimmer: Option<&Shimmer>,
) {
    let label_h = 5.0;
    for idx in 0..TIME_AXIS_TICK_COUNT {
        let x = chart_w * (idx as f32 + 0.5) / TIME_AXIS_TICK_COUNT as f32;
        let width = 36.0 + (idx % 2) as f32 * 10.0;
        let mark_color = shimmer
            .and_then(|shimmer| shimmer.color_at(x))
            .unwrap_or(color);
        if mark_color.a <= 0.0 {
            continue;
        }
        frame.fill_rectangle(
            Point::new(x - 18.0, axis_y + axis_h * 0.5 - label_h * 0.5),
            Size::new(width, label_h),
            mark_color,
        );
    }
}

fn draw_axis_borders(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    funding_h: f32,
    height: f32,
    palette: &SkeletonPalette,
) {
    let stroke = canvas::Stroke::default()
        .with_color(palette.axis)
        .with_width(1.0);
    frame.stroke(
        &canvas::Path::line(Point::new(chart_w, 0.0), Point::new(chart_w, height)),
        stroke,
    );
    frame.stroke(
        &canvas::Path::line(
            Point::new(0.0, chart_h + funding_h),
            Point::new(chart_w, chart_h + funding_h),
        ),
        stroke,
    );

    if funding_h > 0.0 {
        frame.stroke(
            &canvas::Path::line(Point::new(0.0, chart_h), Point::new(chart_w, chart_h)),
            stroke,
        );
    }
}
