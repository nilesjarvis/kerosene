use crate::app_state::TradingTerminal;
use crate::chart_indicator::ChartIndicatorId;

use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

const SNAPSHOT_SCHEMA_VERSION: u32 = 4;
const ASSISTANT_CURRENT_DATA_MAX_AGE_MS: u64 = 15_000;
const MAX_MARKETS: usize = 250;
const MAX_WORKSPACE_CHARTS: usize = 32;
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
            "markets": self.agent_markets_snapshot(generated_at_ms),
            "journal": self.agent_journal_snapshot(generated_at_ms),
            "positioning": self.agent_positioning_snapshot(generated_at_ms),
            "sessions": self.agent_sessions_snapshot(generated_at_ms),
            "_tool_data": self.agent_tool_data_snapshot(
                generated_at_ms,
                pnl_card_match_allowed,
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

    fn agent_workspace_snapshot(&self, generated_at_ms: u64) -> Value {
        let selected_chart_id = self
            .primary_chart_id
            .filter(|id| self.charts.contains_key(id));
        let total_chart_count = self.charts.len();
        let mut chart_instances = self.charts.values().collect::<Vec<_>>();
        chart_instances
            .sort_by_key(|instance| (selected_chart_id != Some(instance.id), instance.id));
        let mut charts = chart_instances
            .into_iter()
            .take(MAX_WORKSPACE_CHARTS)
            .map(|instance| {
                let indicators = ChartIndicatorId::ASSISTANT_VISIBLE
                    .iter()
                    .map(|indicator| {
                        (
                            indicator.key().to_string(),
                            Value::Bool(indicator.is_enabled(instance)),
                        )
                    })
                    .collect::<serde_json::Map<_, _>>();
                json!({
                    "id": instance.id,
                    "surface": if self.chart_is_docked(instance.id) { "docked" } else { "detached" },
                    "symbol": instance.symbol,
                    "display_symbol": instance.symbol_display,
                    "timeframe": instance.interval.label(),
                    "timeframe_config": instance.interval.config_str(),
                    "selected": selected_chart_id == Some(instance.id),
                    "indicators": indicators,
                })
            })
            .collect::<Vec<_>>();
        charts.sort_by_key(|chart| chart.get("id").and_then(Value::as_u64).unwrap_or_default());
        let returned_chart_count = charts.len();

        let indicator_catalog = ChartIndicatorId::ASSISTANT_VISIBLE
            .iter()
            .map(|indicator| {
                let available = !indicator.requires_hydromancer()
                    || !self.hydromancer_api_key.trim().is_empty();
                json!({
                    "id": indicator.key(),
                    "label": indicator.label(),
                    "group": indicator.group(),
                    "aliases": indicator.aliases(),
                    "available": available,
                    "unavailable_reason": (!available).then_some(
                        "Requires a Hydromancer API key in Settings > Integrations"
                    ),
                    "persisted": true,
                })
            })
            .collect::<Vec<_>>();

        json!({
            "provenance": section_provenance(
                "kerosene_open_chart_state",
                Some(generated_at_ms),
                generated_at_ms,
                Some(0),
            ),
            "selected_chart_id": selected_chart_id,
            "charts": charts,
            "indicator_catalog": indicator_catalog,
            "action_policy": {
                "set_chart_indicators_available": true,
                "scope": "allowlisted reversible visual settings on already-open candlestick charts",
                "excluded": [
                    "orders",
                    "signing",
                    "presentation_labels",
                    "quick_trade_controls",
                    "arbitrary_indicator_code",
                    "chart_creation",
                    "symbol_changes",
                    "timeframe_changes"
                ]
            },
            "coverage": {
                "returned_count": returned_chart_count,
                "total_count": total_chart_count,
                "truncated": returned_chart_count < total_chart_count,
                "complete_for_current_state": returned_chart_count == total_chart_count,
            }
        })
    }

    fn agent_account_snapshot(&self, generated_at_ms: u64) -> Value {
        let Some(data) = self.account_data.as_ref() else {
            return json!({
                "provenance": section_provenance(
                    "kerosene_account_state",
                    None,
                    generated_at_ms,
                    Some(ASSISTANT_CURRENT_DATA_MAX_AGE_MS),
                ),
                "available": false,
                "loading": self.account_loading,
                "error_present": self.account_error.is_some(),
                "coverage": {
                    "positions": list_coverage(0, 0, false, false),
                    "open_orders": list_coverage(0, 0, false, false),
                    "recent_fills": list_coverage(0, 0, false, false),
                    "recent_funding": list_coverage(0, 0, false, false),
                }
            });
        };

        let positions = data
            .clearinghouse
            .asset_positions
            .iter()
            .take(MAX_ACCOUNT_ROWS)
            .map(|asset| {
                let position = &asset.position;
                json!({
                    "coin": position.coin,
                    "size": position.szi,
                    "entry_price": position.entry_px,
                    "position_value": position.position_value,
                    "unrealized_pnl": position.unrealized_pnl,
                    "liquidation_price": position.liquidation_px.as_ref().or(asset.liquidation_px.as_ref()),
                    "leverage_type": position.leverage.leverage_type,
                    "leverage": position.leverage.value,
                    "margin_used": position.margin_used,
                    "funding_since_open": position.cum_funding.as_ref().map(|funding| funding.since_open.as_str()),
                })
            })
            .collect::<Vec<_>>();

        let spot_balances = data
            .spot
            .balances
            .iter()
            .take(MAX_ACCOUNT_ROWS)
            .map(|balance| {
                json!({
                    "coin": balance.coin,
                    "total": balance.total,
                    "held": balance.hold,
                    "entry_notional": balance.entry_ntl,
                    "supplied": balance.supplied,
                })
            })
            .collect::<Vec<_>>();

        let open_orders = data
            .open_orders
            .iter()
            .take(MAX_ACCOUNT_ROWS)
            .map(|order| {
                json!({
                    "coin": order.coin,
                    "side": order.side,
                    "limit_price": order.limit_px,
                    "size": order.sz,
                    "timestamp_ms": order.timestamp,
                    "reduce_only": order.reduce_only,
                    "is_trigger": order.is_trigger,
                    "order_type": order.order_type,
                    "time_in_force": order.tif,
                    "trigger_price": order.trigger_px,
                })
            })
            .collect::<Vec<_>>();

        let recent_fills = data
            .fills
            .iter()
            .take(MAX_RECENT_ROWS)
            .map(|fill| {
                json!({
                    "coin": fill.coin,
                    "price": fill.px,
                    "size": fill.sz,
                    "side": fill.side,
                    "direction": fill.dir,
                    "time_ms": fill.time,
                    "closed_pnl": fill.closed_pnl,
                    "fee": fill.fee,
                    "fee_token": fill.fee_token,
                })
            })
            .collect::<Vec<_>>();

        let recent_funding = data
            .funding_history
            .iter()
            .take(MAX_RECENT_ROWS)
            .map(|entry| {
                json!({
                    "coin": entry.delta.coin,
                    "funding_rate": entry.delta.funding_rate,
                    "position_size": entry.delta.szi,
                    "usdc": entry.delta.usdc,
                    "time_ms": entry.time,
                })
            })
            .collect::<Vec<_>>();

        let completeness = &data.completeness;
        json!({
            "provenance": section_provenance(
                "kerosene_account_state",
                Some(data.fetched_at_ms),
                generated_at_ms,
                Some(ASSISTANT_CURRENT_DATA_MAX_AGE_MS),
            ),
            "available": true,
            "fetched_at_ms": data.fetched_at_ms,
            "account_abstraction": format!("{:?}", data.account_abstraction),
            "margin": {
                "account_value": data.clearinghouse.margin_summary.account_value,
                "total_position_notional": data.clearinghouse.margin_summary.total_ntl_pos,
                "total_margin_used": data.clearinghouse.margin_summary.total_margin_used,
                "withdrawable": data.clearinghouse.withdrawable,
                "cross_maintenance_margin_used": data.clearinghouse.cross_maintenance_margin_used,
            },
            "spot": {
                "portfolio_margin_enabled": data.spot.portfolio_margin_enabled,
                "portfolio_margin_ratio": data.spot.portfolio_margin_ratio,
                "balances": spot_balances,
                "total_balance_count": data.spot.balances.len(),
            },
            "positions": positions,
            "total_position_count": data.clearinghouse.asset_positions.len(),
            "open_orders": open_orders,
            "total_open_order_count": data.open_orders.len(),
            "recent_fills": recent_fills,
            "total_fill_count": data.fills.len(),
            "recent_funding": recent_funding,
            "total_funding_count": data.funding_history.len(),
            "coverage": {
                "spot_balances": list_coverage(
                    data.spot.balances.len().min(MAX_ACCOUNT_ROWS),
                    data.spot.balances.len(),
                    completeness.spot_balances_complete,
                    true,
                ),
                "positions": list_coverage(
                    data.clearinghouse.asset_positions.len().min(MAX_ACCOUNT_ROWS),
                    data.clearinghouse.asset_positions.len(),
                    completeness.positions_complete,
                    completeness.positions_actionable,
                ),
                "open_orders": list_coverage(
                    data.open_orders.len().min(MAX_ACCOUNT_ROWS),
                    data.open_orders.len(),
                    completeness.open_orders_complete,
                    true,
                ),
                "recent_fills": list_coverage(
                    data.fills.len().min(MAX_RECENT_ROWS),
                    data.fills.len(),
                    completeness.fills_complete,
                    false,
                ),
                "recent_funding": list_coverage(
                    data.funding_history.len().min(MAX_RECENT_ROWS),
                    data.funding_history.len(),
                    completeness.funding_complete,
                    false,
                ),
            },
            "completeness": {
                "spot_balances_complete": completeness.spot_balances_complete,
                "positions_complete": completeness.positions_complete,
                "positions_actionable": completeness.positions_actionable,
                "open_orders_complete": completeness.open_orders_complete,
                "fills_complete": completeness.fills_complete,
                "funding_complete": completeness.funding_complete,
                "fees_complete": completeness.fees_complete,
            }
        })
    }

    fn agent_portfolio_snapshot(&self, generated_at_ms: u64) -> Value {
        let history = self.portfolio.data.as_ref().map(|history| {
            let mut buckets = history
                .buckets
                .iter()
                .map(|(name, bucket)| {
                    let account_first = bucket.account_value_history.first().copied();
                    let account_latest = bucket.account_value_history.last().copied();
                    let pnl_latest = bucket.pnl_history.last().copied();
                    (
                        name.clone(),
                        json!({
                            "account_value_first": account_first.map(|(_, value)| value),
                            "account_value_latest": account_latest.map(|(_, value)| value),
                            "latest_timestamp_ms": account_latest.map(|(time, _)| time),
                            "pnl_latest": pnl_latest.map(|(_, value)| value),
                            "volume": bucket.vlm,
                            "point_count": bucket.account_value_history.len(),
                            "skipped_invalid_points": bucket.skipped_invalid_points,
                            "invalid_volume": bucket.invalid_vlm,
                        }),
                    )
                })
                .collect::<Vec<_>>();
            buckets.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(buckets.into_iter().collect())
        });

        let income = self.income.data.as_ref().map(|income| {
            json!({
                "earned_total": income.earned_total,
                "earned_24h": income.earned_24h,
                "earned_7d": income.earned_7d,
                "earned_30d": income.earned_30d,
                "net_yearly_projection": income.net_yearly_projection,
                "current_supply_usd": income.current_supply_usd,
                "current_borrow_usd": income.current_borrow_usd,
                "health": income.health,
                "health_factor": income.health_factor,
                "tokens": income.token_rows.iter().take(MAX_ACCOUNT_ROWS).map(|row| json!({
                    "token": row.token_label,
                    "supply_usd": row.supply_usd,
                    "borrow_usd": row.borrow_usd,
                    "supply_rate": row.supply_rate,
                    "net_yearly_usd": row.net_yearly_usd,
                })).collect::<Vec<_>>(),
            })
        });

        let latest_timestamp_ms = history.as_ref().and_then(|history| {
            history.as_object().and_then(|buckets| {
                buckets
                    .values()
                    .filter_map(|bucket| bucket.get("latest_timestamp_ms").and_then(Value::as_u64))
                    .max()
            })
        });
        let history_bucket_count = history
            .as_ref()
            .and_then(Value::as_object)
            .map_or(0, serde_json::Map::len);
        let income_available = income.is_some();
        json!({
            "provenance": section_provenance(
                "hyperliquid_portfolio_and_kerosene_income_state",
                latest_timestamp_ms,
                generated_at_ms,
                None,
            ),
            "loading": self.portfolio.loading,
            "error_present": self.portfolio.last_error.is_some(),
            "selected_scope": match self.portfolio.scope {
                crate::portfolio_state::PortfolioScope::All => "all",
                crate::portfolio_state::PortfolioScope::Perp => "perp",
            },
            "selected_window": self.portfolio.window.label(),
            "history": history,
            "income_loading": self.income.loading,
            "income_error_present": self.income.last_error.is_some(),
            "income": income,
            "coverage": {
                "history_bucket_count": history_bucket_count,
                "history_complete": self.portfolio.last_error.is_none() && self.portfolio.data.is_some(),
                "history_points_exposed": false,
                "income_available": income_available,
            }
        })
    }

    fn agent_markets_snapshot(&self, generated_at_ms: u64) -> Value {
        let priority = self.agent_market_priority();
        let mut markets = self.agent_market_rows();
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

    fn agent_journal_snapshot(&self, generated_at_ms: u64) -> Value {
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
        let data_state = if !available {
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
        };

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

    fn agent_journal_tool_snapshot(&self, generated_at_ms: u64) -> Value {
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
            "data_state": self.agent_journal_snapshot(generated_at_ms)["data_state"],
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
        let net_pnl = trade.pnl - trade.fee;
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

    fn sanitized_journal_text(&self, text: &str) -> String {
        let mut sanitized = crate::helpers::redact_sensitive_response_text(text);
        for key in [
            self.openrouter_api_key.trim(),
            self.hyperdash_api_key.trim(),
        ] {
            if !key.is_empty() {
                sanitized = sanitized.replace(key, "<redacted>");
            }
        }
        crate::helpers::text_excerpt(&sanitized, MAX_JOURNAL_REFLECTION_CHARS)
    }

    fn agent_positioning_snapshot(&self, generated_at_ms: u64) -> Value {
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

    fn agent_sessions_snapshot(&self, generated_at_ms: u64) -> Value {
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

    fn agent_market_rows(&self) -> Vec<Value> {
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

    fn agent_tool_data_snapshot(
        &self,
        generated_at_ms: u64,
        pnl_card_match_allowed: bool,
    ) -> Value {
        let market_rows = self.agent_market_rows();
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
            "journal": self.agent_journal_tool_snapshot(generated_at_ms),
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

    fn agent_risk_snapshot(&self, generated_at_ms: u64) -> Value {
        let Some(data) = self.account_data.as_ref() else {
            return json!({
                "available": false,
                "as_of_ms": null,
                "snapshot_generated_at_ms": generated_at_ms,
                "reason": "account_data_unavailable",
            });
        };
        let portfolio_latest = self.portfolio.data.as_ref().and_then(|history| {
            history
                .buckets
                .values()
                .filter_map(|bucket| bucket.account_value_history.last().copied())
                .max_by_key(|(timestamp, _)| *timestamp)
        });
        json!({
            "available": true,
            "as_of_ms": data.fetched_at_ms,
            "snapshot_generated_at_ms": generated_at_ms,
            "account_abstraction": format!("{:?}", data.account_abstraction),
            "portfolio_margin_enabled": data.spot.portfolio_margin_enabled,
            "portfolio_margin_ratio": data.spot.portfolio_margin_ratio,
            "clearinghouse": {
                "account_value": data.clearinghouse.margin_summary.account_value,
                "total_position_notional": data.clearinghouse.margin_summary.total_ntl_pos,
                "total_margin_used": data.clearinghouse.margin_summary.total_margin_used,
                "cross_maintenance_margin_used": data.clearinghouse.cross_maintenance_margin_used,
                "withdrawable": data.clearinghouse.withdrawable,
            },
            "token_available_after_maintenance": data.spot.token_to_available_after_maintenance,
            "spot_balances": data.spot.balances.iter().take(MAX_ACCOUNT_ROWS).map(|balance| json!({
                "coin": balance.coin,
                "token_index": balance.token,
                "total": balance.total,
                "held": balance.hold,
                "supplied": balance.supplied,
            })).collect::<Vec<_>>(),
            "portfolio_latest": portfolio_latest.map(|(timestamp_ms, account_value)| json!({
                "timestamp_ms": timestamp_ms,
                "account_value": account_value,
            })),
            "income": self.income.data.as_ref().map(|income| json!({
                "current_supply_usd": income.current_supply_usd,
                "current_borrow_usd": income.current_borrow_usd,
                "health": income.health,
                "health_factor": income.health_factor,
            })),
            "current_state": {
                "position_count": data.clearinghouse.asset_positions.len(),
                "open_order_count": data.open_orders.len(),
                "positions_complete": data.completeness.positions_complete,
                "positions_actionable": data.completeness.positions_actionable,
                "open_orders_complete": data.completeness.open_orders_complete,
            },
            "scope_warning": "Clearinghouse, spot, portfolio-history, and income values have different source semantics. Report them separately unless a deterministic reconciliation explicitly bridges them.",
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
        journal_trade_net_pnl(&trades[*right])
            .partial_cmp(&journal_trade_net_pnl(&trades[*left]))
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
                journal_trade_net_pnl(&trades[*left]),
                trades[*left].total_entry_notional,
            ),
            positive_ratio_pct(
                journal_trade_net_pnl(&trades[*right]),
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

fn journal_trade_net_pnl(trade: &crate::journal::AggregatedTrade) -> f64 {
    trade.pnl - trade.fee
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

fn market_priority_index(priority: &[String], symbol: &str) -> usize {
    priority
        .iter()
        .position(|candidate| candidate == symbol)
        .unwrap_or(usize::MAX)
}

fn agent_fill_snapshot(fill: &crate::account::UserFill) -> Value {
    json!({
        "coin": fill.coin,
        "price": fill.px,
        "size": fill.sz,
        "side": fill.side,
        "direction": fill.dir,
        "time_ms": fill.time,
        "closed_pnl": fill.closed_pnl,
        "fee": fill.fee,
        "fee_token": fill.fee_token,
    })
}

fn agent_funding_snapshot(entry: &crate::account::FundingEntry) -> Value {
    json!({
        "coin": entry.delta.coin,
        "funding_rate": entry.delta.funding_rate,
        "position_size": entry.delta.szi,
        "usdc": entry.delta.usdc,
        "time_ms": entry.time,
    })
}

pub(crate) async fn write_agent_snapshot(
    workspace_dir: PathBuf,
    generation: u64,
    request_id: u64,
    snapshot: Vec<u8>,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(&workspace_dir)
        .map_err(|error| format!("Could not create the assistant workspace: {error}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&workspace_dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("Could not secure the assistant workspace: {error}"))?;
    }

    let path = staged_snapshot_path(&workspace_dir, generation, request_id);
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .map_err(|error| format!("Could not open the assistant snapshot: {error}"))?;
    file.write_all(&snapshot)
        .map_err(|error| format!("Could not write the assistant snapshot: {error}"))?;
    Ok(path)
}

pub(crate) fn activate_agent_snapshot(
    workspace_dir: &Path,
    staged_path: &Path,
) -> Result<PathBuf, String> {
    let active_path = workspace_dir.join("snapshot.json");
    match std::fs::rename(staged_path, &active_path) {
        Ok(()) => Ok(active_path),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            std::fs::remove_file(&active_path)
                .map_err(|error| format!("Could not replace the assistant snapshot: {error}"))?;
            std::fs::rename(staged_path, &active_path)
                .map_err(|error| format!("Could not activate the assistant snapshot: {error}"))?;
            Ok(active_path)
        }
        Err(error) => Err(format!(
            "Could not activate the assistant snapshot: {error}"
        )),
    }
}

pub(crate) fn workspace_dir() -> PathBuf {
    std::env::temp_dir().join(format!("kerosene-agent-{}", std::process::id()))
}

pub(crate) fn clear_sensitive_runtime_files(
    workspace_dir: &Path,
    generation: u64,
    request_id: u64,
) {
    let snapshot_path = workspace_dir.join("snapshot.json");
    let _ = std::fs::remove_file(snapshot_path);
    let _ = std::fs::remove_file(staged_snapshot_path(workspace_dir, generation, request_id));
}

fn staged_snapshot_path(workspace_dir: &Path, generation: u64, request_id: u64) -> PathBuf {
    workspace_dir.join(format!("snapshot-{generation}-{request_id}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    #[test]
    fn pnl_card_match_authorization_is_private_and_scoped_to_the_request() {
        let (terminal, _) = TradingTerminal::boot();
        let regular: Value = serde_json::from_slice(
            &terminal
                .build_agent_snapshot_for_request(false)
                .expect("regular snapshot"),
        )
        .expect("regular snapshot json");
        let attached: Value = serde_json::from_slice(
            &terminal
                .build_agent_snapshot_for_request(true)
                .expect("attached snapshot"),
        )
        .expect("attached snapshot json");

        assert_eq!(
            regular["_tool_data"]["assistant_request"]["pnl_card_match_allowed"],
            false
        );
        assert_eq!(
            attached["_tool_data"]["assistant_request"]["pnl_card_match_allowed"],
            true
        );
        assert!(attached.get("assistant_request").is_none());
        assert!(
            attached["data_policy"]["omitted"]
                .as_array()
                .is_some_and(|values| values.iter().any(|value| value == "wallet_addresses"))
        );
    }

    #[test]
    fn empty_snapshot_has_versioned_sanitized_contract() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".into());
        terminal.openrouter_api_key = "sk-or-secret".into();
        terminal.hyperdash_api_key = "hyperdash-secret".into();

        let bytes = terminal.build_agent_snapshot().expect("snapshot");
        let text = String::from_utf8(bytes).expect("utf8");
        let value: Value = serde_json::from_str(&text).expect("json");

        assert_eq!(value["schema_version"], SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(value["data_policy"]["access"], "read_only");
        assert_eq!(value["account"]["provenance"]["as_of_ms"], Value::Null);
        assert_eq!(
            value["account"]["provenance"]["observed_at_ms"],
            Value::Null
        );
        assert_eq!(
            value["account"]["provenance"]["freshness"]["state"],
            "unknown"
        );
        assert!(value["account"]["provenance"]["snapshot_generated_at_ms"].is_u64());
        assert_eq!(value["_tool_data"]["markets"]["as_of_ms"], Value::Null);
        assert_eq!(value["_tool_data"]["risk"]["as_of_ms"], Value::Null);
        assert!(!text.contains("0xabc0000000000000000000000000000000000000"));
        assert!(!text.contains("sk-or-secret"));
        assert!(!text.contains("hyperdash-secret"));
        assert_eq!(value["_tool_data"]["contract"]["private"], true);
        assert!(value["_tool_data"]["glossary"]["funding_usdc"].is_string());
    }

    #[test]
    fn workspace_snapshot_exposes_selected_chart_and_safe_indicator_catalog() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        let mut chart = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        chart.macro_indicators.tf_ema_50 = true;
        chart.chart.macro_indicators = chart.macro_indicators.clone();
        terminal.charts.insert(7, chart);
        terminal.primary_chart_id = Some(7);
        terminal.hydromancer_api_key = String::new().into();

        let bytes = terminal.build_agent_snapshot().expect("snapshot");
        let value: Value = serde_json::from_slice(&bytes).expect("json");
        let workspace = &value["workspace"];
        let catalog = workspace["indicator_catalog"]
            .as_array()
            .expect("indicator catalog");

        assert_eq!(workspace["selected_chart_id"], 7);
        assert_eq!(workspace["charts"][0]["id"], 7);
        assert_eq!(workspace["charts"][0]["selected"], true);
        assert_eq!(workspace["charts"][0]["symbol"], "BTC");
        assert_eq!(workspace["charts"][0]["timeframe"], "1H");
        assert_eq!(workspace["charts"][0]["indicators"]["tf_ema_50"], true);
        assert!(catalog.iter().any(|entry| entry["id"] == "tf_ema_50"));
        assert!(catalog.iter().any(|entry| {
            entry["id"] == "funding_rate"
                && entry["available"] == false
                && entry["unavailable_reason"].is_string()
        }));
        assert!(!catalog.iter().any(|entry| entry["id"] == "quick_trade"));
        assert!(!catalog.iter().any(|entry| entry["id"] == "labels"));
    }

    #[test]
    fn workspace_snapshot_is_bounded_and_keeps_the_selected_chart() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        for id in 1..=MAX_WORKSPACE_CHARTS as u64 + 3 {
            terminal.charts.insert(
                id,
                ChartInstance::new(id, format!("ASSET{id}"), Timeframe::H1),
            );
        }
        terminal.primary_chart_id = Some(MAX_WORKSPACE_CHARTS as u64 + 3);

        let bytes = terminal.build_agent_snapshot().expect("snapshot");
        let value: Value = serde_json::from_slice(&bytes).expect("json");
        let workspace = &value["workspace"];
        let charts = workspace["charts"].as_array().expect("charts");

        assert_eq!(charts.len(), MAX_WORKSPACE_CHARTS);
        assert!(charts.iter().any(|chart| chart["selected"] == true));
        assert_eq!(
            workspace["coverage"]["returned_count"],
            MAX_WORKSPACE_CHARTS
        );
        assert_eq!(
            workspace["coverage"]["total_count"],
            MAX_WORKSPACE_CHARTS + 3
        );
        assert_eq!(workspace["coverage"]["truncated"], true);
        assert_eq!(workspace["coverage"]["complete_for_current_state"], false);
    }

    #[test]
    fn provenance_reports_age_and_staleness_without_rewriting_observation_time() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.all_mids.insert("BTC".to_string(), 65_000.0);
        terminal.all_mids_updated_at_ms.insert("BTC".to_string(), 1);

        let bytes = terminal.build_agent_snapshot().expect("snapshot");
        let value: Value = serde_json::from_slice(&bytes).expect("json");
        let provenance = &value["markets"]["provenance"];

        assert_eq!(provenance["observed_at_ms"], 1);
        assert_eq!(provenance["as_of_ms"], 1);
        assert!(
            provenance["age_ms"]
                .as_u64()
                .is_some_and(|age| age > 15_000)
        );
        assert_eq!(provenance["freshness"]["state"], "stale");
        assert_eq!(
            provenance["freshness"]["max_age_ms"],
            ASSISTANT_CURRENT_DATA_MAX_AGE_MS
        );
    }

    #[test]
    fn public_market_cap_prioritizes_active_symbol_and_private_index_is_complete() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.active_symbol = "BTC".to_string();
        terminal.active_symbol_display = "BTC".to_string();
        terminal.all_mids.clear();
        terminal.all_mids_updated_at_ms.clear();
        for index in 0..300 {
            let symbol = format!("@{index}");
            terminal.all_mids.insert(symbol.clone(), index as f64 + 1.0);
            terminal.all_mids_updated_at_ms.insert(symbol, 123);
        }
        terminal.all_mids.insert("BTC".to_string(), 65_000.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), 456);

        let bytes = terminal.build_agent_snapshot().expect("snapshot");
        let value: Value = serde_json::from_slice(&bytes).expect("json");
        let public_markets = value["markets"]["markets"].as_array().expect("markets");
        let private_markets = value["_tool_data"]["markets"]["rows"]
            .as_array()
            .expect("private markets");

        assert_eq!(public_markets.len(), MAX_MARKETS);
        assert_eq!(public_markets[0]["symbol"], "BTC");
        assert_eq!(public_markets[0]["raw_symbol_is_sanitized"], false);
        assert_eq!(value["markets"]["coverage"]["returned_count"], MAX_MARKETS);
        assert_eq!(value["markets"]["coverage"]["total_count"], 301);
        assert_eq!(value["markets"]["coverage"]["truncated"], true);
        assert_eq!(private_markets.len(), 301);
    }

    #[test]
    fn journal_snapshot_exposes_rankable_trades_and_redacted_reflections() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".into());
        terminal.journal.active_account_key = Some("private-account-key".to_string());
        terminal.journal.last_refresh_time = Some(1_234_567);
        terminal.journal.sync_status.complete = true;
        terminal.openrouter_api_key = "sk-or-private-journal-key".into();
        terminal.journal.trades.push(journal_trade(
            "internal-trade-id",
            "BTC",
            250.0,
            10.0,
            1_000,
        ));
        terminal.journal.entries.insert(
            "internal-trade-id".to_string(),
            crate::journal::JournalNote {
                open: "breakout thesis sk-or-private-journal-key".to_string(),
                close: "wallet 0xabc0000000000000000000000000000000000000".to_string(),
                cause_of_error: String::new(),
                tags: vec!["momentum".to_string()],
            },
        );

        let bytes = terminal.build_agent_snapshot().expect("snapshot");
        let text = String::from_utf8(bytes).expect("utf8");
        let value: Value = serde_json::from_str(&text).expect("json");
        let row = &value["_tool_data"]["journal"]["trades"][0];

        assert_eq!(value["journal"]["data_state"], "ready");
        assert_eq!(value["journal"]["total_trade_count"], 1);
        assert_eq!(row["symbol"], "BTC");
        assert_eq!(row["side"], "long");
        assert_eq!(row["gross_realized_pnl_usd"], 250.0);
        assert_eq!(row["fees_usd"], 10.0);
        assert_eq!(row["net_realized_pnl_usd"], 240.0);
        assert_eq!(row["return_on_entry_pct"], 24.0);
        assert_eq!(row["reflection"]["tags"][0], "momentum");
        assert!(
            row["reflection"]["open_thesis"]
                .as_str()
                .is_some_and(|note| note.contains("<redacted>"))
        );
        assert!(!text.contains("internal-trade-id"));
        assert!(!text.contains("private-account-key"));
        assert!(!text.contains("sk-or-private-journal-key"));
        assert!(!text.contains("0xabc0000000000000000000000000000000000000"));
    }

    #[test]
    fn capped_journal_selection_preserves_net_pnl_extremes() {
        let trades = vec![
            journal_trade("best", "BTC", 1_000.0, 0.0, 1),
            journal_trade("worst", "ETH", -900.0, 0.0, 2),
            journal_trade("middle-1", "SOL", 5.0, 0.0, 3),
            journal_trade("middle-2", "HYPE", 4.0, 0.0, 4),
            journal_trade("middle-3", "DOGE", 3.0, 0.0, 5),
            journal_trade("recent", "XRP", 2.0, 0.0, 6),
        ];

        let selected = journal_trade_selection_indexes(&trades, &HashMap::new(), 4);

        assert!(selected.contains(&0), "best trade should be retained");
        assert!(selected.contains(&1), "worst trade should be retained");
        assert_eq!(selected.len(), 4);
    }

    fn journal_trade(
        id: &str,
        coin: &str,
        pnl: f64,
        fee: f64,
        start_time: u64,
    ) -> crate::journal::AggregatedTrade {
        crate::journal::AggregatedTrade {
            id: id.to_string(),
            legacy_note_ids: Vec::new(),
            coin: coin.to_string(),
            start_time,
            end_time: Some(start_time + 60_000),
            max_position: 1.0,
            volume: 2_000.0,
            fee,
            pnl,
            status: "CLOSED".to_string(),
            fill_count: 2,
            avg_entry_price: 100.0,
            total_entry_notional: 1_000.0,
            total_entry_size: 10.0,
            is_long: true,
            basis_complete: true,
        }
    }
}
