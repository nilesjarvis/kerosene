use super::ChartId;

use iced::{Point, Size, window};

// ---------------------------------------------------------------------------
// Chart Surfaces
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ChartSurfaceId {
    Docked(ChartId),
    Detached(window::Id),
}

impl ChartSurfaceId {
    pub(crate) fn widget_suffix(self) -> String {
        match self {
            Self::Docked(chart_id) => format!("docked_{chart_id}"),
            Self::Detached(window_id) => format!("detached_{window_id:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DetachedChartWindowState {
    pub(crate) chart_id: ChartId,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) x: Option<f32>,
    pub(crate) y: Option<f32>,
}

impl DetachedChartWindowState {
    pub(crate) fn new(chart_id: ChartId) -> Self {
        Self {
            chart_id,
            width: crate::config::default_detached_chart_window_width(),
            height: crate::config::default_detached_chart_window_height(),
            x: None,
            y: None,
        }
    }

    pub(crate) fn from_config(config: &crate::config::DetachedChartWindowConfig) -> Self {
        Self {
            chart_id: config.chart_id,
            width: normalize_detached_chart_window_dimension(
                config.width,
                crate::config::default_detached_chart_window_width(),
            ),
            height: normalize_detached_chart_window_dimension(
                config.height,
                crate::config::default_detached_chart_window_height(),
            ),
            x: config.x.and_then(finite_f32_value),
            y: config.y.and_then(finite_f32_value),
        }
    }

    pub(crate) fn to_config(&self) -> crate::config::DetachedChartWindowConfig {
        crate::config::DetachedChartWindowConfig {
            chart_id: self.chart_id,
            width: normalize_detached_chart_window_dimension(
                self.width,
                crate::config::default_detached_chart_window_width(),
            ),
            height: normalize_detached_chart_window_dimension(
                self.height,
                crate::config::default_detached_chart_window_height(),
            ),
            x: self.x.and_then(finite_f32_value),
            y: self.y.and_then(finite_f32_value),
        }
    }

    pub(crate) fn size(&self) -> Size {
        Size::new(
            normalize_detached_chart_window_dimension(
                self.width,
                crate::config::default_detached_chart_window_width(),
            ),
            normalize_detached_chart_window_dimension(
                self.height,
                crate::config::default_detached_chart_window_height(),
            ),
        )
    }

    pub(crate) fn position(&self) -> window::Position {
        self.x
            .zip(self.y)
            .and_then(|(x, y)| finite_chart_window_point(x, y))
            .map(crate::window_chrome::restored_position)
            .unwrap_or(window::Position::Centered)
    }
}

fn normalize_detached_chart_window_dimension(value: f32, fallback: f32) -> f32 {
    finite_f32_value(value).map_or(fallback, |value| value.max(320.0))
}

fn finite_chart_window_point(x: f32, y: f32) -> Option<Point> {
    Some(Point::new(finite_f32_value(x)?, finite_f32_value(y)?))
}

fn finite_f32_value(value: f32) -> Option<f32> {
    value.is_finite().then_some(value)
}
