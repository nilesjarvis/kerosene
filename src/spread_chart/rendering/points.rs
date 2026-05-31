use iced::Point;

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Spread Chart Point Mapping
// ---------------------------------------------------------------------------

const SPREAD_CHART_WINDOW: Duration = Duration::from_secs(300);

pub(super) struct SpreadChartScale {
    start_time: Instant,
    now: Instant,
    y_min: f64,
    y_max: f64,
    width: f32,
    height: f32,
}

impl SpreadChartScale {
    pub(super) fn new(
        data: &VecDeque<(Instant, f64)>,
        now: Instant,
        width: f32,
        height: f32,
    ) -> Self {
        let start_time = now.checked_sub(SPREAD_CHART_WINDOW).unwrap_or(now);
        let (min_spread, max_spread) =
            visible_spread_bounds(data, start_time, now).unwrap_or((0.0, 1.0));
        let (y_min, y_max) = padded_spread_bounds(min_spread, max_spread);

        Self {
            start_time,
            now,
            y_min,
            y_max,
            width,
            height,
        }
    }

    fn time_to_x(&self, time: Instant) -> f32 {
        if time < self.start_time {
            return 0.0;
        }
        if time > self.now {
            return self.width;
        }

        let elapsed = time.duration_since(self.start_time).as_secs_f32();
        (elapsed / SPREAD_CHART_WINDOW.as_secs_f32()) * self.width
    }

    fn spread_to_y(&self, spread: f64) -> f32 {
        if !spread.is_finite() {
            return self.height;
        }

        let range = self.y_max - self.y_min;
        let normalized = ((spread - self.y_min) / range).clamp(0.0, 1.0) as f32;
        self.height - (normalized * self.height)
    }

    fn point_for(&self, time: Instant, spread: f64) -> Point {
        Point::new(self.time_to_x(time), self.spread_to_y(spread))
    }
}

fn visible_spread_bounds(
    data: &VecDeque<(Instant, f64)>,
    start_time: Instant,
    now: Instant,
) -> Option<(f64, f64)> {
    data.iter()
        .filter(|(time, spread)| *time >= start_time && *time <= now && spread.is_finite())
        .map(|(_, spread)| *spread)
        .fold(None, |bounds, spread| {
            Some(match bounds {
                Some((min_spread, max_spread)) => (min_spread.min(spread), max_spread.max(spread)),
                None => (spread, spread),
            })
        })
}

fn padded_spread_bounds(min_spread: f64, max_spread: f64) -> (f64, f64) {
    let range = max_spread - min_spread;
    let padding = if range > f64::EPSILON {
        range * 0.1
    } else {
        max_spread.abs().max(1.0) * 0.05
    };

    (min_spread - padding, max_spread + padding)
}

pub(super) fn rendered_spread_points(
    data: &VecDeque<(Instant, f64)>,
    scale: &SpreadChartScale,
) -> Vec<(Point, f64)> {
    data.iter()
        .rev()
        .filter(|(time, spread)| {
            *time >= scale.start_time && *time <= scale.now && spread.is_finite()
        })
        .map(|(time, spread)| (scale.point_for(*time, *spread), *spread))
        .collect()
}

pub(super) fn closest_spread_point(
    rendered_points: &[(Point, f64)],
    hover_pos: Point,
) -> Option<(Point, f64)> {
    rendered_points
        .iter()
        .min_by(|(left, _), (right, _)| {
            (left.x - hover_pos.x)
                .abs()
                .total_cmp(&(right.x - hover_pos.x).abs())
        })
        .copied()
}
