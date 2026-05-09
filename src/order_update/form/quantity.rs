// ---------------------------------------------------------------------------
// Order Quantity Math
// ---------------------------------------------------------------------------

mod denomination;
mod percentage;
#[cfg(test)]
mod tests;

pub(super) use self::denomination::toggled_order_quantity_text;
pub(super) use self::percentage::{order_percentage_for_quantity, quantity_for_percentage};
