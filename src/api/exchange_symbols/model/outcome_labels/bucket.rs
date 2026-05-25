use super::OutcomeSymbolInfo;

impl OutcomeSymbolInfo {
    pub(super) fn bucket_event_label(&self, now_ms: Option<u64>, include_expiry: bool) -> String {
        self.bucket_label_for_side(true, now_ms, include_expiry)
    }

    pub(super) fn complement_label(&self, now_ms: Option<u64>, include_expiry: bool) -> String {
        if self.question_class.as_deref() == Some("priceBucket") {
            return self.bucket_label_for_side(false, now_ms, include_expiry);
        }
        if self.class.as_deref() == Some("priceBinary")
            && let (Some(underlying), Some(target)) = (&self.underlying, &self.target_price)
        {
            let label = Self::price_threshold_label(underlying, target, false);
            if !include_expiry {
                return label;
            }
            let Some(expiry) = &self.expiry else {
                return label;
            };
            return format!("{label} at {}", Self::format_expiry_at(expiry, now_ms));
        }

        format!("not {}", self.market_label_at(now_ms, include_expiry))
    }

    fn bucket_label_for_side(
        &self,
        affirmative: bool,
        now_ms: Option<u64>,
        include_expiry: bool,
    ) -> String {
        if self.is_question_fallback {
            return self.fallback_label_for_side(affirmative, now_ms, include_expiry);
        }

        let Some(index) = self.bucket_index.map(|index| index as usize) else {
            return self
                .question_name
                .clone()
                .unwrap_or_else(|| self.outcome_name.clone());
        };
        let Some(underlying) = self.question_underlying.as_deref() else {
            return format!("Bucket {}", index + 1);
        };
        let thresholds = &self.question_price_thresholds;
        if thresholds.is_empty() {
            return format!("Bucket {}", index + 1);
        }

        let expiry = self
            .question_expiry
            .as_ref()
            .filter(|_| include_expiry)
            .map(|expiry| Self::format_expiry_at(expiry, now_ms));
        let with_expiry = |label: String| match &expiry {
            Some(expiry) => format!("{label} at {expiry}"),
            None => label,
        };

        if index == 0 {
            let threshold = Self::format_target_price(&thresholds[0]);
            let label = if affirmative {
                format!("{underlying} is below {threshold}")
            } else {
                format!("{underlying} is at or above {threshold}")
            };
            return with_expiry(label);
        }

        if index < thresholds.len() {
            let lower = Self::format_target_price(&thresholds[index - 1]);
            let upper = Self::format_target_price(&thresholds[index]);
            if affirmative {
                return with_expiry(format!(
                    "{underlying} is at or above {lower} and below {upper}"
                ));
            }
            return with_expiry(format!(
                "{underlying} is below {lower} or at or above {upper}"
            ));
        }

        let threshold =
            Self::format_target_price(thresholds.last().map(String::as_str).unwrap_or(""));
        let label = if affirmative {
            format!("{underlying} is at or above {threshold}")
        } else {
            format!("{underlying} is below {threshold}")
        };
        with_expiry(label)
    }

    fn fallback_label_for_side(
        &self,
        affirmative: bool,
        now_ms: Option<u64>,
        include_expiry: bool,
    ) -> String {
        let expiry = self
            .question_expiry
            .as_ref()
            .filter(|_| include_expiry)
            .map(|expiry| Self::format_expiry_at(expiry, now_ms));
        let label = if affirmative {
            "fallback / other settlement".to_string()
        } else if let Some(underlying) = self.question_underlying.as_deref() {
            format!("a named {underlying} bucket settles")
        } else {
            "a named outcome settles".to_string()
        };

        match expiry {
            Some(expiry) => format!("{label} at {expiry}"),
            None => label,
        }
    }

    pub(super) fn is_no_side(&self) -> bool {
        self.side_name.trim().eq_ignore_ascii_case("no") || self.side_index == 1
    }
}
