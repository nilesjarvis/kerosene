use super::super::state::DragKind;
use super::super::{CandlestickChart, ChartState};
use iced::Rectangle;
use iced::mouse;

// ---------------------------------------------------------------------------
// Mouse Cursor Selection
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(in crate::chart) fn mouse_interaction_for(
        &self,
        state: &ChartState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let Some(pos) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };
        let chart_w = bounds.width - self.price_axis_width();
        let (chart_h, funding_panel_h, session_panel_h) = self.chart_area_heights(bounds.height);
        if chart_w <= 0.0
            || chart_h <= 0.0
            || !chart_w.is_finite()
            || !chart_h.is_finite()
            || !bounds.width.is_finite()
            || !bounds.height.is_finite()
        {
            return mouse::Interaction::default();
        }

        let drawable_h = chart_h + funding_panel_h + session_panel_h;
        let on_funding_resize = pos.x < chart_w
            && self
                .funding_panel_resize_target_y(bounds.height, pos.y)
                .is_some();
        let on_price_axis = pos.x >= chart_w && pos.y < chart_h;
        let on_funding_axis =
            pos.x >= chart_w && pos.y >= chart_h && pos.y < chart_h + funding_panel_h;

        match state.drag {
            Some(DragKind::PanX) => mouse::Interaction::Grabbing,
            Some(DragKind::PanY) => mouse::Interaction::ResizingVertically,
            Some(DragKind::PanFundingY) => mouse::Interaction::ResizingVertically,
            Some(DragKind::ResizeFundingPanel) => mouse::Interaction::ResizingVertically,
            Some(DragKind::MoveOrder { .. }) => mouse::Interaction::Grabbing,
            None => {
                if self.active_tool.is_some() && pos.x < chart_w && pos.y < chart_h {
                    // Custom reticles are drawn on the canvas; hide the OS cursor over the plot.
                    mouse::Interaction::Hidden
                } else if state.hover_order_oid.is_some() && pos.x < chart_w && pos.y < chart_h {
                    mouse::Interaction::Grab
                } else if on_funding_resize || on_price_axis || on_funding_axis {
                    mouse::Interaction::ResizingVertically
                } else if pos.x < chart_w && pos.y < drawable_h {
                    // Custom reticles are drawn on the canvas; hide the OS cursor over the plot.
                    mouse::Interaction::Hidden
                } else {
                    mouse::Interaction::default()
                }
            }
        }
    }
}
