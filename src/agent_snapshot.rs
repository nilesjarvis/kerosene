use crate::app_state::TradingTerminal;
use serde_json::{Value, json};

mod account;
mod files;
mod journal;
mod markets;
mod workspace;

use account::{agent_fill_snapshot, agent_funding_snapshot};
pub(crate) use files::{
    activate_agent_snapshot, clear_sensitive_runtime_files, workspace_dir, write_agent_snapshot,
};

const SNAPSHOT_SCHEMA_VERSION: u32 = 5;
const ASSISTANT_CURRENT_DATA_MAX_AGE_MS: u64 = 15_000;
const MAX_MARKETS: usize = 250;
const MAX_WORKSPACE_CHARTS: usize = 32;
const MAX_WORKSPACE_DRAWINGS: usize = 128;
const MAX_WORKSPACE_DRAWING_LABEL_CHARS: usize = 160;
const MAX_WORKSPACE_PEN_POINTS: usize = 64;
const MAX_ACCOUNT_ROWS: usize = 100;
const MAX_RECENT_ROWS: usize = 50;
const MAX_TOOL_ACTIVITY_ROWS: usize = 2_000;
const MAX_TOOL_JOURNAL_TRADES: usize = 5_000;
const MAX_JOURNAL_REFLECTION_CHARS: usize = 2_000;
const MAX_JOURNAL_TAGS: usize = 32;

// ---------------------------------------------------------------------------
// Read-only Agent Snapshot
// ---------------------------------------------------------------------------

impl TradingTerminal {
    #[cfg(test)]
    pub(crate) fn build_agent_snapshot(&self) -> Result<Vec<u8>, String> {
        self.build_agent_snapshot_for_request(false)
    }

    pub(crate) fn build_agent_snapshot_for_request(
        &self,
        pnl_card_match_allowed: bool,
    ) -> Result<Vec<u8>, String> {
        let generated_at_ms = Self::now_ms();
        let market_rows = self.agent_market_rows();
        let snapshot = json!({
            "schema_version": SNAPSHOT_SCHEMA_VERSION,
            "generated_at_ms": generated_at_ms,
            "data_policy": {
                "access": "read_only",
                "sanitized": true,
                "omitted": [
                    "api_keys",
                    "private_keys",
                    "wallet_addresses",
                    "order_ids",
                    "transaction_hashes",
                    "internal_journal_trade_ids",
                    "legacy_journal_note_ids"
                ],
                "row_limits": {
                    "markets": MAX_MARKETS,
                    "workspace_charts": MAX_WORKSPACE_CHARTS,
                    "workspace_drawings": MAX_WORKSPACE_DRAWINGS,
                    "account_rows": MAX_ACCOUNT_ROWS,
                    "recent_rows": MAX_RECENT_ROWS,
                    "tool_activity_rows": MAX_TOOL_ACTIVITY_ROWS,
                    "tool_journal_trades": MAX_TOOL_JOURNAL_TRADES,
                    "journal_reflection_chars": MAX_JOURNAL_REFLECTION_CHARS
                },
                "list_contract": "returned_count is the number serialized in the section; total_count is the number available in Kerosene state; endpoint_fetch_complete does not mean an Assistant-capped list is untruncated",
                "market_symbol_contract": "symbol is the raw exchange/API key; canonical_symbol and display_symbol provide user-facing identity where metadata is available",
                "time_contract": "generated_at_ms is when Kerosene serialized the snapshot. provenance.observed_at_ms/as_of_ms is when the underlying data was observed and remains null when unknown; snapshot generation time is never substituted for missing observation time"
            },
            "overview": self.agent_overview_snapshot(generated_at_ms),
            "workspace": self.agent_workspace_snapshot(generated_at_ms),
            "account": self.agent_account_snapshot(generated_at_ms),
            "portfolio": self.agent_portfolio_snapshot(generated_at_ms),
            "markets": self.agent_markets_snapshot(generated_at_ms, &market_rows),
            "journal": self.agent_journal_snapshot(generated_at_ms),
            "positioning": self.agent_positioning_snapshot(generated_at_ms),
            "sessions": self.agent_sessions_snapshot(generated_at_ms),
            "_tool_data": self.agent_tool_data_snapshot(
                generated_at_ms,
                pnl_card_match_allowed,
                &market_rows,
            ),
        });

        serde_json::to_vec(&snapshot)
            .map_err(|error| format!("Could not serialize the assistant snapshot: {error}"))
    }

    fn agent_overview_snapshot(&self, generated_at_ms: u64) -> Value {
        json!({
            "provenance": section_provenance(
                "kerosene_state",
                Some(generated_at_ms),
                generated_at_ms,
                Some(0),
            ),
            "active_symbol": self.active_symbol,
            "active_symbol_display": self.active_symbol_display,
            "account_connected": self.connected_address.is_some(),
            "account_loading": self.account_loading,
            "account_error_present": self.account_error.is_some(),
            "account_data_revision": self.account_data_revision,
            "hide_pnl_enabled_in_ui": self.hide_pnl,
            "market_count": self.all_mids.len(),
            "favourite_symbols": self.favourite_symbols,
        })
    }

