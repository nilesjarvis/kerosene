use iced::Point;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum DragKind {
    PanX,
    PanY,
}

#[derive(Debug)]
pub struct SpaghettiChartState {
    pub(super) cursor_position: Option<Point>,
    /// Scroll offset in milliseconds from the right edge (positive = past).
    pub(super) scroll_offset_ms: f64,
    /// Pixels per millisecond (controls zoom level).
    pub(super) px_per_ms: f64,
    pub(super) y_auto: bool,
    pub(super) y_offset: f64,
    pub(super) y_scale: f64,
    pub(super) drag: Option<DragKind>,
    pub(super) drag_start: Option<Point>,
    pub(super) drag_start_scroll: f64,
    pub(super) drag_start_y_offset: f64,
    pub(super) reset_epoch_seen: u64,
}

/// Default zoom: ~10 pixels per hour.
pub(super) const DEFAULT_PX_PER_MS: f64 = 10.0 / 3_600_000.0;
pub(super) const MIN_PX_PER_MS: f64 = 0.01 / 3_600_000.0;
pub(super) const MAX_PX_PER_MS: f64 = 200.0 / 3_600_000.0;

impl Default for SpaghettiChartState {
    fn default() -> Self {
        Self {
            cursor_position: None,
            scroll_offset_ms: 0.0,
            px_per_ms: DEFAULT_PX_PER_MS,
            y_auto: true,
            y_offset: 0.0,
            y_scale: 1.0,
            drag: None,
            drag_start: None,
            drag_start_scroll: 0.0,
            drag_start_y_offset: 0.0,
            reset_epoch_seen: 0,
        }
    }
}

impl SpaghettiChartState {
    pub(super) fn reset_view_with_px(&mut self, reset_epoch: u64, px_per_ms: f64) {
        self.scroll_offset_ms = 0.0;
        self.px_per_ms = px_per_ms.clamp(MIN_PX_PER_MS, MAX_PX_PER_MS);
        self.y_auto = true;
        self.y_offset = 0.0;
        self.y_scale = 1.0;
        self.drag = None;
        self.drag_start = None;
        self.drag_start_scroll = 0.0;
        self.drag_start_y_offset = 0.0;
        self.reset_epoch_seen = reset_epoch;
    }
}
