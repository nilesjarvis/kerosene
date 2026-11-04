use crate::account::OpenOrder;
use crate::account_state::position_tpsl::TpslOutcome;
use crate::api::{MarketType, fetch_chart_asset_context};
use crate::app_state::TradingTerminal;
use crate::helpers::{parse_positive_number, redact_sensitive_response_text};
use crate::signing::{
    CapturedAgentKey, ExchangeResponse, PositionTpslOrder, TpslKind, float_to_wire,
    place_position_tpsl, round_price,
};
use std::sync::atomic::{AtomicU64, Ordering};

static LAST_TPSL_NONCE: AtomicU64 = AtomicU64::new(0);

fn next_cloid() -> String {
    let now = TradingTerminal::now_ms();
    let nonce = LAST_TPSL_NONCE.fetch_max(now, Ordering::SeqCst).max(now);
    let nonce = LAST_TPSL_NONCE.fetch_add(1, Ordering::SeqCst).max(nonce);
    format!("0x7470736c{nonce:024x}")
}

pub(crate) fn position_trigger_order(order: &OpenOrder, coin: &str) -> bool {
    order.coin == coin
        && (order.is_trigger == Some(true)
            || order
                .trigger_px
                .as_deref()
                .and_then(parse_positive_number)
                .is_some()
            || order.order_type.as_deref().is_some_and(|kind| {
                let kind = kind.to_ascii_lowercase();
                kind.contains("stop") || kind.contains("take profit")
            }))
}

pub(crate) fn existing_tpsl_kind(order: &OpenOrder) -> Option<TpslKind> {
    let kind = order.order_type.as_deref()?.to_ascii_lowercase();
    if kind.contains("take profit") {
        Some(TpslKind::TakeProfit)
    } else if kind.contains("stop") {
        Some(TpslKind::StopLoss)
    } else {
        None
    }
}

pub(crate) fn tpsl_prices(
    input: &str,
    sz_decimals: u32,
    is_buy: bool,
) -> Result<(String, String), String> {
    let trigger = parse_positive_number(input).ok_or("Enter a valid positive trigger price")?;
    let rounded = round_price(trigger, sz_decimals, false);
    if !rounded.is_finite() || rounded <= 0.0 || (rounded - trigger).abs() > trigger.abs() * 1e-10 {
        return Err("Trigger price does not match this market's price precision".into());
    }
    // Hyperliquid market TP/SL uses a 10% execution buffer from the trigger.
    let limit = round_price(trigger * if is_buy { 1.1 } else { 0.9 }, sz_decimals, false);
    if !limit.is_finite() || limit <= 0.0 {
        return Err("Trigger price is too small or too large".into());
    }
    Ok((float_to_wire(trigger), float_to_wire(limit)))
}

impl TradingTerminal {
    pub(crate) fn prepare_position_tpsl(&self) -> Result<Vec<PositionTpslOrder>, String> {
        let form = self
            .position_tpsl
            .form
            .as_ref()
            .ok_or("Open a position first")?;
        if !self.connected_order_account_matches(&form.account_address) {
            return Err("Account changed; reopen TP/SL".into());
        }
        if self.account_loading || self.account_reconciliation_required {
            return Err("Wait for the account refresh before setting TP/SL".into());
        }
        let data = self
            .account_data_for_order_account(&form.account_address)
            .ok_or("Refresh account data before setting TP/SL")?;
        let now = Self::now_ms();
        if !data.completeness.positions_actionable
            || !data.completeness.open_orders_complete
            || !data.is_fresh_for_position_action(now)
            || !data.is_fresh_for_open_order_action_for_symbol(&form.coin, now)
        {
            return Err("Account data is stale or incomplete; refresh before setting TP/SL".into());
        }
        let position = data
            .clearinghouse
            .asset_positions
            .iter()
            .find(|ap| ap.position.coin == form.coin)
            .ok_or("Position is no longer open")?;
        let size = position
            .position
            .szi
            .parse::<f64>()
            .ok()
            .filter(|size| size.is_finite() && size.abs() > 1e-12)
            .ok_or("Position is no longer open")?;
        if (size > 0.0) != form.is_long {
            return Err("Position direction changed; reopen TP/SL".into());
        }
        let symbol = self
            .exchange_symbol_for_key(&form.coin)
            .ok_or("Market metadata unavailable")?;
        if symbol.market_type != MarketType::Perp {
            return Err("Position TP/SL is available for perpetuals only".into());
        }
        self.validate_exchange_symbol_orderable(symbol, "Position")?;
        let mut orders = Vec::new();
        for (kind, input) in [
            (TpslKind::TakeProfit, &form.take_profit),
            (TpslKind::StopLoss, &form.stop_loss),
        ] {
            if input.trim().is_empty() {
                continue;
            }
            if data.open_orders.iter().any(|order| {
                position_trigger_order(order, &form.coin)
                    && existing_tpsl_kind(order).is_none_or(|existing| existing == kind)
            }) {
                return Err(format!(
                    "{} already exists; cancel it before replacing it",
                    kind.label()
                ));
            }
            let (trigger_px, price) = tpsl_prices(input, symbol.sz_decimals, size < 0.0)?;
            orders.push(PositionTpslOrder {
                asset: symbol.asset_index,
                is_buy: size < 0.0,
                price,
                trigger_px,
                kind,
                cloid: next_cloid(),
            });
        }
        if orders.is_empty() {
            return Err("Enter a take-profit or stop-loss price".into());
        }
        Ok(orders)
    }
}

