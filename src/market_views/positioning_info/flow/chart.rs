use super::super::metrics::{PositioningFlowData, PositioningFlowKind};
use crate::denomination::DisplayDenominationContext;
use crate::helpers::ease_out_cubic;
use crate::message::Message;

use drawing::{FlowLayout, axis_color, draw_tooltip};
use formatting::{build_chart_row, compact_size, compact_usd};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use iced::{Point, Rectangle, Renderer, Theme, mouse, time};

mod drawing;
mod formatting;

// ---------------------------------------------------------------------------
// Positioning Change Flow Canvas
//
// A diverging horizontal bar chart: each trader's signed change over the
// selected timeframe extends right (more long, success color) or left (more
// short, danger color) from a center axis, scaled to the largest move in view.
// A net-flow "tug of war" header summarizes aggregate long vs short flow.
// ---------------------------------------------------------------------------

const HEADER_HEIGHT: f32 = 30.0;
const HEADER_GAP: f32 = 10.0;
const ROW_HEIGHT: f32 = 22.0;
const ROW_GAP: f32 = 3.0;
const SIDE_PADDING: f32 = 8.0;
const MIN_BAR_PX: f32 = 2.0;
const TOOLTIP_ANIMATION_EASE: f32 = 0.34;
const TOOLTIP_ANIMATION_EPSILON: f32 = 0.01;
const TOOLTIP_ANIMATION_FRAME_MS: u64 = 16;
const TOOLTIP_ANIMATION_OFFSET_PX: f32 = 5.0;

// Aggressive, stepped collapsing (mirrors the positioning column toggle): the
// bar is always shown; labels/value/tag are revealed only with real headroom.
const SHOW_VALUE_MIN_WIDTH: f32 = 240.0;
const SHOW_LABEL_MIN_WIDTH: f32 = 340.0;
const SHOW_TAG_MIN_WIDTH: f32 = 480.0;

const LABEL_WIDTH: f32 = 120.0;
const VALUE_WIDTH: f32 = 84.0;
const TAG_WIDTH: f32 = 42.0;

#[derive(Debug, Default)]
pub(in crate::market_views::positioning_info) struct PositioningFlowState {
    hovered: Option<usize>,
    tooltip_progress: f32,
}

impl PositioningFlowState {
    fn set_hovered(&mut self, hovered: Option<usize>) -> bool {
        if self.hovered == hovered {
            return false;
        }

        self.hovered = hovered;
        if hovered.is_some() {
            self.tooltip_progress = self.tooltip_progress.min(0.25);
        } else {
            self.tooltip_progress = 0.0;
        }
        true
    }

    fn tooltip_animation_active(&self) -> bool {
        self.hovered.is_some() && self.tooltip_progress < 1.0
    }

    fn advance_tooltip_animation(&mut self) {
        if self.hovered.is_none() {
            self.tooltip_progress = 0.0;
            return;
        }

        let delta = 1.0 - self.tooltip_progress;
        if delta <= TOOLTIP_ANIMATION_EPSILON {
            self.tooltip_progress = 1.0;
            return;
        }

        self.tooltip_progress =
            (self.tooltip_progress + delta * TOOLTIP_ANIMATION_EASE).clamp(0.0, 1.0);
    }

    fn tooltip_visibility(&self) -> f32 {
        if self.hovered.is_some() {
            ease_out_cubic(self.tooltip_progress)
        } else {
            0.0
        }
    }
}

/// A single row prepared for rendering. Labels and tooltip text are resolved at
/// view-build time so the canvas program stays a pure function of its inputs.
#[derive(Debug, Clone)]
pub(in crate::market_views::positioning_info) struct PositioningFlowChartRow {
    pub(in crate::market_views::positioning_info) address: String,
    pub(in crate::market_views::positioning_info) label: String,
    pub(in crate::market_views::positioning_info) hover_key: String,
    pub(in crate::market_views::positioning_info) value_text: String,
    pub(in crate::market_views::positioning_info) magnitude: f64,
    pub(in crate::market_views::positioning_info) more_long: bool,
    pub(in crate::market_views::positioning_info) kind: PositioningFlowKind,
    pub(in crate::market_views::positioning_info) tooltip: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub(in crate::market_views::positioning_info) struct PositioningFlowChart {
    pub(super) rows: Vec<PositioningFlowChartRow>,
    max_magnitude: f64,
    long_flow: f64,
    short_flow: f64,
    long_label: String,
    short_label: String,
    net_label: String,
    empty_text: String,
    /// Wallet-action hover key whose label is replaced by action buttons in the
    /// widget overlay; the canvas blanks that row's label to avoid overdraw.
    pub(super) hovered_action_key: Option<String>,
}

impl PositioningFlowChart {
    pub(in crate::market_views::positioning_info) fn new(
        data: &PositioningFlowData,
        denomination: &DisplayDenominationContext,
    ) -> Self {
        let rows = data
            .rows
            .iter()
            .map(|row| build_chart_row(row, data.usd_scaled, denomination))
            .collect();

        let net = data.long_flow - data.short_flow;
        let (long_label, short_label, net_label) = if data.usd_scaled {
            (
                compact_usd(data.long_flow, denomination),
                compact_usd(data.short_flow, denomination),
                format!(
                    "{} {}",
                    if net >= 0.0 { "NET +" } else { "NET -" },
                    compact_usd(net.abs(), denomination)
                ),
            )
        } else {
            (
                compact_size(data.long_flow),
                compact_size(data.short_flow),
                format!(
                    "NET {}{}",
                    if net >= 0.0 { "+" } else { "-" },
                    compact_size(net.abs())
                ),
            )
        };

        let empty_text = if data.usd_scaled {
            "No measurable position changes".to_string()
        } else {
            "Awaiting live mark for USD scaling".to_string()
        };

        Self {
            rows,
            max_magnitude: data.max_magnitude,
            long_flow: data.long_flow,
            short_flow: data.short_flow,
            long_label,
            short_label,
            net_label,
            empty_text,
            hovered_action_key: None,
        }
    }

