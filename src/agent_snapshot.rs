use crate::app_state::TradingTerminal;

use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

const SNAPSHOT_SCHEMA_VERSION: u32 = 1;
const MAX_MARKETS: usize = 250;
const MAX_ACCOUNT_ROWS: usize = 100;
const MAX_RECENT_ROWS: usize = 50;

// ---------------------------------------------------------------------------
// Read-only Agent Snapshot
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn build_agent_snapshot(&self) -> Result<Vec<u8>, String> {
        let snapshot = json!({
            "schema_version": SNAPSHOT_SCHEMA_VERSION,
            "generated_at_ms": Self::now_ms(),
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
                    "recent_rows": MAX_RECENT_ROWS
                }
            },
            "overview": self.agent_overview_snapshot(),
            "account": self.agent_account_snapshot(),
            "portfolio": self.agent_portfolio_snapshot(),
            "markets": self.agent_markets_snapshot(),
            "positioning": self.agent_positioning_snapshot(),
            "sessions": self.agent_sessions_snapshot(),
        });

        serde_json::to_vec(&snapshot)
            .map_err(|error| format!("Could not serialize the assistant snapshot: {error}"))
    }

    fn agent_overview_snapshot(&self) -> Value {
        json!({
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

    fn agent_account_snapshot(&self) -> Value {
        let Some(data) = self.account_data.as_ref() else {
            return json!({
                "available": false,
                "loading": self.account_loading,
                "error_present": self.account_error.is_some(),
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

    fn agent_portfolio_snapshot(&self) -> Value {
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

        json!({
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
        })
    }

    fn agent_markets_snapshot(&self) -> Value {
        let mut markets = self
            .all_mids
            .iter()
            .map(|(symbol, mid)| {
                json!({
                    "symbol": symbol,
                    "display_symbol": self.display_name_for_symbol(symbol),
                    "mid": mid,
                    "updated_at_ms": self.all_mids_updated_at_ms.get(symbol),
                    "favourite": self.favourite_symbols.contains(symbol),
                })
            })
            .collect::<Vec<_>>();
        markets.sort_by(|left, right| {
            left.get("symbol")
                .and_then(Value::as_str)
                .cmp(&right.get("symbol").and_then(Value::as_str))
        });
        markets.truncate(MAX_MARKETS);

        json!({
            "active_symbol": self.active_symbol,
            "active_symbol_display": self.active_symbol_display,
            "markets": markets,
            "total_market_count": self.all_mids.len(),
            "truncated": self.all_mids.len() > MAX_MARKETS,
        })
    }

    fn agent_positioning_snapshot(&self) -> Value {
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

        json!({
            "panes": panes,
            "note": "Wallet-level HyperDash addresses and labels are intentionally omitted; only aggregates are exposed.",
        })
    }

    fn agent_sessions_snapshot(&self) -> Value {
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
        json!({ "panes": sessions })
    }
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

        let bytes = terminal.build_agent_snapshot().expect("snapshot");
        let text = String::from_utf8(bytes).expect("utf8");
        let value: Value = serde_json::from_str(&text).expect("json");

        assert_eq!(value["schema_version"], SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(value["data_policy"]["access"], "read_only");
        assert!(!text.contains("0xabc0000000000000000000000000000000000000"));
        assert!(!text.contains("sk-or-secret"));
    }
}