    fn agent_tool_data_snapshot(
        &self,
        generated_at_ms: u64,
        pnl_card_match_allowed: bool,
        market_rows: &[Value],
    ) -> Value {
        let account_activity = self.account_data.as_ref().map(|data| {
            let fills = data
                .fills
                .iter()
                .take(MAX_TOOL_ACTIVITY_ROWS)
                .map(agent_fill_snapshot)
                .collect::<Vec<_>>();
            let funding = data
                .funding_history
                .iter()
                .take(MAX_TOOL_ACTIVITY_ROWS)
                .map(agent_funding_snapshot)
                .collect::<Vec<_>>();
            json!({
                "as_of_ms": data.fetched_at_ms,
                "fills": fills,
                "funding": funding,
                "coverage": {
                    "fills": list_coverage(
                        data.fills.len().min(MAX_TOOL_ACTIVITY_ROWS),
                        data.fills.len(),
                        data.completeness.fills_complete,
                        false,
                    ),
                    "funding": list_coverage(
                        data.funding_history.len().min(MAX_TOOL_ACTIVITY_ROWS),
                        data.funding_history.len(),
                        data.completeness.funding_complete,
                        false,
                    ),
                }
            })
        });

        json!({
            "contract": {
                "private": true,
                "description": "Internal sanitized backing data for typed Kerosene tools; kerosene_data never returns this object.",
            },
            "assistant_request": {
                "pnl_card_match_allowed": pnl_card_match_allowed,
                "authorization_scope": if pnl_card_match_allowed {
                    "one attached P&L card turn"
                } else {
                    "none"
                },
            },
            "markets": {
                "as_of_ms": self.all_mids_updated_at_ms.values().copied().max(),
                "rows": market_rows,
                "coverage": list_coverage(self.all_mids.len(), self.all_mids.len(), true, true),
            },
            "activity": account_activity,
            "journal": self.agent_journal_tool_snapshot(),
            "risk": self.agent_risk_snapshot(generated_at_ms),
            "positioning_cache": self.agent_positioning_snapshot(generated_at_ms),
            "sessions_cache": self.agent_sessions_snapshot(generated_at_ms),
            "glossary": {
                "funding_usdc": "Account cash flow: negative means paid; positive means received.",
                "margin_account_value": "The clearinghouse margin-summary value and its scope come from the selected Hyperliquid account abstraction; do not replace it with spot or portfolio equity.",
                "portfolio_history": "Windowed account-value and PnL series may use different baselines; a shorter-window PnL can exceed all-time PnL after earlier losses.",
                "raw_market_symbols": "@N and #N are real exchange/API identifiers, not Assistant privacy redaction. Use canonical_symbol/display_symbol metadata rather than guessing mappings.",
                "completeness": "endpoint_fetch_complete describes the upstream fetch. An Assistant list can still be truncated when returned_count is below total_count.",
                "journal_best_trade": "By default, best/worst journal trades are closed, basis-complete records ranked by fee-adjusted realized PnL. Gross PnL, return on entry, and PnL per volume are separate selectable metrics.",
            }
        })
    }

    fn sanitized_journal_text(&self, text: &str) -> String {
        let mut sanitized = crate::helpers::redact_sensitive_response_text(text);
        for key in [
            self.hydromancer_api_key.trim(),
            self.openrouter_api_key.trim(),
            self.hyperdash_api_key.trim(),
        ] {
            if !key.is_empty() {
                sanitized = sanitized.replace(key, "<redacted>");
            }
        }
        crate::helpers::text_excerpt(&sanitized, MAX_JOURNAL_REFLECTION_CHARS)
    }
}

fn section_provenance(
    source: &str,
    observed_at_ms: Option<u64>,
    snapshot_generated_at_ms: u64,
    freshness_max_age_ms: Option<u64>,
) -> Value {
    let age_ms = observed_at_ms.and_then(|observed| snapshot_generated_at_ms.checked_sub(observed));
    let freshness_state = match (observed_at_ms, age_ms, freshness_max_age_ms) {
        (None, _, _) => "unknown",
        (Some(_), None, _) => "invalid_future_timestamp",
        (Some(_), Some(age), Some(max_age)) if age <= max_age => "fresh",
        (Some(_), Some(_), Some(_)) => "stale",
        (Some(_), Some(_), None) => "not_evaluated",
    };
    json!({
        "source": source,
        "as_of_ms": observed_at_ms,
        "observed_at_ms": observed_at_ms,
        "snapshot_generated_at_ms": snapshot_generated_at_ms,
        "age_ms": age_ms,
        "freshness": {
            "state": freshness_state,
            "max_age_ms": freshness_max_age_ms,
        },
    })
}

fn list_coverage(
    returned_count: usize,
    total_count: usize,
    endpoint_fetch_complete: bool,
    complete_for_current_state: bool,
) -> Value {
    json!({
        "returned_count": returned_count,
        "total_count": total_count,
        "truncated": returned_count < total_count,
        "endpoint_fetch_complete": endpoint_fetch_complete,
        "complete_for_current_state": complete_for_current_state,
    })
}

#[cfg(test)]
mod tests;
