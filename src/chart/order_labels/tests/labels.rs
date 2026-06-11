use super::*;
use crate::chart::OrderOverlayPendingState;

fn order(coin: &str, sz: f64) -> OrderOverlay {
    OrderOverlay {
        coin: coin.to_string(),
        limit_px: 0.5,
        sz,
        is_buy: true,
        oid: 1,
        is_moving: false,
        pending_state: None,
    }
}

#[test]
fn order_side_label_formats_outcome_sizes_as_whole_contracts() {
    assert_eq!(order_side_label(&order("#950", 5.0)), "BUY 5");

    let mut sell = order("#950", 12.0);
    sell.is_buy = false;
    assert_eq!(order_side_label(&sell), "SELL 12");
}

#[test]
fn order_side_label_keeps_fractional_sizes_for_non_outcome_coins() {
    assert_eq!(order_side_label(&order("BTC", 0.5)), "BUY 0.5000");
}

#[test]
fn pending_order_labels_use_outcome_size_formatting() {
    let mut cancelling = order("#950", 3.0);
    cancelling.pending_state = Some(OrderOverlayPendingState::Cancelling);
    assert_eq!(order_side_label(&cancelling), "CXL 3");

    let mut modifying = order("#950", 3.0);
    modifying.pending_state = Some(OrderOverlayPendingState::Modifying);
    assert_eq!(order_side_label(&modifying), "MOD 3");
}
