use std::fmt;

// ---------------------------------------------------------------------------
// Order Status Model
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OrderStatusResult {
    pub(crate) status: String,
    pub(crate) oid: Option<u64>,
    pub(crate) cloid: Option<String>,
    pub(crate) raw_summary: String,
}

impl fmt::Debug for OrderStatusResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderStatusResult")
            .field("status", &self.status)
            .field("has_oid", &self.oid.is_some())
            .field("has_cloid", &self.cloid.is_some())
            .field("raw_summary", &"<redacted>")
            .finish()
    }
}

impl OrderStatusResult {
    pub(crate) fn is_missing(&self) -> bool {
        let status = self.status.to_ascii_lowercase();
        status.contains("unknown") || status.contains("missing")
    }

    pub(crate) fn is_open(&self) -> bool {
        self.status.eq_ignore_ascii_case("open")
    }

    pub(crate) fn is_filled(&self) -> bool {
        self.status.eq_ignore_ascii_case("filled")
    }

    pub(crate) fn is_no_fill_terminal(&self) -> bool {
        let status = self.status.to_ascii_lowercase();
        matches!(
            status.as_str(),
            "canceled"
                | "cancelled"
                | "rejected"
                | "ioccancelrejected"
                | "mintradentlrejected"
                | "tickrejected"
                | "reduceonlyrejected"
                | "reduceonlycanceled"
                | "selftradecanceled"
                | "scheduledcancel"
                | "margincanceled"
                | "perpmarginrejected"
                | "insufficientspotbalancerejected"
                | "oraclejected"
                | "oraclerejected"
                | "openinterestcapcanceled"
                | "positionincreaseatopeninterestcaprejected"
                | "positionflipatopeninterestcaprejected"
                | "tooaggressiveatopeninterestcaprejected"
                | "openinterestincreaserejected"
                | "perpmaxpositionrejected"
                | "delistedcanceled"
                | "liquidatedcanceled"
        )
    }

    pub(crate) fn is_definitive_no_fill_terminal(&self) -> bool {
        let status = self.status.to_ascii_lowercase();
        matches!(
            status.as_str(),
            "rejected"
                | "ioccancelrejected"
                | "mintradentlrejected"
                | "tickrejected"
                | "reduceonlyrejected"
                | "perpmarginrejected"
                | "insufficientspotbalancerejected"
                | "oraclejected"
                | "oraclerejected"
                | "positionincreaseatopeninterestcaprejected"
                | "positionflipatopeninterestcaprejected"
                | "tooaggressiveatopeninterestcaprejected"
                | "openinterestincreaserejected"
                | "perpmaxpositionrejected"
        )
    }
}