pub(crate) fn validate_tpsl_trigger_sides(
    orders: &[PositionTpslOrder],
    mark: f64,
) -> Result<(), String> {
    if !mark.is_finite() || mark <= 0.0 {
        return Err("Current mark price unavailable".into());
    }
    for order in orders {
        let trigger = parse_positive_number(&order.trigger_px).ok_or("Invalid trigger price")?;
        let above = (order.kind == TpslKind::TakeProfit) != order.is_buy;
        if (above && trigger <= mark) || (!above && trigger >= mark) {
            return Err(format!(
                "{} must be {} the current mark price",
                order.kind.label(),
                if above { "above" } else { "below" }
            ));
        }
    }
    Ok(())
}

pub(crate) async fn submit_position_tpsl_orders(
    key: CapturedAgentKey,
    coin: String,
    orders: Vec<PositionTpslOrder>,
) -> Result<ExchangeResponse, String> {
    // Fetch the mark for this exact market, including HIP-3, before signing.
    // Failures here cannot have placed any orders.
    let mark = match fetch_chart_asset_context(coin).await {
        Ok(context) => context
            .and_then(|ctx| ctx.mark_px)
            .as_deref()
            .and_then(parse_positive_number),
        Err(_) => None,
    };
    let validation = mark
        .ok_or_else(|| "Current mark price unavailable; try again".to_string())
        .and_then(|mark| validate_tpsl_trigger_sides(&orders, mark));
    if let Err(error) = validation {
        return Ok(ExchangeResponse::local_rejection(error));
    }
    place_position_tpsl(key, orders).await
}

pub(crate) fn position_tpsl_outcomes(
    result: Result<ExchangeResponse, String>,
    count: usize,
) -> Vec<TpslOutcome> {
    let Ok(response) = result else {
        return vec![TpslOutcome::Unknown; count];
    };
    if response.is_local_rejection() || response.status == "err" {
        return vec![
            TpslOutcome::Rejected(redact_sensitive_response_text(&response.summary()));
            count
        ];
    }
    if response.status != "ok" {
        return vec![TpslOutcome::Unknown; count];
    }
    let Some(inner) = response
        .response
        .filter(|inner| inner.response_type == "order")
    else {
        return vec![TpslOutcome::Unknown; count];
    };
    let Some(data) = inner.data.filter(|data| data.statuses.len() == count) else {
        return vec![TpslOutcome::Unknown; count];
    };
    data.statuses
        .iter()
        .map(|status| {
            if status.as_object().is_some_and(|object| object.len() == 1) {
                if let Some(error) = status.get("error").and_then(serde_json::Value::as_str) {
                    return TpslOutcome::Rejected(redact_sensitive_response_text(error));
                }
                if ["resting", "filled"].iter().any(|kind| {
                    status
                        .get(kind)
                        .and_then(|value| value.get("oid"))
                        .and_then(serde_json::Value::as_u64)
                        .is_some_and(|oid| oid > 0)
                }) {
                    return TpslOutcome::Accepted;
                }
            }
            TpslOutcome::Unknown
        })
        .collect()
}

#[cfg(test)]
mod tests;
