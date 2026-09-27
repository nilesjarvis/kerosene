use super::OutcomeSymbolInfo;
use serde::{Deserialize, Serialize};

/// Resolved public contract terms. Cached terms may be displayed but never
/// authorize trading until a live metadata + template refresh verifies them.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct OutcomeContract {
    #[serde(skip)]
    pub verified: bool,
    pub title: Option<String>,
    pub rules: Option<String>,
    pub blocked_reason: Option<String>,
    pub deadline_ms: Option<u64>,
    pub resolution_deadline: bool,
    pub scalar: bool,
    pub fee_scale: Option<String>,
    pub deployer_fee_scale: Option<String>,
}

impl OutcomeContract {
    #[cfg(test)]
    pub(crate) fn verified_fixture() -> Self {
        Self {
            verified: true,
            ..Self::default()
        }
    }
}

impl OutcomeSymbolInfo {
    /// The clock is supplied by update/order preparation or the view snapshot.
    /// Cancellation deliberately does not use this gate.
    pub fn trading_block_reason(&self, now_ms: u64) -> Option<&str> {
        if self.is_question_fallback {
            return Some("Fallback settlement contract");
        }
        if self
            .question_settled_named_outcomes
            .contains(&self.outcome_id)
        {
            return Some("Settled");
        }
        if !self.contract.verified {
            return Some("Live outcome metadata is unverified");
        }
        if let Some(reason) = self.contract.blocked_reason.as_deref() {
            return Some(reason);
        }
        if self.quote_token_index.is_none() {
            return Some("Unsupported outcome quote token");
        }
        if self
            .contract
            .deadline_ms
            .is_some_and(|deadline| now_ms >= deadline)
        {
            return Some(if self.contract.resolution_deadline {
                "Resolution deadline passed; awaiting settlement"
            } else {
                "Expired; awaiting settlement"
            });
        }
        None
    }

    pub fn contract_deadline_label(&self) -> Option<String> {
        let timestamp = i64::try_from(self.contract.deadline_ms?).ok()?;
        let date = chrono::DateTime::from_timestamp_millis(timestamp)?;
        let label = if self.contract.resolution_deadline {
            "Resolution deadline"
        } else {
            "Expiry"
        };
        Some(format!("{label}: {} UTC", date.format("%Y-%m-%d %H:%M")))
    }

    pub fn fee_terms_label(&self) -> String {
        if !self.contract.verified {
            return "Live outcome fee terms unavailable".to_string();
        }
        let scale = self.contract.fee_scale.as_deref().unwrap_or("unavailable");
        let deployer = self
            .contract
            .deployer_fee_scale
            .as_deref()
            .unwrap_or("unavailable");
        format!(
            "Fee scales — protocol: {scale}; deployer: {deployer}. Fees apply on close/settlement; no maker rebates."
        )
    }
}
