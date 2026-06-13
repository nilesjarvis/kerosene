use super::ChartInstance;
use crate::order_execution::QuickOrderForm;

// ---------------------------------------------------------------------------
// Quick Order Form State
// ---------------------------------------------------------------------------

const QUICK_ORDER_LIMIT_LINE_STRIDE: f32 = 12.0;
const QUICK_ORDER_LIMIT_LINE_PHASE_STEP: f32 = 1.2;

impl ChartInstance {
    pub(crate) fn quick_order_reopen_values(
        &self,
        fallback_quantity_is_usd: bool,
    ) -> (String, bool, f32, bool) {
        if let Some(form) = &self.quick_order {
            return (
                form.quantity.clone(),
                form.quantity_is_usd,
                form.percentage,
                form.is_limit,
            );
        }

        if self.last_quick_order_symbol == self.symbol {
            return (
                self.last_quick_order_quantity.clone(),
                self.last_quick_order_quantity_is_usd,
                self.last_quick_order_percentage,
                self.last_quick_order_is_limit,
            );
        }

        (
            String::new(),
            fallback_quantity_is_usd,
            0.0,
            self.last_quick_order_is_limit,
        )
    }

    pub(crate) fn set_quick_order(&mut self, form: QuickOrderForm) {
        self.remember_quick_order_form(&form);
        self.chart.quick_order_limit_price = form.is_limit.then_some(form.price);
        self.chart.quick_order_line_phase = 0.0;
        self.quick_order = Some(form);
        self.chart.quick_order_open = true;
    }

    pub(crate) fn clear_quick_order(&mut self) {
        self.remember_current_quick_order();
        self.quick_order = None;
        self.chart.quick_order_open = false;
        self.chart.quick_order_limit_price = None;
        self.chart.quick_order_line_phase = 0.0;
    }

    pub(crate) fn reset_quick_order_for_account_reset(&mut self) {
        self.quick_order = None;
        self.chart.quick_order_open = false;
        self.chart.quick_order_limit_price = None;
        self.chart.quick_order_line_phase = 0.0;
        self.last_quick_order_symbol.clear();
        self.last_quick_order_quantity.clear();
        self.last_quick_order_quantity_is_usd = false;
        self.last_quick_order_percentage = 0.0;
    }

    pub(crate) fn take_quick_order(&mut self) -> Option<QuickOrderForm> {
        self.remember_current_quick_order();
        let form = self.quick_order.take();
        self.chart.quick_order_open = false;
        self.chart.quick_order_limit_price = None;
        self.chart.quick_order_line_phase = 0.0;
        form
    }

    fn remember_current_quick_order(&mut self) {
        let Some(form) = self.quick_order.as_ref() else {
            return;
        };
        let (quantity, quantity_is_usd, percentage) = if form.quantity_provenance.is_some() {
            (String::new(), form.quantity_is_usd, 0.0)
        } else {
            (form.quantity.clone(), form.quantity_is_usd, form.percentage)
        };
        let is_limit = form.is_limit;

        self.last_quick_order_symbol = self.symbol.clone();
        self.last_quick_order_quantity = quantity;
        self.last_quick_order_quantity_is_usd = quantity_is_usd;
        self.last_quick_order_percentage = percentage;
        self.last_quick_order_is_limit = is_limit;
    }

    fn remember_quick_order_form(&mut self, form: &QuickOrderForm) {
        let (quantity, percentage) = if form.quantity_provenance.is_some() {
            (String::new(), 0.0)
        } else {
            (form.quantity.clone(), form.percentage)
        };

        self.last_quick_order_symbol = self.symbol.clone();
        self.last_quick_order_quantity = quantity;
        self.last_quick_order_quantity_is_usd = form.quantity_is_usd;
        self.last_quick_order_percentage = percentage;
        self.last_quick_order_is_limit = form.is_limit;
    }

    pub(crate) fn advance_quick_order_limit_line(&mut self) {
        if self.chart.quick_order_limit_price.is_some() {
            self.chart.quick_order_line_phase = (self.chart.quick_order_line_phase
                + QUICK_ORDER_LIMIT_LINE_PHASE_STEP)
                .rem_euclid(QUICK_ORDER_LIMIT_LINE_STRIDE);
        }
    }
}
