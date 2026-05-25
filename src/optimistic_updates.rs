use crate::account::{
    AccountData, AssetPosition, OpenOrder, Position, PositionLeverage, SpotBalance, UserFill,
};
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::helpers::{parse_finite_number, parse_positive_finite_number};
use crate::signing::{ExchangeResponse, OrderKind, float_to_wire};

use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

// ---------------------------------------------------------------------------
// Optimistic Account Updates
// ---------------------------------------------------------------------------

pub(crate) const OPTIMISTIC_EFFECT_TTL_MS: u64 = 30_000;
const POSITION_EPSILON: f64 = 1e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptimisticOrderSource {
    OrderForm,
    QuickOrder { chart_id: ChartId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptimisticOrderContext {
    pub(crate) account_address: String,
    pub(crate) symbol: String,
    pub(crate) is_buy: bool,
    pub(crate) size: String,
    pub(crate) price: String,
    pub(crate) order_kind: OrderKind,
    pub(crate) reduce_only: bool,
    pub(crate) submitted_at_ms: u64,
    pub(crate) pending_id: Option<u64>,
    pub(crate) source: OptimisticOrderSource,
}

#[derive(Debug, Clone)]
pub(crate) struct OrderSubmissionResult {
    pub(crate) context: OptimisticOrderContext,
    pub(crate) result: Result<ExchangeResponse, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingOrderChangeKind {
    Placing,
    Cancelling,
    Modifying,
}

#[derive(Debug, Clone)]
pub(crate) struct OrderCancellationContext {
    pub(crate) account_address: String,
    pub(crate) symbol: String,
    pub(crate) oid: u64,
    pub(crate) pending_id: Option<u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct OrderCancellationResult {
    pub(crate) context: OrderCancellationContext,
    pub(crate) result: Result<ExchangeResponse, String>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct OptimisticAccountEffects {
    open_orders: BTreeMap<u64, OptimisticOpenOrder>,
    fills: BTreeMap<u64, OptimisticFill>,
    position_effects: BTreeMap<u64, OptimisticPositionEffect>,
    pending_order_changes: BTreeMap<u64, PendingOrderChangeEffect>,
}

#[derive(Debug, Clone)]
struct OptimisticOpenOrder {
    account_address: String,
    order: OpenOrder,
    created_at_ms: u64,
}

#[derive(Debug, Clone)]
struct OptimisticFill {
    account_address: String,
    fill: UserFill,
    created_at_ms: u64,
}

#[derive(Debug, Clone)]
struct OptimisticPositionEffect {
    account_address: String,
    symbol: String,
    oid: u64,
    fill_delta: f64,
    avg_price: f64,
    expected_szi: Option<f64>,
    expected_entry_px: Option<f64>,
    created_at_ms: u64,
}

#[derive(Debug, Clone)]
struct PendingOrderChangeEffect {
    account_address: String,
    symbol: String,
    oid: Option<u64>,
    is_buy: bool,
    size: String,
    price: String,
    kind: PendingOrderChangeKind,
    created_at_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectedOpenOrder<'a> {
    pub(crate) order: &'a OpenOrder,
    pub(crate) is_optimistic: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectedUserFill<'a> {
    pub(crate) fill: &'a UserFill,
    pub(crate) is_optimistic: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectedAssetPosition {
    pub(crate) asset_position: AssetPosition,
    pub(crate) is_optimistic: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectedPendingOrderChange {
    pub(crate) pending_id: u64,
    pub(crate) symbol: String,
    pub(crate) oid: Option<u64>,
    pub(crate) is_buy: bool,
    pub(crate) size: String,
    pub(crate) price: String,
    pub(crate) kind: PendingOrderChangeKind,
}

impl OptimisticAccountEffects {
    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.open_orders.is_empty()
            && self.fills.is_empty()
            && self.position_effects.is_empty()
            && self.pending_order_changes.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.open_orders.clear();
        self.fills.clear();
        self.position_effects.clear();
        self.pending_order_changes.clear();
    }

    #[cfg(test)]
    fn open_order_count(&self) -> usize {
        self.open_orders.len()
    }

    #[cfg(test)]
    fn fill_count(&self) -> usize {
        self.fills.len()
    }

    #[cfg(test)]
    fn position_effect_count(&self) -> usize {
        self.position_effects.len()
    }

    #[cfg(test)]
    fn pending_order_change_count(&self) -> usize {
        self.pending_order_changes.len()
    }

    pub(crate) fn add_pending_order_placement(
        &mut self,
        context: &mut OptimisticOrderContext,
    ) -> u64 {
        let pending_id = self.next_pending_order_change_id(context.submitted_at_ms);
        self.pending_order_changes.insert(
            pending_id,
            PendingOrderChangeEffect {
                account_address: context.account_address.clone(),
                symbol: context.symbol.clone(),
                oid: None,
                is_buy: context.is_buy,
                size: context.size.clone(),
                price: context.price.clone(),
                kind: PendingOrderChangeKind::Placing,
                created_at_ms: context.submitted_at_ms,
            },
        );
        context.pending_id = Some(pending_id);
        pending_id
    }

    pub(crate) fn add_pending_order_cancellation(
        &mut self,
        account_address: impl Into<String>,
        order: &OpenOrder,
        created_at_ms: u64,
    ) -> Option<u64> {
        let is_buy = match order.side.as_str() {
            "B" => true,
            "A" => false,
            _ => return None,
        };
        parse_positive_finite_number(&order.limit_px)?;
        parse_positive_finite_number(&order.sz)?;

        let pending_id = self.next_pending_order_change_id(created_at_ms);
        self.pending_order_changes.insert(
            pending_id,
            PendingOrderChangeEffect {
                account_address: account_address.into(),
                symbol: order.coin.clone(),
                oid: Some(order.oid),
                is_buy,
                size: order.sz.clone(),
                price: order.limit_px.clone(),
                kind: PendingOrderChangeKind::Cancelling,
                created_at_ms,
            },
        );
        Some(pending_id)
    }

    pub(crate) fn add_pending_order_modification(
        &mut self,
        account_address: impl Into<String>,
        order: &OpenOrder,
        new_price: impl Into<String>,
        created_at_ms: u64,
    ) -> Option<u64> {
        let is_buy = match order.side.as_str() {
            "B" => true,
            "A" => false,
            _ => return None,
        };
        parse_positive_finite_number(&order.sz)?;
        let new_price = new_price.into();
        parse_positive_finite_number(&new_price)?;

        let pending_id = self.next_pending_order_change_id(created_at_ms);
        self.pending_order_changes.insert(
            pending_id,
            PendingOrderChangeEffect {
                account_address: account_address.into(),
                symbol: order.coin.clone(),
                oid: Some(order.oid),
                is_buy,
                size: order.sz.clone(),
                price: new_price,
                kind: PendingOrderChangeKind::Modifying,
                created_at_ms,
            },
        );
        Some(pending_id)
    }

    pub(crate) fn clear_pending_order_change(&mut self, pending_id: Option<u64>) -> bool {
        pending_id
            .and_then(|pending_id| self.pending_order_changes.remove(&pending_id))
            .is_some()
    }

    fn next_pending_order_change_id(&self, seed: u64) -> u64 {
        let mut pending_id = seed;
        while self.pending_order_changes.contains_key(&pending_id) {
            pending_id = pending_id.saturating_add(1);
        }
        pending_id
    }

    pub(crate) fn apply_exchange_response(
        &mut self,
        context: OptimisticOrderContext,
        response: &ExchangeResponse,
        account_data: Option<&AccountData>,
        resolve_mid: impl Fn(&str) -> Option<f64>,
    ) -> bool {
        if response.is_error() || response.is_ambiguous_order_result() {
            return false;
        }

        let Some(status) = first_exchange_status(response) else {
            return false;
        };

        if let Some(resting) = status.get("resting") {
            let Some(oid) = resting.get("oid").and_then(|value| value.as_u64()) else {
                return false;
            };
            self.open_orders.insert(
                oid,
                OptimisticOpenOrder {
                    account_address: context.account_address.clone(),
                    order: optimistic_open_order(&context, oid),
                    created_at_ms: context.submitted_at_ms,
                },
            );
            return true;
        }

        if let Some(filled) = status.get("filled") {
            let Some(oid) = filled.get("oid").and_then(|value| value.as_u64()) else {
                return false;
            };
            let Some(total_size) = positive_wire_string(filled.get("totalSz")) else {
                return false;
            };
            let Some(avg_price) = positive_wire_string(filled.get("avgPx")) else {
                return false;
            };
            self.open_orders.remove(&oid);
            let total_size_value = parse_positive_finite_number(total_size.as_str());
            let avg_price_value = parse_positive_finite_number(avg_price.as_str());
            self.fills.insert(
                oid,
                OptimisticFill {
                    account_address: context.account_address.clone(),
                    fill: optimistic_fill(&context, oid, total_size, avg_price),
                    created_at_ms: context.submitted_at_ms,
                },
            );
            if let (Some(total_size), Some(avg_price)) = (total_size_value, avg_price_value) {
                let fill_delta = if context.is_buy {
                    total_size
                } else {
                    -total_size
                };
                let mut position_effect = OptimisticPositionEffect {
                    account_address: context.account_address.clone(),
                    symbol: context.symbol.clone(),
                    oid,
                    fill_delta,
                    avg_price,
                    expected_szi: None,
                    expected_entry_px: None,
                    created_at_ms: context.submitted_at_ms,
                };
                if let Some((expected_szi, expected_entry_px)) = self
                    .projected_position_after_effect(
                        account_data,
                        &context.account_address,
                        &position_effect,
                        &resolve_mid,
                    )
                {
                    position_effect.expected_szi = Some(expected_szi);
                    position_effect.expected_entry_px = expected_entry_px;
                }
                self.position_effects.insert(oid, position_effect);
            }
            return true;
        }

        false
    }

    pub(crate) fn reconcile_with_account_data(
        &mut self,
        account_data: &AccountData,
        now_ms: u64,
    ) -> bool {
        let open_oids: HashSet<u64> = account_data
            .open_orders
            .iter()
            .map(|order| order.oid)
            .collect();
        let fill_oids: HashSet<u64> = account_data
            .fills
            .iter()
            .filter_map(|fill| fill.oid)
            .collect();

        let before_open = self.open_orders.len();
        let before_fills = self.fills.len();
        let before_positions = self.position_effects.len();
        let before_pending = self.pending_order_changes.len();
        self.open_orders
            .retain(|oid, _| !open_oids.contains(oid) && !fill_oids.contains(oid));
        self.fills.retain(|oid, _| !fill_oids.contains(oid));
        self.reconcile_position_effects_with_authoritative_positions(account_data);
        self.reconcile_pending_order_changes_with_authoritative_orders(account_data);

        let removed_authoritative = before_open != self.open_orders.len()
            || before_fills != self.fills.len()
            || before_positions != self.position_effects.len()
            || before_pending != self.pending_order_changes.len();
        self.expire_stale(now_ms) || removed_authoritative
    }

    pub(crate) fn expire_stale(&mut self, now_ms: u64) -> bool {
        let before_open = self.open_orders.len();
        let before_fills = self.fills.len();
        let before_positions = self.position_effects.len();
        let before_pending = self.pending_order_changes.len();
        self.open_orders
            .retain(|_, order| effect_is_fresh(order.created_at_ms, now_ms));
        self.fills
            .retain(|_, fill| effect_is_fresh(fill.created_at_ms, now_ms));
        self.position_effects
            .retain(|_, effect| effect_is_fresh(effect.created_at_ms, now_ms));
        self.pending_order_changes
            .retain(|_, effect| effect_is_fresh(effect.created_at_ms, now_ms));
        before_open != self.open_orders.len()
            || before_fills != self.fills.len()
            || before_positions != self.position_effects.len()
            || before_pending != self.pending_order_changes.len()
    }

    fn project_open_orders<'a>(
        &'a self,
        account_data: Option<&'a AccountData>,
        account_address: Option<&str>,
    ) -> Vec<ProjectedOpenOrder<'a>> {
        let mut rows = Vec::new();
        let mut authoritative_oids = HashSet::new();
        let mut fill_oids = HashSet::new();

        if let Some(data) = account_data {
            rows.extend(data.open_orders.iter().map(|order| {
                authoritative_oids.insert(order.oid);
                ProjectedOpenOrder {
                    order,
                    is_optimistic: false,
                }
            }));
            fill_oids.extend(data.fills.iter().filter_map(|fill| fill.oid));
        }

        rows.extend(self.open_orders.iter().filter_map(|(oid, optimistic)| {
            (account_address == Some(optimistic.account_address.as_str())
                && !authoritative_oids.contains(oid)
                && !fill_oids.contains(oid))
            .then_some(ProjectedOpenOrder {
                order: &optimistic.order,
                is_optimistic: true,
            })
        }));
        rows
    }

    fn project_fills<'a>(
        &'a self,
        account_data: Option<&'a AccountData>,
        account_address: Option<&str>,
    ) -> Vec<ProjectedUserFill<'a>> {
        let mut rows = Vec::new();
        let mut authoritative_oids = HashSet::new();

        if let Some(data) = account_data {
            authoritative_oids.extend(data.fills.iter().filter_map(|fill| fill.oid));
        }

        rows.extend(self.fills.iter().filter_map(|(oid, optimistic)| {
            (account_address == Some(optimistic.account_address.as_str())
                && !authoritative_oids.contains(oid))
            .then_some(ProjectedUserFill {
                fill: &optimistic.fill,
                is_optimistic: true,
            })
        }));

        if let Some(data) = account_data {
            rows.extend(data.fills.iter().map(|fill| ProjectedUserFill {
                fill,
                is_optimistic: false,
            }));
        }
        rows
    }

    fn project_positions(
        &self,
        account_data: Option<&AccountData>,
        account_address: Option<&str>,
        resolve_mid: impl Fn(&str) -> Option<f64>,
        resolve_outcome_balance_coin: impl Fn(&str) -> Option<String>,
    ) -> Vec<ProjectedAssetPosition> {
        let mut positions = Vec::new();
        if let Some(data) = account_data {
            positions.extend(
                data.clearinghouse
                    .asset_positions
                    .iter()
                    .map(|asset_position| ProjectedAssetPosition {
                        asset_position: asset_position.clone(),
                        is_optimistic: false,
                    }),
            );
            positions.extend(data.spot.balances.iter().filter_map(|balance| {
                let trade_coin = resolve_outcome_balance_coin(&balance.coin)?;
                let mark_px = resolve_mid(&trade_coin);
                outcome_asset_position_from_balance(balance, trade_coin, mark_px).map(
                    |asset_position| ProjectedAssetPosition {
                        asset_position,
                        is_optimistic: false,
                    },
                )
            }));
        }

        let Some(account_address) = account_address else {
            return positions;
        };

        let mut effects: Vec<&OptimisticPositionEffect> = self
            .position_effects
            .values()
            .filter(|effect| effect.account_address == account_address)
            .collect();
        effects.sort_by_key(|effect| (effect.created_at_ms, effect.oid));

        for effect in effects {
            apply_position_effect_to_rows(&mut positions, effect, &resolve_mid);
        }

        positions
    }

    fn project_pending_order_changes(
        &self,
        account_address: Option<&str>,
    ) -> Vec<ProjectedPendingOrderChange> {
        let Some(account_address) = account_address else {
            return Vec::new();
        };

        self.pending_order_changes
            .iter()
            .filter_map(|(pending_id, effect)| {
                (effect.account_address == account_address).then_some(ProjectedPendingOrderChange {
                    pending_id: *pending_id,
                    symbol: effect.symbol.clone(),
                    oid: effect.oid,
                    is_buy: effect.is_buy,
                    size: effect.size.clone(),
                    price: effect.price.clone(),
                    kind: effect.kind,
                })
            })
            .collect()
    }

    fn projected_position_after_effect(
        &self,
        account_data: Option<&AccountData>,
        account_address: &str,
        pending_effect: &OptimisticPositionEffect,
        resolve_mid: impl Fn(&str) -> Option<f64>,
    ) -> Option<(f64, Option<f64>)> {
        let mut position = account_data
            .and_then(|data| {
                data.clearinghouse
                    .asset_positions
                    .iter()
                    .find(|asset_position| asset_position.position.coin == pending_effect.symbol)
            })
            .cloned();

        let mut effects: Vec<&OptimisticPositionEffect> = self
            .position_effects
            .values()
            .filter(|effect| {
                effect.account_address == account_address && effect.symbol == pending_effect.symbol
            })
            .collect();
        effects.push(pending_effect);
        effects.sort_by_key(|effect| (effect.created_at_ms, effect.oid));

        for effect in effects {
            position = apply_position_effect(position, effect, resolve_mid(effect.symbol.as_str()));
        }

        let Some(position) = position else {
            return Some((0.0, None));
        };
        Some((
            parse_finite_number(&position.position.szi)?,
            Some(parse_positive_finite_number(&position.position.entry_px)?),
        ))
    }

    fn reconcile_position_effects_with_authoritative_positions(
        &mut self,
        account_data: &AccountData,
    ) {
        let mut reconciled_keys = HashSet::new();
        let mut effects: Vec<&OptimisticPositionEffect> = self.position_effects.values().collect();
        effects.sort_by_key(|effect| (effect.created_at_ms, effect.oid));

        for effect in effects {
            if !position_effect_matches_authoritative(effect, account_data) {
                continue;
            }
            for earlier in self.position_effects.values().filter(|earlier| {
                earlier.account_address == effect.account_address
                    && earlier.symbol == effect.symbol
                    && (earlier.created_at_ms, earlier.oid) <= (effect.created_at_ms, effect.oid)
            }) {
                reconciled_keys.insert(earlier.oid);
            }
        }

        self.position_effects
            .retain(|oid, _| !reconciled_keys.contains(oid));
    }

    fn reconcile_pending_order_changes_with_authoritative_orders(
        &mut self,
        account_data: &AccountData,
    ) {
        let open_oids: HashSet<u64> = account_data
            .open_orders
            .iter()
            .map(|order| order.oid)
            .collect();

        self.pending_order_changes
            .retain(|_, effect| match effect.kind {
                PendingOrderChangeKind::Placing => {
                    !authoritative_order_matches_pending_place(effect, account_data)
                        && !authoritative_fill_matches_pending_place(effect, account_data)
                }
                PendingOrderChangeKind::Cancelling => {
                    effect.oid.is_none_or(|oid| open_oids.contains(&oid))
                }
                PendingOrderChangeKind::Modifying => effect.oid.is_some_and(|oid| {
                    account_data.open_orders.iter().any(|order| {
                        order.oid == oid
                            && !authoritative_order_matches_pending_change(effect, order)
                    })
                }),
            });
    }
}

impl TradingTerminal {
    pub(crate) fn optimistic_order_context_matches_current_account(
        &self,
        context: &OptimisticOrderContext,
    ) -> bool {
        self.connected_address.as_deref() == Some(context.account_address.as_str())
    }

    pub(crate) fn apply_optimistic_order_result(
        &mut self,
        context: OptimisticOrderContext,
        response: &ExchangeResponse,
    ) -> bool {
        if !self.optimistic_order_context_matches_current_account(&context) {
            return false;
        }
        let changed = self.optimistic_account.apply_exchange_response(
            context,
            response,
            self.account_data.as_ref(),
            |_| None,
        );
        if changed {
            self.sync_all_chart_overlays();
        }
        changed
    }

    pub(crate) fn expire_optimistic_account_effects(&mut self) -> bool {
        let changed = self.optimistic_account.expire_stale(Self::now_ms());
        if changed {
            self.sync_all_chart_overlays();
        }
        changed
    }

    pub(crate) fn add_pending_order_submission(&mut self, context: &mut OptimisticOrderContext) {
        self.optimistic_account.add_pending_order_placement(context);
        self.sync_all_chart_orders();
    }

    pub(crate) fn add_pending_order_cancellation(
        &mut self,
        account_address: &str,
        order: &OpenOrder,
    ) -> Option<u64> {
        let pending_id = self.optimistic_account.add_pending_order_cancellation(
            account_address.to_string(),
            order,
            Self::now_ms(),
        );
        if pending_id.is_some() {
            self.sync_all_chart_orders();
        }
        pending_id
    }

    pub(crate) fn add_pending_order_modification(
        &mut self,
        account_address: &str,
        order: &OpenOrder,
        new_price: String,
    ) -> Option<u64> {
        let pending_id = self.optimistic_account.add_pending_order_modification(
            account_address.to_string(),
            order,
            new_price,
            Self::now_ms(),
        );
        if pending_id.is_some() {
            self.sync_all_chart_orders();
        }
        pending_id
    }

    pub(crate) fn clear_pending_order_change(&mut self, pending_id: Option<u64>) -> bool {
        let changed = self
            .optimistic_account
            .clear_pending_order_change(pending_id);
        if changed {
            self.sync_all_chart_orders();
        }
        changed
    }

    pub(crate) fn has_pending_order_changes(&self) -> bool {
        !self
            .optimistic_account
            .project_pending_order_changes(self.connected_address.as_deref())
            .is_empty()
    }

    pub(crate) fn projected_open_orders(&self) -> Vec<ProjectedOpenOrder<'_>> {
        self.optimistic_account.project_open_orders(
            self.account_data.as_ref(),
            self.connected_address.as_deref(),
        )
    }

    pub(crate) fn projected_user_fills(&self) -> Vec<ProjectedUserFill<'_>> {
        self.optimistic_account.project_fills(
            self.account_data.as_ref(),
            self.connected_address.as_deref(),
        )
    }

    pub(crate) fn projected_positions(&self) -> Vec<ProjectedAssetPosition> {
        self.optimistic_account.project_positions(
            self.account_data.as_ref(),
            self.connected_address.as_deref(),
            |symbol| self.resolve_mid_for_symbol(symbol),
            |coin| self.outcome_trade_coin_for_balance_coin(coin),
        )
    }

    pub(crate) fn projected_pending_order_changes(&self) -> Vec<ProjectedPendingOrderChange> {
        self.optimistic_account
            .project_pending_order_changes(self.connected_address.as_deref())
    }

    pub(crate) fn merged_open_orders(&self) -> Vec<OpenOrder> {
        self.projected_open_orders()
            .into_iter()
            .map(|row| row.order.clone())
            .collect()
    }

    pub(crate) fn merged_user_fills(&self) -> Vec<UserFill> {
        self.projected_user_fills()
            .into_iter()
            .map(|row| row.fill.clone())
            .collect()
    }
}

