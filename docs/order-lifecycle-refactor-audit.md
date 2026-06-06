# Order Lifecycle Refactor Audit

Audited: 2026-06-05

## Scope

This audit traces order-related flows from user intent through message routing,
exchange signing, result handling, and account reconciliation. The focus is on
whether order execution is centralized, whether all order types pass through the
same components, and where complexity or edge cases can be removed.

Primary files reviewed:

- `src/message.rs`
- `src/app_update/routing.rs`
- `src/order_update.rs`
- `src/order_execution/submit.rs`
- `src/order_execution/quick_order/submit.rs`
- `src/order_update/hud.rs`
- `src/order_execution/chase.rs`
- `src/order_execution/chase/lifecycle/*`
- `src/order_update/chase/*`
- `src/order_execution/twap/*`
- `src/twap_state/*`
- `src/signing.rs`
- `src/signing/client.rs`
- `src/signing/actions/*`
- `src/order_execution/position_actions/*`
- `src/order_execution/quick_order/move_order.rs`
- `src/account_update/stream/chase/*`
- `src/account_update/connection/refresh.rs`
- `src/subscription_state/market/{chase,twap}.rs`

## Executive Summary

The system is centralized only at the outer routing and signing layers.

All order-related `Message` variants are routed to `TradingTerminal::update_order`
through `UpdateRoute::Order`, and all signed Hyperliquid L1 mutations eventually
use the shared signing pipeline in `signing::client::sign_and_post`.

The order preparation and execution layer is not centralized. Standard ticket
orders, presets, quick orders, HUD chart orders, close-position orders, NUKE
orders, move-order modifies, Chase, and TWAP each prepare or validate their own
inputs before calling `place_order`, `place_order_with_cloid`, `modify_order`, or
`cancel_order`. These paths duplicate symbol lookup, hidden-symbol checks, mid
resolution, slippage pricing, price-band checks, size quantization, reduce-only
rules, pending UI state, and result refresh policy.

This duplication has already produced divergent behavior. For example, main
ticket orders support outcome markets, quick/HUD controls reject them, move order
allows outcome order moves with extra validation, and close/NUKE reject outcomes.
Those differences may be intentional by surface, but they are encoded as scattered
branches rather than as a single capability policy.

## Current Lifecycle Map

### 1. User Intent Enters as a Message

The main order ticket emits `PlaceBuy` or `PlaceSell` from
`order_views/actions.rs`. If `order_kind` is `Market`, `Limit`, or `LimitIoc`,
`update_order` calls `execute_order`. If `order_kind` is `Chase` or `Twap`, the
same button path starts a client-side advanced order instead.

Presets in `order_update/presets.rs` mutate the shared order form, then enqueue a
synthetic `PlaceBuy`/`PlaceSell`, `StartChase`, or `StartTwap` message.

Chart quick orders emit `SubmitQuickOrder`. Chart HUD mode emits `SubmitHudOrder`.
Chart order-line clicks emit `CancelOrder`, and chart order-line drags emit
`MoveOrder`.

Account open-order rows emit `CancelOrder` and can emit `ChaseRestingOrder` to
adopt an existing resting order into the Chase state machine.

Position rows emit `ClosePosition`. The emergency close-all flow emits
`NukePositions`, arms a confirmation, then batches one reduce-only market order
per routable position.

### 2. Routing

`app_update/routing.rs` maps every order-related message to `UpdateRoute::Order`.
`app_update.rs` delegates that route to `TradingTerminal::update_order`.
`order_update.rs` is therefore the main dispatcher for order actions, but it is a
dispatcher, not a central execution engine.

### 3. Standard Ticket Order

`PlaceBuy`/`PlaceSell` with `OrderKind::Market`, `Limit`, or `LimitIoc` calls
`order_execution/submit.rs::execute_order`.

Lifecycle:

