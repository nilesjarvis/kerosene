use crate::api::Candle;
use crate::app_state::TradingTerminal;
use crate::chart::TradeMarker;
use crate::helpers::{format_price, format_signed_percent_value};
use crate::journal::{
    AggregatedTrade, JournalTradeSnapshot, JournalTradeSnapshotMetrics, JournalTradeSnapshotStatus,
};
use crate::message::Message;
use iced::mouse;
use iced::widget::canvas;
use iced::widget::{Column, Row, Space, column, container, row, text};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme, alignment};

const SNAPSHOT_HEIGHT: f32 = 240.0;
const SNAPSHOT_MARKER_RADIUS: f32 = 2.8;
const SNAPSHOT_MARKER_GROUP_RADIUS: f32 = 3.8;
const SNAPSHOT_MARKER_CHART_GAP: f32 = 2.0;
const SNAPSHOT_MARKER_GROUP_GAP: f32 = 2.0;
const SNAPSHOT_LEFT_PAD: f32 = 6.0;
const SNAPSHOT_RIGHT_PAD: f32 = 6.0;
const SNAPSHOT_TOP_PAD: f32 = 48.0;
const SNAPSHOT_BOTTOM_PAD: f32 = 34.0;
const SNAPSHOT_ZOOM_FACTOR: f64 = 0.82;
const SNAPSHOT_MARKER_OFFSET: f32 = SNAPSHOT_MARKER_GROUP_RADIUS + SNAPSHOT_MARKER_CHART_GAP;
const SNAPSHOT_MARKER_GROUP_DISTANCE: f32 =
    SNAPSHOT_MARKER_GROUP_RADIUS * 2.0 + SNAPSHOT_MARKER_GROUP_GAP;
const SNAPSHOT_VISUAL_RANGE_FRACTION: u64 = 1;
const SNAPSHOT_DEFAULT_EMPTY_SPACE_FRACTION: u64 = 12;
const SNAPSHOT_MIN_DATA_OVERLAP_FRACTION: u64 = 12;

impl TradingTerminal {
    pub(in crate::journal_views) fn view_journal_trade_snapshot<'a>(
        &'a self,
        trade: &'a AggregatedTrade,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let muted = theme.extended_palette().background.weak.text;

        if let Some(request) = self.journal.snapshot_requests.get(&trade.id) {
            return container(
                row![
                    text("Loading chart snapshot")
                        .size(11)
                        .color(theme.palette().success),
                    Space::new().width(8.0),
                    text(request.timeframe.label()).size(11).color(muted),
                ]
                .align_y(iced::Alignment::Center),
            )
            .width(Fill)
            .height(SNAPSHOT_HEIGHT)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .into();
        }

        let Some(snapshot) = self.journal.snapshots.get(&trade.id) else {
            return unavailable_snapshot_view("Snapshot unavailable.", &theme);
        };

        match &snapshot.status {
            JournalTradeSnapshotStatus::Loaded => loaded_snapshot_view(snapshot.clone(), &theme),
            JournalTradeSnapshotStatus::Unavailable(reason) => {
                unavailable_snapshot_view(reason.as_str(), &theme)
            }
        }
    }
}

fn loaded_snapshot_view(
    snapshot: JournalTradeSnapshot,
    theme: &Theme,
) -> Element<'static, Message> {
    let chart: Element<'static, Message> = iced::widget::canvas(JournalSnapshotCanvas {
        snapshot: snapshot.clone(),
    })
    .width(Fill)
    .height(SNAPSHOT_HEIGHT)
    .into();

    column![chart, metrics_rows(&snapshot.metrics, theme)]
        .spacing(6)
        .into()
}

