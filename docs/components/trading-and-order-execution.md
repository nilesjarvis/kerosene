# Trading And Order Execution

Kerosene's trading system turns UI intent into signed Hyperliquid exchange
actions. It supports standard ticket orders, presets, chart quick orders, chart
Quick Trade actions, chart HUD orders, drag-to-move orders, close-position
actions, NUKE, Chase, TWAP, and leverage updates. Wallet cluster orders reuse
the same execution boundary to submit one order leg per saved profile in the
selected cluster.

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

`order_execution.rs` re-exports shared types and helpers. Its account and pending
state implementations live beside their responsibilities:

- `order_execution/account_context.rs`: account-bound snapshots, spot-balance
  invalidation, reconciliation guards, and committed signing-key capture.
- `order_execution/pending.rs`: pending action, NUKE, and leverage models;
  shared trading-request guards and the HUD concurrency limit.
- `order_execution/identities.rs`: captured spot metadata for Chase/TWAP and
  open-order identity checks for Chase.
- `order_execution/exchange_errors.rs`: shared closed-order cancellation error
  matching for ordinary orders, Chase, and TWAP, plus retryable error matching for
  Chase and TWAP; callers retain their own reconciliation policies and precedence.
- `order_execution/quick_order/model.rs`: quick-order form, recovery, and
  percentage provenance, including redacted formatting.
- `order_execution/quick_order/move_order/context.rs`: captured move-order
  identity, replacement-key validation, and pending-move cleanup.

Account snapshot matching ignores case and surrounding whitespace. Move-order
replacement preserves its stricter policy: the trimmed current account must
exactly match the captured account. Standard pending-request guards and HUD
placement guards remain separate because HUD limit placements can overlap their
own tracking, indicators, and status checks while other requests still block.

## Order Surfaces

Orders can originate from several surfaces:

- main order entry pane
- order presets
- chart right-click quick order
- chart Quick Trade action strip
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

### Subaccount Trading Identity

`capture_profile_signing_key` captures the committed agent key and effective
subaccount target together in `CapturedAgentKey`. A profile with
`master_address: Some(parent)` must have valid, distinct parent and child
addresses. It signs the child's `wallet_address` as `vaultAddress`; ordinary
profiles retain a null target. Invalid metadata blocks trading.

Place, cancel, cancel-by-CLOID, modify, and leverage actions include the captured
target in both the action hash and JSON payload. Task clones, Chase/TWAP
lifecycles, late-result cancellation, move-order replacements, and cluster legs
retain that target. Reads, optimistic indicators, and result reconciliation use
the same effective child address. The existing pending-request and automation
guards continue to govern account switching.

## Standard Ticket Flow

The order-entry pane keeps submission controls and status feedback below the
scrolling form. Wider panes place inputs beside the order-value, fee, and
execution-option summary; narrow panes stack these groups. Price and size keep
explicit denomination labels, percentage shortcuts only update sizing, and the
GTC/IOC selector uses the existing limit-order kinds. Quick trade presets remain
immediate Buy/Sell actions. Chase status and TWAP settings scroll with the form
while their start controls remain in the footer. All controls use the active
application theme and retain the shared order-submission boundary.

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

`order_execution/core.rs` exposes the boundary between user intent and signed
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

The implementation lives in focused modules under `order_execution/core/`:

- `model.rs`: intents, prepared orders, redacted formatting, and request/context
  construction. Both placement helpers share the request field mapping.
- `capabilities.rs`: market-type policy, surface labels, and capability errors.
- `cloid.rs`: monotonic nonce allocation and one-shot client-order ID hashing.
- `preparation.rs`: place/cancel/modify validation and prepared wire values,
  including cancellation-only recovery when metadata is missing.
- `tasks.rs`: owned signing inputs and asynchronous exchange task wrappers.

Tests live beside each responsibility; preparation tests are grouped by place,
cancel, and modify actions. Validation order stays explicit in preparation.

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