fn first_exchange_status(response: &ExchangeResponse) -> Option<&Value> {
    response.response.as_ref()?.data.as_ref()?.statuses.first()
}

fn optimistic_open_order(context: &OptimisticOrderContext, oid: u64) -> OpenOrder {
    OpenOrder {
        coin: context.symbol.clone(),
        side: order_side(context.is_buy).to_string(),
        limit_px: context.price.clone(),
        sz: context.size.clone(),
        oid,
        timestamp: context.submitted_at_ms,
        reduce_only: Some(context.reduce_only),
    }
}

fn optimistic_fill(
    context: &OptimisticOrderContext,
    oid: u64,
    total_size: String,
    avg_price: String,
) -> UserFill {
    UserFill {
        coin: context.symbol.clone(),
        px: avg_price,
        sz: total_size,
        side: order_side(context.is_buy).to_string(),
        time: context.submitted_at_ms,
        oid: Some(oid),
        dir: "Pending".to_string(),
        closed_pnl: String::new(),
        fee: String::new(),
    }
}

fn order_side(is_buy: bool) -> &'static str {
    if is_buy { "B" } else { "A" }
}

fn positive_wire_string(value: Option<&Value>) -> Option<String> {
    let raw = value?.as_str()?;
    parse_positive_finite_number(raw).map(|_| raw.to_string())
}

