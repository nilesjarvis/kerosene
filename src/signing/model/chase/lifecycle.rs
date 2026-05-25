use std::time::Duration;

// ---------------------------------------------------------------------------
// Chase Lifecycle Model
// ---------------------------------------------------------------------------

/// Maximum number of consecutive cancel failures before the chase is
/// automatically stopped to prevent an unbounded retry storm.
pub const MAX_CHASE_CANCEL_RETRIES: u32 = 5;
/// Maximum number of successful/attempted reprices before a chase is stopped.
pub const MAX_CHASE_REPRICES: u32 = 1_000;
/// Maximum wall-clock duration for a single chase lifecycle.
pub const MAX_CHASE_DURATION: Duration = Duration::from_secs(15 * 60);
/// Maximum absolute drift from the initial chase price before auto-stop.
pub const MAX_CHASE_DRIFT_FRACTION: f64 = 0.05;
/// Minimum delay between chase reprice requests.
pub const MIN_CHASE_REPRICE_INTERVAL: Duration = Duration::from_secs(1);
/// Cooldown after a retryable chase exchange error, such as a rate limit.
pub const CHASE_RETRY_COOLDOWN: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseVerificationReason {
    Placement,
    Reprice,
    SizeCorrection,
    MissingOrder,
    MissingOrderResolvedNoFill,
    Modify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseQueuedAction {
    Place,
    Reprice,
    SizeCorrection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseStopPhase {
    AwaitingPlace,
    AwaitingModify { oid: u64 },
    Canceling { oid: u64 },
    VerifyingCancel { oid: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseLifecycle {
    LoadingBook,
    Placing,
    Resting,
    Verifying { reason: ChaseVerificationReason },
    Queued { action: ChaseQueuedAction },
    Modifying { oid: u64 },
    Stopping { phase: ChaseStopPhase },
}

impl ChaseLifecycle {
    pub fn label(self) -> &'static str {
        match self {
            Self::LoadingBook => "Starting",
            Self::Placing => "Placing",
            Self::Resting => "Resting",
            Self::Verifying { .. } => "Checking",
            Self::Queued { .. } => "Queued",
            Self::Modifying { .. } => "Repricing",
            Self::Stopping {
                phase: ChaseStopPhase::Canceling { .. },
            } => "Canceling",
            Self::Stopping { .. } => "Stopping",
        }
    }

    pub fn has_exchange_request(self) -> bool {
        matches!(
            self,
            Self::Placing
                | Self::Modifying { .. }
                | Self::Stopping {
                    phase: ChaseStopPhase::AwaitingPlace
                        | ChaseStopPhase::AwaitingModify { .. }
                        | ChaseStopPhase::Canceling { .. }
                }
        )
    }

    pub fn is_stopping(self) -> bool {
        matches!(self, Self::Stopping { .. })
    }

    pub fn is_book_repriceable(self) -> bool {
        matches!(self, Self::Resting | Self::Queued { .. })
    }

    pub fn expects_place_result(self) -> bool {
        matches!(
            self,
            Self::Placing
                | Self::Stopping {
                    phase: ChaseStopPhase::AwaitingPlace
                }
        )
    }

    pub fn expects_modify_result(self, oid: u64) -> bool {
        matches!(
            self,
            Self::Modifying { oid: pending_oid }
                | Self::Stopping {
                    phase: ChaseStopPhase::AwaitingModify { oid: pending_oid }
                } if pending_oid == oid
        )
    }

    pub fn expects_cancel_result(self, oid: u64) -> bool {
        matches!(
            self,
            Self::Stopping {
                phase: ChaseStopPhase::Canceling { oid: pending_oid }
            } if pending_oid == oid
        )
    }
}
