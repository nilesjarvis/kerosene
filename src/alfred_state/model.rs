use crate::message::Message;

use std::fmt;

// ---------------------------------------------------------------------------
// Alfred Model
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub(crate) struct AlfredState {
    pub(crate) open: bool,
    pub(crate) query: String,
    pub(crate) selected_index: usize,
}

impl fmt::Debug for AlfredState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AlfredState")
            .field("open", &self.open)
            .field("has_query", &!self.query.is_empty())
            .field("selected_index", &self.selected_index)
            .finish()
    }
}

impl AlfredState {
    pub(crate) fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected_index = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlfredSelectionStep {
    Previous,
    Next,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlfredCommandId {
    AddCandlestickChart,
    AddComparisonChart,
    AddPairRatioChart,
    AddSessionDataPane,
    AddPositionsHistoryPane,
    AddPortfolioPane,
    AddIncomePane,
    AddOutcomesPane,
    AddHypeEtfsPane,
    AddHypeUnstakingQueuePane,
    AddLiquidationsPane,
    AddLiquidationsDistributionPane,
    AddTrackedTradesPane,
    AddTelegramFeedPane,
    AddXFeedPane,
    AddCalendarPane,
    AddOrderBookPane,
    AddLiveWatchlistPane,
    ToggleTickerTape,
    AddPositioningInfoPane,
    AddAdvancedOrdersPane,
    OpenTradingJournal,
    OpenWalletTrackerWindow,
    OpenWalletClustersWindow,
    OpenScreenerWindow,
    OpenSettingsWindow,
    NaturalLanguageTrading,
    NukePositions,
    ClosePosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlfredCommandKind {
    AddWidget,
    OpenWindow,
    Trading,
}

#[derive(Clone)]
pub(crate) struct AlfredCommand {
    pub(crate) id: AlfredCommandId,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) tag: String,
    pub(crate) icon_symbol: Option<String>,
    pub(crate) icon_title_anchor: Option<String>,
    pub(crate) kind: AlfredCommandKind,
    pub(crate) enabled: bool,
    pub(crate) disabled_reason: Option<String>,
    pub(crate) message: Option<Message>,
    aliases: &'static [&'static str],
}

impl fmt::Debug for AlfredCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AlfredCommand")
            .field("id", &self.id)
            .field("title", &"<redacted>")
            .field("detail", &"<redacted>")
            .field("tag", &"<redacted>")
            .field("has_icon_symbol", &self.icon_symbol.is_some())
            .field("has_icon_title_anchor", &self.icon_title_anchor.is_some())
            .field("kind", &self.kind)
            .field("enabled", &self.enabled)
            .field("has_disabled_reason", &self.disabled_reason.is_some())
            .field("has_message", &self.message.is_some())
            .field("aliases_count", &self.aliases.len())
            .finish()
    }
}

impl AlfredCommand {
    pub(super) fn new(
        id: AlfredCommandId,
        title: &'static str,
        detail: &'static str,
        tag: &'static str,
        kind: AlfredCommandKind,
        message: Option<Message>,
        aliases: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            title: title.to_string(),
            detail: detail.to_string(),
            tag: tag.to_string(),
            icon_symbol: None,
            icon_title_anchor: None,
            kind,
            enabled: true,
            disabled_reason: None,
            message,
            aliases,
        }
    }

    pub(super) fn disabled(mut self, reason: &'static str) -> Self {
        self.enabled = false;
        self.disabled_reason = Some(reason.to_string());
        self.message = None;
        self
    }

    pub(super) fn with_dynamic_text(mut self, title: String, detail: String, tag: String) -> Self {
        self.title = title;
        self.detail = detail;
        self.tag = tag;
        self
    }

    pub(super) fn with_title_icon(
        mut self,
        icon_symbol: Option<String>,
        icon_title_anchor: Option<String>,
    ) -> Self {
        self.icon_symbol = icon_symbol;
        self.icon_title_anchor = icon_title_anchor;
        self
    }

    pub(super) fn disabled_with_message(mut self, reason: String) -> Self {
        self.enabled = false;
        self.disabled_reason = Some(reason);
        self.message = None;
        self
    }

    pub(super) fn matches_query(&self, query: &str) -> bool {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return self.kind != AlfredCommandKind::Trading;
        }

        let searchable = self.searchable_text();
        query
            .split_whitespace()
            .all(|token| searchable.contains(token))
    }

    fn searchable_text(&self) -> String {
        let mut text = format!(
            "{} {} {} {:?}",
            self.title.to_ascii_lowercase(),
            self.detail.to_ascii_lowercase(),
            self.tag.to_ascii_lowercase(),
            self.kind
        );
        for alias in self.aliases {
            text.push(' ');
            text.push_str(alias);
        }
        text
    }
}