fn apply_position_effect_to_rows(
    positions: &mut Vec<ProjectedAssetPosition>,
    effect: &OptimisticPositionEffect,
    resolve_mid: impl Fn(&str) -> Option<f64>,
) {
    let existing_index = positions
        .iter()
        .position(|row| row.asset_position.position.coin == effect.symbol);
    let existing = existing_index.map(|index| positions.remove(index).asset_position);
    if let Some(asset_position) =
        apply_position_effect(existing, effect, resolve_mid(&effect.symbol))
    {
        positions.push(ProjectedAssetPosition {
            asset_position,
            is_optimistic: true,
        });
    }
}

fn apply_position_effect(
    asset_position: Option<AssetPosition>,
    effect: &OptimisticPositionEffect,
    mark_px: Option<f64>,
) -> Option<AssetPosition> {
    let current_szi = asset_position
        .as_ref()
        .and_then(|position| parse_finite_number(&position.position.szi))
        .unwrap_or(0.0);
    let current_entry_px = asset_position
        .as_ref()
        .and_then(|position| parse_positive_finite_number(&position.position.entry_px))
        .unwrap_or(effect.avg_price);
    let projected_szi = current_szi + effect.fill_delta;
    if projected_szi.abs() <= POSITION_EPSILON {
        return None;
    }

    let projected_entry_px = projected_entry_price(
        current_szi,
        current_entry_px,
        effect.fill_delta,
        effect.avg_price,
        projected_szi,
    );
    let projected_mark_px = mark_px.unwrap_or(projected_entry_px);
    let projected_value = projected_szi.abs() * projected_mark_px;
    let projected_upnl = projected_szi * (projected_mark_px - projected_entry_px);

    let mut asset_position =
        asset_position.unwrap_or_else(|| optimistic_asset_position(&effect.symbol));
    asset_position.position.szi = float_to_wire(projected_szi);
    asset_position.position.entry_px = float_to_wire(projected_entry_px);
    asset_position.position.position_value = float_to_wire(projected_value);
    asset_position.position.unrealized_pnl = float_to_wire(projected_upnl);
    asset_position.position.liquidation_px = None;
    asset_position.position.margin_used.clear();
    asset_position.position.cum_funding = None;
    asset_position.position.leverage = pending_position_leverage();
    asset_position.liquidation_px = None;
    Some(asset_position)
}