Quick Trade is a serialized one-click market surface. It revalidates the chart
surface, symbol, action index, and action snapshot before preparing an order,
uses the shared stale-account and pending-request gates, and creates the normal
pending market indicator and CLOID-backed result context. Its actions are
explicit entries and therefore use fixed `reduce_only = false` rather than the
main ticket's current toggle.

## Spot Execution Safety

Spot execution is pair-specific and must not borrow perpetual-market state:

- A spot order resolves a fresh mid only from its exact API pair key or a
  metadata-verified canonical/legacy alias. It never falls through to a bare
  ticker, same-ticker perpetual, or `U`-prefixed alias.
- Spot metadata preserves the pair's quote-token identity. Percentage buys use
  that quote token's spendable balance; percentage sells use the base token's
  sellable `total - hold`, floored to the market's size precision.
- The shared Buy/Sell ticket cannot determine a side while previewing a slider
  value. At submission it recomputes the coin quantity for the clicked side.
  If the displayed quantity changes, submission stops and requires the user to
  review and submit again.
- Percentage provenance is tied to the connected account and the independent
  spot-balance revision. Incomplete or stale spot balances, or a changed spot
  revision, block submission and request reconciliation instead of reusing the
  old quantity.
- Placement is allowed only when the pair has a recognized USD-stable quote.
  Crypto-quoted or unknown-quote pairs stay fail-closed across ticket, quick,
  cluster, Chase, and TWAP surfaces until quote-to-USD accounting is verified.

Dispatching a spot placement, modification, cancellation, Chase action, or
TWAP child invalidates the connected account's spot-balance completeness. A
live spot fill does the same because fills and `spotState` are separate stream
lanes. A subsequent targeted `spotState` frame or full account refresh restores
completeness and advances the spot-balance revision.

Cached or retained spot metadata is visible but not orderable until a live,
strict metadata response verifies it. New spot actions fail closed during this
state. Existing spot TWAPs pause and can resume after verification; an existing
spot Chase stops and cancels a known resting order, or enters verification when
prior exchange exposure is uncertain. Cancellation remains available without
metadata only for strictly parsed `@N` order keys and the canonical
`PURR/USDC` pair; placement and modification never use that fallback.

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

Action hashing feeds the encoded action, nonce, vault target, and optional expiry
directly into Keccak in wire order. EIP-712 hashes share a helper that streams
their byte slices, avoiding concatenation buffers. The wire field order, marker
bytes, validation errors, and signature format are preserved by signing tests.

Exchange-response parsing borrows the retained JSON body while constructing
owned typed fields; malformed bodies remain available for fallback handling.
Response predicates share a borrowed status-list lookup while retaining their
separate confirmation rules. IOC no-match detection scans messages directly,
including the existing redacted raw-body summary, without collecting copies.

Signing uses agent private keys held in zeroizing strings. Do not log keys,
print payloads containing keys, or serialize keys into plaintext config.

## Result Handling And Verification

Exchange acknowledgements can be confirmed, rejected, or ambiguous. Result
handlers in `order_update/results/` and advanced-order modules decide whether
to:

- show confirmed success
- show a failure
- refresh account data
- query `orderStatus` by CLOID or OID
- mark a pending indicator uncertain until a later update

`order_update/results.rs` exposes the shared result types and classifier. Its
implementations are split by responsibility:

- `classification.rs`: execution outcomes, error redaction, and refresh policy.
- `pending.rs`: captured placement/cancel/move status requests and refresh cleanup.
- `one_shot.rs`: serialized placement completion and client-order ID reconciliation.
- `cancel.rs`: cancellation completion, order-ID verification, and local removal.
- `nuke.rs`: child-result reconciliation and aggregate completion.

Ticket, close-position, and Quick Trade results share serialized completion.
Quick-order recovery and concurrent HUD tracking retain their own cleanup
before applying the common placement outcome. Unknown cancellation results and
possibly completed cancellations share status verification while preserving
their distinct messages. Task callbacks own the captured placement context.

Close-menu toggling and workspace transient cleanup live in
`order_update/transient_ui.rs`, with tests for docked and detached windows.

Pending order indicators are keyed and shown in UI/account surfaces so users
can see in-flight actions. The app should not assume an order succeeded merely
because an HTTP request returned.

