# Chase Metrics Audit

This audit defines the metrics that matter for Chase orders and compares them
with the current implementation. A Chase is client-side automation that keeps a
single exchange limit order at the best bid for buys or best ask for sells, so
the important measurements are execution progress, price quality, lifecycle
safety, reconciliation reliability, and post-trade auditability.

## Current Coverage

| Metric area | Current evidence | Audit result |
| --- | --- | --- |
| Identity and scope | `ChaseOrder` stores `id`, `coin`, `account_address`, side, asset, precision, spot/perp, reduce-only, current OID/CLOID, and known OIDs in `src/signing/model/chase/order.rs:13`. | Covered for runtime state and basic history. |
| Size progress | Runtime tracks `target_size`, `filled_size`, and `remaining_size`; `residual_size`, `set_filled_size`, and `sync_open_remaining_size` enforce monotonic progress and clamp remaining size in `src/signing/model/chase/order.rs:93`. | Covered for active state. |
| Initial start context | Start validation resolves account, agent key, symbol, outcome-market exclusion, USD reference price, quantity precision, side, and start timestamps in `src/order_execution/chase.rs:66`. | Covered for runtime safety. |
| Price state | Runtime stores `current_price`, rounded wire price, `initial_price`, and queued `desired_price`; repricing accepts only price moves toward fill in `src/signing/model/chase/order.rs:134` and `src/order_execution/chase/lifecycle/reprice.rs:67`. | Covered for active state. |
| Lifecycle state | `ChaseLifecycle`, `ChaseVerificationReason`, `ChaseQueuedAction`, and `ChaseStopPhase` model loading, placing, resting, checking, queued, repricing, canceling, and stopping in `src/signing/model/chase/lifecycle.rs:21`. | Covered for control flow and UI status. |
| Safety limits | Chase has max cancel retries, max reprices, max duration, max drift, per-order reprice interval, and retry cooldown in `src/signing/model/chase/lifecycle.rs:7`; global advanced-order exchange gating is checked before request send in `src/order_execution/chase/lifecycle.rs:21`. | Covered. |
| Reprice gating | Reprice skips missing OID, same rounded price, price movement away from fill, pending operations, account mismatch, throttle, and global gate in `src/order_execution/chase/lifecycle/reprice.rs:73`. | Covered. |
| Fill summaries | Fill aggregation deduplicates by OID and fill identity, then computes filled size and total notional in `src/account_update/stream/fills.rs:70`. | Partially covered. Summary text can report average fill price. |
| Account reconciliation | Complete account refresh checks fills and open orders before removing, replacing, correcting size, or status-checking a Chase in `src/account_update/stream/chase/refresh.rs:14`. | Covered for fail-closed behavior. |
| Active UI visibility | Advanced Orders rows show side, coin, filled/target/remaining, current price, reprice count, reduce-only flag, lifecycle status, and stop action in `src/order_views/advanced/rows.rs:26`. | Covered for compact monitoring. |
| History snapshot | `AdvancedOrderHistoryEntry::from_chase` persists target, filled, remaining, current price, reduce-only, reprice count, status, start/completion times, and two log lines in `src/advanced_order_history/snapshots.rs:95`. | Partial. Good summary, weak forensic detail. |

## Gaps

### P0 - Execution Average Is Not Persisted Correctly

The fill path computes a weighted average from matched fills for summary text
(`total_notional / filled_size` in `src/account_update/stream/fills.rs:153`),
but the Chase history snapshot stores `average_price` as
`positive_finite_value(chase.current_price)` in
`src/advanced_order_history/snapshots.rs:140`.

That means the Advanced Order history "Average" metric can show the last
working limit price rather than the actual fill VWAP. This is the highest
priority audit gap because it makes a displayed execution metric materially
ambiguous after partial fills, replacements, or final fills away from the last
tracked price.

Recommended fix:

- Extend the Chase completion/archive path to pass fill totals into
  `AdvancedOrderHistoryEntry::from_chase`, or persist fill notional on
  `ChaseOrder`.
- Store `average_price = total_fill_notional / filled_size` when fill data is
  available.
- Keep `current_price` as a separate "last working price" metric.
- Add tests covering multi-fill and multi-OID Chase completion history.

### P1 - Fees, Closed PnL, And Cost Are Not Preserved

`chase_fill_totals` uses `fee` and `closed_pnl` only as dedupe identity inputs,
but the returned totals include only side, coin, filled size, and total notional
in `src/account_update/stream/fills.rs:62`. Chase history therefore cannot
answer basic post-trade questions: total fees paid, fee rate, net notional, or
closed PnL for reduce-only fills.

Recommended metrics:

- `total_fee`
- `average_fee_rate`
- `closed_pnl`
- `gross_notional`
- `net_notional_after_fees`

### P1 - No Child Order Ledger For Chase

TWAP history stores child order records, but Chase history always writes
`children: Vec::new()` in `src/advanced_order_history/snapshots.rs:171`.
Runtime knows `known_oids`, `current_oid`, `current_cloid`, and
`place_attempt_count`, but it does not preserve a per-OID ledger with placed,
modified, canceled, filled, or terminal status.

Recommended child-order metrics:

