use crate::app_state::TradingTerminal;

use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

const SNAPSHOT_SCHEMA_VERSION: u32 = 2;
const MAX_MARKETS: usize = 250;
const MAX_ACCOUNT_ROWS: usize = 100;
const MAX_RECENT_ROWS: usize = 50;
const MAX_TOOL_ACTIVITY_ROWS: usize = 2_000;

// ---------------------------------------------------------------------------
// Read-only Agent Snapshot
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn build_agent_snapshot(&self) -> Result<Vec<u8>, String> {
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
                    "transaction_hashes"
                ],
                "row_limits": {
                    "markets": MAX_MARKETS,
                    "account_rows": MAX_ACCOUNT_ROWS,
                    "recent_rows": MAX_RECENT_ROWS,
                    "tool_activity_rows": MAX_TOOL_ACTIVITY_ROWS
                },
                "list_contract": "returned_count is the number serialized in the section; total_count is the number available in Kerosene state; endpoint_fetch_complete does not mean an Assistant-capped list is untruncated",
                "market_symbol_contract": "symbol is the raw exchange/API key; canonical_symbol and display_symbol provide user-facing identity where metadata is available"
            },
            "overview": self.agent_overview_snapshot(generated_at_ms),
            "account": self.agent_account_snapshot(generated_at_ms),
            "portfolio": self.agent_portfolio_snapshot(generated_at_ms),
            "markets": self.agent_markets_snapshot(generated_at_ms),
            "positioning": self.agent_positioning_snapshot(generated_at_ms),
            "sessions": self.agent_sessions_snapshot(generated_at_ms),
            "_tool_data": self.agent_tool_data_snapshot(generated_at_ms),
        });

        serde_json::to_vec(&snapshot)
            .map_err(|error| format!("Could not serialize the assistant snapshot: {error}"))
    }

    fn agent_overview_snapshot(&self, generated_at_ms: u64) -> Value {
        json!({
            "provenance": section_provenance("kerosene_state", Some(generated_at_ms)),
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

    fn agent_account_snapshot(&self, generated_at_ms: u64) -> Value {
        let Some(data) = self.account_data.as_ref() else {
            return json!({
                "provenance": section_provenance("kerosene_account_state", Some(generated_at_ms)),
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
            "provenance": section_provenance("kerosene_account_state", Some(data.fetched_at_ms)),
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
                latest_timestamp_ms.or(Some(generated_at_ms)),
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
            "provenance": section_provenance("hyperliquid_all_mids_and_kerosene_symbol_metadata", as_of_ms.or(Some(generated_at_ms))),
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
            "provenance": section_provenance("hyperdash_aggregate_positioning_cache", as_of_ms.or(Some(generated_at_ms))),
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
            "provenance": section_provenance("kerosene_session_analysis_cache", as_of_ms.or(Some(generated_at_ms))),
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

    fn agent_tool_data_snapshot(&self, generated_at_ms: u64) -> Value {
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
            "markets": {
                "as_of_ms": self.all_mids_updated_at_ms.values().copied().max().unwrap_or(generated_at_ms),
                "rows": market_rows,
                "coverage": list_coverage(self.all_mids.len(), self.all_mids.len(), true, true),
            },
            "activity": account_activity,
            "risk": self.agent_risk_snapshot(generated_at_ms),
            "positioning_cache": self.agent_positioning_snapshot(generated_at_ms),
            "sessions_cache": self.agent_sessions_snapshot(generated_at_ms),
            "glossary": {
                "funding_usdc": "Account cash flow: negative means paid; positive means received.",
                "margin_account_value": "The clearinghouse margin-summary value and its scope come from the selected Hyperliquid account abstraction; do not replace it with spot or portfolio equity.",
                "portfolio_history": "Windowed account-value and PnL series may use different baselines; a shorter-window PnL can exceed all-time PnL after earlier losses.",
                "raw_market_symbols": "@N and #N are real exchange/API identifiers, not Assistant privacy redaction. Use canonical_symbol/display_symbol metadata rather than guessing mappings.",
                "completeness": "endpoint_fetch_complete describes the upstream fetch. An Assistant list can still be truncated when returned_count is below total_count.",
            }
        })
    }

    fn agent_risk_snapshot(&self, generated_at_ms: u64) -> Value {
        let Some(data) = self.account_data.as_ref() else {
            return json!({
                "available": false,
                "as_of_ms": generated_at_ms,
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

fn section_provenance(source: &str, as_of_ms: Option<u64>) -> Value {
    json!({
        "source": source,
        "as_of_ms": as_of_ms,
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
        assert!(!text.contains("0xabc0000000000000000000000000000000000000"));
        assert!(!text.contains("sk-or-secret"));
        assert!(!text.contains("hyperdash-secret"));
        assert_eq!(value["_tool_data"]["contract"]["private"], true);
        assert!(value["_tool_data"]["glossary"]["funding_usdc"].is_string());
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
}
