// ---------------------------------------------------------------------------
// Status Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TwapStatus {
    Running,
    WaitingForMarket,
    Paused,
    Stopping,
    Stopped,
    Completed,
    CompletedPartial,
    Error,
}

impl TwapStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::WaitingForMarket => "Waiting",
            Self::Paused => "Paused",
            Self::Stopping => "Stopping",
            Self::Stopped => "Stopped",
            Self::Completed => "Done",
            Self::CompletedPartial => "Partial",
            Self::Error => "Error",
        }
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Stopped | Self::Completed | Self::CompletedPartial | Self::Error
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TwapPauseReason {
    StaleMarketData,
    SpotMetadataUnverified,
    RateLimited,
    NetworkError,
    StatusUnknown,
    UnexpectedResting,
}

impl TwapPauseReason {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::StaleMarketData => "Stale market data",
            Self::SpotMetadataUnverified => "Spot metadata unverified",
            Self::RateLimited => "Rate limited",
            Self::NetworkError => "Network error",
            Self::StatusUnknown => "Checking status",
            Self::UnexpectedResting => "Canceling child",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TwapChildStatus {
    Pending,
    Retrying,
    Filled,
    NoFill,
    Rejected,
    UnexpectedResting,
    UnexpectedRestingCancelled,
    AwaitingReconciliation,
    AwaitingNoFillConfirmation,
    StatusUnknown,
}

impl TwapChildStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Retrying => "Retrying",
            Self::Filled => "Filled",
            Self::NoFill => "No fill",
            Self::Rejected => "Rejected",
            Self::UnexpectedResting => "Resting",
            Self::UnexpectedRestingCancelled => "Canceled",
            Self::AwaitingReconciliation => "Reconciling",
            Self::AwaitingNoFillConfirmation => "Verifying",
            Self::StatusUnknown => "Unknown",
        }
    }
}