fn optimistic_asset_position(symbol: &str) -> AssetPosition {
    AssetPosition {
        position: Position {
            coin: symbol.to_string(),
            szi: String::new(),
            entry_px: String::new(),
            position_value: String::new(),
            unrealized_pnl: String::new(),
            liquidation_px: None,
            leverage: pending_position_leverage(),
            margin_used: String::new(),
            cum_funding: None,
        },
        liquidation_px: None,
    }
}

fn outcome_asset_position_from_balance(
    balance: &SpotBalance,
    trade_coin: String,
    mark_px: Option<f64>,
) -> Option<AssetPosition> {
    let total = parse_finite_number(&balance.total)?;
    if total.abs() <= POSITION_EPSILON {
        return None;
    }

    let size = total.abs();
    let entry_notional = parse_finite_number(&balance.entry_ntl).unwrap_or(0.0).abs();
    let entry_px = if entry_notional > POSITION_EPSILON {
        entry_notional / size
    } else {
        mark_px.unwrap_or(0.0)
    };
    let position_value = mark_px
        .map(|mark_px| size * mark_px)
        .or_else(|| (entry_notional > POSITION_EPSILON).then_some(entry_notional))
        .unwrap_or(0.0);
    let unrealized_pnl = position_value - entry_notional;

    Some(AssetPosition {
        position: Position {
            coin: trade_coin,
            szi: float_to_wire(total),
            entry_px: float_to_wire(entry_px),
            position_value: float_to_wire(position_value),
            unrealized_pnl: float_to_wire(unrealized_pnl),
            liquidation_px: None,
            leverage: PositionLeverage {
                leverage_type: "outcome".to_string(),
                value: 1,
            },
            margin_used: String::new(),
            cum_funding: None,
        },
        liquidation_px: None,
    })
}