1. Read `wallet_key_input` and `connected_address`.
2. Find `active_symbol` in `exchange_symbols`.
3. Validate the symbol through `validate_exchange_symbol_orderable`.
4. Call `prepare_order_submission`.
5. Parse positive quantity.
6. For limit orders, parse and round `order_price`.
7. For market orders, resolve a live mid, apply configured slippage, then round.
8. For outcomes, clamp/validate outcome prices and force coin quantity mode.
9. Validate price distance against the current mid.
10. Convert USD quantity to coin size when needed and quantize to `sz_decimals`.
11. Apply reduce-only unless the market is spot-like.
12. Set `order_status`, set `pending_order_action`, add a chart pending indicator.
13. Call `signing::place_order`.
14. Map the async result to `Message::OrderResult`.
15. `handle_order_result` clears `pending_order_action`, clears the indicator,
    displays the exchange summary or error, and refreshes account data on success
    or local/transport failure.

Standard ticket placements do not use a client order id. If the request fails
locally after the exchange accepts it, the app refreshes account data but cannot
query the exact order by cloid.

### 4. Quick Order

`SubmitQuickOrder` calls `order_execution/quick_order/submit.rs`.

Lifecycle:

1. Validate key and connected address.
2. Remove the quick-order form from the chart, restoring it on validation failure.
3. Resolve the chart symbol.
4. Reject hidden symbols and outcome markets.
5. Derive limit price from the click price or market price from live mid plus
   slippage.
6. Validate price band.
7. Quantize size from the quick-order quantity and denomination.
8. Apply reduce-only unless spot.
9. Add a pending indicator.
10. Call `signing::place_order`.
11. `handle_quick_order_result` clears the indicator, sets status, and refreshes
    account data according to the shared basic refresh policy.

This path duplicates most of `prepare_order_submission` but intentionally uses
the chart symbol and quick-order form state instead of the main ticket state.

### 5. HUD Chart Order

`SubmitHudOrder` calls `order_update/hud.rs::handle_submit_hud_order`.

Lifecycle:

1. Validate the chart still exists and the chart surface id is current.
2. Require HUD order submission to be armed.
3. Validate key and connected address.
4. Resolve chart symbol and reject hidden symbols.
5. Validate orderability, then reject outcome markets.
6. Parse HUD quantity.
7. For a limit order, round the clicked price and infer side from price versus
   reference mid or last candle close.
8. For a market order, use explicit HUD side and live mid plus slippage.
9. Validate price band.
10. Quantize size in coin units only.
11. Apply reduce-only unless spot-like.
12. Set status, set `pending_order_action`, start chart animation, optionally
    play sound, and add a pending indicator.
13. Call `signing::place_order`.
14. `handle_hud_order_result` clears the indicator, sets status, and refreshes
    account data.

Edge case: this path sets `pending_order_action` but does not clear it in
`handle_hud_order_result`. Standard ticket orders clear the same flag in
`handle_order_result`, and Chase clears it after the placement result. A settled
HUD order can therefore leave standard buy/sell controls and advanced-order
startup blocked by stale pending state.

### 6. Cancel Order

`CancelOrder` calls `order_execution/position_actions/cancel.rs::execute_cancel`.

Lifecycle:

1. Require an agent key, but not explicitly a connected address.
2. Reject hidden symbols.
3. Resolve symbol metadata to asset id.
4. Add a pending cancellation indicator if the current account/open-order data
   can identify the order.
5. Call `signing::cancel_order`.
6. `handle_cancel_result` clears the indicator, sets status, and refreshes
   account data according to the shared basic refresh policy.

### 7. Move Order

`MoveOrder` calls `order_execution/quick_order/move_order.rs::handle_move_order`.

Lifecycle:

1. Require key and connected address.
2. Refuse if this oid already has a pending move.
3. Find the order in current `account_data.open_orders`.
4. Validate hidden symbol, side, size, original price, symbol orderability, and
   reduce-only metadata.
5. Round the new price and refuse no-op moves after rounding.
6. Validate outcome price when applicable and validate price band.
7. Capture `PendingMoveOrderContext` with account address and agent key.
8. Add pending modify indicator.
9. Call `signing::modify_order`.
10. `handle_move_order_modify_result` removes the context, clears the indicator,
    syncs chart orders, sets status, and refreshes account data.

