use super::{MAX_JOURNAL_TAGS, MAX_TOOL_JOURNAL_TRADES, list_coverage, section_provenance};
use crate::app_state::TradingTerminal;
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Assistant Journal Snapshot
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn agent_journal_snapshot(&self, generated_at_ms: u64) -> Value {
        let available =
            self.connected_address.is_some() && self.journal.active_account_key.is_some();
        let total_trade_count = self.journal.trades.len();
        let annotated_trade_count = self
            .journal
            .trades
            .iter()
            .filter(|trade| crate::journal::note_for_trade(&self.journal.entries, trade).is_some())
            .count();
        let as_of_ms = self.journal.last_refresh_time;
        let data_state = self.agent_journal_data_state();

        json!({
            "provenance": section_provenance(
                "kerosene_account_scoped_trading_journal",
                as_of_ms,
                generated_at_ms,
                None,
            ),
            "available": available,
            "data_state": data_state,
            "loading": self.journal.loading,
            "error_present": self.journal.error.is_some(),
            "warning_present": self.journal.warning.is_some(),
            "last_refresh_time_ms": self.journal.last_refresh_time,
            "total_trade_count": total_trade_count,
            "annotated_trade_count": annotated_trade_count,
            "include_fees_in_journal_ui": self.journal.include_fees_in_pnl,
            "sync": {
                "complete": self.journal.sync_status.complete,
                "pages_loaded": self.journal.sync_status.pages_loaded,
                "fills_loaded": self.journal.sync_status.fills_loaded,
                "pagination_warning_present": self.journal.sync_status.pagination_warning.is_some(),
            },
            "coverage": {
                "returned_count": 0,
                "total_count": total_trade_count,
                "trades_exposed_in_public_section": false,
                "on_demand_tool_available": true,
                "endpoint_fetch_complete": self.journal.sync_status.complete,
                "complete_for_current_state": !self.journal.loading && self.journal.sync_status.complete,
            },
            "ranking_contract": "Best/worst defaults to closed, basis-complete trades ranked by net_realized_pnl_usd (gross realized PnL minus journal fees). The typed journal tool can use other explicit metrics.",
        })
    }

    fn agent_journal_data_state(&self) -> &'static str {
        let available =
            self.connected_address.is_some() && self.journal.active_account_key.is_some();
        let total_trade_count = self.journal.trades.len();
        if !available {
            "account_not_connected"
        } else if self.journal.loading && total_trade_count == 0 {
            "loading"
        } else if self.journal.error.is_some() && total_trade_count == 0 {
            "unavailable"
        } else if total_trade_count == 0 && self.journal.sync_status.complete {
            "complete_empty"
        } else if total_trade_count == 0 {
            "not_loaded_or_partial"
        } else if self.journal.loading || !self.journal.sync_status.complete {
            "partial"
        } else {
            "ready"
        }
    }

    pub(super) fn agent_journal_tool_snapshot(&self) -> Value {
        let total_count = self.journal.trades.len();
        let selected_indexes = journal_trade_selection_indexes(
            &self.journal.trades,
            &self.journal.entries,
            MAX_TOOL_JOURNAL_TRADES,
        );
        let rows = selected_indexes
            .iter()
            .map(|index| self.agent_journal_trade_row(*index))
            .collect::<Vec<_>>();
        let returned_count = rows.len();

        json!({
            "available": self.connected_address.is_some() && self.journal.active_account_key.is_some(),
            "as_of_ms": self.journal.last_refresh_time,
            "data_state": self.agent_journal_data_state(),
            "trades": rows,
            "coverage": list_coverage(
                returned_count,
                total_count,
                self.journal.sync_status.complete,
                !self.journal.loading && self.journal.sync_status.complete,
            ),
            "selection_policy": "All trades when within the cap. Above the cap, annotated trades plus the largest/smallest net-PnL, largest return-on-entry, and most recent trades are prioritized; overall net-PnL extremes are preserved.",
            "ranking_defaults": {
                "status": "CLOSED",
                "basis_complete": true,
                "metric": "net_pnl",
                "net_pnl_formula": "gross_realized_pnl_usd - fees_usd",
            },
            "privacy": "Wallet addresses, fill/order/transaction identifiers, and internal journal trade IDs are omitted. Free-form reflections and tags are bounded and credential-redacted.",
        })
    }

    fn agent_journal_trade_row(&self, index: usize) -> Value {
        let trade = &self.journal.trades[index];
        let market_type = journal_market_type(&trade.coin);
        let side = match market_type {
            "perp" if trade.is_long => "long",
            "perp" => "short",
            "spot" => "spot",
            _ => "outcome",
        };
        let net_pnl = trade.effective_pnl(true);
        let return_on_entry_pct = positive_ratio_pct(net_pnl, trade.total_entry_notional);
        let net_pnl_per_volume_pct = positive_ratio_pct(net_pnl, trade.volume);
        let reflection = crate::journal::note_for_trade(&self.journal.entries, trade).map(|note| {
            json!({
                "open_thesis": self.sanitized_journal_text(&note.open),
                "close_reflection": self.sanitized_journal_text(&note.close),
                "cause_of_error": self.sanitized_journal_text(&note.cause_of_error),
                "tags": note.tags.iter().take(MAX_JOURNAL_TAGS).map(|tag| self.sanitized_journal_text(tag)).collect::<Vec<_>>(),
                "tag_count": note.tags.len(),
                "tags_truncated": note.tags.len() > MAX_JOURNAL_TAGS,
            })
        });

        json!({
            "journal_ref": format!("trade-{}", index.saturating_add(1)),
            "symbol": trade.coin,
            "display_symbol": self.display_coin_for_journal(&trade.coin),
            "market_type": market_type,
            "side": side,
            "status": trade.status,
            "start_time_ms": trade.start_time,
            "end_time_ms": trade.end_time,
            "duration_ms": trade.end_time.map(|end| end.saturating_sub(trade.start_time)),
            "gross_realized_pnl_usd": trade.pnl,
            "fees_usd": trade.fee,
            "net_realized_pnl_usd": net_pnl,
            "return_on_entry_pct": return_on_entry_pct,
            "net_pnl_per_volume_pct": net_pnl_per_volume_pct,
            "volume_usd": trade.volume,
            "max_position": trade.max_position,
            "average_entry_price": trade.avg_entry_price,
            "entry_notional_usd": trade.total_entry_notional,
            "entry_size": trade.total_entry_size,
            "fill_count": trade.fill_count,
            "basis_complete": trade.basis_complete,
            "annotated": reflection.is_some(),
            "reflection": reflection,
        })
    }
}

