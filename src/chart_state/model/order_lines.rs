use super::ChartInstance;

// ---------------------------------------------------------------------------
// Active Order Line Animation
// ---------------------------------------------------------------------------

const ORDER_LINE_STRIDE: f32 = 12.0;
const ORDER_LINE_PHASE_STEP: f32 = 1.2;

impl ChartInstance {
    pub(crate) fn advance_order_line_animation(&mut self) {
        if !self.chart.active_orders.is_empty() {
            self.chart.order_line_phase =
                (self.chart.order_line_phase + ORDER_LINE_PHASE_STEP).rem_euclid(ORDER_LINE_STRIDE);
        }
    }
}
