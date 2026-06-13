// ---------------------------------------------------------------------------
// Account Data Completeness
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountDataSection {
    Positions,
    OpenOrders,
    Fills,
    Funding,
    Fees,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountDataCompleteness {
    pub positions_complete: bool,
    pub open_orders_complete: bool,
    pub fills_complete: bool,
    pub funding_complete: bool,
    pub fees_complete: bool,
    pub positions_fetched_at_ms: Option<u64>,
    pub open_orders_fetched_at_ms: Option<u64>,
    warnings: Vec<(AccountDataSection, String)>,
}

impl Default for AccountDataCompleteness {
    fn default() -> Self {
        Self {
            positions_complete: true,
            open_orders_complete: true,
            fills_complete: true,
            funding_complete: true,
            fees_complete: true,
            positions_fetched_at_ms: None,
            open_orders_fetched_at_ms: None,
            warnings: Vec::new(),
        }
    }
}

impl AccountDataCompleteness {
    pub fn is_complete(&self) -> bool {
        self.positions_complete
            && self.open_orders_complete
            && self.fills_complete
            && self.funding_complete
            && self.fees_complete
    }

    pub fn mark_incomplete(&mut self, section: AccountDataSection, warning: impl Into<String>) {
        match section {
            AccountDataSection::Positions => self.positions_complete = false,
            AccountDataSection::OpenOrders => self.open_orders_complete = false,
            AccountDataSection::Fills => self.fills_complete = false,
            AccountDataSection::Funding => self.funding_complete = false,
            AccountDataSection::Fees => self.fees_complete = false,
        }

        let warning = warning.into();
        if !warning.is_empty()
            && !self
                .warnings
                .iter()
                .any(|existing| existing == &(section, warning.clone()))
        {
            self.warnings.push((section, warning));
        }
    }

    pub fn warning_summary(&self) -> Option<String> {
        if self.warnings.is_empty() {
            None
        } else {
            Some(format!(
                "Partial account data: {}",
                self.warning_messages().join("; ")
            ))
        }
    }

    pub fn section_warning(&self, section: AccountDataSection) -> Option<String> {
        let complete = match section {
            AccountDataSection::Positions => self.positions_complete,
            AccountDataSection::OpenOrders => self.open_orders_complete,
            AccountDataSection::Fills => self.fills_complete,
            AccountDataSection::Funding => self.funding_complete,
            AccountDataSection::Fees => self.fees_complete,
        };
        if complete {
            return None;
        }

        let label = match section {
            AccountDataSection::Positions => "Positions",
            AccountDataSection::OpenOrders => "Open orders",
            AccountDataSection::Fills => "Trade history",
            AccountDataSection::Funding => "Funding history",
            AccountDataSection::Fees => "Fee rates",
        };
        let warnings = self
            .warnings
            .iter()
            .filter_map(|(warning_section, warning)| {
                (*warning_section == section).then_some(warning.as_str())
            })
            .collect::<Vec<_>>();
        let detail = if warnings.is_empty() {
            "refresh account data before relying on this section".to_string()
        } else {
            warnings.join("; ")
        };

        Some(format!("{label} may be incomplete: {detail}"))
    }

    fn warning_messages(&self) -> Vec<String> {
        let mut messages = Vec::new();
        for (_, warning) in &self.warnings {
            if !messages.contains(warning) {
                messages.push(warning.clone());
            }
        }
        messages
    }
}
