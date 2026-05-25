use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Skeleton Colors And Shimmer
// ---------------------------------------------------------------------------

const SHIMMER_MIN_WIDTH: f32 = 80.0;
const SHIMMER_MAX_WIDTH: f32 = 220.0;

pub(super) struct SkeletonPalette {
    pub(super) background: Color,
    pub(super) grid: Color,
    pub(super) axis: Color,
    pub(super) axis_label: Color,
    pub(super) candle: Color,
    pub(super) volume: Color,
    pub(super) funding: Color,
    pub(super) shimmer: Color,
}

impl SkeletonPalette {
    pub(super) fn new(theme: &Theme) -> Self {
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

pub(super) struct Shimmer {
    center_x: f32,
    half_width: f32,
    color: Color,
}

impl Shimmer {
    pub(super) fn new(width: f32, phase: f32, palette: &SkeletonPalette) -> Self {
        let band_w = (width * 0.24).clamp(SHIMMER_MIN_WIDTH, SHIMMER_MAX_WIDTH);
        let progress = phase.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
        let travel_w = width + band_w * 2.0;

        Self {
            center_x: progress * travel_w - band_w,
            half_width: band_w * 0.5,
            color: palette.shimmer,
        }
    }

    pub(super) fn color_at(&self, x: f32) -> Option<Color> {
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

    pub(super) fn color(&self) -> Color {
        self.color
    }
}
