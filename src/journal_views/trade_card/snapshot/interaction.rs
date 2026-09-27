use super::plot::{SNAPSHOT_LEFT_PAD, point_in_snapshot_plot, snapshot_plot_width};
use crate::journal::JournalTradeSnapshot;
use crate::message::Message;
use iced::widget::canvas;
use iced::{Point, Rectangle, Size, mouse};

const SNAPSHOT_ZOOM_FACTOR: f64 = 0.82;
const SNAPSHOT_VISUAL_RANGE_FRACTION: u64 = 1;
const SNAPSHOT_DEFAULT_EMPTY_SPACE_FRACTION: u64 = 12;
const SNAPSHOT_MIN_DATA_OVERLAP_FRACTION: u64 = 12;

#[derive(Debug, Clone, Default)]
pub(super) struct JournalSnapshotCanvasState {
    reset_key: String,
    view_start_ms: u64,
    view_end_ms: u64,
    drag: Option<SnapshotDrag>,
}

#[derive(Debug, Clone, Copy)]
struct SnapshotDrag {
    start_pos: Point,
    view_start_ms: u64,
    view_end_ms: u64,
}

impl JournalSnapshotCanvasState {
    pub(super) fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub(super) fn view_or_full_range(
        &self,
        snapshot: &JournalTradeSnapshot,
        loaded_range: (u64, u64),
    ) -> (u64, u64) {
        let visual_range = visual_time_range(snapshot, loaded_range);
        if self.reset_key == snapshot_reset_key(snapshot)
            && self.view_end_ms > self.view_start_ms
            && ranges_overlap((self.view_start_ms, self.view_end_ms), loaded_range)
        {
            clamp_view_range(
                self.view_start_ms,
                self.view_end_ms,
                loaded_range,
                visual_range,
                min_view_span_ms(snapshot),
            )
        } else {
            default_view_range(snapshot, loaded_range, visual_range)
        }
    }

    fn reset_to_loaded_range(&mut self, snapshot: &JournalTradeSnapshot) {
        let loaded_range = loaded_time_range(snapshot);
        let visual_range = visual_time_range(snapshot, loaded_range);
        let (view_start_ms, view_end_ms) = default_view_range(snapshot, loaded_range, visual_range);
        self.reset_key = snapshot_reset_key(snapshot);
        self.view_start_ms = view_start_ms;
        self.view_end_ms = view_end_ms;
        self.drag = None;
    }
}

