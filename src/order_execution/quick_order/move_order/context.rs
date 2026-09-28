use crate::app_state::TradingTerminal;
use crate::signing::CapturedAgentKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MoveOrderContextError {
    MissingAgentKey,
    AccountChanged,
}

impl MoveOrderContextError {
    pub(crate) fn status_text(self) -> &'static str {
        match self {
            Self::MissingAgentKey => "Move failed: original agent key is no longer available",
            Self::AccountChanged => {
                "Move stopped: account changed before replacement; original order was cancelled"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MoveOrderKey {
    coin: String,
    oid: u64,
}

impl MoveOrderKey {
    pub(crate) fn new(coin: impl Into<String>, oid: u64) -> Self {
        Self {
            coin: coin.into(),
            oid,
        }
    }

    pub(crate) fn coin(&self) -> &str {
        &self.coin
    }
}

#[derive(Clone)]
pub(crate) struct PendingMoveOrderContext {
    account_address: String,
    agent_key: CapturedAgentKey,
}

impl PendingMoveOrderContext {
    /// Captures the trading identity used to cancel an order so the replacement
    /// cannot silently switch to a different account/key before placement.
    pub(crate) fn new(
        account_address: impl Into<String>,
        agent_key: CapturedAgentKey,
    ) -> Result<Self, MoveOrderContextError> {
        if agent_key.is_empty() {
            return Err(MoveOrderContextError::MissingAgentKey);
        }

        Ok(Self {
            account_address: account_address.into(),
            agent_key,
        })
    }

    pub(crate) fn replacement_agent_key(
        &self,
        current_account: Option<&str>,
    ) -> Result<CapturedAgentKey, MoveOrderContextError> {
        match current_account {
            Some(current) => {
                let current = current.trim();
                if current.is_empty() || current != self.account_address {
                    Err(MoveOrderContextError::AccountChanged)
                } else {
                    Ok(self.agent_key.clone_for_task())
                }
            }
            _ => Err(MoveOrderContextError::AccountChanged),
        }
    }

    pub(crate) fn matches_account(&self, account_address: &str) -> bool {
        self.account_address == account_address
    }
}

impl TradingTerminal {
    pub(crate) fn clear_pending_move_order_state(&mut self) {
        self.pending_move_order_contexts.clear();
        self.active_move_order_drag = None;
    }
}
