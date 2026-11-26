use crate::api::Candle;
use crate::chart::PRICE_AXIS_WIDTH;
use crate::helpers::format_decimal_with_commas;
use iced::{Point, Size};

pub(super) const SNAPSHOT_LEFT_PAD: f32 = 6.0;
pub(super) const SNAPSHOT_TOP_PAD: f32 = 26.0;
pub(super) const SNAPSHOT_BOTTOM_PAD: f32 = 12.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct SnapshotPlot {
    pub(super) left: f32,
    pub(super) top: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) start_ms: u64,
    pub(super) end_ms: u64,
    pub(super) min_price: f64,
    pub(super) max_price: f64,
}

impl SnapshotPlot {
    pub(super) fn new(
        size: Size,
        start_ms: u64,
        end_ms: u64,
        candles: &[Candle],
        extra_prices: impl IntoIterator<Item = f64>,
    ) -> Self {
        let left = SNAPSHOT_LEFT_PAD;
        let top = SNAPSHOT_TOP_PAD;
        let width = snapshot_plot_width(size);
        let height = (size.height - SNAPSHOT_TOP_PAD - SNAPSHOT_BOTTOM_PAD).max(1.0);
        let (min_price, max_price) = price_range(candles, extra_prices);
        Self {
            left,
            top,
            width,
            height,
            start_ms,
            end_ms: end_ms.max(start_ms.saturating_add(1)),
            min_price,
            max_price,
        }
    }

    pub(super) fn x_for_time(self, time_ms: u64) -> f32 {
        let span = self.end_ms.saturating_sub(self.start_ms).max(1) as f64;
        let offset = time_ms.saturating_sub(self.start_ms) as f64 / span;
        self.left + (offset as f32).clamp(0.0, 1.0) * self.width
    }

    pub(super) fn y_for_price(self, price: f64) -> f32 {
        let span = (self.max_price - self.min_price).max(f64::EPSILON);
        let offset = ((price - self.min_price) / span).clamp(0.0, 1.0);
        self.top + (1.0 - offset as f32) * self.height
    }

    pub(super) fn price_label_at_y(self, y: f32) -> String {
        let from_bottom =
            (f64::from(self.top + self.height - y) / f64::from(self.height)).clamp(0.0, 1.0);
        let span = self.max_price - self.min_price;
        let price = self.min_price + from_bottom * span;
        // Resolve a one-pixel move even when the visible price range is tight.
        let price_per_pixel = span / f64::from(self.height);
        let decimals = (-price_per_pixel.log10()).ceil().clamp(0.0, 8.0) as usize;
        format_decimal_with_commas(price, decimals)
    }
}

pub(super) fn price_axis_left(size: Size) -> f32 {
    size.width - PRICE_AXIS_WIDTH.min(size.width * 0.5)
}

pub(super) fn snapshot_plot_width(size: Size) -> f32 {
    (price_axis_left(size) - SNAPSHOT_LEFT_PAD).max(1.0)
}

pub(super) fn point_in_snapshot_plot(size: Size, pos: Point) -> bool {
    pos.x >= SNAPSHOT_LEFT_PAD
        && pos.x <= price_axis_left(size)
        && pos.y >= SNAPSHOT_TOP_PAD
        && pos.y <= size.height - SNAPSHOT_BOTTOM_PAD
}

fn price_range(candles: &[Candle], extra_prices: impl IntoIterator<Item = f64>) -> (f64, f64) {
    let (mut min_price, mut max_price) = candles.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(min_price, max_price), candle| (min_price.min(candle.low), max_price.max(candle.high)),
    );

    for extra in extra_prices
        .into_iter()
        .filter(|price| price.is_finite() && *price > 0.0)
    {
        min_price = min_price.min(extra);
        max_price = max_price.max(extra);
    }

    if !min_price.is_finite() || !max_price.is_finite() || min_price <= 0.0 {
        return (0.0, 1.0);
    }

    let span = (max_price - min_price).max(max_price * 0.002);
    let padding = span * 0.08;
    (min_price - padding, max_price + padding)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_price_inverts_the_visible_candle_scale_including_its_padding() {
        let candles = [Candle::test_ohlcv(
            100,
            199,
            [101.0, 104.0, 100.0, 102.0],
            1.0,
        )];
        let size = Size::new(400.0, 240.0);
        let plot = SnapshotPlot::new(size, 100, 199, &candles, []);
        assert_eq!(plot.price_label_at_y(plot.top), "104.32");
        assert_eq!(plot.price_label_at_y(plot.top + plot.height), "99.68");
        assert_eq!(plot.price_label_at_y(plot.y_for_price(101.37)), "101.37");

        let tight = SnapshotPlot::new(size, 100, 199, &[Candle::test_flat(100, 1.2345)], []);
        assert_eq!(
            tight.price_label_at_y(tight.y_for_price(1.23456)),
            "1.234560"
        );
    }
}
