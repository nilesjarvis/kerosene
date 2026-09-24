mod bucket;
mod expiry;
mod price;

use super::OutcomeSymbolInfo;

// ---------------------------------------------------------------------------
// Outcome Labels
// ---------------------------------------------------------------------------

impl OutcomeSymbolInfo {
    pub fn venue_display_name(venue: &str) -> &str {
        match venue {
            "skew" => "Skew",
            "out" => "Outcome",
            "txyz" => "TradeXYZ",
            other => other,
        }
    }

    pub fn venue_label(&self) -> Option<&str> {
        self.venue.as_deref().map(Self::venue_display_name)
    }

    /// Describe the source/window published on-chain without assuming the
    /// averaging algorithm: Skew's trade feed uses VWAP despite the template's
    /// generic TWAP wording, while its Pyth feed uses different weighting.
    pub fn settlement_source_label(&self) -> Option<String> {
        if self.contract.blocked_reason.is_some()
            || !matches!(
                self.outcome_name.as_str(),
                "template:binaryPrice" | "template:priceTouch" | "template:scalarPrice"
            )
        {
            return None;
        }
        let value = |key| {
            self.description.split('|').find_map(|part| {
                let (name, value) = part.split_once(':')?;
                (name.trim() == key && !value.trim().is_empty()).then_some(value.trim())
            })
        };
        let source = value("priceDescription")?;
        let window = value("seconds")?.parse::<u64>().ok()?;
        (window > 0).then(|| {
            if self.outcome_name == "template:priceTouch" {
                format!("Settlement: {source} | {window}s observation window through expiry")
            } else {
                format!("Settlement: {source} | {window}s window ending at expiry")
            }
        })
    }

    pub fn market_label(&self) -> String {
        self.market_label_at(None, true)
    }

    pub fn market_label_with_countdown(&self, now_ms: u64) -> String {
        self.market_label_at(Some(now_ms), true)
    }

    fn market_label_at(&self, now_ms: Option<u64>, include_expiry: bool) -> String {
        if self.contract.blocked_reason.is_some()
            && let Some(title) = &self.contract.title
        {
            return title.clone();
        }
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
        if let Some(title) = &self.contract.title {
            return title.clone();
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
        let label = format!(
            "{}: {}",
            self.side_name.to_ascii_uppercase(),
            self.side_condition_label()
        );
        match self.venue_label() {
            Some(venue) => format!("{venue} | {label}"),
            None => label,
        }
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
        if self.contract.blocked_reason.is_some() && self.contract.title.is_some() {
            return self.market_label_at(now_ms, include_expiry);
        }
        if !self.side_name.eq_ignore_ascii_case("yes") && !self.side_name.eq_ignore_ascii_case("no")
        {
            return self.market_label_at(now_ms, include_expiry);
        }
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

        let label = self
            .contract
            .title
            .as_deref()
            .unwrap_or(&self.outcome_name)
            .trim();
        if label.is_empty() || self.question_name.as_deref() == Some(label) {
            None
        } else {
            Some(label.to_string())
        }
    }
}
