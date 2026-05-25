use super::*;

#[test]
fn status_retry_advances_attempt_and_uses_twap_delay() {
    assert_eq!(
        next_twap_status_retry(0),
        TwapStatusRetryDecision::Retry {
            attempt: 1,
            delay: TwapOrder::retry_delay(1),
        }
    );
}

#[test]
fn status_retry_exhausts_on_max_attempt() {
    assert_eq!(
        next_twap_status_retry(TWAP_MAX_RETRY_ATTEMPTS - 1),
        TwapStatusRetryDecision::Exhausted {
            attempt: TWAP_MAX_RETRY_ATTEMPTS,
        }
    );
}

#[test]
fn status_retry_attempt_saturates_when_counter_is_already_maxed() {
    assert_eq!(
        next_twap_status_retry(u32::MAX),
        TwapStatusRetryDecision::Exhausted { attempt: u32::MAX }
    );
}