The account/key capture is a good safety pattern. It prevents a drag-started move
from silently using a different account/key if the UI state changes before the
exchange mutation.

### 8. Close Position

`ClosePosition` calls `order_execution/position_actions/close.rs`.

Lifecycle:

1. Require key and connected address.
2. Reject hidden position symbols.
3. Require account data to be loaded and fresh for position actions.
4. Find the position in current account data.
5. Derive side from signed position size and close size from requested fraction.
6. Reject outcome markets.
7. Resolve mid and build either a market price with slippage or a limit price at
   mid.
8. Call `signing::place_order` with `reduce_only = true`.
9. `handle_close_position_result` sets status and refreshes account data.

Close-position orders do not add pending chart indicators and do not use cloids.

### 9. NUKE Positions

`NukePositions` first arms a confirmation in `order_update/nuke.rs`, then calls
`order_execution/position_actions/nuke.rs::execute_nuke_positions`.

Lifecycle:

1. Require key and connected address.
2. Require fresh account data.
3. Plan one reduce-only market order for each visible, routable, non-muted
   position.
4. Abort if no position can be routed.
5. Batch `signing::place_order` tasks, one per position.
6. Each response arrives as `NukeResult` and is handled independently by
   `handle_nuke_result`.

This is intentionally urgent, but result handling is not aggregate-aware. The
status line can be overwritten by whichever child result arrives last, and each
child result can independently trigger account refresh.

### 10. Chase

Chase is a persistent client-side strategy over repeated exchange mutations.

Startup:

1. `StartChase` calls `advanced_order_start_context`, which enforces the maximum
   active advanced orders, refuses startup while `pending_order_action` is set,
   captures connected address and agent key, and rejects hidden active symbols.
2. `start_chase` parses quantity, rejects outcomes, converts USD size through
   a fresh mid when needed, quantizes size, captures symbol metadata, captures
   reduce-only, creates a `ChaseOrder`, sets `pending_order_action`, and fetches
   the initial order book.
3. `ChaseInitialBookLoaded` takes the current best bid/ask and calls
   `chase_place_at_best`.
4. `chase_place_at_best` validates account identity, prior exchange exposure,
   price rounding, replacement safety, drift/reprice limits, and global advanced
   request rate limits.
5. It generates a deterministic cloid through `chase_place_cloid`, stores it,
   sets lifecycle to `Placing`, and calls `place_order_with_cloid` as a GTC
   limit order.

Result and ongoing lifecycle:

1. `ChasePlaceResult` clears `pending_order_action`.
2. On accepted resting order with oid, Chase records the oid and enters
   account-verification state.
3. On full fill, Chase records fills and refreshes account data.
4. On ambiguous/local failure, Chase queries `orderStatus` by cloid and refreshes
   account data before allowing replacement.
5. Account refresh and websocket open-order/fill updates reconcile fills,
   missing orders, oversized remaining orders, replacement safety, and stop
   cleanup.
6. Book subscriptions send `ChaseBookUpdate` only once Chase has a current oid.
   Book updates can request a reprice, but Chase generally verifies account state
   before modifying.
7. Reprice uses `modify_order` with the current oid.
8. Stop uses `cancel_order`, then verifies the cancel through account refresh or
   status checks.

Chase is cautious and uses cloids for placements, but it has a complex state
machine spread across order execution, order update, account stream
reconciliation, and subscriptions.

### 11. TWAP

TWAP is a persistent client-side slicer that sends IOC child orders.

Startup:

1. `StartTwap` calls the same `advanced_order_start_context` as Chase.
2. `start_twap` resolves symbol metadata, rejects outcomes, validates duration,
   slice count, aggregate slice capacity, price range, quantity, USD reference
   price, and minimum planned child notional.
3. It creates a `TwapOrder` with captured account/key, target size, schedule,
   price bounds, reduce-only, and status `WaitingForMarket`.

Scheduling and execution:

1. `subscription_state/market/twap.rs` subscribes active TWAPs to order books and
   adds a one-second `TwapTick`.
