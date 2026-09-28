use crate::message::Message;

use drawing::{LaneLayout, draw_lane_row, draw_lane_tooltip, draw_section_header};
use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Theme, mouse};

mod drawing;

// ---- Edge-lane geometry ----
const ROW_HEIGHT: f32 = 22.0;
const SECTION_HEADER_HEIGHT: f32 = 17.0;
const LANE_TOP_PAD: f32 = 6.0;
const LANE_BOTTOM_PAD: f32 = 8.0;
const SECTION_GAP: f32 = 10.0;
const LABEL_GUTTER: f32 = 50.0;
const VALUE_GUTTER: f32 = 56.0;
const BULLET_GUTTER: f32 = 56.0;
const N_GUTTER: f32 = 34.0;
const BULLET_WIDTH: f32 = 42.0;
const BULLET_HEIGHT: f32 = 5.0;
const BAR_THICKNESS: f32 = 12.0;
const BAR_RADIUS: f32 = 2.5;
/// Sample count at which a bucket reaches full confidence (opacity). Chosen so
/// the default 4-week lookback (~4 samples/weekday) reads at roughly half
/// confidence and fills in by ~8 weeks, while sample-rich session buckets stay
/// solid throughout.
const CONF_FULL: f32 = 8.0;
/// Floor opacity for any bucket with at least one sample, so a real-but-thin
/// edge never vanishes.
const MIN_CONF: f32 = 0.35;
const TOOLTIP_WIDTH: f32 = 190.0;
const TOOLTIP_HEIGHT: f32 = 84.0;
const ZERO_LINE_ALPHA: f32 = 0.24;

// ---------------------------------------------------------------------------
// Edge lane (weekday + session breakdown)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct LaneRow {
    pub(super) label: &'static str,
    pub(super) sample_count: usize,
    pub(super) average_return_pct: f64,
    pub(super) win_rate_pct: f64,
    pub(super) dispersion_pct: Option<f64>,
    pub(super) is_best: bool,
}

#[derive(Debug, Clone)]
pub(super) struct SessionLane {
    pub(super) weekday_rows: Vec<LaneRow>,
    pub(super) session_rows: Vec<LaneRow>,
    pub(super) scale_max: f32,
    pub(super) compact: bool,
}

#[derive(Default)]
pub(super) struct SessionLaneState {
    hovered: Option<usize>,
}

pub(super) fn lane_height(weekday_rows: usize, session_rows: usize) -> f32 {
    LANE_TOP_PAD
        + SECTION_HEADER_HEIGHT
        + weekday_rows as f32 * ROW_HEIGHT
        + SECTION_GAP
        + SECTION_HEADER_HEIGHT
        + session_rows as f32 * ROW_HEIGHT
        + LANE_BOTTOM_PAD
}

impl SessionLane {
    fn total_rows(&self) -> usize {
        self.weekday_rows.len() + self.session_rows.len()
    }

    fn row_at(&self, index: usize) -> Option<&LaneRow> {
        let weekday = self.weekday_rows.len();
        if index < weekday {
            self.weekday_rows.get(index)
        } else {
            self.session_rows.get(index - weekday)
        }
    }

    fn row_y_center(&self, index: usize) -> f32 {
        let weekday = self.weekday_rows.len();
        if index < weekday {
            LANE_TOP_PAD + SECTION_HEADER_HEIGHT + index as f32 * ROW_HEIGHT + ROW_HEIGHT * 0.5
        } else {
            let session_index = index - weekday;
            LANE_TOP_PAD
                + SECTION_HEADER_HEIGHT
                + weekday as f32 * ROW_HEIGHT
                + SECTION_GAP
                + SECTION_HEADER_HEIGHT
                + session_index as f32 * ROW_HEIGHT
                + ROW_HEIGHT * 0.5
        }
    }

    fn row_at_cursor(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let pos = cursor.position_in(bounds)?;
        if pos.x < 0.0 || pos.x > bounds.width {
            return None;
        }
        (0..self.total_rows())
            .find(|&index| (pos.y - self.row_y_center(index)).abs() <= ROW_HEIGHT * 0.5)
    }
}

impl canvas::Program<Message> for SessionLane {
    type State = SessionLaneState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let next = match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                self.row_at_cursor(bounds, cursor)
            }
            iced::Event::Mouse(mouse::Event::CursorLeft) => None,
            _ => return None,
        };
        if state.hovered != next {
            state.hovered = next;
            return Some(canvas::Action::request_redraw());
        }
        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

        if self.total_rows() == 0 || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let layout = LaneLayout::new(bounds.size(), self.compact);

        // Shared zero baseline running through both sections.
        let baseline = Path::line(
            Point::new(layout.mid_x, LANE_TOP_PAD + SECTION_HEADER_HEIGHT),
            Point::new(layout.mid_x, bounds.height - LANE_BOTTOM_PAD),
        );
        frame.stroke(
            &baseline,
            Stroke::default()
                .with_color(Color {
                    a: ZERO_LINE_ALPHA,
                    ..theme.palette().text
                })
                .with_width(1.0),
        );

        let mut y = LANE_TOP_PAD;
        draw_section_header(&mut frame, theme, "WEEKDAY", &layout, y);
        y += SECTION_HEADER_HEIGHT;
        for (index, row) in self.weekday_rows.iter().enumerate() {
            draw_lane_row(
                &mut frame,
                theme,
                &layout,
                row,
                self.scale_max,
                y,
                state.hovered == Some(index),
            );
            y += ROW_HEIGHT;
        }

        y += SECTION_GAP;
        draw_section_header(&mut frame, theme, "SESSION", &layout, y);
        y += SECTION_HEADER_HEIGHT;
        let weekday = self.weekday_rows.len();
        for (index, row) in self.session_rows.iter().enumerate() {
            draw_lane_row(
                &mut frame,
                theme,
                &layout,
                row,
                self.scale_max,
                y,
                state.hovered == Some(weekday + index),
            );
            y += ROW_HEIGHT;
        }

        if let Some(index) = state.hovered
            && let Some(row) = self.row_at(index)
        {
            draw_lane_tooltip(
                &mut frame,
                theme,
                bounds.size(),
                &layout,
                row,
                self.row_y_center(index),
            );
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests;
