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
        let max_spread = data
            .iter()
            .map(|(_, spread)| *spread)
            .fold(0.0_f64, f64::max)
            .max(1.0);

        Self {
            start_time: now.checked_sub(SPREAD_CHART_WINDOW).unwrap_or(now),
            now,
            y_max: max_spread * 1.1,
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
        let normalized = (spread / self.y_max) as f32;
        self.height - (normalized * self.height)
    }

    fn point_for(&self, time: Instant, spread: f64) -> Point {
        Point::new(self.time_to_x(time), self.spread_to_y(spread))
    }
}

pub(super) fn rendered_spread_points(
    data: &VecDeque<(Instant, f64)>,
    scale: &SpreadChartScale,
) -> Vec<(Point, f64)> {
    data.iter()
        .rev()
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
