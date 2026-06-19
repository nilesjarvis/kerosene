use serde::{Deserialize, Deserializer, Serialize};

// ---------------------------------------------------------------------------
// Journal Appearance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum JournalTradesView {
    #[default]
    Cards,
    Table,
}

impl JournalTradesView {
    pub const ALL: [Self; 2] = [Self::Cards, Self::Table];

    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "Cards" => Some(Self::Cards),
            "Table" => Some(Self::Table),
            _ => None,
        }
    }

    fn config_value(self) -> &'static str {
        match self {
            Self::Cards => "Cards",
            Self::Table => "Table",
        }
    }

    pub fn is_table(self) -> bool {
        matches!(self, Self::Table)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cards => "Cards",
            Self::Table => "Table",
        }
    }
}

impl<'de> Deserialize<'de> for JournalTradesView {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_config_value(&value).unwrap_or_else(|| {
            let default = Self::default();
            crate::config::push_config_warning(format!(
                "Unknown journal trades view {value:?} in config; using {}",
                default.config_value()
            ));
            default
        }))
    }
}

impl std::fmt::Display for JournalTradesView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::JournalTradesView;

    #[test]
    fn journal_trades_view_defaults_to_cards() {
        assert_eq!(JournalTradesView::default(), JournalTradesView::Cards);
        assert!(!JournalTradesView::Cards.is_table());
        assert!(JournalTradesView::Table.is_table());
        assert_eq!(
            JournalTradesView::ALL,
            [JournalTradesView::Cards, JournalTradesView::Table]
        );
    }
}