2. `TwapBookUpdate` caches the latest book and resumes stale-market pauses.
3. `TwapTick` picks the next schedulable TWAP and calls `execute_due_twap_slice`.
4. Slice execution refuses account mismatch, hidden symbols, stale market data,
   account loading/reconciliation, and global advanced request rate limits.
5. It computes or retries the slice size, validates it against the book, price
   range, and minimum notional, creates a `TwapPendingSlice`, generates a
   deterministic cloid through `twap_child_cloid`, and calls
   `place_order_with_cloid` with `OrderKind::LimitIoc`.

Result and reconciliation:

1. `TwapSliceResult` classifies IOC no-match, exchange errors, fills, ambiguous
   responses, unexpected resting orders, and transport failures.
2. Ambiguous/transport failures trigger account refresh and `orderStatus` lookup
   by cloid.
3. Unexpected resting child orders are canceled, preferring cloid cancellation.
4. Account fills reconcile child fills and can complete, partially complete, or
   error a TWAP if the exchange reported a fill but account fills do not catch up
   within `TWAP_RECONCILIATION_TIMEOUT`.
5. Stop requests either mark the TWAP stopped immediately or wait for an
   in-flight child operation to settle.

TWAP has the strongest exact-reconciliation story because every child placement
uses a cloid. Its complexity is mostly in retry, status, and unexpected resting
handling.

## Centralization Verdict

Centralized:

- `Message` routing to the order update route.
- The final signed L1 action pipeline: msgpack serialization, signing, nonce,
  `/exchange` post, and response parsing.
- Some shared helper functions: market slippage, rounding, quantity-to-size
  conversion, price-band validation, response summary/refresh policy.

Not centralized:

- Order intent modeling.
- Symbol/orderability/capability checks.
- Outcome capability policy.
- Price derivation.
- Size derivation.
- Reduce-only derivation.
- Client order id generation for one-shot orders.
- Pending UI state.
- Pending chart indicators.
- Result handling and refresh policy.
- Reconciliation after ambiguous one-shot placements.

The same economic operation can go through different preparation code depending
on whether the user used the main ticket, a preset, quick order, HUD order,
position close, NUKE, Chase, or TWAP.

## Findings and Simplification Opportunities

### Finding 1: HUD Orders Can Leave `pending_order_action` Stuck

`handle_submit_hud_order` sets `pending_order_action` to `Buy` or `Sell`, but
`handle_hud_order_result` does not clear it. This is inconsistent with
`handle_order_result` and `handle_chase_place_result`.

Impact:

- Standard buy/sell buttons can remain in a pending state.
- `advanced_order_start_context` rejects advanced-order startup while the stale
  pending action remains set.
- Account switching/deletion flows that check `pending_order_action` can also be
  unnecessarily blocked.

Recommended fix:

- Add a targeted regression test for HUD result handling.
- Clear `pending_order_action` in `handle_hud_order_result`.
- Longer term, replace this raw option with a central pending-operation tracker
  keyed by request id/source.

### Finding 2: `OrderKind` Mixes UI Modes, Strategies, and Exchange TIF

`OrderKind` includes `Market`, `Limit`, `LimitIoc`, `Chase`, and `Twap`.
`signing/actions/builders.rs` maps `OrderKind::Chase` to GTC and
`OrderKind::Twap` to IOC even though normal `execute_order` returns early before
advanced modes reach signing.

Impact:

- A signing helper can technically receive `Chase` or `Twap` and emit an
  exchange order, even though those are strategy modes, not exchange order types.
- `unreachable!` protects one path, but the type system does not prevent misuse
  elsewhere.

Recommended fix:

- Split UI/strategy selection from exchange wire order type.
- Example: `OrderEntryMode::{Market, Limit, LimitIoc, Chase, Twap}` and
  `ExchangeOrderTif::{Ioc, Gtc}` or `PreparedOrderKind::{MarketIoc, LimitGtc,
  LimitIoc}`.
- Make signing accept only exchange-order types.

### Finding 3: One-Shot Placements Lack Cloids

