/// Request identity and a coalesced follow-up for portfolio and income reads.
#[derive(Debug, Clone, Default)]
pub(crate) struct AnalyticsRefreshState {
    pub(crate) loading: bool,
    pub(crate) request_id: u64,
    pub(crate) followup_pending: bool,
}

impl AnalyticsRefreshState {
    pub(crate) fn begin(&mut self) -> u64 {
        self.request_id = self.request_id.saturating_add(1);
        self.loading = true;
        self.request_id
    }

    pub(crate) fn finish(&mut self, request_id: u64) -> bool {
        if self.request_id != request_id {
            return false;
        }
        self.request_id = self.request_id.saturating_add(1);
        self.loading = false;
        true
    }

    pub(crate) fn queue_followup(&mut self) {
        self.followup_pending = true;
    }

    pub(crate) fn take_followup(&mut self) -> bool {
        std::mem::take(&mut self.followup_pending)
    }

    pub(crate) fn invalidate(&mut self) {
        self.request_id = self.request_id.saturating_add(1);
        self.loading = false;
        self.followup_pending = false;
    }
}