fn pending_position_leverage() -> PositionLeverage {
    PositionLeverage {
        leverage_type: "pending".to_string(),
        value: 0,
    }
}

fn projected_entry_price(
    current_szi: f64,
    current_entry_px: f64,
    fill_delta: f64,
    fill_px: f64,
    projected_szi: f64,
) -> f64 {
    if current_szi.abs() <= POSITION_EPSILON || current_szi.signum() != projected_szi.signum() {
        return fill_px;
    }

    if current_szi.signum() == fill_delta.signum() {
        let current_abs = current_szi.abs();
        let fill_abs = fill_delta.abs();
        return (current_abs * current_entry_px + fill_abs * fill_px) / (current_abs + fill_abs);
    }

    current_entry_px
}

fn position_effect_matches_authoritative(
    effect: &OptimisticPositionEffect,
    account_data: &AccountData,
) -> bool {
    let Some(expected_szi) = effect.expected_szi else {
        return false;
    };
    if expected_szi.abs() <= POSITION_EPSILON {
        let authoritative = account_data
            .clearinghouse
            .asset_positions
            .iter()
            .find(|position| position.position.coin == effect.symbol);
        return authoritative.is_none_or(|position| {
            parse_finite_number(&position.position.szi)
                .is_some_and(|szi| szi.abs() <= POSITION_EPSILON)
        });
    }

    let Some(authoritative) = account_data
        .clearinghouse
        .asset_positions
        .iter()
        .find(|position| position.position.coin == effect.symbol)
    else {
        return false;
    };
    let Some(authoritative_szi) = parse_finite_number(&authoritative.position.szi) else {
        return false;
    };
    // Position size is the canonical confirmation that a fill has reached the
    // account snapshot. Entry price can differ from our estimate because the
    // exchange rounds/account-adjusts it; requiring an exact entry match leaves
    // stale effects that get applied again on top of the authoritative row.
    nearly_equal(authoritative_szi, expected_szi)
}

fn authoritative_order_matches_pending_place(
    effect: &PendingOrderChangeEffect,
    account_data: &AccountData,
) -> bool {
    account_data
        .open_orders
        .iter()
        .any(|order| authoritative_order_matches_pending_change(effect, order))
}

fn authoritative_order_matches_pending_change(
    effect: &PendingOrderChangeEffect,
    order: &OpenOrder,
) -> bool {
    order.coin == effect.symbol
        && order.side == order_side(effect.is_buy)
        && order.limit_px == effect.price
        && order.sz == effect.size
}

fn authoritative_fill_matches_pending_place(
    effect: &PendingOrderChangeEffect,
    account_data: &AccountData,
) -> bool {
    account_data.fills.iter().any(|fill| {
        fill.coin == effect.symbol
            && fill.side == order_side(effect.is_buy)
            && fill.sz == effect.size
    })
}

fn nearly_equal(left: f64, right: f64) -> bool {
    let tolerance = POSITION_EPSILON.max((left.abs() + right.abs()) * 1e-10);
    (left - right).abs() <= tolerance
}