Standard ticket, quick, HUD, close-position, and NUKE placements use
`place_order` without a cloid. Chase and TWAP use deterministic cloids and can
query `orderStatus` by cloid after ambiguous responses.

Impact:

- Basic result handling refreshes account data on local/transport/parse failure,
  but cannot ask the exchange whether a specific placement landed.
- Pending chart indicators are cleared as soon as the local result arrives, even
  when the exchange outcome is uncertain.
- Close/NUKE can leave the user with less precise feedback after partial network
  failure.

Recommended fix:

- Generate a cloid for every placement, including one-shot orders.
- Track one-shot placement requests by cloid until account data or orderStatus
  confirms filled/resting/rejected/missing.
- Use the same reconciliation primitive as Chase/TWAP, with a simpler policy for
  non-automated orders.

### Finding 4: Preparation Logic Is Duplicated Across Submitters

`execute_order`, quick order, HUD order, close position, and NUKE independently
derive price, size, reduce-only, symbol metadata, and validation.

Examples:

- Standard and HUD market orders both use mid plus slippage but implement their
  own parse/round/validate flow.
- Quick and standard limit orders both round a price and validate the price band.
- Close and NUKE both create reduce-only market orders from account positions.
- Hidden-symbol and outcome checks are repeated with different wording and
  capability rules.

Impact:

- Bug fixes must be repeated across surfaces.
- Surface-specific behavior is hard to distinguish from accidental divergence.
- Test coverage has to chase many paths for the same invariant.

Recommended fix:

- Introduce a central order preparation module that accepts an `OrderIntent` and
  returns a `PreparedExchangeOrder`.
- Keep surface-specific inputs at the edge, but normalize them before signing.

### Finding 5: Capability Rules Are Scattered

Outcome support is the clearest example:

- Main ticket supports outcome market/limit orders.
- Quick and HUD explicitly reject outcome trading.
- Outcome presets route back through the main ticket.
- Move order allows outcome moves with price/contract validation.
- Chase/TWAP reject outcomes.
- Close/NUKE reject outcomes.

Impact:

- The capability matrix is real, but it is implicit in many branches.
- A future order surface can accidentally enable or disable an asset class.

Recommended fix:

- Define a central capability policy such as
  `OrderSurfaceCapability::allows_market_type(surface, market_type, operation)`.
- Have every submit path call it and use shared user-facing error text.

### Finding 6: Pending State Is a Mix of Global Flag, Indicators, and Strategy State

The app currently has:

- `pending_order_action` for main buy/sell and Chase startup, plus HUD.
- `pending_order_indicators` for chart overlays.
- `pending_move_order_contexts` for move operations.
- Chase lifecycle state for place/modify/cancel.
- TWAP `pending_op` and child statuses.

Impact:

- It is easy to clear one pending state but not another.
- Some flows block unrelated actions globally, while others allow concurrent
  submissions.
- Pending chart indicators are presentation state but are coupled into submit
  handlers.

Recommended fix:

- Create a unified `OrderRequestTracker` or `ExecutionRegistry` keyed by request
  id/cloid.
- Store source, operation, account, symbol, optional indicator id, refresh policy,
  and pending UI behavior in one place.
- Let Chase/TWAP keep strategy state, but submit actual exchange mutations
  through the same tracker.

### Finding 7: Result Handling Is Similar but Forked

Basic order, quick order, HUD, cancel, close, nuke, move, Chase, and TWAP each
handle exchange results with custom code.

Some divergence is necessary for strategy state machines, but the common parts
are repeated:

- Determine whether account refresh is needed.
- Set status from `ExchangeResponse::summary`.
- Treat local errors as requiring reconciliation.
- Clear pending chart indicators.
- Sync chart order overlays.

Impact:

- Fixes like the HUD pending-state bug are easy to miss.
- Multiple-result flows such as NUKE have no aggregate result model.

Recommended fix:

- Centralize generic exchange result handling into a small result policy layer.
- Strategy handlers should receive normalized outcomes:
  `AcceptedResting`, `Filled`, `Rejected`, `Ambiguous`, `TransportUnknown`,
  `Cancelled`, `ModifyAccepted`, etc.