pub(super) fn update_snapshot_interaction(
    state: &mut JournalSnapshotCanvasState,
    snapshot: &JournalTradeSnapshot,
    event: &iced::Event,
    bounds: Rectangle,
    cursor: mouse::Cursor,
) -> Option<canvas::Action<Message>> {
    if snapshot.candles.is_empty() || bounds.width <= 20.0 || bounds.height <= 20.0 {
        return None;
    }

    let loaded_range = loaded_time_range(snapshot);
    if state.reset_key != snapshot_reset_key(snapshot)
        || state.view_end_ms <= state.view_start_ms
        || !ranges_overlap((state.view_start_ms, state.view_end_ms), loaded_range)
    {
        state.reset_to_loaded_range(snapshot);
    }

    let Some(pos) = cursor.position_in(bounds) else {
        if state.drag.take().is_some() {
            return Some(canvas::Action::request_redraw());
        }
        return None;
    };

    match event {
        iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
            let dy = wheel_delta_lines(delta);
            if dy.abs() <= f32::EPSILON {
                return None;
            }
            zoom_snapshot_view(state, snapshot, loaded_range, bounds.size(), pos, dy);
            Some(canvas::Action::request_redraw().and_capture())
        }
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
            if point_in_snapshot_plot(bounds.size(), pos) {
                state.drag = Some(SnapshotDrag {
                    start_pos: pos,
                    view_start_ms: state.view_start_ms,
                    view_end_ms: state.view_end_ms,
                });
                Some(canvas::Action::capture())
            } else {
                None
            }
        }
        iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
            let drag = state.drag?;
            pan_snapshot_view(state, snapshot, loaded_range, bounds.size(), drag, pos);
            Some(canvas::Action::request_redraw().and_capture())
        }
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
            if state.drag.take().is_some() {
                Some(canvas::Action::request_redraw().and_capture())
            } else {
                None
            }
        }
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
            if point_in_snapshot_plot(bounds.size(), pos) {
                state.reset_to_loaded_range(snapshot);
                Some(canvas::Action::request_redraw().and_capture())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn zoom_snapshot_view(
    state: &mut JournalSnapshotCanvasState,
    snapshot: &JournalTradeSnapshot,
    loaded_range: (u64, u64),
    size: Size,
    pos: Point,
    dy: f32,
) {
    if !point_in_snapshot_plot(size, pos) {
        return;
    }
    let plot_w = snapshot_plot_width(size);
    if plot_w <= 0.0 {
        return;
    }
    let current = state.view_or_full_range(snapshot, loaded_range);
    let current_span = current.1.saturating_sub(current.0).max(1) as f64;
    let visual_range = visual_time_range(snapshot, loaded_range);
    let visual_span = visual_range.1.saturating_sub(visual_range.0).max(1) as f64;
    let min_span = min_view_span_ms(snapshot) as f64;
    let next_span = if dy > 0.0 {
        current_span * SNAPSHOT_ZOOM_FACTOR
    } else {
        current_span / SNAPSHOT_ZOOM_FACTOR
    }
    .clamp(min_span.min(visual_span), visual_span);

    let cursor_fraction = ((pos.x - SNAPSHOT_LEFT_PAD) / plot_w).clamp(0.0, 1.0) as f64;
    let anchor_time = current.0 as f64 + current_span * cursor_fraction;
    let next_start = anchor_time - next_span * cursor_fraction;
    let next_end = next_start + next_span;
    let (start, end) = clamp_view_range(
        next_start.round().max(0.0) as u64,
        next_end.round().max(0.0) as u64,
        loaded_range,
        visual_range,
        min_view_span_ms(snapshot),
    );
    state.view_start_ms = start;
    state.view_end_ms = end;
}

fn pan_snapshot_view(
    state: &mut JournalSnapshotCanvasState,
    snapshot: &JournalTradeSnapshot,
    loaded_range: (u64, u64),
    size: Size,
    drag: SnapshotDrag,
    pos: Point,
) {
    let plot_w = snapshot_plot_width(size);
    if plot_w <= 0.0 {
        return;
    }
    let span = drag.view_end_ms.saturating_sub(drag.view_start_ms).max(1);
    let ms_per_px = span as f64 / plot_w as f64;
    let dx = pos.x - drag.start_pos.x;
    let shift_ms = -(dx as f64 * ms_per_px).round() as i128;
    let start = shifted_time(drag.view_start_ms, shift_ms);
    let end = shifted_time(drag.view_end_ms, shift_ms);
    let visual_range = visual_time_range(snapshot, loaded_range);
    let (start, end) = clamp_view_range(
        start,
        end,
        loaded_range,
        visual_range,
        min_view_span_ms(snapshot),
    );
    state.view_start_ms = start;
    state.view_end_ms = end;
}

pub(super) fn loaded_time_range(snapshot: &JournalTradeSnapshot) -> (u64, u64) {
    let start = snapshot.start_ms.min(
        snapshot
            .candles
            .first()
            .map(|candle| candle.open_time)
            .unwrap_or(snapshot.start_ms),
    );
    let end = snapshot
        .end_ms
        .max(
            snapshot
                .candles
                .last()
                .map(|candle| candle.close_time)
                .unwrap_or(snapshot.end_ms),
        )
        .max(start.saturating_add(1));
    (start, end)
}

fn min_view_span_ms(snapshot: &JournalTradeSnapshot) -> u64 {
    snapshot.timeframe.duration_ms().saturating_mul(6).max(1)
}

fn visual_time_range(snapshot: &JournalTradeSnapshot, loaded_range: (u64, u64)) -> (u64, u64) {
    let span = loaded_range.1.saturating_sub(loaded_range.0).max(1);
    let overscroll = (span / SNAPSHOT_VISUAL_RANGE_FRACTION)
        .max(snapshot.timeframe.duration_ms().saturating_mul(24));
    (
        loaded_range.0.saturating_sub(overscroll),
        loaded_range.1.saturating_add(overscroll),
    )
}

fn default_view_range(
    snapshot: &JournalTradeSnapshot,
    loaded_range: (u64, u64),
    visual_range: (u64, u64),
) -> (u64, u64) {
    let span = loaded_range.1.saturating_sub(loaded_range.0).max(1);
    let margin = (span / SNAPSHOT_DEFAULT_EMPTY_SPACE_FRACTION)
        .max(snapshot.timeframe.duration_ms().saturating_mul(4));
    clamp_view_range(
        loaded_range.0.saturating_sub(margin),
        loaded_range.1.saturating_add(margin),
        loaded_range,
        visual_range,
        min_view_span_ms(snapshot),
    )
}

fn clamp_view_range(
    start_ms: u64,
    end_ms: u64,
    loaded_range: (u64, u64),
    visual_range: (u64, u64),
    min_span_ms: u64,
) -> (u64, u64) {
    let visual_span = visual_range.1.saturating_sub(visual_range.0).max(1);
    let target_span = end_ms
        .saturating_sub(start_ms)
        .max(min_span_ms)
        .min(visual_span);
    let loaded_span = loaded_range.1.saturating_sub(loaded_range.0).max(1);
    let min_overlap = (target_span / SNAPSHOT_MIN_DATA_OVERLAP_FRACTION)
        .max(min_view_span_ms_for_span(min_span_ms))
        .min(loaded_span);

    let visual_min_start = visual_range.0;
    let visual_max_start = visual_range.1.saturating_sub(target_span);
    let overlap_min_start = loaded_range
        .0
        .saturating_add(min_overlap)
        .saturating_sub(target_span);
    let overlap_max_start = loaded_range.1.saturating_sub(min_overlap);
    let min_start = visual_min_start.max(overlap_min_start);
    let max_start = visual_max_start.min(overlap_max_start).max(min_start);

    let start = start_ms.clamp(min_start, max_start);
    let end = start.saturating_add(target_span).min(visual_range.1);
    (start, end.max(start.saturating_add(1)))
}

fn min_view_span_ms_for_span(min_span_ms: u64) -> u64 {
    (min_span_ms / 2).max(1)
}

fn shifted_time(time_ms: u64, shift_ms: i128) -> u64 {
    if shift_ms >= 0 {
        time_ms.saturating_add(shift_ms as u64)
    } else {
        time_ms.saturating_sub((-shift_ms) as u64)
    }
}

fn ranges_overlap(left: (u64, u64), right: (u64, u64)) -> bool {
    left.0 < right.1 && right.0 < left.1
}

fn snapshot_reset_key(snapshot: &JournalTradeSnapshot) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        snapshot.trade_id,
        snapshot.source.label(),
        snapshot.coverage.label(),
        snapshot.timeframe.api_str(),
        snapshot.start_ms,
        snapshot.end_ms,
        snapshot.candles.len()
    )
}

fn wheel_delta_lines(delta: &mouse::ScrollDelta) -> f32 {
    match delta {
        mouse::ScrollDelta::Lines { y, .. } => *y,
        mouse::ScrollDelta::Pixels { y, .. } => *y / 28.0,
    }
}

#[cfg(test)]
mod tests;
