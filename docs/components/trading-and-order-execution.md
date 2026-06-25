# Trading And Order Execution

Kerosene's trading system turns UI intent into signed Hyperliquid exchange
actions. It supports standard ticket orders, presets, chart quick orders, chart
HUD orders, drag-to-move orders, close-position actions, NUKE, Chase, TWAP, and
leverage updates. Wallet cluster orders reuse the same execution boundary to
submit one order leg per saved profile in the selected cluster.

This is one of the highest-risk parts of the app. Changes must preserve account
identity, key handling, stale-account checks, market-type restrictions,
reduce-only semantics, and order-status verification.

## Component Map

| Component | Key files | Responsibility |
| --- | --- | --- |
| Order state | `src/app_state.rs`, `src/twap_state/`, `src/advanced_order_history/` | Form fields, pending contexts, active Chase/TWAP orders, order history. |
| Order views | `src/order_views.rs`, `src/order_views/` | Order ticket, inputs, presets, advanced orders, quick order card, detail windows. |
| Order updates | `src/order_update.rs`, `src/order_update/` | Form handling, submit/cancel results, Chase/TWAP lifecycle, close/nuke, move order. |
| Execution boundary | `src/order_execution.rs`, `src/order_execution/` | Validation, sizing, prepared orders, task wrappers, advanced order lifecycle. |
| Wallet cluster execution | `src/wallet_cluster_update.rs` | Split order intents, per-wallet signing tasks, ambiguous leg status checks, aggregate close actions. |
| Signing | `src/signing.rs`, `src/signing/` | Hyperliquid action payloads, nonces, action hash, EIP-712 signing, exchange POSTs. |
| Market symbol helpers | `src/order_execution/symbols/` | Market lookup, outcome handling, fees, display labels, orderability. |
| Risk filters | `src/risk_state/` | Hidden-symbol and market-universe checks that affect routing and order eligibility. |

## Order Surfaces

Orders can originate from several surfaces:

- main order entry pane
- order presets
- chart right-click quick order
- chart HUD order controls
- chart order-line drag-to-move
- chart/open-order cancel controls
- positions table close menu
- NUKE button
- wallet cluster window
- Alfred command palette
- Chase/TWAP advanced orders

All surfaces should route through the same execution boundary rather than
duplicating signing or order construction logic.

## Standard Ticket Flow

```text
PlaceBuy / PlaceSell
  -> order_update.rs chooses by OrderKind
  -> order_execution/submit.rs prepares order
  -> order_execution/core.rs validates surface and market type
  -> signing::place_order_with_cloid
  -> Message::OrderResult
  -> result classification
  -> local feedback, account refresh, or orderStatus check
```

The order form stores:

- price
- quantity
- quantity denomination
- percentage slider
- order kind
- reduce-only flag
- leverage input and margin mode
- presets menu/edit state

Market and limit orders share validation and prepared-order construction. IOC
limit behavior is represented by `OrderKind::LimitIoc`.

## Prepared Order Boundary

`order_execution/core.rs` defines the boundary between user intent and signed
exchange action:

- `OrderSurface`
- `PlaceIntent`
- `CancelIntent`
- `ModifyIntent`
- `PreparedExchangeOrder`
- `PreparedModifyOrder`
- `OrderOperation`
- `PriceSource`
- `QuantitySource`
- `QuantityDenomination`
- `ReduceOnlySource`

This layer centralizes:

- market-type capability checks
- symbol/orderability checks
- quantity parsing and sizing
- USD-notional to coin-size conversion
- reduce-only semantics
- slippage/market-price handling
- CLOID generation
- task wrappers for place/cancel/modify

Feature-specific surfaces should build intents and let this layer prepare the
wire action.

Wallet clusters add two order surfaces:

- `OrderSurface::Cluster` for standard split entries across member profiles.
- `OrderSurface::ClusterClose` for reduce-only perpetual closes derived from
  fresh member snapshots.

Each cluster leg receives its own CLOID and result/status row. Ambiguous or
transport-unknown legs query `orderStatus` by CLOID before being marked
confirmed, failed, or uncertain.

## Signing

`src/signing/` is the only signed exchange-action implementation.

Key files:

- `signing/client.rs`: signed Hyperliquid `/exchange` POST path.
- `signing/actions.rs`: order, cancel, cancel-by-CLOID, modify, leverage update
  wire actions.
- `signing/crypto.rs`: action hash and EIP-712 agent signing.
- `signing/model.rs`: order kinds, Chase model, exchange response model.
- `signing/numbers.rs`: wire number formatting and price rounding.

Signing uses agent private keys held in zeroizing strings. Do not log keys,
print payloads containing keys, or serialize keys into plaintext config.

## Result Handling And Verification

Exchange acknowledgements can be confirmed, rejected, or ambiguous. Result
handlers in `order_update/results.rs` and advanced-order modules decide whether
to:

- show confirmed success
- show a failure
- refresh account data
- query `orderStatus` by CLOID or OID
- mark a pending indicator uncertain until a later update

Pending order indicators are keyed and shown in UI/account surfaces so users
can see in-flight actions. The app should not assume an order succeeded merely
because an HTTP request returned.

## Cancel And Move Order

Cancel flow:

```text
CancelOrder { coin, oid }
  -> signed cancel task
  -> CancelResult
  -> confirmed local removal or account refresh/status feedback
```

