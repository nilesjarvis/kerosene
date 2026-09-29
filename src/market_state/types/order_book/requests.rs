use super::OrderBookInstance;
use crate::helpers::tick_sizes_match;

// ---------------------------------------------------------------------------
// Pending Order Book Requests
// ---------------------------------------------------------------------------

pub(super) struct PendingOrderBookRequest {
    request_id: u64,
    symbol: String,
    tick_size: f64,
    sigfigs: (Option<u8>, Option<u8>),
}

impl PendingOrderBookRequest {
    fn matches(&self, symbol: &str, tick_size: f64, sigfigs: (Option<u8>, Option<u8>)) -> bool {
        self.symbol == symbol
            && tick_sizes_match(self.tick_size, tick_size)
            && self.sigfigs == sigfigs
    }
}

impl OrderBookInstance {
    pub fn pending_book_sigfigs(&self) -> Option<(Option<u8>, Option<u8>)> {
        self.pending_book_request
            .as_ref()
            .map(|request| request.sigfigs)
    }

    #[cfg(test)]
    pub(crate) fn pending_book_request_id(&self) -> Option<u64> {
        self.pending_book_request
            .as_ref()
            .map(|request| request.request_id)
    }

    pub fn pending_book_request_matches(
        &self,
        symbol: &str,
        tick_size: f64,
        sigfigs: (Option<u8>, Option<u8>),
    ) -> bool {
        self.pending_book_request
            .as_ref()
            .is_some_and(|request| request.matches(symbol, tick_size, sigfigs))
    }

    pub fn pending_book_request_matches_id(
        &self,
        request_id: u64,
        symbol: &str,
        tick_size: f64,
        sigfigs: (Option<u8>, Option<u8>),
    ) -> bool {
        self.pending_book_request.as_ref().is_some_and(|request| {
            request.request_id == request_id && request.matches(symbol, tick_size, sigfigs)
        })
    }

    pub fn mark_book_request(
        &mut self,
        symbol: String,
        tick_size: f64,
        sigfigs: (Option<u8>, Option<u8>),
    ) -> u64 {
        self.next_book_request_id = self.next_book_request_id.wrapping_add(1);
        let request_id = self.next_book_request_id;
        self.pending_book_request = Some(PendingOrderBookRequest {
            request_id,
            symbol,
            tick_size,
            sigfigs,
        });
        request_id
    }

    pub fn clear_matching_book_request(
        &mut self,
        request_id: u64,
        symbol: &str,
        tick_size: f64,
        sigfigs: (Option<u8>, Option<u8>),
    ) {
        if self.pending_book_request_matches_id(request_id, symbol, tick_size, sigfigs) {
            self.pending_book_request = None;
        }
    }

    pub fn clear_book_request(&mut self) {
        self.pending_book_request = None;
    }
}

#[cfg(test)]
mod tests;
