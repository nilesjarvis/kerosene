use super::{
    ASSISTANT_CURRENT_DATA_MAX_AGE_MS, MAX_MARKETS, MAX_RECENT_ROWS, list_coverage,
    section_provenance,
};
use crate::app_state::TradingTerminal;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Assistant Market and Analysis Snapshots
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn agent_markets_snapshot(
        &self,
        generated_at_ms: u64,
        market_rows: &[Value],
    ) -> Value {
        let priority = self.agent_market_priority();
        let mut markets = market_rows.iter().collect::<Vec<_>>();
        markets.sort_by(|left, right| {
            let left_symbol = left
                .get("symbol")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let right_symbol = right
                .get("symbol")
                .and_then(Value::as_str)
                .unwrap_or_default();
            market_priority_index(&priority, left_symbol)
                .cmp(&market_priority_index(&priority, right_symbol))
                .then_with(|| left_symbol.cmp(right_symbol))
        });
        markets.truncate(MAX_MARKETS);
        let as_of_ms = self.all_mids_updated_at_ms.values().copied().max();

        json!({
            "provenance": section_provenance(
                "hyperliquid_all_mids_and_kerosene_symbol_metadata",
                as_of_ms,
                generated_at_ms,
                Some(ASSISTANT_CURRENT_DATA_MAX_AGE_MS),
            ),
            "active_symbol": self.active_symbol,
            "active_symbol_display": self.active_symbol_display,
            "markets": markets,
            "total_market_count": self.all_mids.len(),
            "truncated": self.all_mids.len() > MAX_MARKETS,
            "coverage": list_coverage(
                self.all_mids.len().min(MAX_MARKETS),
                self.all_mids.len(),
                true,
                false,
            ),
            "selection_policy": "active symbol, favourites, and account-relevant symbols first; remaining rows sorted by raw symbol",
        })
    }

    pub(super) fn agent_positioning_snapshot(&self, generated_at_ms: u64) -> Value {
        let mut panes = self
            .positioning_infos
            .values()
            .map(|instance| {
                let data = instance.data.as_ref().map(|data| {
                    json!({
                        "coin": data.coin,
                        "total_long_notional": data.total_long_notional,
                        "total_short_notional": data.total_short_notional,
                        "total_notional": data.total_notional,
                        "long_count": data.long_count,
                        "short_count": data.short_count,
                        "total_count": data.total_count,
                        "has_more": data.has_more,
                        "timestamp": data.timestamp,
                    })
                });
                let changes = instance.change_data.as_ref().map(|changes| {
                    let net_delta = changes.deltas.iter().map(|entry| entry.delta).sum::<f64>();
                    let gross_delta = changes
                        .deltas
                        .iter()
                        .map(|entry| entry.delta.abs())
                        .sum::<f64>();
                    json!({
                        "market": changes.market,
                        "timeframe": changes.timeframe,
                        "wallet_count": changes.deltas.len(),
                        "net_delta": net_delta,
                        "gross_delta": gross_delta,
                    })
                });
                json!({
                    "id": instance.id,
                    "symbol": instance.symbol,
                    "loading": instance.loading,
                    "error_present": instance.error.is_some(),
                    "last_fetch_ms": instance.last_fetch_ms,
                    "market_context": instance.asset_ctx.as_ref().map(|context| json!({
                        "funding": context.funding,
                        "open_interest": context.open_interest,
                        "oracle_price": context.oracle_px,
                        "mark_price": context.mark_px,
                        "mid_price": context.mid_px,
                        "previous_day_price": context.prev_day_px,
                        "day_notional_volume": context.day_ntl_vlm,
                    })),
                    "aggregate": data,
                    "changes": changes,
                })
            })
            .collect::<Vec<_>>();
        panes.sort_by_key(|pane| pane.get("id").and_then(Value::as_u64).unwrap_or_default());

        let as_of_ms = self
            .positioning_infos
            .values()
            .filter_map(|instance| instance.last_fetch_ms)
            .max();
        json!({
            "provenance": section_provenance(
                "hyperdash_aggregate_positioning_cache",
                as_of_ms,
                generated_at_ms,
                None,
            ),
            "panes": panes,
            "note": "Wallet-level HyperDash addresses and labels are intentionally omitted; only aggregates are exposed.",
            "coverage": {
                "returned_count": self.positioning_infos.len(),
                "depends_on_open_panes": true,
                "on_demand_tool_available": true,
            }
        })
    }

    pub(super) fn agent_sessions_snapshot(&self, generated_at_ms: u64) -> Value {
        let mut sessions = self
            .session_data
            .values()
            .map(|instance| {
                json!({
                    "id": instance.id,
                    "symbol": instance.symbol,
                    "lookback": instance.lookback.label(),
                    "loading": instance.loading,
                    "error_present": instance.error.is_some(),
                    "last_fetch_ms": instance.last_fetch_ms,
                    "daily_sample_count": instance.bars.len(),
                    "weekday_summaries": instance.weekday_summaries.iter().map(|summary| json!({
                        "weekday": summary.weekday.label(),
                        "sample_count": summary.sample_count,
                        "average_return_pct": summary.average_return_pct,
                        "win_rate_pct": summary.win_rate_pct,
                    })).collect::<Vec<_>>(),
                    "market_session_summaries": instance.session_summaries.iter().map(|summary| json!({
                        "session": summary.session.label(),
                        "sample_count": summary.sample_count,
                        "average_return_pct": summary.average_return_pct,
                        "win_rate_pct": summary.win_rate_pct,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();
        sessions.sort_by_key(|session| {
            session
                .get("id")
                .and_then(Value::as_u64)
                .unwrap_or_default()
        });
        let as_of_ms = self
            .session_data
            .values()
            .filter_map(|instance| instance.last_fetch_ms)
            .max();
        json!({
            "provenance": section_provenance(
                "kerosene_session_analysis_cache",
                as_of_ms,
                generated_at_ms,
                None,
            ),
            "panes": sessions,
            "coverage": {
                "returned_count": self.session_data.len(),
                "depends_on_open_panes": true,
                "on_demand_tool_available": true,
            }
        })
    }

    pub(super) fn agent_market_rows(&self) -> Vec<Value> {
        let metadata = self
            .exchange_symbols
            .iter()
            .map(|symbol| (symbol.key.as_str(), symbol))
            .collect::<HashMap<_, _>>();
        self.all_mids
            .iter()
            .map(|(symbol, mid)| {
                let exchange_symbol = metadata.get(symbol.as_str()).copied();
                json!({
                    "symbol": symbol,
                    "canonical_symbol": exchange_symbol.map(|metadata| metadata.ticker.as_str()).unwrap_or(symbol.as_str()),
                    "display_symbol": self.display_name_for_symbol(symbol),
                    "market_type": exchange_symbol.map(|metadata| match metadata.market_type {
                        crate::api::MarketType::Perp => "perp",
                        crate::api::MarketType::Spot => "spot",
                        crate::api::MarketType::Outcome => "outcome",
                    }),
                    "category": exchange_symbol.map(|metadata| metadata.category.as_str()),
                    "mid": mid,
                    "updated_at_ms": self.all_mids_updated_at_ms.get(symbol),
                    "favourite": self.favourite_symbols.contains(symbol),
                    "max_leverage": exchange_symbol.map(|metadata| metadata.max_leverage),
                    "only_isolated": exchange_symbol.map(|metadata| metadata.only_isolated),
                    "raw_symbol_is_sanitized": false,
                })
            })
            .collect()
    }

    fn agent_market_priority(&self) -> Vec<String> {
        let mut priority = Vec::new();
        let mut seen = HashSet::new();
        let mut add_symbol = |candidate: &str| {
            let candidate = candidate.trim();
            if candidate.is_empty() {
                return;
            }
            if self.all_mids.contains_key(candidate) && seen.insert(candidate.to_string()) {
                priority.push(candidate.to_string());
            }
            for metadata in self
                .exchange_symbols
                .iter()
                .filter(|metadata| metadata.ticker.eq_ignore_ascii_case(candidate))
            {
                if self.all_mids.contains_key(&metadata.key) && seen.insert(metadata.key.clone()) {
                    priority.push(metadata.key.clone());
                }
            }
        };

        add_symbol(&self.active_symbol);
        for symbol in &self.favourite_symbols {
            add_symbol(symbol);
        }
        if let Some(data) = self.account_data.as_ref() {
            for asset in &data.clearinghouse.asset_positions {
                add_symbol(&asset.position.coin);
            }
            for order in &data.open_orders {
                add_symbol(&order.coin);
            }
            for balance in &data.spot.balances {
                add_symbol(&balance.coin);
            }
            for fill in data.fills.iter().take(MAX_RECENT_ROWS) {
                add_symbol(&fill.coin);
            }
            for funding in data.funding_history.iter().take(MAX_RECENT_ROWS) {
                add_symbol(&funding.delta.coin);
            }
        }
        priority
    }
}

fn market_priority_index(priority: &[String], symbol: &str) -> usize {
    priority
        .iter()
        .position(|candidate| candidate == symbol)
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests;