fn unavailable_snapshot_view(reason: &str, theme: &Theme) -> Element<'static, Message> {
    container(
        text(reason.to_string())
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .align_x(alignment::Horizontal::Center)
            .color(theme.extended_palette().background.weak.text),
    )
    .width(Fill)
    .height(SNAPSHOT_HEIGHT)
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center)
    .padding(16)
    .style(|theme: &Theme| iced::widget::container::Style {
        background: Some(
            Color {
                a: 0.04,
                ..theme.palette().primary
            }
            .into(),
        ),
        border: iced::Border {
            color: Color {
                a: 0.5,
                ..theme.palette().primary
            },
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn metrics_rows(metrics: &JournalTradeSnapshotMetrics, theme: &Theme) -> Element<'static, Message> {
    let muted = theme.extended_palette().background.weak.text;
    let text_color = theme.palette().text;

    let top = Row::new()
        .spacing(12)
        .push(metric_pair(
            "TF",
            metrics.timeframe.label().to_string(),
            text_color,
            muted,
        ))
        .push(metric_pair(
            "Candles",
            metrics.candle_count.to_string(),
            text_color,
            muted,
        ))
        .push(metric_pair(
            "Raw",
            format_pct(metrics.raw_asset_move),
            signed_color(metrics.raw_asset_move, theme),
            muted,
        ))
        .push(metric_pair(
            "Dir",
            format_pct(metrics.directional_move),
            signed_color(metrics.directional_move, theme),
            muted,
        ));

    let bottom = Row::new()
        .spacing(12)
        .push(metric_pair(
            "MAE",
            format_pct(metrics.max_adverse_excursion),
            signed_color(metrics.max_adverse_excursion, theme),
            muted,
        ))
        .push(metric_pair(
            "MFE",
            format_pct(metrics.max_favorable_excursion),
            signed_color(metrics.max_favorable_excursion, theme),
            muted,
        ))
        .push(metric_pair(
            "DD",
            format_pct(metrics.asset_drawdown),
            signed_color(metrics.asset_drawdown, theme),
            muted,
        ))
        .push(metric_pair(
            "Entry",
            format_price(metrics.entry_price),
            text_color,
            muted,
        ))
        .push(metric_pair(
            "Exit",
            format_price(metrics.exit_price),
            text_color,
            muted,
        ));

    Column::new().spacing(3).push(top).push(bottom).into()
}

fn metric_pair(
    label: &'static str,
    value: String,
    value_color: Color,
    label_color: Color,
) -> Element<'static, Message> {
    row![
        text(label).size(10).color(label_color),
        text(value)
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(value_color),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .into()
}

fn format_pct(value: f64) -> String {
    format_signed_percent_value(value * 100.0)
}

fn signed_color(value: f64, theme: &Theme) -> Color {
    if value > 0.0 {
        theme.palette().success
    } else if value < 0.0 {
        theme.palette().danger
    } else {
        theme.palette().text
    }
}

#[derive(Debug, Clone)]
struct JournalSnapshotCanvas {
    snapshot: JournalTradeSnapshot,
}

impl canvas::Program<Message> for JournalSnapshotCanvas {
    type State = JournalSnapshotCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        update_snapshot_interaction(state, &self.snapshot, event, bounds, cursor)
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

        draw_snapshot_chart(&mut frame, theme, bounds.size(), &self.snapshot, state);

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }

        if cursor.position_in(bounds).is_some() {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
struct JournalSnapshotCanvasState {
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

fn draw_snapshot_chart(
    frame: &mut canvas::Frame,
    theme: &Theme,
    size: Size,
    snapshot: &JournalTradeSnapshot,
    state: &JournalSnapshotCanvasState,
) {
    if snapshot.candles.is_empty() || size.width <= 20.0 || size.height <= 20.0 {
        return;
    }

    let loaded_range = loaded_time_range(snapshot);
    let (view_start_ms, view_end_ms) = state.view_or_full_range(snapshot, loaded_range);
    let visible_candles: Vec<Candle> = snapshot
        .candles
        .iter()
        .filter(|candle| candle.close_time >= view_start_ms && candle.open_time <= view_end_ms)
        .cloned()
        .collect();
    let visible_markers: Vec<TradeMarker> = snapshot
        .markers
        .iter()
        .filter(|marker| marker.time_ms >= view_start_ms && marker.time_ms <= view_end_ms)
        .copied()
        .collect();
    let scale_candles = if visible_candles.is_empty() {
        snapshot.candles.as_slice()
    } else {
        visible_candles.as_slice()
    };
    // A live position has no opening fills, so keep its entry level inside the
    // price range — the chart's whole purpose is to show price relative to it.
    let extra_price = snapshot
        .live_position
        .then_some(snapshot.metrics.entry_price);
    let plot = SnapshotPlot::new(size, view_start_ms, view_end_ms, scale_candles, extra_price);
    draw_grid(frame, theme, plot);
    if !visible_candles.is_empty() {
        draw_candles(frame, theme, plot, &visible_candles);
    }
    if snapshot.live_position {
        draw_entry_line(frame, theme, plot, snapshot.metrics.entry_price);
    } else {
        draw_guides(
            frame,
            theme,
            plot,
            snapshot.trade_start_ms,
            snapshot.trade_end_ms,
            snapshot.is_open,
        );
    }
    draw_markers(frame, theme, plot, &visible_markers);
}

#[derive(Debug, Clone, Copy)]
struct SnapshotPlot {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    start_ms: u64,
    end_ms: u64,
    min_price: f64,
    max_price: f64,
}

impl SnapshotPlot {
    fn new(
        size: Size,
        start_ms: u64,
        end_ms: u64,
        candles: &[Candle],
        extra_price: Option<f64>,
    ) -> Self {
        let left = SNAPSHOT_LEFT_PAD;
        let top = SNAPSHOT_TOP_PAD;
        let width = (size.width - SNAPSHOT_LEFT_PAD - SNAPSHOT_RIGHT_PAD).max(1.0);
        let height = (size.height - SNAPSHOT_TOP_PAD - SNAPSHOT_BOTTOM_PAD).max(1.0);
        let (min_price, max_price) = price_range(candles, extra_price);
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

    fn x_for_time(self, time_ms: u64) -> f32 {
        let span = self.end_ms.saturating_sub(self.start_ms).max(1) as f64;
        let offset = time_ms.saturating_sub(self.start_ms) as f64 / span;
        self.left + (offset as f32).clamp(0.0, 1.0) * self.width
    }

    fn y_for_price(self, price: f64) -> f32 {
        let span = (self.max_price - self.min_price).max(f64::EPSILON);
        let offset = ((price - self.min_price) / span).clamp(0.0, 1.0);
        self.top + (1.0 - offset as f32) * self.height
    }
}

impl JournalSnapshotCanvasState {
    fn view_or_full_range(
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

fn update_snapshot_interaction(
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

fn loaded_time_range(snapshot: &JournalTradeSnapshot) -> (u64, u64) {
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

fn snapshot_plot_width(size: Size) -> f32 {
    (size.width - SNAPSHOT_LEFT_PAD - SNAPSHOT_RIGHT_PAD).max(1.0)
}

fn point_in_snapshot_plot(size: Size, pos: Point) -> bool {
    pos.x >= SNAPSHOT_LEFT_PAD
        && pos.x <= size.width - SNAPSHOT_RIGHT_PAD
        && pos.y >= SNAPSHOT_TOP_PAD
        && pos.y <= size.height - SNAPSHOT_BOTTOM_PAD
}

fn price_range(candles: &[Candle], extra_price: Option<f64>) -> (f64, f64) {
    let (mut min_price, mut max_price) = candles.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(min_price, max_price), candle| (min_price.min(candle.low), max_price.max(candle.high)),
    );

    if let Some(extra) = extra_price.filter(|price| price.is_finite() && *price > 0.0) {
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

fn draw_grid(frame: &mut canvas::Frame, theme: &Theme, plot: SnapshotPlot) {
    for fraction in [0.25_f32, 0.5, 0.75] {
        let y = plot.top + plot.height * fraction;
        let path = canvas::Path::line(
            Point::new(plot.left, y),
            Point::new(plot.left + plot.width, y),
        );
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.08,
                    ..theme.palette().text
                })
                .with_width(1.0),
        );
    }
}

/// Horizontal entry-level guide for a live position (no opening fills, so the
/// vertical OPEN/CLOSE boundaries don't apply).
fn draw_entry_line(frame: &mut canvas::Frame, theme: &Theme, plot: SnapshotPlot, entry_price: f64) {
    if !entry_price.is_finite() || entry_price <= 0.0 {
        return;
    }

    let color = theme.palette().primary;
    let y = plot.y_for_price(entry_price);
    let path = canvas::Path::line(
        Point::new(plot.left, y),
        Point::new(plot.left + plot.width, y),
    );
    let mut stroke = canvas::Stroke::default()
        .with_color(Color { a: 0.7, ..color })
        .with_width(1.2);
    stroke.line_dash = canvas::stroke::LineDash {
        segments: &[5.0, 3.0],
        offset: 0,
    };
    frame.stroke(&path, stroke);

    let label = format!("ENTRY {}", format_price(entry_price));
    let label_width = (label.len() as f32 * 5.6 + 8.0).min(plot.width);
    let label_top = (y - 13.0).max(plot.top);
    frame.fill_rectangle(
        Point::new(plot.left, label_top),
        Size::new(label_width, 12.0),
        Color {
            a: 0.88,
            ..theme.extended_palette().background.strong.color
        },
    );
    frame.fill_text(canvas::Text {
        content: label,
        position: Point::new(plot.left + 4.0, label_top + 1.0),
        color,
        size: iced::Pixels(9.0),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Top,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

fn draw_guides(
    frame: &mut canvas::Frame,
    theme: &Theme,
    plot: SnapshotPlot,
    trade_start_ms: u64,
    trade_end_ms: u64,
    is_open: bool,
) {
    draw_boundary_marker(
        frame,
        theme,
        plot,
        trade_start_ms,
        "OPEN",
        theme.palette().primary,
        &[5.0, 3.0],
    );
    draw_boundary_marker(
        frame,
        theme,
        plot,
        trade_end_ms,
        if is_open { "NOW" } else { "CLOSE" },
        theme.extended_palette().background.weak.text,
        &[2.0, 3.0],
    );
}

fn draw_boundary_marker(
    frame: &mut canvas::Frame,
    theme: &Theme,
    plot: SnapshotPlot,
    time_ms: u64,
    label: &'static str,
    color: Color,
    dash_segments: &'static [f32],
) {
    if time_ms < plot.start_ms || time_ms > plot.end_ms {
        return;
    }

    let x = plot.x_for_time(time_ms);
    let path = canvas::Path::line(
        Point::new(x, plot.top - 2.0),
        Point::new(x, plot.top + plot.height),
    );
    let mut stroke = canvas::Stroke::default()
        .with_color(Color { a: 0.62, ..color })
        .with_width(1.2);
    stroke.line_dash = canvas::stroke::LineDash {
        segments: dash_segments,
        offset: 0,
    };
    frame.stroke(&path, stroke);

    let label_width = if label == "CLOSE" { 38.0 } else { 30.0 };
    let label_x = (x - label_width / 2.0)
        .max(plot.left)
        .min(plot.left + plot.width - label_width);
    frame.fill_rectangle(
        Point::new(label_x, 1.0),
        Size::new(label_width, 13.0),
        Color {
            a: 0.88,
            ..theme.extended_palette().background.strong.color
        },
    );
    frame.fill_text(canvas::Text {
        content: label.to_string(),
        position: Point::new(label_x + label_width / 2.0, 3.0),
        color,
        size: iced::Pixels(9.0),
        align_x: alignment::Horizontal::Center.into(),
        align_y: alignment::Vertical::Top,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

fn draw_candles(frame: &mut canvas::Frame, theme: &Theme, plot: SnapshotPlot, candles: &[Candle]) {
    let candle_width = (plot.width / candles.len().max(1) as f32 * 0.58).clamp(2.0, 8.0);
    for candle in candles {
        let x = plot.x_for_time(candle.open_time);
        let open_y = plot.y_for_price(candle.open);
        let close_y = plot.y_for_price(candle.close);
        let high_y = plot.y_for_price(candle.high);
        let low_y = plot.y_for_price(candle.low);
        let color = Color {
            a: 0.82,
            ..if candle.close >= candle.open {
                theme.palette().success
            } else {
                theme.palette().danger
            }
        };

        let wick = canvas::Path::line(Point::new(x, high_y), Point::new(x, low_y));
        frame.stroke(
            &wick,
            canvas::Stroke::default().with_color(color).with_width(1.0),
        );

        frame.fill_rectangle(
            Point::new(x - candle_width / 2.0, open_y.min(close_y)),
            Size::new(candle_width, (open_y - close_y).abs().max(1.0)),
            color,
        );
    }
}

fn draw_markers(
    frame: &mut canvas::Frame,
    theme: &Theme,
    plot: SnapshotPlot,
    markers: &[TradeMarker],
) {
    let mut marker_layouts = marker_group_layouts(plot, markers);
    marker_layouts.sort_by(|a, b| a.center.x.total_cmp(&b.center.x));
    let outline_color = Color {
        a: 0.72,
        ..theme.extended_palette().background.strong.color
    };

    for layout in marker_layouts {
        let marker_color = Color {
            a: 0.9,
            ..if layout.is_buy {
                theme.palette().success
            } else {
                theme.palette().danger
            }
        };
        let dot = canvas::Path::circle(layout.center, layout.radius);
        frame.fill(&dot, marker_color);
        frame.stroke(
            &dot,
            canvas::Stroke::default()
                .with_color(outline_color)
                .with_width(0.75),
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct SnapshotMarkerLayout {
    center: Point,
    radius: f32,
    is_buy: bool,
}

fn marker_group_layouts(plot: SnapshotPlot, markers: &[TradeMarker]) -> Vec<SnapshotMarkerLayout> {
    let mut buys: Vec<_> = markers
        .iter()
        .filter(|marker| marker.is_buy)
        .map(|marker| plot.x_for_time(marker.time_ms))
        .collect();
    let mut sells: Vec<_> = markers
        .iter()
        .filter(|marker| !marker.is_buy)
        .map(|marker| plot.x_for_time(marker.time_ms))
        .collect();
    buys.sort_by(|a, b| a.total_cmp(b));
    sells.sort_by(|a, b| a.total_cmp(b));

    let mut layouts = Vec::with_capacity(markers.len());
    push_marker_side_layouts(plot, &sells, false, &mut layouts);
    push_marker_side_layouts(plot, &buys, true, &mut layouts);
    layouts
}

fn push_marker_side_layouts(
    plot: SnapshotPlot,
    marker_xs: &[f32],
    is_buy: bool,
    layouts: &mut Vec<SnapshotMarkerLayout>,
) {
    let Some((&first_x, rest)) = marker_xs.split_first() else {
        return;
    };

    let y = if is_buy {
        plot.top + plot.height + SNAPSHOT_MARKER_OFFSET
    } else {
        plot.top - SNAPSHOT_MARKER_OFFSET
    };
    let mut group_sum = first_x;
    let mut group_count = 1_usize;
    let mut group_last_x = first_x;

    for &x in rest {
        if x - group_last_x > SNAPSHOT_MARKER_GROUP_DISTANCE {
            push_marker_group(layouts, group_sum, group_count, y, is_buy);
            group_sum = x;
            group_count = 1;
        } else {
            group_sum += x;
            group_count += 1;
        }
        group_last_x = x;
    }

    push_marker_group(layouts, group_sum, group_count, y, is_buy);
}

fn push_marker_group(
    layouts: &mut Vec<SnapshotMarkerLayout>,
    group_sum: f32,
    group_count: usize,
    y: f32,
    is_buy: bool,
) {
    let x = group_sum / group_count.max(1) as f32;
    let radius = if group_count > 1 {
        SNAPSHOT_MARKER_GROUP_RADIUS
    } else {
        SNAPSHOT_MARKER_RADIUS
    };
    layouts.push(SnapshotMarkerLayout {
        center: Point::new(x, y),
        radius,
        is_buy,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_view_range_allows_empty_time_beyond_loaded_range() {
        let loaded_range = (1_000, 2_000);
        let visual_range = (500, 2_500);

        let (start, end) = clamp_view_range(500, 2_500, loaded_range, visual_range, 100);

        assert_eq!((start, end), (500, 2_500));
    }

    #[test]
    fn clamp_view_range_keeps_some_loaded_context_visible() {
        let loaded_range = (1_000, 2_000);
        let visual_range = (0, 3_000);

        let (start, end) = clamp_view_range(2_200, 2_800, loaded_range, visual_range, 100);

        assert!(ranges_overlap((start, end), loaded_range));
        assert!(end > loaded_range.1);
    }

    #[test]
    fn marker_group_layouts_collapse_nearby_fills() {
        let plot = SnapshotPlot {
            left: 0.0,
            top: 50.0,
            width: 100.0,
            height: 100.0,
            start_ms: 0,
            end_ms: 1_000,
            min_price: 1.0,
            max_price: 2.0,
        };
        let markers = vec![
            TradeMarker {
                time_ms: 100,
                price: 1.0,
                size: 1.0,
                is_buy: true,
            },
            TradeMarker {
                time_ms: 105,
                price: 1.0,
                size: 1.0,
                is_buy: true,
            },
            TradeMarker {
                time_ms: 350,
                price: 1.0,
                size: 1.0,
                is_buy: true,
            },
        ];

        let layouts = marker_group_layouts(plot, &markers);

        assert_eq!(layouts.len(), 2);
        assert!(layouts.iter().all(|layout| layout.is_buy));
        assert_eq!(layouts[0].radius, SNAPSHOT_MARKER_GROUP_RADIUS);
        assert!(layouts[1].center.x - layouts[0].center.x > layouts[0].radius + layouts[1].radius);
    }

    #[test]
    fn marker_group_layouts_place_sells_above_and_buys_below_plot() {
        let plot = SnapshotPlot {
            left: 0.0,
            top: 50.0,
            width: 100.0,
            height: 100.0,
            start_ms: 0,
            end_ms: 1_000,
            min_price: 1.0,
            max_price: 2.0,
        };
        let markers = vec![
            TradeMarker {
                time_ms: 100,
                price: 1.0,
                size: 1.0,
                is_buy: false,
            },
            TradeMarker {
                time_ms: 900,
                price: 1.0,
                size: 1.0,
                is_buy: true,
            },
        ];

        let layouts = marker_group_layouts(plot, &markers);
        let sell = layouts
            .iter()
            .find(|layout| !layout.is_buy)
            .expect("sell marker");
        let buy = layouts
            .iter()
            .find(|layout| layout.is_buy)
            .expect("buy marker");

        assert_eq!(sell.radius, SNAPSHOT_MARKER_RADIUS);
        assert_eq!(buy.radius, SNAPSHOT_MARKER_RADIUS);
        assert!(sell.center.y + sell.radius <= plot.top - SNAPSHOT_MARKER_CHART_GAP);
        assert!(buy.center.y - buy.radius >= plot.top + plot.height + SNAPSHOT_MARKER_CHART_GAP);
    }
}