fn journal_trade_selection_indexes(
    trades: &[crate::journal::AggregatedTrade],
    entries: &HashMap<String, crate::journal::JournalNote>,
    limit: usize,
) -> Vec<usize> {
    if trades.len() <= limit {
        return (0..trades.len()).collect();
    }

    let quota = (limit / 5).max(1);
    let mut selected = Vec::with_capacity(limit);
    let mut seen = HashSet::with_capacity(limit);

    let annotated = trades
        .iter()
        .enumerate()
        .filter(|(_index, trade)| crate::journal::note_for_trade(entries, trade).is_some())
        .map(|(index, _trade)| index);
    extend_unique_indexes(&mut selected, &mut seen, annotated, quota, limit);

    let mut by_net_pnl = (0..trades.len()).collect::<Vec<_>>();
    by_net_pnl.sort_by(|left, right| {
        trades[*right]
            .effective_pnl(true)
            .partial_cmp(&trades[*left].effective_pnl(true))
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.cmp(right))
    });
    extend_unique_indexes(
        &mut selected,
        &mut seen,
        by_net_pnl.iter().copied(),
        quota,
        limit,
    );
    extend_unique_indexes(
        &mut selected,
        &mut seen,
        by_net_pnl.iter().rev().copied(),
        quota,
        limit,
    );

    let mut by_return = (0..trades.len()).collect::<Vec<_>>();
    by_return.sort_by(|left, right| {
        compare_optional_f64_desc(
            positive_ratio_pct(
                trades[*left].effective_pnl(true),
                trades[*left].total_entry_notional,
            ),
            positive_ratio_pct(
                trades[*right].effective_pnl(true),
                trades[*right].total_entry_notional,
            ),
        )
        .then_with(|| left.cmp(right))
    });
    extend_unique_indexes(&mut selected, &mut seen, by_return, quota, limit);

    let mut recent = (0..trades.len()).collect::<Vec<_>>();
    recent.sort_by(|left, right| {
        trades[*right]
            .start_time
            .cmp(&trades[*left].start_time)
            .then_with(|| left.cmp(right))
    });
    extend_unique_indexes(&mut selected, &mut seen, recent, limit, limit);
    extend_unique_indexes(&mut selected, &mut seen, 0..trades.len(), limit, limit);

    selected.sort_by(|left, right| {
        trades[*right]
            .start_time
            .cmp(&trades[*left].start_time)
            .then_with(|| left.cmp(right))
    });
    selected
}

fn extend_unique_indexes(
    selected: &mut Vec<usize>,
    seen: &mut HashSet<usize>,
    indexes: impl IntoIterator<Item = usize>,
    category_limit: usize,
    total_limit: usize,
) {
    let mut category_count = 0;
    for index in indexes {
        if selected.len() >= total_limit || category_count >= category_limit {
            break;
        }
        if seen.insert(index) {
            selected.push(index);
            category_count += 1;
        }
    }
}

fn positive_ratio_pct(numerator: f64, denominator: f64) -> Option<f64> {
    (numerator.is_finite() && denominator.is_finite() && denominator > 0.0)
        .then_some(numerator / denominator * 100.0)
}

fn compare_optional_f64_desc(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.partial_cmp(&left).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn journal_market_type(symbol: &str) -> &'static str {
    if symbol.starts_with('#') {
        "outcome"
    } else if symbol.starts_with('@') || symbol.contains('/') {
        "spot"
    } else {
        "perp"
    }
}

#[cfg(test)]
mod tests;
