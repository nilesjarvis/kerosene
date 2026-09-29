use super::*;
use std::{cell::RefCell, task::Poll};

#[tokio::test]
async fn recovery_requests_then_emits_then_pauses_and_stops_on_failure() {
    for reconnect_ok in [false, true] {
        for emit_ok in [false, true] {
            for pause in [Duration::ZERO, Duration::from_secs(3600)] {
                let steps = RefCell::new(Vec::new());
                let recovery = emit_after_reconnect(
                    || {
                        steps.borrow_mut().push("reconnect");
                        reconnect_ok
                    },
                    7,
                    |event| {
                        let steps = &steps;
                        async move {
                            assert_eq!(event, 7);
                            steps.borrow_mut().push("emit");
                            emit_ok
                        }
                    },
                    pause,
                );
                assert!(steps.borrow().is_empty(), "recovery starts when polled");
                let mut recovery = std::pin::pin!(recovery);
                let result = futures::poll!(recovery.as_mut());

                assert_eq!(
                    *steps.borrow(),
                    if reconnect_ok {
                        vec!["reconnect", "emit"]
                    } else {
                        vec!["reconnect"]
                    }
                );
                if !reconnect_ok || !emit_ok {
                    assert_eq!(result, Poll::Ready(false));
                } else if pause.is_zero() {
                    assert_eq!(result, Poll::Ready(true));
                } else {
                    assert_eq!(result, Poll::Pending, "pause follows successful emission");
                }
            }
        }
    }
}
