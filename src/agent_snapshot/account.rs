use super::{
    ASSISTANT_CURRENT_DATA_MAX_AGE_MS, MAX_ACCOUNT_ROWS, MAX_RECENT_ROWS, list_coverage,
    section_provenance,
};
use crate::app_state::TradingTerminal;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Assistant Account, Portfolio, and Risk Snapshots
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn agent_account_snapshot(&self, generated_at_ms: u64) -> Value {
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
            .map(agent_fill_snapshot)
            .collect::<Vec<_>>();

        let recent_funding = data
            .funding_history
            .iter()
            .take(MAX_RECENT_ROWS)
            .map(agent_funding_snapshot)
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

    pub(super) fn agent_portfolio_snapshot(&self, generated_at_ms: u64) -> Value {
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
            "loading": self.portfolio.refresh.loading,
            "error_present": self.portfolio.last_error.is_some(),
            "selected_scope": match self.portfolio.scope {
                crate::portfolio_state::PortfolioScope::All => "all",
                crate::portfolio_state::PortfolioScope::Perp => "perp",
            },
            "selected_window": self.portfolio.window.label(),
            "history": history,
            "income_loading": self.income.refresh.loading,
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

    pub(super) fn agent_risk_snapshot(&self, generated_at_ms: u64) -> Value {
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

pub(super) fn agent_fill_snapshot(fill: &crate::account::UserFill) -> Value {
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

pub(super) fn agent_funding_snapshot(entry: &crate::account::FundingEntry) -> Value {
    json!({
        "coin": entry.delta.coin,
        "funding_rate": entry.delta.funding_rate,
        "position_size": entry.delta.szi,
        "usdc": entry.delta.usdc,
        "time_ms": entry.time,
    })
}

#[cfg(test)]
mod tests;
