use crate::account::OpenOrder;
use crate::helpers::parse_positive_finite_number;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestingOrderWireError {
    Trigger,
    NonLimit,
    NonGtc,
}

/// Reject known unsupported metadata before moving or adopting a resting order.
/// Missing type/TIF fields are allowed; a positive trigger price is sufficient
/// to reject an order even when its trigger flag is absent or false.
pub(crate) fn validate_resting_order_wire(order: &OpenOrder) -> Result<(), RestingOrderWireError> {
    if order.is_trigger == Some(true)
        || order
            .trigger_px
            .as_deref()
            .and_then(parse_positive_finite_number)
            .is_some()
    {
        return Err(RestingOrderWireError::Trigger);
    }
    if order
        .order_type
        .as_deref()
        .is_some_and(|kind| !kind.eq_ignore_ascii_case("limit"))
    {
        return Err(RestingOrderWireError::NonLimit);
    }
    if order
        .tif
        .as_deref()
        .is_some_and(|tif| !tif.eq_ignore_ascii_case("Gtc"))
    {
        return Err(RestingOrderWireError::NonGtc);
    }
    Ok(())
}