    pub(in crate::market_views::positioning_info) fn content_height(&self) -> f32 {
        let rows = self.rows.len().max(1) as f32;
        HEADER_HEIGHT + HEADER_GAP + rows * ROW_HEIGHT + (rows - 1.0).max(0.0) * ROW_GAP
    }

    /// Whether the trader label column is shown at this width (labels and the
    /// interactive action overlay only appear with real headroom).
    pub(in crate::market_views::positioning_info) fn labels_visible(width: f32) -> bool {
        width >= SHOW_LABEL_MIN_WIDTH
    }

    /// Pixel offset from the top of the chart to the first row.
    pub(in crate::market_views::positioning_info) fn rows_top() -> f32 {
        HEADER_HEIGHT + HEADER_GAP
    }

    pub(in crate::market_views::positioning_info) fn row_height() -> f32 {
        ROW_HEIGHT
    }

    pub(in crate::market_views::positioning_info) fn row_gap() -> f32 {
        ROW_GAP
    }

    pub(in crate::market_views::positioning_info) fn label_left() -> f32 {
        SIDE_PADDING
    }

    pub(in crate::market_views::positioning_info) fn label_width() -> f32 {
        LABEL_WIDTH
    }

    pub(in crate::market_views::positioning_info) fn rows(&self) -> &[PositioningFlowChartRow] {
        &self.rows
    }

    fn row_index_at(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let pos = cursor.position_in(bounds)?;
        let rows_top = HEADER_HEIGHT + HEADER_GAP;
        if pos.y < rows_top {
            return None;
        }
        let offset = pos.y - rows_top;
        let stride = ROW_HEIGHT + ROW_GAP;
        let index = (offset / stride).floor() as usize;
        if index < self.rows.len() && (offset - index as f32 * stride) <= ROW_HEIGHT {
            Some(index)
        } else {
            None
        }
    }
}

impl canvas::Program<Message> for PositioningFlowChart {
    type State = PositioningFlowState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if state.tooltip_animation_active() {
                state.advance_tooltip_animation();
                if state.tooltip_animation_active() {
                    return Some(canvas::Action::request_redraw_at(
                        *now + time::Duration::from_millis(TOOLTIP_ANIMATION_FRAME_MS),
                    ));
                }
            }
            return None;
        }

        let next = match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                self.row_index_at(bounds, cursor)
            }
            iced::Event::Mouse(mouse::Event::CursorLeft) => None,
            _ => return None,
        };
        if state.set_hovered(next) {
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
        self.draw_header(&mut frame, theme, bounds.width);

        if self.rows.is_empty() {
            frame.fill_text(Text {
                content: self.empty_text.clone(),
                position: Point::new(bounds.width / 2.0, HEADER_HEIGHT + HEADER_GAP + 24.0),
                color: theme.extended_palette().background.weak.text,
                size: iced::Pixels(12.0),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center,
                ..Default::default()
            });
            return vec![frame.into_geometry()];
        }

        let layout = FlowLayout::new(bounds.width);
        for (index, row) in self.rows.iter().enumerate() {
            let top = HEADER_HEIGHT + HEADER_GAP + index as f32 * (ROW_HEIGHT + ROW_GAP);
            // The widget overlay draws action buttons over this row's label when
            // its wallet actions are hovered, so the canvas omits the label.
            let actions_hovered =
                self.hovered_action_key.as_deref() == Some(row.hover_key.as_str());
            self.draw_row(
                &mut frame,
                theme,
                &layout,
                row,
                top,
                state.hovered == Some(index),
                actions_hovered,
            );
        }

        // Center axis line spanning the rows.
        let rows_top = HEADER_HEIGHT + HEADER_GAP;
        let rows_bottom = self.content_height().min(bounds.height);
        let axis = Path::line(
            Point::new(layout.center_x, rows_top),
            Point::new(layout.center_x, rows_bottom),
        );
        frame.stroke(
            &axis,
            Stroke::default()
                .with_color(axis_color(theme))
                .with_width(1.0),
        );

        if let Some(index) = state.hovered
            && let Some(row) = self.rows.get(index)
        {
            draw_tooltip(
                &mut frame,
                theme,
                bounds,
                &layout,
                row,
                index,
                state.tooltip_visibility(),
            );
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.hovered.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

#[cfg(test)]
mod tests;