### Finding 8: Account Freshness Rules Are Not Applied Uniformly

Close and NUKE require fresh account data before placing position-closing orders.
Move order requires the order to be present in current account data but does not
use the same freshness helper. Basic reduce-only ticket orders use the form and
current symbol state rather than forcing fresh account data.

Impact:

- Reduce-only semantics can depend on the surface.
- A stale open order can still be used for a move if it remains in account data.

Recommended fix:

- Define freshness requirements per operation:
  `NewOrder`, `ReduceOnlyFromForm`, `ClosePosition`, `MoveRestingOrder`,
  `AdoptRestingOrder`, `NukePositions`.
- Enforce through one central preflight function.

### Finding 9: Advanced-Order Rate Limiting Is Global but Hidden in Strategy Code

Chase and TWAP share `last_advanced_exchange_request_at` and
`ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL`, but the checks live in strategy
methods.

Impact:

- Any future client-side automation must know to reuse the same field and timing
  rule.
- It is not obvious which exchange mutations are globally rate-limited and which
  are not.

Recommended fix:

- Move rate limiting into the central execution service.
- Allow policies like `ImmediateUserAction`, `AdvancedAutomation`, and
  `EmergencyClose`.

### Finding 10: NUKE Batches Independent Results Without an Aggregate Operation

NUKE submits one task per planned position, each mapped to `NukeResult`.

Impact:

- Status can be overwritten by the last-arriving child result.
- Multiple child results can cause multiple account refresh requests.
- There is no single operation summary like "5 submitted, 4 accepted, 1 failed".

Recommended fix:

- Represent NUKE as a parent execution with child requests.
- Aggregate child results and perform one reconciliation refresh when practical.

## Suggested Target Architecture

### Core Types

Introduce a narrow order execution core, for example under
`src/order_execution/core/`.

Suggested types:

- `OrderSurface`: `Ticket`, `Preset`, `QuickOrder`, `Hud`, `ClosePosition`,
  `Nuke`, `Chase`, `Twap`, `Move`, `Cancel`.
- `OrderOperation`: `Place`, `Cancel`, `Modify`, `UpdateLeverage`.
- `PlaceIntent`: symbol, side, price source, quantity source, reduce-only source,
  market capability requirements, and source metadata.
- `PriceSource`: explicit limit price, market from mid with slippage, book best,
  TWAP IOC book plan, close-at-mid.
- `QuantitySource`: coin size, USD notional, position fraction, existing open
  order size, TWAP planned slice.
- `PreparedExchangeOrder`: account, asset, symbol, side, price wire, size wire,
  exchange TIF/kind, reduce-only, cloid, source.
- `ExecutionContext`: captured account address and agent key.
- `ExecutionRequest`: prepared operation plus UI/reconciliation policy.
- `ExecutionOutcome`: normalized result classification over `ExchangeResponse`.

### Preparation Flow

Every submitter should do only two things:

1. Convert local UI state into an intent.
2. Hand the intent to shared preparation/execution code.

The shared preflight should own:

- Agent key and account capture.
- Symbol lookup and orderability.
- Surface capability checks.
- Hidden-symbol checks.
- Outcome constraints.
- Mid resolution and staleness checks.
- Slippage pricing.
- Limit price rounding.
- Price-band validation.
- Quantity parsing and quantization.
- Reduce-only derivation.
- Optional account freshness requirements.
- Cloid creation.

### Execution Flow

All exchange mutations should pass through one service-like helper:

1. Register the request in an execution tracker.
2. Create chart pending indicators when requested by policy.
3. Set status/pending action through the tracker, not ad hoc fields.
4. Call the signing client.
5. Classify the result.
6. Clear/advance pending state.
7. Refresh or reconcile account state according to policy.
8. Notify strategy-specific state machines with normalized outcomes.

Chase and TWAP can keep their strategy models, but their actual place/cancel/modify
calls should be made through the same exchange mutation helper.

## Refactor Prompt

Use this prompt as the goal for a follow-up refactor task:

