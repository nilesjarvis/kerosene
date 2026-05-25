use crate::app_state::TradingTerminal;
use crate::twap_state::{TwapEventKind, TwapPendingSlice};

use std::time::Instant;

// ---------------------------------------------------------------------------
// TWAP Skip Recording
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn record_twap_slice_skip(
        &mut self,
        twap_id: u64,
        now: Instant,
        retry_slice: Option<&TwapPendingSlice>,
        kind: TwapEventKind,
        message: String,
        is_error: bool,
    ) {
        if let Some(slice) = retry_slice {
            self.record_twap_retry_skip(twap_id, now, slice.index, kind, message, is_error);
        } else {
            self.record_twap_skip(twap_id, now, kind, message, is_error);
        }
    }
}
