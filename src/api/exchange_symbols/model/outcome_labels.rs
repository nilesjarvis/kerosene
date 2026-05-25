mod bucket;
mod expiry;
mod price;

use super::OutcomeSymbolInfo;

// ---------------------------------------------------------------------------
// Outcome Labels
// ---------------------------------------------------------------------------

impl OutcomeSymbolInfo {
    pub fn market_label(&self) -> String {
        self.market_label_at(None, true)
    }

    pub fn market_label_with_countdown(&self, now_ms: u64) -> String {
        self.market_label_at(Some(now_ms), true)
    }

    fn market_label_at(&self, now_ms: Option<u64>, include_expiry: bool) -> String {
        if self.question_class.as_deref() == Some("priceBucket") {
            return self.bucket_event_label(now_ms, include_expiry);
        }
        if self.class.as_deref() == Some("priceBinary")
            && let (Some(underlying), Some(target)) = (&self.underlying, &self.target_price)
        {
            let label = Self::price_threshold_label(underlying, target, true);
            if !include_expiry {
                return label;
            }
            let Some(expiry) = &self.expiry else {
                return label;
            };
            return format!("{label} at {}", Self::format_expiry_at(expiry, now_ms));
        }

        if let Some(question_name) = &self.question_name {
            return question_name.clone();
        }
        if self.outcome_name != "Recurring" {
            return self.outcome_name.clone();
        }
        match (&self.underlying, &self.target_price, &self.expiry) {
            (Some(underlying), Some(target), expiry) => {
                let label = Self::price_threshold_label(underlying, target, true);
                if include_expiry && let Some(expiry) = expiry {
                    format!("{label} at {}", Self::format_expiry_at(expiry, now_ms))
                } else {
                    label
                }
            }
            _ => self.outcome_name.clone(),
        }
    }

    pub fn display_label(&self) -> String {
        format!(
            "{}: {}",
            self.side_name.to_ascii_uppercase(),
            self.side_condition_label()
        )
    }

    pub fn side_condition_label(&self) -> String {
        self.side_condition_label_at(None, true)
    }

    pub fn side_condition_label_with_countdown(&self, now_ms: u64) -> String {
        self.side_condition_label_at(Some(now_ms), true)
    }

    pub fn side_condition_short_label(&self) -> String {
        self.side_condition_label_at(None, false)
    }

    fn side_condition_label_at(&self, now_ms: Option<u64>, include_expiry: bool) -> String {
        if self.is_no_side() {
            return self.complement_label(now_ms, include_expiry);
        }
        if let Some(label) = self.named_outcome_label() {
            return label;
        }
        self.market_label_at(now_ms, include_expiry)
    }

    pub(super) fn named_outcome_label(&self) -> Option<String> {
        if self.question_id.is_none()
            || self.is_question_fallback
            || self.question_class.as_deref() == Some("priceBucket")
        {
            return None;
        }

        let label = self.outcome_name.trim();
        if label.is_empty() || self.question_name.as_deref() == Some(label) {
            None
        } else {
            Some(label.to_string())
        }
    }
}