```text
Refactor Kerosene's order execution lifecycle into a centralized order
preparation and execution layer.

Context:
- Read `docs/order-lifecycle-refactor-audit.md` first.
- This is trading software. Do not log, print, serialize, or expose private keys,
  agent keys, wallet secrets, or secret-bearing config.
- Preserve current user-visible behavior unless the audit identifies it as a bug
  or accidental divergence.
- Keep changes incremental and covered by focused tests.

Primary objectives:
1. Introduce explicit core order execution types so UI/strategy modes are
   separated from exchange wire order types. `Chase` and `Twap` must no longer be
   accepted by signing helpers as order kinds.
2. Add a shared order preflight/preparation layer that converts surface-specific
   intents into prepared exchange mutations. It must centralize account/key
   capture, symbol orderability, hidden-symbol policy, market-type capabilities,
   outcome rules, price derivation, price-band validation, size quantization,
   reduce-only rules, and account freshness requirements.
3. Add a shared exchange execution/result layer for place/cancel/modify requests.
   It should manage request ids/cloids, pending indicators, pending UI state,
   result classification, account refresh/reconciliation policy, and status
   updates.
4. Generate cloids for all order placements, not only Chase and TWAP, so
   ambiguous one-shot placements can be reconciled precisely.
5. Migrate standard ticket orders, presets, quick orders, HUD orders,
   close-position orders, NUKE orders, move-order modifies, Chase, and TWAP to
   the shared layer in small steps.
6. Keep Chase and TWAP strategy state machines, but make their exchange mutations
   use the same execution service and normalized result classifications.
7. Fix the HUD pending-state bug: `pending_order_action` or its replacement must
   be cleared when a HUD order result settles.

Suggested implementation plan:
1. Add regression tests for current edge cases, especially HUD result clearing
   and `OrderKind::Chase/Twap` not being valid signing order types.
2. Introduce exchange-only order type(s), migrate `signing/actions/builders.rs`
   and signing tests, and update call sites with the minimum compatibility
   adapter needed.
3. Add `OrderSurface`, `PlaceIntent`, `PriceSource`, `QuantitySource`,
   `ExecutionContext`, and `PreparedExchangeOrder`.
4. Move standard ticket preparation into the new preflight without changing
   behavior.
5. Migrate quick and HUD order preparation to the same preflight; keep their
   surface-specific constraints as explicit capability policy.
6. Add a central placement helper that registers pending UI state, adds pending
   indicators, calls `place_order_with_cloid`, classifies the result, and applies
   refresh/reconciliation policy.
7. Migrate close and NUKE onto shared preparation/execution. For NUKE, add a
   parent operation summary and avoid redundant refresh storms.
8. Migrate cancel and move modify to shared execution helpers. Preserve
   `PendingMoveOrderContext` account/key safety or replace it with a more general
   captured `ExecutionContext`.
9. Migrate Chase and TWAP exchange calls to shared place/cancel/modify helpers,
   while keeping their lifecycle transition logic local.
10. Remove obsolete duplicate helper code and update tests near each migrated
    module.

Acceptance criteria:
- No signing helper accepts UI strategy modes (`Chase`, `Twap`) as an exchange
  order type.
- All placement paths use cloids.
- HUD orders cannot leave the app in a stale pending state.
- Main ticket, quick order, HUD order, close, NUKE, Chase, and TWAP share the
  same preflight primitives for symbol, price, size, reduce-only, and capability
  checks.
- Ambiguous one-shot placement failures trigger cloid/orderStatus or account
  reconciliation instead of only a blind account refresh.
- Existing Chase and TWAP safety behavior remains intact: account/key capture,
  prior-exposure checks, unexpected resting cancellation, fill reconciliation,
  and global advanced-order pacing.
- Focused regression tests cover request construction, response classification,
  stale account behavior, HUD pending clearing, cloid generation, and surface
  capability policy.

Validation:
- Run `cargo fmt`.
- Run focused tests for changed modules first.
- Run `cargo test` when the refactor reaches all order surfaces.
- Run `cargo clippy --all-targets --all-features -- -D warnings` before final
  handoff if the change is broad enough for CI-level validation.
```
