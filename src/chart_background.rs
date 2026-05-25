use crate::app_state::TradingTerminal;

use iced::widget::canvas;
use iced::{Color, Point, Size, Theme};

// ---------------------------------------------------------------------------
// Chart Background Rendering
// ---------------------------------------------------------------------------

const DOTTED_BACKGROUND_SPACING: f32 = 18.0;
const DOTTED_BACKGROUND_DOT_SIZE: f32 = 2.0;

pub(crate) fn draw_dotted_background(
    frame: &mut canvas::Frame,
    theme: &Theme,
    width: f32,
    height: f32,
    opacity: f32,
) {
    let alpha = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        crate::config::DEFAULT_CHART_DOTTED_BACKGROUND_OPACITY
    };
    let color = Color {
        a: alpha,
        ..theme.palette().text
    };
    let path = dotted_background_path(width, height);
    frame.fill(&path, color);
}

fn dotted_background_path(width: f32, height: f32) -> canvas::Path {
    canvas::Path::new(|path| {
        for_each_dotted_background_dot(width, height, |top_left| {
            path.rectangle(
                top_left,
                Size::new(DOTTED_BACKGROUND_DOT_SIZE, DOTTED_BACKGROUND_DOT_SIZE),
            );
        });
    })
}

#[cfg(test)]
fn dotted_background_dot_origins(width: f32, height: f32) -> Vec<Point> {
    let mut origins = Vec::with_capacity(dotted_background_dot_count(width, height));
    for_each_dotted_background_dot(width, height, |origin| origins.push(origin));
    origins
}

fn for_each_dotted_background_dot(width: f32, height: f32, mut visit: impl FnMut(Point)) {
    if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
        return;
    }

    let radius = DOTTED_BACKGROUND_DOT_SIZE * 0.5;
    let mut y = DOTTED_BACKGROUND_SPACING * 0.5 - radius;
    while y + DOTTED_BACKGROUND_DOT_SIZE <= height {
        let mut x = DOTTED_BACKGROUND_SPACING * 0.5 - radius;
        while x + DOTTED_BACKGROUND_DOT_SIZE <= width {
            visit(Point::new(x, y));
            x += DOTTED_BACKGROUND_SPACING;
        }
        y += DOTTED_BACKGROUND_SPACING;
    }
}

#[cfg(test)]
fn dotted_background_dot_count(width: f32, height: f32) -> usize {
    if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
        return 0;
    }

    let fit_count = |length: f32| -> usize {
        let first_origin = DOTTED_BACKGROUND_SPACING * 0.5 - DOTTED_BACKGROUND_DOT_SIZE * 0.5;
        ((length - DOTTED_BACKGROUND_DOT_SIZE - first_origin) / DOTTED_BACKGROUND_SPACING)
            .floor()
            .max(0.0) as usize
            + 1
    };

    fit_count(width) * fit_count(height)
}

// ---------------------------------------------------------------------------
// Chart Background State
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn sync_chart_dotted_background(&mut self) {
        let enabled = self.chart_dotted_background;
        let opacity = self.chart_dotted_background_opacity;
        for instance in self.charts.values_mut() {
            instance.chart.set_dotted_background(enabled, opacity);
        }
        for instance in self.spaghetti_charts.values_mut() {
            instance.canvas.set_dotted_background(enabled, opacity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{dotted_background_dot_count, dotted_background_dot_origins};

    #[test]
    fn dotted_background_dot_origins_cover_positive_area_on_spacing_centers() {
        let origins = dotted_background_dot_origins(38.0, 38.0);

        assert_eq!(origins.len(), 4);
        assert_eq!(origins[0].x, 8.0);
        assert_eq!(origins[0].y, 8.0);
        assert_eq!(origins[3].x, 26.0);
        assert_eq!(origins[3].y, 26.0);
    }

    #[test]
    fn dotted_background_dot_count_matches_generated_origins() {
        assert_eq!(dotted_background_dot_count(1000.0, 600.0), 1848);
        assert_eq!(
            dotted_background_dot_count(1000.0, 600.0),
            dotted_background_dot_origins(1000.0, 600.0).len()
        );
    }

    #[test]
    fn dotted_background_dot_origins_skip_invalid_areas() {
        assert!(dotted_background_dot_origins(0.0, 20.0).is_empty());
        assert!(dotted_background_dot_origins(20.0, f32::NAN).is_empty());
    }
}
