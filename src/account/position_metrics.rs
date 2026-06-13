// ---------------------------------------------------------------------------
// Position Math
// ---------------------------------------------------------------------------

pub(crate) fn position_notional_from_mark_or_wire(
    szi: Option<f64>,
    wire_value: Option<f64>,
    mark_px: Option<f64>,
) -> Option<f64> {
    match (szi, mark_px) {
        (Some(szi), Some(mark_px)) => Some(szi.abs() * mark_px),
        _ => wire_value.map(f64::abs),
    }
}

pub(crate) fn position_upnl_from_mark_or_wire(
    szi: Option<f64>,
    entry_px: Option<f64>,
    wire_upnl: Option<f64>,
    mark_px: Option<f64>,
) -> Option<f64> {
    match (szi, entry_px, mark_px) {
        (Some(szi), Some(entry_px), Some(mark_px)) => Some(szi * (mark_px - entry_px)),
        _ => wire_upnl,
    }
}

#[cfg(test)]
mod tests {
    use super::{position_notional_from_mark_or_wire, position_upnl_from_mark_or_wire};

    #[test]
    fn position_notional_prefers_live_mark_only_with_valid_size() {
        assert_eq!(
            position_notional_from_mark_or_wire(Some(-2.0), Some(999.0), Some(100.0)),
            Some(200.0)
        );
        assert_eq!(
            position_notional_from_mark_or_wire(None, Some(999.0), Some(100.0)),
            Some(999.0)
        );
        assert_eq!(
            position_notional_from_mark_or_wire(Some(-2.0), Some(-250.0), None),
            Some(250.0)
        );
        assert_eq!(
            position_notional_from_mark_or_wire(Some(-2.0), None, None),
            None
        );
    }

    #[test]
    fn position_upnl_prefers_live_mark_only_with_valid_inputs() {
        assert_eq!(
            position_upnl_from_mark_or_wire(Some(2.0), Some(90.0), Some(1.0), Some(100.0)),
            Some(20.0)
        );
        assert_eq!(
            position_upnl_from_mark_or_wire(Some(2.0), None, Some(1.0), Some(100.0)),
            Some(1.0)
        );
        assert_eq!(
            position_upnl_from_mark_or_wire(Some(2.0), Some(90.0), None, None),
            None
        );
    }

    #[test]
    fn position_upnl_preserves_short_side_sign() {
        assert_eq!(
            position_upnl_from_mark_or_wire(Some(-2.0), Some(100.0), Some(-99.0), Some(90.0)),
            Some(20.0)
        );
        assert_eq!(
            position_upnl_from_mark_or_wire(Some(-2.0), Some(100.0), Some(-99.0), Some(110.0)),
            Some(-20.0)
        );
    }
}
