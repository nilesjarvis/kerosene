use super::super::move_order_wire_is_supported;
use super::fixtures::open_order;

#[test]
fn move_order_wire_rejects_trigger_orders() {
    let mut order = open_order("BTC", 42, "100");
    order.is_trigger = Some(true);
    order.trigger_px = Some("95".to_string());

    let error = move_order_wire_is_supported(&order).unwrap_err();

    assert!(error.contains("trigger orders"));
}

#[test]
fn move_order_wire_accepts_limit_orders_with_zero_trigger_px_metadata() {
    let mut order = open_order("BTC", 42, "100");
    order.is_trigger = Some(false);
    order.trigger_px = Some("0.0".to_string());

    assert!(move_order_wire_is_supported(&order).is_ok());
}

#[test]
fn move_order_wire_rejects_known_non_gtc_orders() {
    let mut order = open_order("BTC", 42, "100");
    order.tif = Some("Ioc".to_string());

    let error = move_order_wire_is_supported(&order).unwrap_err();

    assert!(error.contains("non-GTC"));
}

#[test]
fn move_order_wire_preserves_metadata_acceptance_and_rejection_priority() {
    for is_trigger in [None, Some(false), Some(true)] {
        for (trigger_px, positive_trigger) in [
            (None, false),
            (Some("0"), false),
            (Some("-0"), false),
            (Some("-1"), false),
            (Some("NaN"), false),
            (Some("inf"), false),
            (Some("-inf"), false),
            (Some("1e309"), false),
            (Some("invalid"), false),
            (Some(""), false),
            (Some(" 1 "), true),
            (Some("1e-9"), true),
            (Some("+1"), true),
            (Some("0.0000000001"), true),
        ] {
            for (order_type, limit_type) in [
                (None, true),
                (Some("limit"), true),
                (Some("LiMiT"), true),
                (Some("Market"), false),
                (Some(""), false),
                (Some(" limit"), false),
            ] {
                for (tif, gtc) in [
                    (None, true),
                    (Some("Gtc"), true),
                    (Some("gTc"), true),
                    (Some("Ioc"), false),
                    (Some("Alo"), false),
                    (Some("Gtc "), false),
                ] {
                    let mut order = open_order("BTC", 42, "100");
                    order.is_trigger = is_trigger;
                    order.trigger_px = trigger_px.map(str::to_string);
                    order.order_type = order_type.map(str::to_string);
                    order.tif = tif.map(str::to_string);
                    let expected = if is_trigger == Some(true) || positive_trigger {
                        Err("Move failed: trigger orders cannot be moved safely yet")
                    } else if !limit_type {
                        Err("Move failed: order type cannot be moved safely yet")
                    } else if !gtc {
                        Err("Move failed: non-GTC orders cannot be moved safely yet")
                    } else {
                        Ok(())
                    };

                    assert_eq!(
                        move_order_wire_is_supported(&order),
                        expected,
                        "metadata: {is_trigger:?}, {trigger_px:?}, {order_type:?}, {tif:?}"
                    );
                }
            }
        }
    }
}
