use super::DEFAULT_CANDLE_WIDTH;
use iced::Point;

mod export;

// ---------------------------------------------------------------------------
// Chart Interaction State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum DragKind {
    /// Dragging on the main chart area -- pans the X axis.
    PanX,
    /// Dragging on the price axis -- scales / pans the Y axis.
    PanY,
    /// Dragging on the funding panel -- pans the funding Y axis.
    PanFundingY,
    /// Dragging the top edge of the funding sub-panel.
    ResizeFundingPanel,
    /// Dragging an order line to a new price.
    MoveOrder { oid: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HudOrderKind {
    Limit,
    Market,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HudMarketSide {
    Long,
    Short,
}

impl HudMarketSide {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Long => "LONG",
            Self::Short => "SHORT",
        }
    }
}

/// Widget-local mutable state for the canvas (managed by iced runtime).
#[derive(Debug)]
pub struct ChartState {
    pub(super) cursor_position: Option<Point>,
    pub(super) scroll_offset: f32,
    pub(super) candle_width: f32,
    pub(super) y_auto: bool,
    pub(super) y_offset: f64,
    pub(super) y_scale: f64,
    pub(super) funding_y_scale: f64,
    pub(super) funding_y_offset: f64,
    pub(super) drag: Option<DragKind>,
    pub(super) drag_start: Option<Point>,
    pub(super) drag_start_scroll: f32,
    pub(super) drag_start_y_offset: f64,
    pub(super) drag_start_funding_panel_height: f32,
    pub(super) drag_funding_panel_height: Option<f32>,
    /// Temporary price for an order being dragged (live preview).
    pub(super) drag_order_new_price: Option<f64>,
    /// OID of the order line the cursor is currently hovering near
    /// (used for grab cursor feedback).
    pub(super) hover_order_oid: Option<u64>,
    /// First anchor for two-click drawing tools (trend line).
    /// Stored as (timestamp_ms, price).
    pub(super) pending_anchor: Option<(u64, f64)>,
    /// True while Shift is pressed.
    pub(super) shift_down: bool,
    /// True while Ctrl is pressed.
    pub(super) ctrl_down: bool,
    /// Visual-only order mode used by the HUD game-mode overlay.
    pub(super) hud_order_kind: HudOrderKind,
    /// Visual-only market side used by the HUD game-mode overlay.
    pub(super) hud_market_side: HudMarketSide,
    /// Visual-only coin size shown by the HUD game-mode overlay.
    pub(super) hud_size_input: String,
    /// True while typed keyboard input should update the HUD size.
    pub(super) hud_size_editing: bool,
    /// Clears the current HUD size on the next numeric key after the edit hotkey.
    pub(super) hud_size_replace_on_type: bool,
    /// Last Ctrl+scroll direction, used only to angle the HUD size scroller.
    pub(super) hud_size_scroll_bias: f32,
    /// True while HUD mode keeps the price viewport centered on the latest candle.
    pub(super) hud_follow_price: bool,
    /// Anchor price for Shift+click range measurement.
    pub(super) range_anchor_price: Option<f64>,
    pub(super) reset_epoch_seen: u64,
}

impl Default for ChartState {
    fn default() -> Self {
        Self {
            cursor_position: None,
            scroll_offset: 0.0,
            candle_width: DEFAULT_CANDLE_WIDTH,
            y_auto: true,
            y_offset: 0.0,
            y_scale: 1.0,
            funding_y_scale: 1.0,
            funding_y_offset: 0.0,
            drag: None,
            drag_start: None,
            drag_start_scroll: 0.0,
            drag_start_y_offset: 0.0,
            drag_start_funding_panel_height: 0.0,
            drag_funding_panel_height: None,
            drag_order_new_price: None,
            hover_order_oid: None,
            pending_anchor: None,
            shift_down: false,
            ctrl_down: false,
            hud_order_kind: HudOrderKind::Limit,
            hud_market_side: HudMarketSide::Long,
            hud_size_input: "1".to_string(),
            hud_size_editing: false,
            hud_size_replace_on_type: false,
            hud_size_scroll_bias: 0.0,
            hud_follow_price: false,
            range_anchor_price: None,
            reset_epoch_seen: 0,
        }
    }
}

impl ChartState {
    pub(super) fn reset_view(&mut self, reset_epoch: u64) {
        self.scroll_offset = 0.0;
        self.candle_width = DEFAULT_CANDLE_WIDTH;
        self.y_auto = true;
        self.y_offset = 0.0;
        self.y_scale = 1.0;
        self.funding_y_scale = 1.0;
        self.funding_y_offset = 0.0;
        self.drag = None;
        self.drag_start = None;
        self.drag_start_scroll = 0.0;
        self.drag_start_y_offset = 0.0;
        self.drag_start_funding_panel_height = 0.0;
        self.drag_funding_panel_height = None;
        self.drag_order_new_price = None;
        self.hover_order_oid = None;
        self.pending_anchor = None;
        self.hud_follow_price = false;
        self.range_anchor_price = None;
        self.reset_epoch_seen = reset_epoch;
    }
}

#[cfg(test)]
mod tests;