- OID and CLOID.
- First placed price and size.
- Final price and remaining size.
- Fill size and fill VWAP for that OID.
- Place/modify/cancel status and exchange summary.
- Number of modify attempts for that OID.

### P1 - Latency And Queueing Are Not Measurable

Runtime has `started_at`, `started_at_ms`, `last_reprice_at`, and lifecycle
state, but it does not store request-sent and result-received timestamps per
place, modify, cancel, status check, account refresh, or queued reprice.

Recommended latency metrics:

- Initial book fetch latency.
- Placement request round trip.
- Time from start to first resting OID.
- Time from first resting OID to first fill.
- Time to full fill or stop.
- Modify request round trip.
- Cancel request round trip.
- Account refresh wait time while verifying.
- Time spent queued by per-order throttle or global exchange gate.

### P1 - Reprice Opportunity Quality Is Not Auditable

The implementation stores only the desired/current price and reprice count.
It does not persist the quote that triggered each decision, spread at decision
time, skipped opportunities, or whether a queued price became stale before
being sent.

Recommended quality metrics:

- Best bid, best ask, and spread at placement and each reprice decision.
- Price delta from previous working price.
- Drift from initial price.
- Number of ignored book updates by reason: same price, price moved away,
  pending operation, throttle, incomplete account data, account mismatch.
- Number of queued reprices that were sent, replaced, or cleared.
- Time at best quote versus time behind the best quote.

### P2 - History Logs Are Too Sparse For Incident Review

Chase history logs only a synthetic start event and one terminal event in
`src/advanced_order_history/snapshots.rs:152`. This is enough for a summary
row but not enough to diagnose why an order stopped, missed a reprice, retried,
or required manual checking.

Recommended event log entries:

- Start validation accepted/rejected.
- Initial book loaded.
- Place sent/accepted/ambiguous/filled/failed.
- Account refresh complete/incomplete.
- Reprice queued/sent/cleared.
- Modify sent/accepted/ambiguous/filled/failed/retry-delayed.
- Cancel sent/accepted/ambiguous/failed/retried/manual-check.
- Completion, stop, timeout, drift limit, max-reprice limit.

### P2 - Documentation Mentions Retired Field Names

`docs/advanced-orders/chase-orders.md` still describes older names such as
`pending_op`, `pending_best_price`, `stop_requested`, `oid_confirmed`, and
`missing_open_order_refresh_requested`. The current model uses the
`ChaseLifecycle` enum and `desired_price` instead. The behavior narrative is
mostly still correct, but the state-field list should be updated before using
that document as an implementation reference.

## Metric Set To Treat As Authoritative

The following metric set is the minimum useful target for a complete Chase
audit trail.

| Priority | Metric | Why it matters |
| --- | --- | --- |
| P0 | Target size, filled size, remaining size, residual size | Confirms whether the Chase executed the requested quantity. |
| P0 | Fill VWAP and gross notional | Measures actual execution quality. |
| P0 | Last working price and initial price | Separates execution result from current/last quote state. |
| P0 | Status, terminal reason, and error flag | Makes final outcome unambiguous. |
| P0 | Account, coin, side, reduce-only, spot/perp | Defines the scope and risk context. |
| P0 | OID/CLOID set | Allows reconciliation against exchange data and fills. |
| P1 | Fees, fee rate, closed PnL, net notional | Measures all-in execution cost. |
| P1 | Reprice count, place attempts, cancel retries | Shows operational effort and failure risk. |
| P1 | Runtime, time to first rest, time to first fill, time to completion | Measures responsiveness and opportunity cost. |
| P1 | Request latencies for place/modify/cancel/status/account refresh | Separates exchange/network delays from app logic. |
| P1 | Drift from initial price and spread at each action | Captures risk and market quality. |
| P1 | Queue time and skipped reprice reasons | Explains why Chase did or did not move. |
| P2 | Per-OID child ledger | Enables incident review after replacements or ambiguous results. |
| P2 | Full lifecycle event log | Makes manual check and support cases reconstructible. |

## Recommended Backlog

1. Fix Chase history `average_price` to use fill VWAP, and preserve current
   working price separately.
2. Add Chase fill totals for fees, closed PnL, gross notional, and net notional.
3. Add a Chase child-order ledger to `AdvancedOrderHistoryEntry.children`.
4. Add lifecycle event records to Chase runtime state and copy them into
   history on archive.
5. Instrument request sent/settled timestamps and queue durations.
6. Track quote/spread snapshots and ignored reprice reasons.
7. Update `docs/advanced-orders/chase-orders.md` to match the current
   `ChaseLifecycle` and `desired_price` model.

## Verification Notes

This audit was based on current worktree inspection of:

- `src/signing/model/chase/order.rs`
- `src/signing/model/chase/lifecycle.rs`
- `src/order_execution/chase.rs`
- `src/order_execution/chase/lifecycle/reprice.rs`
- `src/account_update/stream/fills.rs`
- `src/account_update/stream/chase/refresh.rs`
- `src/order_views/advanced/rows.rs`
- `src/advanced_order_history/snapshots.rs`
- `docs/advanced-orders/chase-orders.md`

No runtime behavior was changed by this audit.