fn effect_is_fresh(created_at_ms: u64, now_ms: u64) -> bool {
    now_ms.saturating_sub(created_at_ms) <= OPTIMISTIC_EFFECT_TTL_MS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountDataCompleteness, ClearinghouseState, MarginSummary, Position, PositionLeverage,
        SpotClearinghouseState,
    };
    use crate::app_state::TradingTerminal;
    use crate::chart::OrderOverlayPendingState;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    fn context() -> OptimisticOrderContext {
        OptimisticOrderContext {
            account_address: "0xabc0000000000000000000000000000000000000".to_string(),
            symbol: "BTC".to_string(),
            is_buy: true,
            size: "0.1".to_string(),
            price: "100".to_string(),
            order_kind: OrderKind::Limit,
            reduce_only: false,
            submitted_at_ms: 1_000,
            pending_id: None,
            source: OptimisticOrderSource::OrderForm,
        }
    }

    fn context_for_account(account_address: &str) -> OptimisticOrderContext {
        OptimisticOrderContext {
            account_address: account_address.to_string(),
            ..context()
        }
    }

    fn context_for_side(is_buy: bool) -> OptimisticOrderContext {
        OptimisticOrderContext {
            is_buy,
            ..context()
        }
    }

    fn exchange_response(status: serde_json::Value) -> ExchangeResponse {
        serde_json::from_value(serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [status]
                }
            }
        }))
        .expect("test exchange response should deserialize")
    }

    fn filled_response(oid: u64, total_size: &str, avg_price: &str) -> ExchangeResponse {
        exchange_response(serde_json::json!({
            "filled": {
                "totalSz": total_size,
                "avgPx": avg_price,
                "oid": oid
            }
        }))
    }

    fn apply_response(
        effects: &mut OptimisticAccountEffects,
        context: OptimisticOrderContext,
        response: &ExchangeResponse,
    ) -> bool {
        effects.apply_exchange_response(context, response, None, |_| None)
    }

    fn account_data(open_orders: Vec<OpenOrder>, fills: Vec<UserFill>) -> AccountData {
        AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "0".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
                asset_positions: Vec::new(),
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders,
            fills,
            funding_history: Vec::new(),
            fee_rates: Default::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: 1_000,
        }
    }

    fn account_data_with_positions(positions: Vec<AssetPosition>) -> AccountData {
        let mut data = account_data(Vec::new(), Vec::new());
        data.clearinghouse.asset_positions = positions;
        data
    }

    fn account_data_with_spot_balances(balances: Vec<SpotBalance>) -> AccountData {
        let mut data = account_data(Vec::new(), Vec::new());
        data.spot.balances = balances;
        data
    }

    fn spot_balance(coin: &str, total: &str, entry_ntl: &str) -> SpotBalance {
        SpotBalance {
            coin: coin.to_string(),
            token: None,
            total: total.to_string(),
            hold: "0".to_string(),
            entry_ntl: entry_ntl.to_string(),
            supplied: None,
        }
    }

    fn authoritative_position(coin: &str, szi: &str, entry_px: &str) -> AssetPosition {
        AssetPosition {
            position: Position {
                coin: coin.to_string(),
                szi: szi.to_string(),
                entry_px: entry_px.to_string(),
                position_value: "0".to_string(),
                unrealized_pnl: "0".to_string(),
                liquidation_px: Some("50".to_string()),
                leverage: PositionLeverage {
                    leverage_type: "cross".to_string(),
                    value: 10,
                },
                margin_used: "0".to_string(),
                cum_funding: None,
            },
            liquidation_px: Some("50".to_string()),
        }
    }

    fn authoritative_open_order(oid: u64) -> OpenOrder {
        OpenOrder {
            coin: "BTC".to_string(),
            side: "B".to_string(),
            limit_px: "100".to_string(),
            sz: "0.1".to_string(),
            oid,
            timestamp: 1_001,
            reduce_only: Some(false),
        }
    }

    fn authoritative_fill(oid: u64) -> UserFill {
        UserFill {
            coin: "BTC".to_string(),
            px: "100".to_string(),
            sz: "0.1".to_string(),
            side: "B".to_string(),
            time: 1_001,
            oid: Some(oid),
            dir: "Open Long".to_string(),
            closed_pnl: "0".to_string(),
            fee: "0.01".to_string(),
        }
    }

    fn projected_position_values(row: &ProjectedAssetPosition) -> (f64, f64) {
        (
            parse_finite_number(&row.asset_position.position.szi)
                .expect("projected size should parse"),
            parse_positive_finite_number(&row.asset_position.position.entry_px)
                .expect("projected entry should parse"),
        )
    }

    fn assert_near(left: f64, right: f64) {
        assert!(
            nearly_equal(left, right),
            "expected {left} to be nearly equal to {right}"
        );
    }

    fn terminal_with_btc_chart() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(context().account_address);
        terminal.charts.clear();
        terminal
            .charts
            .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
        terminal
    }

    #[test]
    fn pending_order_submission_draws_pending_chart_overlay() {
        let mut terminal = terminal_with_btc_chart();
        let mut context = context();

        terminal.add_pending_order_submission(&mut context);

        assert!(context.pending_id.is_some());
        assert_eq!(terminal.optimistic_account.pending_order_change_count(), 1);
        let order = terminal
            .charts
            .get(&1)
            .expect("chart should exist")
            .chart
            .active_orders
            .first()
            .expect("pending order should render");
        assert_eq!(order.coin, "BTC");
        assert_eq!(order.limit_px, 100.0);
        assert_eq!(order.sz, 0.1);
        assert_eq!(order.pending_state, Some(OrderOverlayPendingState::Placing));
    }

    #[test]
    fn pending_order_cancellation_marks_existing_chart_overlay() {
        let mut terminal = terminal_with_btc_chart();
        let order = authoritative_open_order(42);
        terminal.account_data = Some(account_data(vec![order.clone()], Vec::new()));
        terminal.sync_all_chart_orders();

        let pending_id =
            terminal.add_pending_order_cancellation(&context().account_address, &order);

        assert!(pending_id.is_some());
        assert_eq!(terminal.optimistic_account.pending_order_change_count(), 1);
        let order = terminal
            .charts
            .get(&1)
            .expect("chart should exist")
            .chart
            .active_orders
            .first()
            .expect("order should render");
        assert_eq!(order.oid, 42);
        assert_eq!(
            order.pending_state,
            Some(OrderOverlayPendingState::Cancelling)
        );
    }

    #[test]
    fn pending_order_modification_moves_existing_chart_overlay() {
        let mut terminal = terminal_with_btc_chart();
        let order = authoritative_open_order(42);
        terminal.account_data = Some(account_data(vec![order.clone()], Vec::new()));
        terminal.sync_all_chart_orders();

        let pending_id = terminal.add_pending_order_modification(
            &context().account_address,
            &order,
            "101".to_string(),
        );

        assert!(pending_id.is_some());
        assert_eq!(terminal.optimistic_account.pending_order_change_count(), 1);
        let order = terminal
            .charts
            .get(&1)
            .expect("chart should exist")
            .chart
            .active_orders
            .first()
            .expect("order should render");
        assert_eq!(order.oid, 42);
        assert_eq!(order.limit_px, 101.0);
        assert_eq!(order.sz, 0.1);
        assert_eq!(
            order.pending_state,
            Some(OrderOverlayPendingState::Modifying)
        );
    }

    #[test]
    fn authoritative_modified_order_reconciles_pending_modification() {
        let mut effects = OptimisticAccountEffects::default();
        let order = authoritative_open_order(42);
        let mut modified_order = order.clone();
        modified_order.limit_px = "101".to_string();

        assert!(
            effects
                .add_pending_order_modification(
                    context().account_address,
                    &order,
                    "101".to_string(),
                    1_000,
                )
                .is_some()
        );

        assert!(
            effects.reconcile_with_account_data(
                &account_data(vec![modified_order], Vec::new()),
                1_001,
            )
        );
        assert_eq!(effects.pending_order_change_count(), 0);
    }

    #[test]
    fn quick_order_success_clears_pending_submission() {
        let mut terminal = terminal_with_btc_chart();
        let mut context = OptimisticOrderContext {
            source: OptimisticOrderSource::QuickOrder { chart_id: 1 },
            ..context()
        };
        terminal.add_pending_order_submission(&mut context);
        let response = exchange_response(serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }));

        let _task = terminal.handle_quick_order_result(OrderSubmissionResult {
            context,
            result: Ok(response),
        });

        assert_eq!(terminal.optimistic_account.pending_order_change_count(), 0);
        let chart = &terminal.charts.get(&1).expect("chart should exist").chart;
        assert_eq!(chart.active_orders.len(), 1);
        assert_eq!(chart.active_orders[0].oid, 42);
        assert_eq!(chart.active_orders[0].pending_state, None);
    }

    #[test]
    fn resting_response_creates_optimistic_open_order() {
        let mut effects = OptimisticAccountEffects::default();
        let response = exchange_response(serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }));

        assert!(apply_response(&mut effects, context(), &response));

        assert_eq!(effects.open_order_count(), 1);
        assert_eq!(effects.fill_count(), 0);
        let projected = effects.project_open_orders(None, Some(context().account_address.as_str()));
        assert_eq!(projected[0].order.oid, 42);
        assert_eq!(projected[0].order.coin, "BTC");
        assert!(projected[0].is_optimistic);
    }

    #[test]
    fn resting_response_updates_chart_order_overlay_immediately() {
        let mut terminal = terminal_with_btc_chart();
        let response = exchange_response(serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }));

        assert!(terminal.apply_optimistic_order_result(context(), &response));

        let chart = &terminal.charts.get(&1).expect("chart should exist").chart;
        assert_eq!(chart.active_orders.len(), 1);
        assert_eq!(chart.active_orders[0].oid, 42);
        assert_eq!(chart.active_orders[0].limit_px, 100.0);
    }

    #[test]
    fn filled_response_creates_optimistic_fill() {
        let mut effects = OptimisticAccountEffects::default();
        let response = filled_response(43, "0.1", "100");

        assert!(apply_response(&mut effects, context(), &response));

        assert_eq!(effects.open_order_count(), 0);
        assert_eq!(effects.fill_count(), 1);
        assert_eq!(effects.position_effect_count(), 1);
        let projected = effects.project_fills(None, Some(context().account_address.as_str()));
        assert_eq!(projected[0].fill.oid, Some(43));
        assert_eq!(projected[0].fill.dir, "Pending");
        assert!(projected[0].is_optimistic);
    }

    #[test]
    fn filled_response_projects_new_long_position_from_flat() {
        let mut effects = OptimisticAccountEffects::default();
        let response = filled_response(43, "0.1", "100");

        assert!(apply_response(&mut effects, context(), &response));

        let projected = effects.project_positions(
            None,
            Some(context().account_address.as_str()),
            |_| Some(110.0),
            |_| None,
        );
        assert_eq!(projected.len(), 1);
        assert!(projected[0].is_optimistic);
        let (szi, entry_px) = projected_position_values(&projected[0]);
        assert_near(szi, 0.1);
        assert_near(entry_px, 100.0);
        assert_eq!(projected[0].asset_position.position.position_value, "11");
        assert_eq!(projected[0].asset_position.position.unrealized_pnl, "1");
        assert_eq!(projected[0].asset_position.position.liquidation_px, None);
    }

    #[test]
    fn outcome_spot_balances_project_as_positions() {
        let effects = OptimisticAccountEffects::default();
        let data = account_data_with_spot_balances(vec![spot_balance("+950", "30", "15")]);

        let projected = effects.project_positions(
            Some(&data),
            Some(context().account_address.as_str()),
            |_| Some(0.6),
            |coin| TradingTerminal::outcome_balance_coin_to_trade_coin(coin),
        );

        assert_eq!(projected.len(), 1);
        assert!(!projected[0].is_optimistic);
        assert_eq!(projected[0].asset_position.position.coin, "#950");
        assert_eq!(projected[0].asset_position.position.szi, "30");
        assert_eq!(projected[0].asset_position.position.entry_px, "0.5");
        assert_eq!(projected[0].asset_position.position.position_value, "18");
        assert_eq!(projected[0].asset_position.position.unrealized_pnl, "3");
    }

    #[test]
    fn filled_response_adds_to_existing_position_with_weighted_entry() {
        let mut effects = OptimisticAccountEffects::default();
        let data = account_data_with_positions(vec![authoritative_position("BTC", "1", "90")]);
        let response = filled_response(43, "1", "110");

        assert!(
            effects.apply_exchange_response(context(), &response, Some(&data), |_| Some(120.0))
        );

        let projected = effects.project_positions(
            Some(&data),
            Some(context().account_address.as_str()),
            |_| Some(120.0),
            |_| None,
        );
        let (szi, entry_px) = projected_position_values(&projected[0]);
        assert_near(szi, 2.0);
        assert_near(entry_px, 100.0);
        assert!(projected[0].is_optimistic);
    }

    #[test]
    fn filled_response_reduces_existing_position_without_changing_entry() {
        let mut effects = OptimisticAccountEffects::default();
        let data = account_data_with_positions(vec![authoritative_position("BTC", "2", "90")]);
        let response = filled_response(43, "0.5", "110");

        assert!(effects.apply_exchange_response(
            context_for_side(false),
            &response,
            Some(&data),
            |_| Some(120.0)
        ));

        let projected = effects.project_positions(
            Some(&data),
            Some(context().account_address.as_str()),
            |_| Some(120.0),
            |_| None,
        );
        let (szi, entry_px) = projected_position_values(&projected[0]);
        assert_near(szi, 1.5);
        assert_near(entry_px, 90.0);
    }

    #[test]
    fn filled_response_closes_position_and_suppresses_row() {
        let mut effects = OptimisticAccountEffects::default();
        let data = account_data_with_positions(vec![authoritative_position("BTC", "1", "90")]);
        let response = filled_response(43, "1", "110");

        assert!(effects.apply_exchange_response(
            context_for_side(false),
            &response,
            Some(&data),
            |_| Some(120.0)
        ));

        let projected = effects.project_positions(
            Some(&data),
            Some(context().account_address.as_str()),
            |_| Some(120.0),
            |_| None,
        );
        assert!(projected.is_empty());
    }

    #[test]
    fn filled_response_flips_position_entry_to_fill_price() {
        let mut effects = OptimisticAccountEffects::default();
        let data = account_data_with_positions(vec![authoritative_position("BTC", "1", "90")]);
        let response = filled_response(43, "2", "110");

        assert!(effects.apply_exchange_response(
            context_for_side(false),
            &response,
            Some(&data),
            |_| Some(120.0)
        ));

        let projected = effects.project_positions(
            Some(&data),
            Some(context().account_address.as_str()),
            |_| Some(120.0),
            |_| None,
        );
        let (szi, entry_px) = projected_position_values(&projected[0]);
        assert_near(szi, -1.0);
        assert_near(entry_px, 110.0);
    }

    #[test]
    fn optimistic_positions_project_only_for_matching_account() {
        let mut effects = OptimisticAccountEffects::default();
        let response = filled_response(43, "0.1", "100");
        assert!(apply_response(&mut effects, context(), &response));

        assert!(
            effects
                .project_positions(
                    None,
                    Some("0xdef0000000000000000000000000000000000000"),
                    |_| Some(110.0),
                    |_| None,
                )
                .is_empty()
        );
        assert_eq!(
            effects
                .project_positions(
                    None,
                    Some(context().account_address.as_str()),
                    |_| Some(110.0),
                    |_| None,
                )
                .len(),
            1
        );
    }

    #[test]
    fn authoritative_position_reconciles_optimistic_position_effect() {
        let mut effects = OptimisticAccountEffects::default();
        let data_before =
            account_data_with_positions(vec![authoritative_position("BTC", "1", "90")]);
        let response = filled_response(43, "1", "110");
        assert!(
            effects
                .apply_exchange_response(context(), &response, Some(&data_before), |_| Some(120.0))
        );

        let data_after =
            account_data_with_positions(vec![authoritative_position("BTC", "2", "100")]);
        assert!(effects.reconcile_with_account_data(&data_after, 1_001));
        assert_eq!(effects.position_effect_count(), 0);
    }

    #[test]
    fn authoritative_position_size_reconciles_even_when_entry_differs() {
        let mut effects = OptimisticAccountEffects::default();
        let data_before =
            account_data_with_positions(vec![authoritative_position("BTC", "1", "90")]);
        let response = filled_response(43, "1", "110");
        assert!(
            effects
                .apply_exchange_response(context(), &response, Some(&data_before), |_| Some(120.0))
        );

        let data_after =
            account_data_with_positions(vec![authoritative_position("BTC", "2", "100.05")]);
        assert!(effects.reconcile_with_account_data(&data_after, 1_001));
        assert_eq!(effects.position_effect_count(), 0);

        let projected = effects.project_positions(
            Some(&data_after),
            Some(context().account_address.as_str()),
            |_| Some(120.0),
            |_| None,
        );
        let (szi, entry_px) = projected_position_values(&projected[0]);
        assert_near(szi, 2.0);
        assert_near(entry_px, 100.05);
        assert!(!projected[0].is_optimistic);
    }

    #[test]
    fn authoritative_fill_without_position_match_keeps_optimistic_position_effect() {
        let mut effects = OptimisticAccountEffects::default();
        let response = filled_response(43, "0.1", "100");
        assert!(apply_response(&mut effects, context(), &response));

        let data = account_data(Vec::new(), vec![authoritative_fill(43)]);
        assert!(effects.reconcile_with_account_data(&data, 1_001));

        assert_eq!(effects.fill_count(), 0);
        assert_eq!(effects.position_effect_count(), 1);
        assert_eq!(
            effects
                .project_positions(
                    None,
                    Some(context().account_address.as_str()),
                    |_| Some(110.0),
                    |_| None,
                )
                .len(),
            1
        );
    }

    #[test]
    fn filled_response_updates_chart_position_overlay_immediately() {
        let mut terminal = terminal_with_btc_chart();
        let response = filled_response(43, "0.1", "100");

        assert!(terminal.apply_optimistic_order_result(context(), &response));

        let chart = &terminal.charts.get(&1).expect("chart should exist").chart;
        let position = chart
            .active_position
            .as_ref()
            .expect("position overlay should exist");
        assert_near(position.szi, 0.1);
        assert_near(position.entry_px, 100.0);
        assert_eq!(position.liquidation_px, None);
    }

    #[test]
    fn filled_response_updates_chart_trade_marker_immediately() {
        let mut terminal = terminal_with_btc_chart();
        let response = exchange_response(serde_json::json!({
            "filled": {
                "totalSz": "0.1",
                "avgPx": "100",
                "oid": 43_u64
            }
        }));

        assert!(terminal.apply_optimistic_order_result(context(), &response));

        let chart = &terminal.charts.get(&1).expect("chart should exist").chart;
        assert_eq!(chart.trade_markers.len(), 1);
        assert_eq!(chart.trade_markers[0].time_ms, 1_000);
        assert_eq!(chart.trade_markers[0].price, 100.0);
        assert!(chart.trade_markers[0].is_buy);
    }

    #[test]
    fn exchange_error_and_ambiguous_response_do_not_create_effects() {
        let mut effects = OptimisticAccountEffects::default();
        let error = exchange_response(serde_json::json!({
            "error": "Order rejected"
        }));
        let ambiguous = exchange_response(serde_json::json!({
            "resting": {}
        }));

        assert!(!apply_response(&mut effects, context(), &error));
        assert!(!apply_response(&mut effects, context(), &ambiguous));

        assert!(effects.is_empty());
    }

    #[test]
    fn optimistic_effects_project_only_for_matching_account() {
        let mut effects = OptimisticAccountEffects::default();
        let response = exchange_response(serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }));
        apply_response(&mut effects, context(), &response);

        assert_eq!(
            effects
                .project_open_orders(None, Some("0xdef0000000000000000000000000000000000000"))
                .len(),
            0
        );
        assert_eq!(
            effects
                .project_open_orders(None, Some(context().account_address.as_str()))
                .len(),
            1
        );
    }

    #[test]
    fn late_normal_order_result_after_account_switch_is_ignored() {
        let mut terminal = terminal_with_btc_chart();
        terminal.connected_address = Some("0xdef0000000000000000000000000000000000000".to_string());
        let response = exchange_response(serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }));

        let _task = terminal.handle_order_result(OrderSubmissionResult {
            context: context_for_account("0xabc0000000000000000000000000000000000000"),
            result: Ok(response),
        });

        assert!(terminal.optimistic_account.is_empty());
        assert_eq!(
            terminal
                .charts
                .get(&1)
                .expect("chart should exist")
                .chart
                .active_orders
                .len(),
            0
        );
        assert_eq!(terminal.order_status, None);
    }

    #[test]
    fn late_quick_order_result_after_disconnect_is_ignored() {
        let mut terminal = terminal_with_btc_chart();
        terminal.connected_address = None;
        let response = exchange_response(serde_json::json!({
            "filled": {
                "totalSz": "0.1",
                "avgPx": "100",
                "oid": 43_u64
            }
        }));

        let _task = terminal.handle_quick_order_result(OrderSubmissionResult {
            context: OptimisticOrderContext {
                source: OptimisticOrderSource::QuickOrder { chart_id: 1 },
                ..context_for_account("0xabc0000000000000000000000000000000000000")
            },
            result: Ok(response),
        });

        assert!(terminal.optimistic_account.is_empty());
        assert_eq!(
            terminal
                .charts
                .get(&1)
                .expect("chart should exist")
                .chart
                .trade_markers
                .len(),
            0
        );
        assert_eq!(terminal.order_status, None);
    }

    #[test]
    fn authoritative_open_order_reconciles_optimistic_open_order() {
        let mut effects = OptimisticAccountEffects::default();
        let response = exchange_response(serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }));
        apply_response(&mut effects, context(), &response);

        let data = account_data(vec![authoritative_open_order(42)], Vec::new());

        assert!(effects.reconcile_with_account_data(&data, 1_001));
        assert!(effects.is_empty());
    }

    #[test]
    fn authoritative_fill_reconciles_optimistic_order_and_fill() {
        let mut effects = OptimisticAccountEffects::default();
        let resting = exchange_response(serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }));
        let filled = exchange_response(serde_json::json!({
            "filled": {
                "totalSz": "0.1",
                "avgPx": "100",
                "oid": 43_u64
            }
        }));
        apply_response(&mut effects, context(), &resting);
        apply_response(&mut effects, context(), &filled);

        let mut data = account_data(
            Vec::new(),
            vec![authoritative_fill(42), authoritative_fill(43)],
        );
        data.clearinghouse
            .asset_positions
            .push(authoritative_position("BTC", "0.1", "100"));

        assert!(effects.reconcile_with_account_data(&data, 1_001));
        assert!(effects.is_empty());
    }

    #[test]
    fn stale_optimistic_effects_expire_without_account_data_mutation() {
        let mut effects = OptimisticAccountEffects::default();
        let response = exchange_response(serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }));
        apply_response(&mut effects, context(), &response);
        let data = account_data(Vec::new(), Vec::new());

        assert!(effects.expire_stale(1_000 + OPTIMISTIC_EFFECT_TTL_MS + 1));
        assert!(effects.is_empty());
        assert!(data.open_orders.is_empty());
        assert!(data.fills.is_empty());
    }
}