Move-order flow captures the original trading identity:

- `PendingMoveOrderContext` stores account address and agent key when the move
  starts.
- The original order is canceled.
- Replacement placement uses the captured key only if the active account still
  matches.

This prevents an account switch from silently placing the replacement order on a
different account after canceling the original order.

## Close Position

Close-position actions are reduce-only orders derived from current account
positions. They require:

- connected account
- usable agent key
- fresh account data
- routable perpetual market
- usable mid/price reference
- valid fraction

The positions table and Alfred close commands use the same close-position
execution path.

## NUKE

NUKE closes visible open perpetual positions with reduce-only market orders.
The planner classifies each position before routing:

- routable
- hidden/muted
- unsupported market
- missing/invalid mid
- stale or missing account data
- other validation failure

Hidden exposure is a risk boundary. NUKE should not silently route hidden
positions, and it should surface skipped/failed/uncertain counts.

NUKE progress is tracked by `PendingNukeExecution`:

- total
- completed
- confirmed
- failed
- uncertain
- skipped
- refresh needed

Uncertain children can trigger order-status checks or account refreshes.

## Chase Orders

Chase orders are client-side advanced orders that rest a limit order near the
best bid/ask and reprice until filled, stopped, or expired.

Runtime state:

- `chase_orders: BTreeMap<u64, ChaseOrder>`
- `next_chase_id`
- `selected_chase_id`

Key modules:

- `order_execution/chase.rs`
- `order_execution/chase/lifecycle/`
- `order_update/chase/`
- `subscription_state/market/chase.rs`
- `signing/model.rs`
- `advanced_order_history/`

Startup validates:

- max active Chase limit
- no conflicting pending action
- account and agent key availability
- hidden-symbol filters
- market type/orderability
- size and reduce-only constraints
- initial book availability

Lifecycle messages include:

- `StartChase`
- `ChaseInitialBookLoaded`
- `ChaseBookUpdate`
- `ChaseRepriceTick`
- `ChasePlaceResult`
- `ChaseModifyResult`
- `ChaseCancelResult`
- `ChaseOrderStatusLoaded`
- `ChaseOrderOidStatusLoaded`
- `StopChase`, `StopChaseById`, `StopAllAdvancedOrders`

Websocket open-order/fill updates reconcile Chase progress. Terminal or removed
Chase orders are archived into advanced order history.

## TWAP Orders

TWAP orders are client-side scheduled IOC slices. They are modeled in
`twap_state/` and executed through `order_execution/twap/`.

Runtime state:

- `twap_orders: BTreeMap<u64, TwapOrder>`
- `twap_form`
- `next_twap_id`
- `selected_twap_id`

TWAP validates:

- connected account and key
- supported market
- duration and slice limits
- minimum notional
- aggregate slice-rate limits
- price gates
- stale-book timeout
- duplicate-start window
- randomization settings

Lifecycle messages include:

- `StartTwap`
- `TwapTick`
- `TwapBookUpdate`
- `TwapSliceResult`
- `TwapUnexpectedCancelResult`
- `TwapOrderStatusLoaded`
- `StopTwap`
- `OpenTwapDetails`

Terminal TWAPs are archived into advanced order history. Active TWAPs are
runtime-only and are not resumed as live automation after restart.

## Advanced Order History

`advanced_order_history/` stores bounded snapshots of terminal advanced orders.
It exists so users can inspect completed or removed Chase/TWAP behavior without
keeping active lifecycle state alive.

Persisted:

- terminal advanced-order history entries
- detail window mapping where needed

Not persisted:

- active Chase/TWAP state
- in-flight order status requests
- open websocket subscriptions

## Leverage Updates

Leverage updates use signed `update_leverage` actions and include:

- account address
- symbol key/display
- asset ID
- optional HIP-3 dex
- cross/isolated flag
- leverage value

Results trigger scoped account refreshes so UI margin state catches up.

## Outcome Markets

Outcome markets have special handling:

- outcome symbol parsing and labels live under `api/exchange_symbols/outcomes/`
  and `order_execution/symbols/outcome/`
- some forms force coin-size rather than USD-notional input
- outcome sell prefill can use held outcome balances
- unsupported order surfaces should disable rather than route

Do not assume all market symbols are main-dex perpetuals.

## Security Boundaries

- Agent keys are secret-bearing and zeroized.
- Signing happens only in `signing/`.
- Config snapshots intentionally blank agent/API key fields.
- Pending move-order replacement cannot switch accounts.
- Stale account data should block close/NUKE and high-risk automation.
- Hidden-symbol and market-universe filters must be honored by automation.
- Do not log exchange payloads that contain signatures or key material.

## Tests To Check

Use focused tests for order changes:

- `src/order_execution/**/tests`
- `src/order_update/**/tests`
- `src/order_execution/chase/lifecycle/tests/**`
- `src/order_execution/twap/tests/**`
- `src/twap_state/tests/**`
- `src/advanced_order_history/tests/**`
- `src/signing/tests/**`
- `src/signing/client/tests/**`
- `src/risk_state/matching/tests/**`
- `src/account_update/stream/tests/**` for websocket/order reconciliation

For signing, close-position, NUKE, Chase, or TWAP changes, run the narrow tests
first and then broader `cargo test` when feasible.