Ticket, quick-order, close-position, HUD, and Quick Trade submission share
`add_prepared_order_placement_indicator` in `order_pending_indicators.rs` to
copy prepared wire fields into independently owned indicators. Each submission
path chooses the projection kind: ticket and quick-order IOC limits project
like market orders. Indicator validation, ID allocation, and chart sync use the
same insertion path as other pending indicators.

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

Live and historical fill aggregation check borrowed known IDs plus the current
OID. Live totals retain coin, side, and adoption-cutoff filtering; history keeps
its separate fee/P&L rules. Completion formats the totals already computed for
reconciliation, preserving matched-fill amounts even when recorded progress is
ahead of that snapshot.

Book repricing and final modify dispatch share the ordered spot-market checks
in `chase/lifecycle/reprice.rs`: captured identity, quote support, then live
metadata verification. Each caller keeps its existing lifecycle, account, price,
and cooldown gates around those checks. Failures cancel a known order or use the
existing stop path. Placement retains its distinct prior-exposure recovery rules.

Place, modify, and cancel result handlers read the order and its lifecycle from
one lookup before accepting a result. Results for absent orders do nothing;
stale place/cancel results retain their outcome-based refresh policy for the
original account, while stale modify results do not refresh. Placement- and
OID-status handling borrow the admitted order through its transition, releasing
it before terminal-wide stop, removal, or refresh operations. Client-order ID
verification retains the original account for refresh even when no client-order
ID is available. OID status-request setup and failed status responses share
`ChaseLifecycle::verifying_order_status`, which preserves stop and missing-order
intent while selecting the next verification state.

Chase modify errors check closed-order text before retryable text. TWAP checks
retryable text before its terminal-error classification. Both use the same retry
predicate; TWAP reuses its normalized summary for further classification.

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

Slice planning borrows the cached order book and retry size. Retry plans remain
in state through admission, price validation, and key checks, then move into
dispatch. Skip handling takes only the retry's slice index. New-slice random sizing
and retry accounting retain their existing order.

Slice-result handling consumes a pending placement by ownership; a late result
leaves pending cancellations intact and retains its existing account-refresh
policy. Retry plans and final-use client-order IDs move into the next operation,
while child history, reconciliation state, and independent tasks retain their
own copies. Response classification, retry limits, and fill accounting stay in
the slice-result handler.

Unexpected-child cancellation shares completion cleanup and retry accounting
while preserving distinct events for confirmed, rejected, and transport-unknown
results. It prepares the exchange summary once and updates only matching children.
Transport errors still follow the retry budget when their text mentions a closed
order. TWAP account refresh applies its policy to the original order through one
lookup; status and cancellation callbacks move their final-use client-order IDs.

Status reconciliation shares matching-child updates for the returned order ID,
child state, and exchange summary. Matching remains exact by client-order ID,
and absent returned order IDs preserve each child's existing ID. Unknown statuses
and transport failures share retry accounting while retaining distinct pause
reasons and messages; missing-status exhaustion keeps its separate recovery rules.
Requested stops and confirmed absence of fills share completion eligibility checks.
Fill reconciliation confirms no-fill children in the same pass that applies late
fills. Only children with an exchange order ID can be confirmed absent, and a
matching fill takes precedence over no-fill confirmation.

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

Outcome placement and modification additionally require live verified contract
metadata, supported quote tokens, and an unpassed expiry/resolution deadline.
These checks run in shared preparation, including ticket, presets, and moving
existing orders. Cached, unknown, malformed, settled, and fallback contracts
are not orderable. Cancellation stays available and can recover deterministic
asset IDs from canonical `#(10 * outcome + side)` keys after metadata removal;
that recovery never authorizes placement or modification.

The ticket exposes template-derived rules, parent-question context, and
published fee scales. Scalar prices represent fractional payout value, not
event probabilities. Split/merge/negate operations are not exposed. See
[market data and symbols](market-data-and-symbols.md#contract-verification-and-lifecycle)
for metadata provenance and supported template families.

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
