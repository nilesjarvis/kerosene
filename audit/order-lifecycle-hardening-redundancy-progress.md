# Order Lifecycle Hardening and Redundancy Progress

## Baseline

- Starting HEAD: `cbd48106a75fcbeb982f5e6ed2d53772bbf7123b`
- Branch: `main` (`origin/main` plus the committed goal prompt)
- Pre-existing dirty paths: none
- Architecture snapshot date: 2026-07-10
- Baseline validation:
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed.
  - `cargo test --package kerosene --bin kerosene signing::tests` did not
    compile Kerosene because `alsa-sys` could not find the system `alsa.pc`
    package through `pkg-config`.
  - `cargo check` stopped at the same missing system ALSA dependency before
    checking Kerosene.
- Validation constraint: do not describe Rust tests or type-checking as passed
  until the system ALSA development package is available. Continue using
  source inspection, formatting, diff checks, and focused test additions, and
  rerun the blocked commands when the environment permits.

## Verified Architecture Snapshot

- All signed L1 mutations converge in `src/signing/client.rs` through
  `sign_and_post`. The current action set is place, cancel by OID, cancel by
  CLOID, modify, and leverage update (`src/signing/actions.rs:28-34`,
  `src/signing/client.rs:234-293`).
- Place and modify preparation preserves asset, side, price and size strings,
  TIF/order kind, reduce-only, OID/CLOID, and action field order without a
  conversion after `PreparedExchangeOrder`/`PreparedModifyOrder`. The final
  signed-payload boundary now independently rejects non-positive/non-finite
  numeric strings and a missing or malformed placement CLOID before hashing or
  posting (`src/signing/actions.rs:36-158`, `src/signing/client.rs:106-135`).
- User and automation placement paths use `place_order_task`; cancel/modify
  paths use the corresponding task wrappers in `src/order_execution/core.rs`.
  A repository-wide call-site search found no feature-owned signing pipeline.
- General placement preparation is centralized by `PlaceIntent` and
  `PreparedExchangeOrder` in `src/order_execution/core.rs:51-61` and
  `src/order_execution/core.rs:326-380`. Chase, TWAP, and NUKE construct
  prepared values after their own state-machine/planner validation, then reuse
  the same task/signing boundary.
- One-shot placements receive a 128-bit hashed CLOID and immutable
  `OneShotPlacementContext`; uncertain results use `orderStatus` by CLOID plus
  an account refresh (`src/order_execution/core.rs:354-380`,
  `src/order_update/results.rs:616-663`).
- One-shot result classification distinguishes accepted-resting, filled,
  cancelled, rejected, ambiguous, and transport-unknown outcomes
  (`src/order_update/results.rs:15-30`, `src/order_update/results.rs:173-215`).
- Signed mutation transport uses a dedicated client with no redirects or HTTP
  retries, a 5-second connect timeout, a 15-second total timeout at both client
  and request scope, and a 60-second idle-pool timeout. Every local-build,
  connect/send, body-read, and JSON-parse failure remains conservatively
  transport-unknown and is order-aware redacted before task mapping into
  `Message`; no placement or modify is automatically retried. Parsed responses
  also preserve uncertainty whenever an explicit error conflicts with a
  structured possible effect (`src/signing/client.rs:74-232`,
  `src/signing/model/exchange_response/analysis.rs:83-144`).
- Order-status REST parsing validates the returned OID/CLOID for concrete order
  bodies before handing a result to lifecycle code. Both public task exits now
  sanitize every error before message mapping; HTTP previews redact before
  truncation; correlation failures are value-neutral; and successful result
  diagnostics retain only redacted/boolean identifier metadata
  (`src/api/order_status.rs:16-82`, `src/api/order_status/parsing.rs:10-62`,
  `src/api/order_status/model.rs:8-24`).
- Transient order-message OID/CLOID fields use exact-value wrappers whose
  diagnostic representations are redacted. Producers wrap only when publishing
  a message, and the order/chart update boundary restores the original
  primitive before invoking unchanged handlers. This covers cancel, move,
  Chase, TWAP, and chart cancel-hover messages without changing runtime state or
  exchange types (`src/message.rs:254-308`, `src/message.rs:1141-1429`,
  `src/order_update.rs:80-104`, `src/order_update.rs:205-431`,
  `src/chart_update.rs:218-240`).
- Cancel and move direct/status callbacks carry a collision-aware runtime
  request sequence in addition to account, symbol, and OID. Cancel keeps one
  immutable request owner across its awaiting-result and checking-status
  phases; move keeps the sequence and exact prepared target price in its
  captured-key context and status request. Successful refresh cleanup requires
  the origin symbol's open-order lane; move cleanup additionally distinguishes
  target absence, the expected price, and a different valid live price. These
  values are local correlation/reconciliation only and do not change the signed
  action or add mutation retries (`src/order_execution.rs:992-1053`,
  `src/order_update/results.rs:90-290`,
  `src/order_update/results.rs:761-788`).
- Connected-account task results carry read-provider, request-generation, and
  dispatch-time account-revision context. Address-scoped user-data changes
  supersede an earlier request before it can replace state or drive lifecycle
  reconciliation; an initial-load frame with no mergeable base queues one
  post-frame refresh. Websocket lag retains its existing reconciliation path
  (`src/read_data_provider.rs:23-157`,
  `src/account_update/connection/refresh.rs:94-213`,
  `src/account_update/stream.rs:45-230`).
- User-data application messages carry their complete stream parameters,
  including a runtime-only consumer generation whose diagnostics redact the
  address. Account, wallet-detail, and wallet-cluster routes require an exact
  current recipe plus its normalized emitted source before applying state.
  Generation rotation covers same-address account sessions, detail-window
  disappearance/recreation (including connected-address exclusion), selected
  cluster topology/profile bindings, and visible-dex changes
  (`src/ws/user_streams.rs:20-94`,
  `src/subscription_state/user_data.rs:11-213`,
  `src/account_update.rs:97-103`,
  `src/wallet_cluster_update.rs:92-98`).
- Chase and TWAP retain captured account/key identity and explicit lifecycle or
  pending-operation state. Their active state and in-flight requests are
  runtime-only. Each TWAP CLOID status task now has one separately armed retry
  attempt because the CLOID remains live during later account-fill repair.
  Unexpected-child cancellation likewise separates its scheduled retry count
  from the exact attempt that owns an in-flight target-specific cancel task.
  Authoritative TWAP fill size, price, fee, remaining target, and completion
  derive from per-child fill metrics independently of that child's display
  status. Terminal TWAPs cannot schedule slices, status checks, cancel retries,
  or timer ticks; a retryable cancel result that was already in flight still
  requests immediate account reconciliation but no longer creates a delayed
  retry trigger after authoritative fills complete the TWAP.
- Config snapshots contain terminal advanced-order history but no live
  Chase/TWAP maps, pending order contexts, or captured signing keys; boot
  reconstructs those runtime owners empty. When main-window closure leaves the
  daemon alive for a final config write, the exit flag now stays armed through
  the exit task and independently fences new Chase place/modify progress and
  TWAP slices. Status reconciliation and exposure-reducing cancellation remain
  available, and a failed save clears the fence so queued automation resumes.
- Chase reconciliation for fills, refreshes, stops, archives, and final
  replacements now derives open-order authority independently for each active
  Chase's origin symbol. Account-wide fill completeness remains a separate
  requirement, and websocket disappearance remains dex-scoped; an unrelated
  scoped REST lane cannot authorize absence or a new placement
  (`src/account_update/stream/chase/fill_reconciliation.rs:17-146`,
  `src/account_update/stream/chase/refresh.rs:37-292`,
  `src/order_execution/chase/lifecycle/place.rs:247-290`).
- Wallet-cluster fan-out has a separate update route, but each member leg uses
  shared preparation/signing, its own CLOID, and a tuple of execution ID,
  profile secret ID, and CLOID for local correlation.
- This structure remains appropriate: the campaign should harden handoffs and
  idempotence, not replace it with a monolithic order manager.

## Lifecycle Assurance Matrix

| Surface/operation | Immutable origin identity | Correlation key | Idempotency key | Immediate-result classifier | Authoritative reconciliation | Stale-result guard | Terminal cleanup | Existing tests | Gaps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Ticket place | Submission snapshot; captured account/key; `OneShotPlacementContext` with account, surface, symbol, kind | Indicator ID + placement context; request ID after uncertain result | Unique one-shot CLOID | `classify_execution_result` | `orderStatus` by CLOID + connected-account refresh; fills consume market projections | Serialized pending gate; snapshot equality; current-account and pending-request match | Clears global action/indicator; exact status or a complete open-order/fill snapshot covering the origin symbol removes the status request | `order_execution/submit/tests.rs`, `order_execution/core.rs` tests, `order_update/results/tests.rs` | F-02 addressed in Turn 3; executable regression validation remains environment-blocked by missing ALSA metadata |
| Preset place | Preset preflight then the shared ticket fields/context with `OrderSurface::Preset` | Same as ticket | Unique one-shot CLOID | Shared classifier | Shared one-shot reconciliation | Pending/reconciliation gates run before preflight and again on submit | Shared ticket/result cleanup | `order_update/presets.rs` tests | No confirmed gap; retain double preflight as deliberate queued-event defense |
| Alfred place | Parsed draft preflight; then current form and captured signing context | Same ticket result message/context | Unique one-shot CLOID | Shared classifier | Shared one-shot reconciliation | Alfred preflight plus shared submit gates | Shared ticket/result cleanup | `alfred_update/submit.rs` tests and ticket tests | Verify command-to-form handoff in the later origin-identity track |
| Quick place | Chart ID/surface snapshot and recovery data; captured account; one-shot context | Indicator ID + CLOID context; optional form recovery | Unique one-shot CLOID | Shared classifier | Shared CLOID status + account refresh | Chart/surface/symbol and percentage provenance checks; current-account match | Clears global action/indicator; rejection may restore matching form | `order_execution/quick_order/submit/tests.rs`, `order_update/quick_order/form/tests.rs` | No confirmed gap |
| HUD place | `HudOrderRequest` captures chart/surface/symbol/side; account context; one-shot context | Market: global action + CLOID. Limit: HUD in-flight ID + indicator + CLOID | Unique one-shot CLOID | Shared classifier | Shared CLOID status + account refresh | Chart/surface/symbol/arm checks; per-account limit tracker; current-account match | Market clears global action; limit finishes its tracker entry; both clear indicator | `order_update/hud.rs` tests, `order_execution/hud.rs` tests | Per-tracker ID wraps without collision handling; practically remote, audit with other allocators |
| Close-position place (UI or Alfred) | Fresh connected-account position snapshot; account/key; coin/fraction; one-shot context | Global close action + indicator + CLOID context | Unique one-shot CLOID | Shared classifier | Shared CLOID status + account refresh | Pending/reconciliation/freshness/completeness gates; current-account match | Clears close action/indicator; shared one-shot terminal handling | `order_execution/position_actions/close/tests/**`, result tests | No confirmed gap |
| NUKE child place (UI or Alfred) | Parent execution ID; connected account; planner output; per-child one-shot context | Execution ID + child CLOID in the result context and aggregate settlement set | Unique one-shot CLOID per child; the first terminal transition claims it | Shared classifier | Uncertain child gets CLOID status; parent refreshes after aggregate completion | Current-account and execution-ID checks; duplicate settlement is a no-op | CLOID-keyed confirmed/failed/uncertain totals; parent removed after the unique settled-child count reaches total | `order_execution/position_actions/nuke/tests/**`, direct/status duplicate regressions in `order_update/results/tests.rs` | F-01 addressed in Turn 2; executable regression validation remains environment-blocked by missing ALSA metadata |
| Cancel by OID | Connected account + symbol + OID + runtime request sequence; one request owns awaiting-result/checking-status phases | Request sequence + account + OID + symbol; indicator is presentation only | Target OID; runtime sequence is correlation only | Shared classifier plus confirmed-cancel predicate | `orderStatus` by OID + origin-symbol open-order refresh | Exact request/phase/account match for direct result; exact request/account/OID/symbol match for status; refresh must cover origin lane | Confirmed/terminal result removes matching local order; only status-check phase and a covering snapshot can release refresh uncertainty | `order_execution/position_actions/cancel.rs` tests, cancel result/status duplicate, stale-attempt, and scoped-refresh tests | F-14/F-15 addressed in Turns 13-14; executable validation remains environment-blocked |
| Move/modify | Connected account + symbol + OID + runtime request sequence; original key and exact prepared target captured in `PendingMoveOrderContext` | Request sequence + account + symbol + OID; indicator ID is presentation/local projection only | Target OID; runtime sequence is correlation only | Shared classifier plus confirmed-modify predicate | `orderStatus` by OID + origin-symbol refresh that classifies target absent/expected-price/different-valid-price | Exact request/account/move-key context for direct result; exact request/account/OID/symbol for status; refresh rejects uncovered or malformed target evidence | Removes only the matching move context/indicator; terminal status or sufficient target-lane evidence clears status uncertainty | `order_execution/quick_order/move_order/tests/**`, `order_update/move_order.rs` duplicate/stale-attempt/scoped-refresh tests | F-14/F-15 addressed in Turns 13-14; executable validation remains environment-blocked |
| Chase place/replacement | `ChaseOrder` captures ID, account, agent key, symbol, side, sizes, start time, lifecycle | Chase ID + lifecycle + dispatch-time place attempt; current CLOID is checked by status path | CLOID hashes account + chase ID + start + attempt | Chase-specific strict response analysis | CLOID status + account-wide fills + origin-symbol open-order refresh/stream reconciliation | Exact place-attempt equality + `expects_place_result`; current account, symbol identity, prior-exposure, per-symbol open-order authority, reconciliation, and final-exit progress gates | Moves to verification/resting/stop/archive; late stopped placement is cancelled; final replacement gate repeats origin-lane proof; exit-pending place remains queued | Chase lifecycle/place/result/status tests, duplicate/late direct-result regressions, account stream tests, wrong-scope replacement tests, and exit-fence/resumption tests | F-05/F-16/F-23 addressed in Turns 7/15/22; executable regression validation remains environment-blocked by missing ALSA metadata |
| Chase modify | Chase ID + captured account/key + current OID + lifecycle + desired price | Chase ID + OID + dispatch-time reprice count + `expects_modify_result` | Target OID; no separate exchange idempotency key (runtime sequence is correlation only) | `is_confirmed_modify_result` and Chase-specific error handling | OID status + account-wide fills + origin-symbol open-order refresh/stream reconciliation | Exact reprice-count/lifecycle/OID match; account, symbol, origin-lane, reconciliation, and final-exit progress checks before correction/replacement | Verification/resting/stop flow; terminal Chase archived only from covering evidence; exit-pending reprice/size correction remains queued | `order_update/chase/modify/tests/**`, including duplicate/late results, wrong-scope refresh, and exit-fence/resumption tests | F-05/F-16/F-23 addressed in Turns 7/15/22; executable regression validation remains environment-blocked by missing ALSA metadata |
| Chase cancel | Chase ID + captured account/key + OID + stopping phase | Chase ID + OID + `expects_cancel_result` | Target OID; bounded retry treats terminal-not-open responses specially | Confirmed-cancel predicate plus Chase cancel classification | OID status + account-wide fills + origin-symbol account refresh/open-order disappearance | Exact stopping phase/OID; REST archive requires the origin open-order lane; websocket disappearance is dex-scoped | Verifying-cancel then covering-snapshot archive; bounded manual-check terminal | `order_update/chase/cancel/tests.rs`, Chase stop/status tests, and wrong-scope archive regression | F-08/F-16 addressed in Turns 9/15; retry idempotence depends on target-specific cancel semantics; executable validation remains environment-blocked |
| TWAP child place | `TwapOrder` captures ID/account/key/symbol/plan; `TwapPendingSlice` captures index/size/price/CLOID/retry | TWAP ID + dispatch-time slice index/retry count + current `pending_op`; status path adds exact CLOID and armed retry attempt | Deterministic child CLOID (runtime index/retry tuple is correlation only) | TWAP-specific IOC/fill/resting/transport classification | CLOID status + scoped account-fill refresh + reconciliation deadline | Exact pending slice/retry for placement; status result requires current CLOID/attempt; current-account, terminal, and final-exit progress guards remain | Finishes attempt once; child status/fills updated; status ownership cleared on result or account-fill resolution; terminal TWAP archived; exit-pending due slice stays unsent | `order_execution/twap/tests/**`, including duplicate/late slice/status results and exit-fence/resumption, plus `twap_state/tests/**` | F-05/F-19/F-23 addressed in Turns 7/18/22; executable regression validation remains environment-blocked by missing ALSA metadata |
| TWAP unexpected-child cancel | TWAP ID + captured key + OID/CLOID target + exact armed retry attempt | Pending target plus current retry count and one in-flight attempt; retry and result messages both carry the attempt | Target-specific cancel by CLOID preferred, else OID | Confirmed-cancel/terminal-not-open/error handling | Immediate origin-account refresh; child status and later fills | Dispatch atomically requires non-terminal exact target/retry with no existing owner; result requires exact target/retry/owner | Consumes one owner, then clears pending cancel and finishes once or schedules the next bounded attempt; a result arriving after fill terminalization retains refresh but cannot schedule retry work | `order_execution/twap/tests/cancel.rs`, placement/status entry-path tests, fill/cancel and terminal-result characterizations | F-08/F-20/F-22 addressed in Turns 9/19/21; F-21's delivery-order-dependent child label is deferred for an explicit UX/history semantics decision; financial accounting is order-independent |
| Wallet-cluster order child | Execution ID + profile secret ID + member address/key + one-shot context | Execution/profile/CLOID plus account, symbol, surface, and order kind | Unique one-shot CLOID per member leg; direct result may leave `Pending` once | Shared classifier | CLOID status + member refresh + member user stream | Full origin match; direct requires `Pending`, status requires `Checking`; pending executions are not evicted | First terminal leg outcome is immutable; execution complete when every leg terminal | Cluster planning/member tests plus adversarial result/status tests in `wallet_cluster_update.rs` | F-04 addressed in Turn 5; executable regression validation remains environment-blocked by missing ALSA metadata |
| Wallet-cluster close child | Same as cluster order, plus fresh per-member position snapshot and reduce-only plan | Same full correlation tuple with `ClusterClose` surface | Unique one-shot CLOID per member leg; direct result may leave `Pending` once | Shared classifier | CLOID status + member refresh + member stream | Freshness/side/position preflight plus the shared exact transition guard | Same first-terminal-wins handling as cluster orders | Close sizing/freshness tests and shared adversarial result tests | F-04 addressed by the shared Turn 5 transition guard |
| Leverage update | `PendingLeverageUpdateContext` captures account, symbol, asset, dex, margin mode, leverage | Full pending-context equality | None; mutation is not blindly retried | Confirmed-default predicate; other non-error bodies are uncertain | Scoped account refresh for outcomes that may have committed | Pending-context equality + current-account match | Pending context cleared once; matching form updated only on confirmed default | `order_update/leverage.rs` tests, signing action/response tests | No exact mutation status endpoint; verify refresh completeness is sufficient in transport audit |

Turn 8 verified that every placement and modify row above reaches the shared
signed-action builder. Valid field mapping is direct and covered at both the
prepared-request and signed JSON/msgpack boundaries. The cross-cutting F-07
guard applies to every such row without duplicating market, sizing, precision,
side, reduce-only, or asset/symbol policy in signing.

Turn 9 verified the parsed-response classification for every shared and
advanced mutation handler. The cross-cutting F-08 guard applies to every caller
of the shared classifier, including one-shot, quick/HUD, cancel, move, NUKE, and
wallet-cluster results, and explicitly to Chase place/modify/cancel plus TWAP
child-place/unexpected-cancel handling. Pure errors, unambiguous successes,
transport failures, and unstructured malformed responses retain their prior
paths; only simultaneous error/effect claims reconcile.

Turn 16 adds a causal precondition to every matrix row that uses the connected
account snapshot as authoritative fallback: a successful REST result may reach
one-shot, cancel/move, Chase, connected-account TWAP, leverage, or overlay
reconciliation only when `account_data_revision` is unchanged since dispatch.
If a user-data delta advanced it, the current merged state and uncertainty are
preserved while one sequential post-delta refresh starts. Off-account TWAP fill
reconciliation remains separately address/generation-scoped and does not
replace connected account state.

Turn 17 adds a preceding provenance requirement to every row consuming account
or selected-cluster user data: the frame's address, purpose, dex scope, and
runtime recipe generation must still equal the subscription requested by
current application state. An old same-address recipe therefore cannot drive
order, fill, Chase/TWAP, or cluster-close freshness transitions after account,
window, cluster, profile, or market-scope replacement.

Turn 18 adds exact ownership to TWAP child-status repair. At most one lookup is
armed for the current CLOID/retry attempt, and only that attempt can consume a
result. The CLOID can remain set independently while a definitive exchange
status waits for account-fill confirmation without allowing a duplicate or
older status result to rewrite that later reconciliation phase.

Turn 19 extends exact ownership to TWAP unexpected-child cancellation. A
target-specific retry trigger must atomically arm the current attempt before it
can dispatch, and only a result carrying that target and attempt can consume
retry budget or finish the pending cancellation. Scheduled backoff remains a
separate phase with no in-flight owner.

Turn 20 verifies both fill-versus-cancel delivery orders. Per-child and
aggregate fill quantities, average price, fee, remaining target, completion,
history financial metrics, and repeated-fill idempotence are equivalent. The
child label is deliberately not normalized: current live/history rendering
exposes whether `Filled` or `UnexpectedRestingCancelled` wrote last, and
choosing a dominant or combined label requires the deferred F-21 product
decision.

Turn 21 closes the remaining TWAP terminalization plumbing audit. Terminal
statuses reject slice scheduling, timer subscriptions, status dispatch, cancel
retry dispatch, and delayed status mutation. Terminal archive calls upsert one
history identity and scrub the captured key. If full-fill reconciliation wins
while an unexpected-child cancel is in flight, a retryable late result now
keeps the immediate origin-account refresh without creating a delayed trigger
that terminal state could only reject.

Turn 22 begins Track 9 by proving the restart persistence boundary and fencing
the live final-save interval. Live automation/request owners remain absent from
config and boot empty. After the main window closes, the final-exit flag remains
set until process exit; Chase place/reprice/size correction and due TWAP slices
stay queued, while exact status repair and exposure-reducing cancel paths retain
their existing authority. A failed final save clears the flag and the same
queued values resume through their normal gates.

### Mutation Transport Phase Audit

| Boundary | Can the exchange have observed the mutation? | Downstream treatment | Replay/retry behavior |
| --- | --- | --- | --- |
| Wire validation, msgpack, key decode/signing, JSON serialization, or dedicated-client initialization | No | Preserved `Err` shape; existing handlers conservatively classify or handle it as status unknown | No HTTP dispatch and no application retry |
| Request construction or connect failure | No for builder/connect failures, but reqwest's public error string is deliberately not used as a financial proof | `TransportUnknown`; exact CLOID/OID status and/or scoped account refresh | Dedicated client has retries and redirects disabled |
| Send error or 15-second timeout | Possibly; bytes may have left the process | `TransportUnknown`; exact reconciliation | No transport replay and no placement/modify retry |
| HTTP response body read failure | Yes | `TransportUnknown`; exact reconciliation | No retry |
| Non-JSON or structurally unparseable HTTP body, regardless of HTTP status | Yes | `TransportUnknown`; redacted bounded body snippet, then exact reconciliation | No retry |
| Non-success HTTP envelope with apparently successful/incomplete parsed body | Yes | F-11 `TransportUnknown`; exact reconciliation | No retry |
| Parsed exchange rejection under any HTTP status | Yes, with explicit no-effect error | `Rejected`; no ambiguity cleanup is released by inference | No retry |
| Parsed success or conflicting effect/error statuses | Yes | Strict success classifier, or F-08 `Ambiguous` reconciliation on conflict | No retry |

The result type intentionally remains `Result<ExchangeResponse, String>` in
this turn. Distinguishing provably local errors in downstream UX would change
form recovery, refresh, and automation failure paths without improving safety;
the existing over-approximation is fail-closed. The transport boundary instead
prevents hidden redirects/replays, bounds every request, and sanitizes both
`Ok` and `Err` diagnostic formatting before derived `Message::Debug` can observe
the result. Existing bounded Chase/TWAP cancel retries are lifecycle-level,
target-specific cancellation policy, not HTTP replay.

## Ranked Findings and Audit Candidates

### F-01 — NUKE child aggregation is not idempotent

- Status: addressed in Turn 2; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium, with a High consequence if duplicate delivery occurs during
  a multi-child emergency close
- Scope: NUKE result and status-result plumbing
- Preconditions/event ordering:
  1. A NUKE parent has at least two child placements.
  2. The same child's `NukeResult` or `NukePlacementStatusLoaded` is handled
     twice before a different child settles.
  3. Both messages carry the current execution ID and account context.
- Evidence:
  - `PendingNukeExecution` stores totals only and every `record_*` call
    increments `completed` (`src/order_execution.rs:85-129`).
  - `handle_nuke_result` and `handle_nuke_placement_status_result` receive a
    child CLOID in context but pass only `execution_id` into the record helpers
    (`src/order_update/results.rs:466-510`, `src/order_update/results.rs:785-845`).
- Violated invariant: one logical child contributes exactly once to its parent
  aggregate.
- Risk: the parent can reach `completed >= total`, clear its pending blocker,
  and report completion while another child is still in flight.
- Implemented fix: NUKE parents retain a runtime-only set of settled child
  CLOIDs. The first confirmed, failed, or uncertain terminal transition claims
  its CLOID; subsequent transitions for that CLOID cannot increment any count
  or finish the parent. Completion is derived from the number of unique settled
  children (`src/order_execution.rs:84-186`,
  `src/order_update/results.rs:470-522`,
  `src/order_update/results.rs:813-886`).
- Regression coverage: direct exchange results and `orderStatus` results each
  deliver the same child twice in a two-child execution, assert the unchanged
  `1/2` progress text and pending parent, then settle a distinct CLOID
  (`src/order_update/results/tests.rs:995-1072`). The direct regression also
  proves `PendingNukeExecution` debug output does not expose its retained CLOID.
- Protected behavior: unique child outcomes retain the existing confirmed,
  failed, uncertain, skipped, refresh, error-state, and status-text behavior.
  The change does not affect request construction, signing, dispatch, order
  semantics, views, persistence, or user interaction timing.

### F-02 — Successful refresh may clear unresolved one-shot status state too broadly

- Status: addressed in Turn 3; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: shared one-shot placement reconciliation for ticket, preset, Alfred,
  quick-order, HUD, and close-position surfaces
- Preconditions/event ordering:
  1. A one-shot placement has an unresolved CLOID status request.
  2. The exact `orderStatus` response is missing, non-definitive, or fails.
  3. A connected-account refresh returns `Ok(AccountData)` while open orders or
     fills are incomplete, or while its open-order scope excludes the origin
     symbol's market.
  4. The account-wide cleanup treats that snapshot as resolution.
- Evidence:
  - Open-order and fill requests are best-effort. Failures mark their sections
    incomplete but the bootstrap still returns `Ok(AccountData)`
    (`src/account/data/bootstrap.rs:160-228`,
    `src/account/data/bootstrap/responses/best_effort.rs:26-38`).
  - `AccountDataFetchScope` can contain only one HIP-3 dex, and its completeness
    applies to that fetched scope (`src/account/types/data/fetch_scope.rs:40-73`).
  - The prior pending record discarded `symbol_key`, and successful refresh
    cleanup removed every request for the account without inspecting either
    completeness lane or scope.
- Violated invariant: fallback reconciliation may release an uncertain
  placement only when the snapshot contains both independent outcome lanes —
  open orders for the placement's origin market and account-wide fills.
- Risk: a partial or unrelated-market snapshot can release the pending-trading
  blocker while an open or filled order remains absent from local account state,
  allowing the next mutation to be prepared from incomplete exposure.
- Implemented fix: pending one-shot status records retain the runtime-only
  origin symbol. Refresh cleanup now removes each account-matching request only
  when fills are complete and the snapshot successfully fetched that symbol's
  open-order lane. The account-data helper uses per-market fetch timestamps so
  an unrelated dex failure does not invalidate a healthy origin lane
  (`src/account/types/data.rs:198-213`,
  `src/order_update/results.rs:33-86`,
  `src/order_update/results.rs:583-606`).
- Regression coverage: incomplete open-order and incomplete fill snapshots both
  retain the request and trading blocker; a complete snapshot for another
  HIP-3 dex also retains them; a later complete snapshot covering the origin
  dex performs the existing cleanup (`src/order_update/results/tests.rs:660-729`).
- Protected behavior: exact CLOID status handling is unchanged. Existing tests
  continue to characterize that a complete, covering fallback refresh clears
  missing, cancelled, or errored status requests with the same status text and
  blocker behavior (`src/order_update/results/tests.rs:578-658`). No request,
  signing, order semantics, view, persistence, or normal-path timing changed.

### F-03 — Pending one-shot debug output exposes the CLOID

- Status: addressed in Turn 4; focused test added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium privacy hardening
- Scope: `PendingOneShotStatusRequest` diagnostic formatting only
- Evidence: the formatter redacted the account address but previously emitted
  the full CLOID, while `OneShotPlacementContext`, signed wire types, and order
  status result models already redact the same identifier.
- Violated invariant: a sensitive order correlation identifier retained for
  lifecycle matching must not be exposed through general diagnostic formatting.
- Risk: formatting parent application state or a pending request during
  diagnostics could disclose a stable identifier derived from account and order
  inputs even though adjacent boundaries are redacted.
- Implemented fix: the existing `cloid` debug field now emits `<redacted>` while
  retaining the same struct/field shape (`src/order_update/results.rs:42-50`).
- Regression coverage: the focused formatter test requires explicit redaction
  markers for both the account address and CLOID and rejects both synthetic raw
  values (`src/order_update/results/tests.rs:175-185`).
- Protected behavior: storage, equality, request correlation, status handling,
  UI strings, order semantics, and persistence are unchanged. Only `Debug`
  output differs.

### F-04 — Wallet-cluster result transitions lack focused correlation/idempotence coverage

- Status: addressed in Turn 5; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium
- Scope: wallet-cluster order and close direct-result/status-result plumbing
- Preconditions/event ordering:
  1. A cluster execution retains a pending, checking, or terminal member leg.
  2. A stale or mismatched result reaches the current execution ID, or a direct
     or status result is delivered more than once/out of its expected phase.
  3. The prior helper finds no leg or finds an already-terminal leg but does not
     report whether the update was valid.
  4. The handler still rewrites state or launches repair/refresh and aggregate
     status work.
- Evidence:
  - The direct and status handlers previously assigned any target status to a
    matching execution/profile/CLOID leg regardless of its current phase.
  - The void update helper could not tell callers that an execution, profile, or
    CLOID did not match, so they still refreshed a member, launched CLOID status
    repair, and/or recomputed aggregate UI state.
  - Handler-level adversarial cases prove a confirmed leg could be rewritten as
    `Checking` by a late ambiguous direct result or as `Failed` by a conflicting
    duplicate; a status result could settle a leg that was never `Checking`.
- Violated invariant: each cluster child accepts exactly one direct result from
  `Pending`; only an ambiguity-created `Checking` child accepts one status
  result; terminal outcomes are immutable.
- Risk: duplicate or misrouted lifecycle messages can regress a completed
  execution to pending, change confirmed member outcomes into failures, produce
  false aggregate problem counts, and launch irrelevant repair tasks.
- Implemented fix: one transition helper now verifies execution ID, profile
  secret ID, CLOID, account address, symbol, cluster/close surface, exchange
  order kind, and expected source phase before mutating a leg. Direct handlers
  require `Pending`; status handlers require `Checking`; failed transitions
  return before any repair, refresh, aggregate status, or message mutation
  (`src/wallet_cluster_update.rs:1096-1272`).
- Regression coverage: exact-origin mismatch cases cover execution, profile,
  CLOID, address, symbol, surface, and order kind; both order and close legs
  reject terminal-to-checking and terminal-to-failed duplicates; status results
  are ignored before `Checking`, settle it once, and cannot rewrite the terminal
  outcome (`src/wallet_cluster_update.rs:1652-1834`).
- Protected behavior: valid direct transitions remain
  `Pending -> Confirmed|Failed|Uncertain|Checking`, and valid reconciliation
  remains `Checking -> Confirmed|Failed|Uncertain`. Existing messages, aggregate
  status strings, refresh behavior, fan-out sizing, order preparation, signing,
  UI, and persistence are unchanged for unique correctly correlated results.

### F-05 — Advanced place/modify result messages rely on lifecycle state rather than per-attempt tokens

- Status: addressed in Turn 7; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium invariant hardening
- Scope: Chase place, Chase modify, and TWAP child-place direct-result message
  correlation
- Preconditions/event ordering:
  1. A direct result for attempt N is handled and advances the strategy.
  2. The same strategy later dispatches attempt N+1 and returns to `Placing`,
     `Modifying` with the same OID, or a TWAP `Place` pending operation.
  3. A duplicate or delayed message for attempt N arrives while attempt N+1 is
     in that recurring coarse phase.
  4. The old handler sees a valid current phase (and, for modify, the same OID)
     and applies the earlier exchange outcome to the current attempt.
- Evidence:
  - Chase replacement placement already increments `place_attempt_count` and
    derives a distinct CLOID, but the result closure previously discarded the
    attempt (`src/order_execution/chase/lifecycle/place.rs:358-394`).
  - Chase increments `reprice_count` before each modify, while Hyperliquid may
    retain the OID, but the result message previously retained only the OID
    (`src/order_execution/chase/lifecycle/reprice.rs:333-348`).
  - TWAP pending state already owns a slice index and retry count. A retry
    reuses its logical child and CLOID, but the direct result previously carried
    only the TWAP ID (`src/order_execution/twap/execution.rs:308-318`).
  - The immediate-duplicate phase guards were sound, but they did not identify
    which dispatch owned a result once the same phase recurred.
- Violated invariant: an asynchronous mutation result may settle only the exact
  in-flight attempt whose dispatch created it.
- Risk: a stale rejection, fill, ambiguity, or acceptance can fail, credit,
  pause, verify, cancel, or otherwise settle a later Chase/TWAP attempt despite
  referring to earlier exchange work.
- Implemented fix: the three internal result messages now capture the safe
  runtime sequence already assigned before dispatch: Chase place attempt,
  Chase reprice count, or TWAP slice index plus retry count. Result handlers
  require exact sequence equality in addition to their existing lifecycle,
  OID, and pending-operation checks (`src/message.rs:1170-1175`,
  `src/message.rs:1214-1223`, `src/order_update/chase/result.rs:126-157`,
  `src/order_update/chase/modify.rs:35-59`,
  `src/order_execution/twap/slice_result.rs:38-61`). No CLOID, account value,
  or new persisted/runtime state was added. A mismatched sequence returns
  before account-refresh or status-repair follow-up; a same-attempt result that
  arrives after another lifecycle transition retains the former conservative
  refresh behavior.
- Regression coverage: each surface receives a conflicting duplicate after the
  first direct result has advanced its phase. Separate late-result cases put a
  newer Chase place attempt, same-OID Chase reprice, or TWAP slice/retry in
  flight and prove an earlier result cannot mutate it
  (`src/order_update/chase/result/tests.rs:154-209`,
  `src/order_update/chase/modify/tests/success.rs:58-115`,
  `src/order_execution/twap/tests/place_result.rs:103-188`). The TWAP case
  independently checks both slice-index and retry-count mismatch, and the
  Chase-place/TWAP cases prove stale outcomes cannot launch account refresh.
- Protected behavior: an exactly matching direct result follows the same
  classifier, reconciliation, fill, retry, stop, archive, refresh, and visible
  status paths as before. Chase/TWAP scheduling, pricing, sizing, repricing,
  CLOIDs, signed requests, UI, persistence, and normal timing are unchanged.

### F-06 — Wallet-cluster leg debug output exposes its lifecycle message

- Status: addressed in Turn 6; focused test added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium privacy hardening
- Evidence: `WalletClusterExecutionLeg::Debug` redacts the explicit CLOID,
  address, symbol, size, and price fields but formats `message` verbatim
  (`src/wallet_cluster_state.rs:220-236`). Unexpected-resting result messages
  embed the CLOID and exchange summary (`src/wallet_cluster_update.rs:1141-1149`,
  `src/wallet_cluster_update.rs:1189-1195`).
- Risk: diagnostic formatting can bypass the explicit field redaction and reveal
  the same order identifier or order-result detail through the derived lifecycle
  message.
- Implemented fix: the custom formatter now emits `<redacted>` for `message`
  while retaining the field name and all safe status metadata
  (`src/wallet_cluster_state.rs:220-236`).
- Regression coverage: a synthetic leg embeds its CLOID in the stored lifecycle
  message, proves the formatted output omits it, and independently proves the
  stored message remains unchanged for the existing view
  (`src/wallet_cluster_state.rs:608-630`).
- Protected behavior: the view continues reading `leg.message` directly; result
  handling, state, UI copy, order semantics, and persistence are unchanged.

### F-07 — Signed order actions lack independent structural validation

- Status: addressed in Turn 8; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium invariant hardening
- Scope: every placement and modify action at the shared signing-payload
  boundary
- Preconditions/event ordering:
  1. An upstream regression, malformed retained automation value, or a new
     caller constructs a place/modify request with a non-numeric, non-finite,
     zero, negative-zero, or negative price/size string, or a placement without
     the repository's 128-bit hexadecimal CLOID.
  2. The request reaches `HyperliquidL1Action` through the existing builder.
  3. The prior payload builder serializes, hashes, signs, and posts the malformed
     action without an independent structural check.
- Evidence:
  - `prepare_place_order` and `prepare_modify_order` correctly validate and
    quantize ordinary intents, and the audited NUKE, Chase, and TWAP direct
    constructors perform their own positive-finite planning checks.
  - Nevertheless, `PlaceOrderRequest` and the action constructors retain price,
    size, and CLOID as unconstrained `String`/`Option<String>` values, and the
    sole `build_signed_exchange_payload_with_nonce` path previously serialized
    them without validation (`src/signing/client.rs:17-25`,
    `src/signing/actions/builders.rs:10-72`, `src/signing/client.rs:83-104`).
  - Repository-wide searches found no feature-owned `/exchange`, signing, or
    place/modify path that bypasses this shared payload builder.
- Violated invariant: Kerosene must never sign or post an order action whose
  numeric wire values are not positive and finite, and every placement must
  retain the exact CLOID required for uncertain-outcome correlation.
- Risk: an upstream plumbing regression could send a malformed financial
  mutation instead of failing closed. A placement without a valid CLOID would
  also remove the exact `orderStatus` correlation key used after an ambiguous
  transport outcome.
- Implemented fix: `HyperliquidL1Action::validate_wire_structure` checks every
  order and modify child immediately before msgpack serialization. It parses
  price/size only to validate positive finiteness, never rewrites the original
  strings, and requires placement CLOIDs to be `0x` plus exactly 32 hexadecimal
  digits. Static errors contain no rejected value (`src/signing/actions.rs:36-158`,
  `src/signing/client.rs:83-104`).
- Regression coverage: a valid signed IOC placement and GTC modify prove asset,
  OID, side, exact price/size strings, reduce-only, TIF, and CLOID are unchanged.
  Adversarial cases reject parse failures, NaN, infinities, zero, negative zero,
  negative values, invalid modify size, absent CLOID, wrong length, wrong
  prefix, and non-hex content before signing; synthetic sensitive values are
  absent from errors (`src/signing/client/tests.rs:86-260`). The prepared
  request test now explicitly protects side, order kind, and reduce-only mapping
  in addition to its existing asset/price/size/CLOID assertions
  (`src/order_execution/core.rs:2183-2210`).
- Protected behavior: current prepared values already satisfy the guard. The
  validator borrows wire strings and does not normalize, round, reserialize, or
  alter valid action bytes. Market capability, symbol/asset selection, sizing,
  precision, price/slippage, side, reduce-only, TIF, CLOID generation, signing,
  result handling, UI, timing, and persistence remain unchanged for valid work.

### F-08 — Contradictory exchange acknowledgements can be consumed as definitive outcomes

- Status: addressed in Turn 9; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: shared response analysis; every shared-classifier caller; and Chase
  and TWAP parsed mutation-result handling
- Preconditions/event ordering:
  1. A mutation receives a syntactically parseable `ExchangeResponse` rather
     than a transport `Err`.
  2. The top-level envelope or one structured status explicitly reports an
     error, while a structured status also reports a possible resting, filled,
     successful-cancel, or otherwise non-error exchange effect.
  3. A handler evaluates the error, IOC-no-match, or terminal-cancel signal
     before recognizing the conflicting effect.
  4. The handler removes/fails automation, accounts for no fill, or declares a
     child cancel complete without first resolving the exact CLOID/OID.
- Evidence:
  - `ExchangeResponse::is_error` recognizes both top-level and status-level
    errors. The prior `has_potential_order_effect` returned early for every
    top-level error even when the parsed body retained a concrete order effect,
    while the shared one-shot classifier relied on that predicate to separate
    rejection from ambiguity (`src/signing/model/exchange_response/analysis.rs:83-144`,
    `src/order_update/results.rs:180-215`).
  - Shared one-shot handling already treated an inner resting/fill plus error as
    ambiguous, but Chase place/modify/cancel had earlier `is_error` branches;
    TWAP child placement evaluated IOC/error/fill paths before its ambiguity
    branch; and TWAP unexpected-cancel accepted a terminal-error substring even
    alongside a successful-cancel status.
  - The fixed guards and ordering are visible at
    `src/order_update/chase/result.rs:162-175`,
    `src/order_update/chase/modify.rs:61-85`,
    `src/order_update/chase/cancel.rs:59-84`,
    `src/order_execution/twap/slice_result.rs:69-103`,
    `src/order_execution/twap/slice_result.rs:211-320`, and
    `src/order_execution/twap/cancel.rs:114-139`.
- Violated invariant: when explicit error and possible exchange-effect signals
  disagree, local state must remain uncertain until exact target status or an
  authoritative account snapshot resolves the mutation.
- Risk: Kerosene could discard a Chase whose order actually rests, stop or
  requeue automation despite a modify effect, omit a TWAP fill, or clear an
  unexpected child while the response itself is internally contradictory. That
  can lose live child-operation tracking and allow later work to proceed from an
  incorrect exposure model.
- Why existing checks were insufficient: strict confirmed-result predicates
  correctly rejected mixed responses, but the affected handlers consumed an
  error-specific branch before reaching their existing uncertain-result path.
  The top-level-envelope early return also hid structured effects from the
  otherwise conservative shared classifier.
- Implemented fix: response analysis now detects the conjunction of `is_error`
  and `has_potential_order_effect` as one explicit conflict category, including a
  top-level error with a structured effect. Shared classification consequently
  produces `Ambiguous`; Chase place/modify/cancel route conflicts through their
  existing CLOID/OID verification; TWAP child placement prevents a conflict
  from entering its earlier IOC/error/fill/resting branches while preserving
  the established ordering for every non-conflicting response. It also does not
  seed child OID/fill/average-price bookkeeping from the contradictory payload,
  so a later empty refresh cannot credit an unconfirmed fill. Contradictory
  unexpected-cancel results retain the existing bounded, target-specific
  reconciliation/retry path. No placement/modify retry, new retry mechanism,
  identifier, state field, or policy was added.
- Regression coverage: response-model and shared-classifier tests cover both
  mixed statuses and a top-level error with a structured resting effect, while
  an unstructured top-level error remains a definitive rejection
  (`src/signing/tests/responses/status.rs:116-151`,
  `src/signing/tests/responses/strings.rs:89-107`,
  `src/order_update/results/tests.rs:443-509`). Handler tests prove conflicting
  Chase place, modify, and cancel results retain verification state, and that a
  conflicting TWAP slice neither retains provisional effect fields nor credits
  or settles a fill before CLOID status, while a conflicting unexpected-child
  cancel remains pending
  (`src/order_update/chase/result/tests.rs:132-166`,
  `src/order_update/chase/modify/tests/reconciliation.rs:38-77`,
  `src/order_update/chase/cancel/tests.rs:160-194`,
  `src/order_execution/twap/tests/place_result.rs:79-150`,
  `src/order_execution/twap/tests/cancel.rs:97-126`).
- Protected behavior: existing pure-rejection, valid resting/fill/cancel,
  unstructured-malformed, transport-unknown, refresh, bounded cancel retry,
  visible normal-path status, wire, signing, pricing, sizing, timing,
  persistence, and UI behavior is unchanged. Only a parseable internally
  contradictory acknowledgement now takes an already-established uncertain
  reconciliation path.
- Residual uncertainty: Turn 10 completed the phase audit and retained the
  conservative `String` error shape deliberately; transport replay, timeout,
  HTTP-envelope conflict, and pre-message redaction are now independently
  guarded under F-09 through F-11.

### F-09 — Generic HTTP policy can replay or indefinitely strand a signed mutation

- Status: addressed in Turn 10; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High, with a Critical duplicate-mutation consequence if an
  anomalous redirect is emitted after an exchange-side effect
- Scope: the one `/exchange` HTTP client and request builder used by every
  place, cancel-by-OID, cancel-by-CLOID, modify, and leverage action
- Preconditions/event ordering:
  1. A valid action has already been serialized and signed.
  2. The generic shared reqwest client receives a 307/308 redirect or a
     retryable protocol NACK, or its configured construction fails and the
     fallback client is used.
  3. Redirect/retry middleware can clone and resend the JSON POST below the
     application lifecycle, or the fallback has no total request timeout.
  4. The state machine observes only the eventual response/error and cannot
     correlate how many wire sends occurred.
- Evidence:
  - The prior `post_exchange` cloned the shared `api::CLIENT`; that client used
    reqwest defaults for redirect/retry policy and fell back to `Client::new()`
    if the configured builder failed (`src/api.rs:47-58`).
  - All five signed action wrappers converge on `sign_and_post`, and the JSON
    request body is replayable (`src/signing/client.rs:74-135`,
    `src/signing/client.rs:234-293`).
  - The audited reqwest 0.12 implementation follows up to ten redirects by
    default, preserves POST/body for 307/308, permits safe-protocol retries by
    default, and gives a default client no total timeout. Kerosene must not rely
    on generic HTTP safety assumptions as its mutation idempotency policy.
- Violated invariant: exactly one transport send may be initiated for one
  signed lifecycle attempt; any uncertainty must return to Kerosene for exact
  reconciliation, never trigger hidden middleware replay.
- Risk: an infrastructure redirect or lower-level replay can submit the same
  signed mutation more than once without a second state-machine attempt. CLOID
  and nonce behavior provide useful exchange defenses, but cancel, modify, and
  leverage actions do not all have a placement CLOID, and client code must not
  assume undocumented duplicate suppression. An unbounded fallback request can
  also strand global or automation pending state indefinitely.
- Implemented fix: `/exchange` now has a dedicated lazily built client with
  redirects disabled, retries disabled, the existing 5-second connect and
  15-second total limits, and the existing 60-second idle-pool limit. Client
  construction failure is returned through the normal conservative error path
  instead of falling back to an unbounded generic client. The request builder
  independently reapplies the 15-second limit as prudent redundancy
  (`src/signing/client.rs:16-23`, `src/signing/client.rs:137-176`).
- Regression coverage: a request-construction test proves the mutation-local
  timeout exists even on a default client, and a loopback 307 server proves the
  dedicated client returns the redirect without issuing a second POST
  (`src/signing/client/tests.rs:60-137`).
- Protected behavior: endpoint, method, JSON body, headers/user agent, signing,
  nonce/expiry, response parsing, all application result classifications, and
  the ordinary 5/15/60-second timing policy remain unchanged. No dependency,
  exchange retry, application retry, UI, persistence, or trading-policy change
  was introduced.

### F-10 — Signed mutation results can expose details before update-time redaction

- Status: addressed in Turn 10; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium privacy and diagnostic-boundary hardening
- Scope: signed response/error text between `sign_and_post`, task mapping,
  derived `Message::Debug`, `ExchangeResponse::Debug`, and update handlers
- Preconditions/event ordering:
  1. A local, reqwest, response-body, or parse error includes a credential,
     bearer value, long hexadecimal token, or 128-bit CLOID; or a successful
     response includes size, price, and OID details.
  2. A task closure maps that `Result<ExchangeResponse, String>` directly into
     a `Message` variant.
  3. Derived `Message::Debug` formats the nested result before an update handler
     applies its redundant sanitizer.
- Evidence:
  - `Message` derives `Debug`, and every signed-result variant stores the raw
    result shape (`src/message.rs:571-572`, `src/message.rs:1079-1137`).
  - The prior `sign_and_post` returned build/post errors directly, while
    individual handlers performed redaction only after message delivery.
  - `ExchangeResponse::Debug` previously embedded `summary()`, which exposes
    deliberate user-facing resting/fill size, price, and OID details even when
    the raw response model itself is not formatted.
- Violated invariant: every mutation result must already be diagnostically safe
  at the boundary where it can enter a derived-debug message; handler-time
  redaction is useful redundancy, not the first privacy boundary.
- Risk: diagnostic logging, panic context, or state formatting between task
  completion and update handling can disclose secrets, order correlation IDs,
  or order details that adjacent models deliberately redact.
- Implemented fix: the single signing exit now applies order-aware redaction to
  every `Err` before task mapping. The malformed-body path uses the same
  bounded sanitizer, and response error/unknown summaries redact 128-bit hex
  identifiers without changing the generic application redactor. Successful
  `ExchangeResponse::Debug` keeps safe status/type/count metadata but replaces
  its human summary with `<redacted>` (`src/signing/client.rs:74-95`,
  `src/signing/client.rs:211-218`,
  `src/helpers/formatting/text.rs:65-79`,
  `src/signing/model/exchange_response/analysis.rs:227-246`).
- Regression coverage: tests prove a bearer token, API key, private key, long
  hex token, and CLOID are absent from result/debug output; safe error copy and
  successful response semantics remain exact; the order-only helper preserves
  a short OID; and successful response debug omits fill size, price, and OID
  (`src/signing/client/tests.rs:466-529`,
  `src/helpers/formatting/text.rs:461-469`,
  `src/signing/tests/responses/status.rs:57-76`,
  `src/signing/tests/responses/strings.rs:158-184`).
- Protected behavior: `ExchangeResponse::summary()` remains unchanged for valid
  user-facing results; safe transport error strings remain byte-for-byte
  unchanged; `Ok`/`Err` shape, outcome classification, reconciliation, task
  count/timing, controls, persistence, and wire behavior are untouched. Only
  diagnostic/error content that matches the existing sensitive-order policy is
  redacted earlier.

### F-11 — A non-success HTTP envelope can masquerade as mutation success

- Status: addressed in Turn 10; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High outcome-classification hardening
- Scope: HTTP status/body handoff immediately after `/exchange` returns
- Preconditions/event ordering:
  1. The signed request reaches an exchange server or intermediary.
  2. It returns a non-2xx HTTP status but a syntactically valid body whose
     exchange-level fields appear successful or merely incomplete rather than
     explicitly erroneous.
  3. The prior transport reads only the body and discards the HTTP status.
  4. Normal parsed-success handling can accept, fill, rest, or advance
     automation from a response whose two protocol layers disagree.
- Evidence: the prior `post_exchange` chained `.send().text()` and passed only
  raw body text to `parse_exchange_response`. The current handoff captures both
  values at `src/signing/client.rs:137-152`, while parsed-response conflict
  analysis has no access to HTTP status once it is discarded.
- Violated invariant: a successful local lifecycle transition requires success
  at both the HTTP envelope and exchange response layer; disagreement is
  uncertain exposure, not confirmation.
- Risk: an upstream error page or intermediary response shaped like a valid
  exchange result could be consumed as acceptance/fill and allow automation to
  continue from unverified state.
- Implemented fix: HTTP status now accompanies the raw body into one parser.
  A non-success envelope plus an apparently non-error exchange body becomes a
  value-neutral transport error and exact reconciliation. A structured pure
  rejection remains rejected, and an error/effect conflict remains an F-08
  ambiguity, preserving the stronger information already in the body
  (`src/signing/client.rs:220-232`).
- Regression coverage: a 500 envelope with a resting-success body cannot expose
  its OID or confirm success; a 400 structured rejection remains classifiable;
  and a 500 mixed resting/error body still reaches conflict reconciliation
  (`src/signing/client/tests.rs:429-463`).
- Protected behavior: every 2xx response, structured rejection, error/effect
  conflict, syntactically invalid body, redaction rule, timeout, normal status
  string, and reconciliation path retains its prior behavior. Only a previously
  unrepresented HTTP/exchange success disagreement becomes fail-closed.

### F-12 — `orderStatus` diagnostics can expose correlation values

- Status: addressed in Turn 11; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium privacy hardening
- Scope: OID/CLOID status-query HTTP/parser errors, successful result debug,
  and the result value before task-message mapping
- Preconditions/event ordering: an HTTP error body echoes a 128-bit CLOID, or a
  parsed concrete order returns a mismatching OID/CLOID, or an external status
  string embeds sensitive order text; the result is mapped into a derived
  `Message::Debug` variant before handler-time redaction.
- Evidence: before Turn 11, the parser included expected and returned
  correlation values in mismatch errors, the HTTP preview used the generic
  40-hex rather than order-aware 32-hex redactor, and `OrderStatusResult::Debug`
  emitted `status` verbatim. All status-query callers converge on the two public
  functions now guarded at `src/api/order_status.rs:16-35`; result variants are
  stored in derived messages at `src/message.rs:874-879`,
  `src/message.rs:1094-1099`, `src/message.rs:1128-1137`, and the advanced-order
  variants at `src/message.rs:1188-1239`.
- Violated invariant: exact correlation values must be retained for matching
  but omitted from diagnostic errors and result formatting before message
  mapping.
- Risk: an anomalous or hostile reconciliation response can bypass the custom
  result redaction through either the error lane or the externally supplied
  status field.
- Implemented fix: both public status functions apply one order-aware sanitizer
  to `Err` before returning to `Task::perform`; HTTP bodies are redacted before
  their 160-character preview is taken; parser mismatch/missing diagnostics no
  longer embed expected or returned identifiers; and result debug sanitizes the
  status field while retaining exact stored data (`src/api/order_status.rs:16-35`,
  `src/api/order_status.rs:77-82`, `src/api/order_status/parsing.rs:15-62`,
  `src/api/order_status/model.rs:16-24`).
- Regression coverage: adversarial tests cover a CLOID crossing the prior
  preview truncation boundary, secret/CLOID errors before message mapping,
  unchanged safe errors and successful results, value-neutral OID/CLOID
  mismatch and missing-field errors, external parser errors, and status-field
  debug redaction (`src/api/order_status/tests/validation.rs:3-145`,
  `src/api/order_status/tests/parsing.rs:24-89`).
- Protected behavior: exact request body/correlation comparisons, parsed
  success values, stored status/summary data, status classification,
  retries/timeouts, state transitions, and normal user-visible status paths are
  unchanged. Only sensitive or anomalous diagnostic values are removed.

### F-13 — Derived order diagnostics expose raw OID/CLOID context fields

- Status: addressed in Turn 12; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium privacy hardening
- Scope: order-related `Message` variants with direct numeric OID or owned
  CLOID fields, independent of their already-redacted result payloads, plus the
  adjacent stopped-Chase cancel request formatter
- Preconditions/event ordering: any diagnostic formats the derived `Debug` for
  a cancel, move, Chase, TWAP, or chart-hover message carrying a raw correlation
  identifier, or formats a stopped-Chase cancel request during diagnostics.
- Evidence: before Turn 12, `Message` derived `Debug` over raw fields on
  cancel/status, TWAP cancel/status, Chase modify/cancel/status, move
  intent/result/status, and chart cancel-hover variants. Sanitizing F-12's
  `Result` lane did not affect those sibling context fields. A repository-wide
  custom-formatter scan also found that `StoppedChaseCancelRequest` redacted its
  agent key but emitted its OID (`src/order_update/chase/result/stop_cancel.rs:18-26`).
- Violated invariant: order identifiers needed for routing and equality must
  not be emitted by general diagnostic formatting.
- Risk: a normal or failing lifecycle message can disclose the identifier even
  though response models, placement contexts, account keys, and addresses use
  redacted diagnostic representations.
- Implemented fix: added copy/owned `RedactedOrderId` and
  `RedactedClientOrderId` message-boundary wrappers, converted every direct
  exchange identifier producer at publication, and restored the exact values
  once at `update_order` or the chart update consumer. The stopped-Chase request
  formatter now redacts its OID alongside its key (`src/message.rs:254-308`,
  `src/message.rs:1141-1429`, `src/order_update.rs:80-104`,
  `src/order_update.rs:205-431`, `src/chart/interaction/hud.rs:103-122`,
  `src/chart_update.rs:218-240`,
  `src/order_update/chase/result/stop_cancel.rs:18-26`).
- Regression coverage: one whole-message test formats all fifteen affected
  variants, including optional TWAP identifiers and chart hover, and rejects
  the raw OID/CLOID; a round-trip test proves wrapper values are byte/numerically
  exact; the stopped-Chase formatter test rejects both its key and OID
  (`src/message.rs:1621-1729`,
  `src/order_update/chase/result/tests/stop_cancel.rs:38-52`).
- Protected behavior: message variant names/routing, exact OID/CLOID values,
  handler arguments, account/symbol/attempt context, cancellation, movement,
  Chase/TWAP state transitions, chart hover state, task timing, UI, signing,
  persistence, and trading semantics are unchanged. Only diagnostic formatting
  differs.

### F-14 — Cancel and move results can settle a later same-OID attempt

- Status: addressed in Turn 13; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: cancel and drag-to-move dispatch, direct-result ownership, status-result
  ownership, refresh ordering, and runtime request-ID allocation
- Preconditions/event ordering:
  1. Cancel attempt A is dispatched and its presentation indicator expires, or
     a complete refresh clears its account-scoped pending slot before the direct
     result arrives. A later attempt B is then dispatched for the same account
     and OID.
  2. Alternatively, move attempt A leaves direct/status work in flight, its
     OID-keyed context is cleared by an earlier outcome or account-state reset,
     and attempt B reuses the same account, symbol, and OID.
  3. A delayed or duplicate direct/status message from A arrives while B owns
     the structurally identical account/OID tuple.
- Evidence: before Turn 13, cancel direct results recovered their target from
  the indicator and then whichever pending cancel matched only the account;
  cancel status results matched account/OID/symbol but not an attempt or phase.
  Move direct results matched the current OID-keyed context only by account,
  and move status results likewise matched only account/OID/symbol. The refresh
  cleanup also treated a cancel waiting for its first direct result as though it
  were already in status reconciliation. The corrected ownership checks are at
  `src/order_update/results.rs:88-183`,
  `src/order_update/results.rs:359-459`, and
  `src/order_update/move_order.rs:30-160`.
- Violated invariant: a callback may transition, clear, or reconcile only the
  exact logical mutation attempt that dispatched it; OID reuse and a
  presentation indicator are not sufficient ownership.
- Risk: an old cancel result can remove the wrong local order or release the
  newer cancel blocker; an old move result can consume the newer captured key
  context, clear its indicator, patch the snapshot with the old target price,
  or cause the newer true result to be ignored. An old status result can settle
  either newer attempt before its own outcome is known.
- Implemented fix: one collision-aware, wrap-safe runtime request sequence is
  shared by one-shot status, cancel, and move state and skips every live ID.
  Cancel creates its immutable account/OID/symbol owner before dispatch and
  explicitly transitions it from `AwaitingResult` to `CheckingStatus`; only the
  latter phase can be cleared by refresh. Direct and status messages carry the
  same sequence and must match the current phase. Move captures the sequence in
  `PendingMoveOrderContext`, propagates it to uncertain status work, and
  requires it at both handlers (`src/order_execution/position_actions/cancel.rs:81-104`,
  `src/order_execution/quick_order/move_order.rs:162-200`,
  `src/order_update/results.rs:637-670`).
- Regression coverage: stale same-OID direct/status results preserve the newer
  cancel and move attempt; a duplicate cancel direct result cannot override an
  active status check; a complete account refresh cannot erase a cancel still
  awaiting its direct result; indicator expiry does not remove authoritative
  cancel ownership; and allocator wrap skips live one-shot, cancel, move-status,
  and move-context IDs (`src/order_update/results/tests.rs:794-826`,
  `src/order_update/results/tests.rs:1427-1508`,
  `src/order_update/results/tests.rs:1587-1607`,
  `src/order_update/move_order.rs:683-702`,
  `src/order_update/move_order.rs:767-849`).
- Protected behavior: signed cancel/modify actions, account/key capture, OID,
  symbol, price and size preparation, response classification, normal status
  text, refresh/status task timing, local confirmed-price projection, UI,
  persistence, and retry policy are unchanged. The sequence is runtime-only
  correlation and is never sent to the exchange.

### F-15 — Cancel/move refresh cleanup is not scoped to sufficient evidence

- Status: addressed in Turn 14; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: authoritative fallback cleanup for uncertain cancel and move attempts
- Preconditions/event ordering: an uncertain cancel/move originates on one
  main or HIP-3 symbol lane; the visible market universe changes before its
  refresh is dispatched or completed, or the move target remains open at an
  unverified price; a successful snapshot then reports its own scoped open-order
  section as complete.
- Evidence: refresh scope follows the currently selected market universe
  (`src/account_update/connection/refresh.rs:81-92`), and `AccountData` already
  exposes the correct per-symbol lane predicate
  (`src/account/types/data.rs:198-213`). Before Turn 14, shared cancel/move
  cleanup checked only snapshot-wide `open_orders_complete` plus account before
  dropping either request, and the move's authoritative prepared target was
  discarded with its direct-result context.
- Violated invariant: uncertainty may be released only by authoritative data
  that covers the operation's origin lane and distinguishes the relevant
  terminal/open state; an open move additionally needs evidence of whether its
  expected price committed.
- Risk: a complete but unrelated HIP-3 lane can unblock a cancel or move whose
  exposure remains unknown. A same-lane move refresh can also erase the status
  request without comparing the live order price to the dispatched target,
  allowing later actions to proceed without establishing which modification
  won.
- Implemented fix: cancel refresh cleanup now requires its immutable origin
  symbol's open-order lane and remains phase-gated. Move captures the exact
  already-prepared target price with its request/account/key context before
  dispatch, propagates it into ambiguous status state after the presentation
  indicator is consumed, and releases refresh uncertainty only after a covering
  snapshot establishes one of three explicit states: target absent, target open
  at the expected price, or target open at a different valid price. A malformed
  live or retained price is not reconciliation evidence for an open target
  (`src/order_execution/quick_order/move_order.rs:162-204`,
  `src/order_execution.rs:992-1053`,
  `src/order_update/move_order.rs:30-143`,
  `src/order_update/results.rs:187-290`,
  `src/order_update/results.rs:761-788`).
- Why a different valid price still resolves: the successful post-result
  snapshot is the existing authoritative current-state fallback. Distinguishing
  that state proves the requested price is not the live winner while preserving
  the prior cleanup contract; retaining uncertainty solely because the valid
  live price differs would change enabled/disabled behavior and could strand a
  provably current open order. Wrong-scope, incomplete, absent-account, or
  malformed evidence remains blocked.
- Regression coverage: wrong HIP-3 scope and incomplete origin-lane snapshots
  retain cancel/move requests; a covering cancel snapshot preserves cleanup
  when the target remains open; a move result carries its expected price after
  its indicator disappears; malformed live price retains uncertainty; formatted
  expected, different valid, and absent target states are distinguished; and
  existing valid-price/terminal cleanup remains unchanged
  (`src/order_update/results/tests.rs:1527-1565`,
  `src/order_update/move_order.rs:651-714`,
  `src/order_update/results/tests.rs:230-257`).
- Protected behavior: account refresh scope/cadence/generations, exact status
  tasks, all user-visible strings, normal cleanup timing, pending indicators,
  prepared and signed modify values, local confirmed-price projection, retries,
  UI, and persistence are unchanged. The new target price is runtime-only and
  redacted in the pending request's custom `Debug` output.

### F-16 — Unrelated scoped refresh can replace or archive a Chase with unknown exposure

- Status: addressed in Turn 15; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Critical
- Scope: Chase account-refresh reconciliation, fill-completion cleanup,
  stop/cancel verification, and the final prior-exposure gate before a
  replacement placement
- Preconditions/event ordering:
  1. A Chase owns a known order on one HIP-3 lane, such as `flx:BTC`, and is
     awaiting account verification after a terminal no-fill status, target
     fill, or confirmed cancel/stop transition.
  2. The selected market universe changes before the required account refresh,
     so the refresh scope is another HIP-3 dex, such as `xyz`.
  3. That request returns complete account-wide fills and a complete open-order
     result for `xyz`, but necessarily carries no authoritative `flx` orders.
  4. Before Turn 15, Chase reconciliation treated snapshot-wide
     `open_orders_complete` as authority for every Chase. It could therefore
     queue a replacement, archive a fully filled Chase without cancelling a
     still-open known order, or archive `Stopping::VerifyingCancel` without
     proving the origin lane clear. The final known-OID placement guard repeated
     the same global check.
- Affected order surfaces: adopted and newly placed Chase orders across
  replacement, reprice verification, fill completion, stop, and archive paths.
- Evidence: refresh scope is derived from the current market universe
  (`src/account_update/connection/refresh.rs:81-92`), while the account model's
  authority predicate is explicitly per symbol
  (`src/account/types/data.rs:198-213`). The affected reconciliation decisions
  converge in `reconcile_chase_after_account_refresh`,
  `reconcile_chase_fills_from_snapshot`, and `chase_place_at_best`; their fixed
  boundaries are now at `src/account_update/stream/chase/refresh.rs:37-292`,
  `src/account_update/stream/chase/fill_reconciliation.rs:17-146`, and
  `src/order_execution/chase/lifecycle/place.rs:247-290`.
- Violated invariant: only an open-order snapshot that successfully fetched a
  Chase's immutable origin-symbol lane may prove its known exposure absent,
  authorize terminal archive, or permit a replacement placement. Fill evidence
  remains independently account-wide.
- Risk: a wrong-scope snapshot can falsely declare unknown exchange exposure
  safe. At the strongest boundary it can dispatch a replacement while another
  known Chase order remains live; after a target fill it can instead leave a
  residual live order unmanaged by archiving the Chase that should cancel it.
- Why existing checks did not cover it: account address, provider generation,
  lifecycle, OID/CLOID, attempt, fill completeness, and pending-operation gates
  can all be valid in this ordering. Snapshot-wide completeness describes only
  the request's own scope. The already-correct websocket disappearance gate is
  keyed by dex (`src/account_update/stream/chase/disappearance.rs:13-51`) and
  does not constrain the separate REST-refresh path.
- Implemented fix: derive the set of active Chase symbols whose open-order lane
  the snapshot actually covers, using the established
  `has_complete_open_orders_for_symbol` predicate. Pass that immutable evidence
  into fill reconciliation; gate each refresh, stop, removal, correction, and
  replacement decision by its Chase's symbol; and repeat the same per-symbol
  requirement at the final known-exposure placement guard. Missing authority
  retains verification and requests refresh rather than mutating exchange
  state. No polling, retry, or request was added.
- Regression coverage: an unrelated HIP-3 refresh cannot place a resolved
  no-fill replacement, archive a stopping Chase, or archive a fully filled
  Chase whose open orders are unknown; a direct final-dispatch test proves the
  redundant placement guard also blocks the replacement
  (`src/account_update/stream/tests/chase_reprice/account_refresh.rs:152-173`,
  `src/account_update/stream/tests/chase_stop.rs:64-91`,
  `src/account_update/stream/tests/chase_fills/completion.rs:23-47`,
  `src/order_execution/chase/lifecycle/tests/place.rs:101-125`). Existing
  covering-snapshot replacement, stop cleanup, fill archive, and CLOID-attempt
  tests characterize unchanged normal behavior.
- Smallest behavior-preserving fix: reuse the existing account-lane predicate
  at the three authoritative boundaries; do not change Chase state types,
  exchange payloads, task cadence, pricing/sizing, or status copy.
- Protected behavior: repricing, limits, timing, status strings, retry policy,
  fill totals, order identity, normal covering-snapshot transitions, signing,
  UI, persistence, and secret handling are unchanged. Only an unrelated or
  incomplete origin lane remains in the existing verification state.
- Residual uncertainty: formatting, call-site inventory, and state-transition
  review pass, but Rust type-check/tests/clippy must execute on a host with ALSA
  development metadata. Reversed REST-versus-websocket delivery remains a
  separate Track 8 candidate and is not claimed resolved by this fix.

### F-17 — In-flight REST snapshot can erase a newer user-data event

- Status: addressed in Turn 16; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Critical
- Scope: connected-account REST request ownership, ordinary user-data position,
  open-order, fill, and spot-balance frames, initial account loading, and every
  order lifecycle that treats a successful connected snapshot as authoritative
  fallback evidence
- Preconditions/event ordering:
  1. A connected-account REST refresh starts at local account revision `R`.
  2. The server can capture or serve that request's snapshot before a later
     user-data frame, while delivery of the REST result remains in flight.
  3. A valid same-account websocket frame then changes positions, open orders,
     fills, or balances and advances local state to `R+1`. No second refresh or
     lag notification is otherwise required.
  4. The first REST result arrives last with the same address, read provider,
     key generation, and request generation. Before Turn 16, it replaced the
     websocket-merged `AccountData`, bumped the revision again, and immediately
     ran one-shot, cancel/move, Chase, and TWAP reconciliation from the
     potentially pre-event body.
  5. During initial loading the related gap was earlier: without a base
     `AccountData`, an ordinary account frame was not merged and did not queue a
     follow-up, so the pre-frame result could reconcile as final.
- Affected order surfaces: ticket, preset, Alfred, quick, HUD, close, NUKE,
  cancel, move, Chase, connected-account TWAP, and leverage flows that depend on
  the shared connected snapshot cleanup boundary. Wallet-detail/cluster state
  and off-account TWAP reconciliation use separate state/generations.
- Evidence: all mergeable account websocket variants set
  `account_data_changed` and advance `account_data_revision`
  (`src/account_update/stream.rs:101-230`). Before Turn 16,
  `AccountDataRequestContext::ConnectedSnapshot` carried provider and request
  generation but not its starting revision, while `apply_account_data_loaded`
  unconditionally installed a same-generation success and invoked shared
  cleanup (`src/order_update/results.rs:736-788`) plus Chase/TWAP reconciliation.
  The fixed ownership and application boundaries are now
  `src/read_data_provider.rs:23-157` and
  `src/account_update/connection/refresh.rs:94-213`; initial-load coalescing is
  at `src/account_update/stream.rs:193-230`.
- Violated invariant: data that may predate a newer authoritative user-data
  event must not erase that event or prove a financial mutation absent. A
  connected snapshot may settle lifecycle uncertainty only if its causal
  request context still owns the current account revision.
- Risk: a live order or fill can disappear locally, a pending one-shot/cancel/
  move check can be cleared by pre-operation absence, and Chase can archive or
  progress toward replacement after newer exposure was already observed. This
  falsely declares unknown exchange exposure safe and can enable a duplicate
  or unmanaged mutation.
- Why existing checks did not cover it: the address, provider/key generation,
  and request generation all remain valid because there is only one REST
  request. `account_refresh_followup_pending` was set by an explicit refresh or
  lag event, not by ordinary mergeable websocket frames. The revision existed
  for sizing provenance but was not captured by the REST task.
- Implemented fix: capture `account_data_revision_at_dispatch` in every
  connected-snapshot request context. After the existing provider, generation,
  and address checks, reject a successful body whose revision no longer owns
  current account state before changing loading data, revisions, pending
  uncertainty, or automation. Preserve the websocket-merged snapshot, consume
  any queued reason into one sequential post-frame refresh, and reuse the
  existing backoff/generation machinery. If initial loading has no base to
  merge into, mark the in-flight result for the existing follow-up branch; it
  may populate display state but cannot reconcile orders before the post-frame
  request completes.
- Regression coverage: an adversarial request-before/open-order-frame/result-
  after test proves the live order and account revision remain, the pending
  one-shot status is not cleared, and exactly one newer request generation is
  active. A separate initial-load test proves an otherwise unmergeable account
  frame queues reconciliation without starting a competing request
  (`src/account_update/connection/refresh.rs:805-869`,
  `src/account_update/stream/tests.rs:200-225`). Existing same-generation load,
  queued-follow-up, stale-generation/provider, scoped-open-order, and
  off-account TWAP tests characterize protected behavior.
- Smallest behavior-preserving fix: add one runtime-only revision value to the
  existing context and one pre-application ownership check, plus reuse the
  existing initial-load follow-up bit. Do not merge lanes speculatively, add a
  timer, accept causally unproven absence, or alter response/status semantics.
- Protected behavior: REST fetch scope, cadence, providers, errors, 429
  backoff, explicit/lag follow-ups, successful no-conflict application,
  display layout/copy, order preparation/signing, exact status handlers,
  persistence, and secrets are unchanged. Conflict handling starts at most one
  sequential request at a time; continued live deltas remain fail-closed under
  the existing rate-limit policy rather than accepting an unowned snapshot.
- Residual uncertainty: Rust type-check/tests/clippy remain blocked by ALSA
  metadata. Same-address user-stream subscription identity across reconnects
  and channel closure is a separate Track 8 candidate; this REST ownership
  guard does not claim to establish stream-generation provenance.

### F-18 — Replaced same-address user stream can apply one late frame

- Status: addressed in Turn 17; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Critical
- Scope: iced user-data recipe identity/cancellation, connected-account stream
  application, wallet-detail streams, selected wallet-cluster streams, account
  session and consumer-topology changes, dex-scope changes, manager reconnect,
  lag, and broadcast closure
- Preconditions/event ordering:
  1. An account, wallet-detail, or cluster recipe has pulled a valid frame from
     its shared-manager receiver.
  2. Before iced can send that frame to the application queue, state replaces
     or removes the recipe. Real paths include a same-address wallet reconnect,
     account switch away/back, detail exclusion or close/reopen, selected
     cluster/member/profile changes, and visible-dex scope changes.
  3. The old and new recipe use the same normalized wallet address. Before
     Turn 17, the resulting `Message` carried only that address and data.
  4. iced cancellation races only with `stream.next()`. Once a message wins,
     the tracker awaits the application-queue send without selecting
     cancellation again; the canceled old recipe can therefore enqueue that
     one frame after state already requests the new recipe.
  5. The address-only handler accepts the frame as belonging to the new
     session. Independent old/new recipe tasks can also reverse delivery.
- Affected order surfaces: every connected-account lifecycle consumer of open
  orders, fills, positions, balances, or lag—including one-shot, cancel/move,
  Chase, TWAP, leverage, and overlays—and wallet-cluster close/order planning
  that relies on member position freshness. Wallet details have the same stale
  display risk but do not dispatch trades.
- Evidence: `WsUserDataStreamParams` was hashed into recipe identity but was
  discarded by each `.map`, leaving only `Option<RedactedAddress>` in the three
  messages. iced 0.14's tracker selects cancellation against `stream.next()` at
  lines 96-105 of its `subscription/tracker.rs`, then awaits `receiver.send`
  without another cancellation check. The shared manager's ordinary socket
  reconnect remains ordered inside one receiver; its broadcast sender closes
  only if the singleton manager task/command channel tears down, not on normal
  reconnect. Thus recipe replacement—not the manager reconnect loop—is the
  concrete overlap boundary.
- Violated invariant: a user-data frame may change trading or freshness state
  only when both its source address and the exact runtime stream incarnation
  still belong to the current consumer.
- Risk: an old open-order/fill frame can regress or falsely advance automation
  reconciliation after a same-address reconnect. A late old cluster position
  frame can clear `stale`, refresh `positions_refreshed_ms`, and authorize a
  reduce-only close size from superseded member exposure. That can falsely
  declare unknown exchange state safe and enable a wrong or duplicate
  mutation.
- Why existing checks did not cover it: address normalization proves wallet
  ownership, and purpose/dex fields distinguish simultaneously live recipes,
  but none identifies a replacement incarnation with equal parameters.
  Manager refcounts prevent duplicate wire subscriptions from colliding; they
  cannot retract an item the old iced task already pulled. REST request
  generations and Turn 16's revision guard do not identify websocket recipe
  ownership.
- Implemented fix: add a runtime-only generation to
  `WsUserDataStreamParams`, retain the entire params value in each subscription
  output with `Subscription::with`, and add one shared guard that requires the
  exact current params plus exact normalized source before any route applies
  data. Rotate the account generation at successful connect/reconnect,
  disconnect, switch, invalidation, and clear. Allocate unique per-address
  detail generations across open/close/reopen and connected-address exclusion,
  while leaving unrelated detail streams unchanged and clearing retained
  address keys on config clear. Rotate the selected-cluster generation for
  topology/profile changes. Rotate all affected consumers only when visible
  dex inputs change. Generations participate in iced identity but are ignored
  by exchange subscription construction and are not persisted.
- Regression coverage: a canceled same-address account recipe queues a lag
  frame and cannot mark the new account session loading/reconciling
  (`src/account_update/stream/tests.rs:85-105`). A canceled cluster recipe
  queues a full position snapshot and cannot clear `stale` or refresh its
  close-sizing timestamp (`src/wallet_cluster_update.rs:2142-2194`). Parameter
  tests prove generation and source are both required, a reopened detail gets
  a unique generation without rotating another address, config clear removes
  detail provenance addresses, and diagnostics redact the wallet address
  (`src/subscription_state/user_data.rs:226-348`,
  `src/ws/user_streams.rs:309-327`, `src/message.rs:1969-2189`). The existing
  same-account reconnect test now proves the production connect path rotates
  its generation.
- Smallest behavior-preserving fix: enrich the existing recipe/message context
  and reject non-current queued events at update entry. Do not change parsing,
  wire topics, reconnect policy, manager buffering/refcounts, REST cadence,
  lifecycle classification, order preparation/signing, or views.
- Protected behavior: current recipe frames take the identical update paths;
  normal manager reconnects stay within the same sequential receiver; lag
  still emits before requesting the existing shared reconnect and REST repair;
  address/dex subscription payloads, stream cadence outside an actual recipe
  replacement, order semantics, status copy, UI, persistence, and secrets are
  unchanged.
- Residual uncertainty: formatting, exhaustive recipe-input mutation search,
  manager reconnect/lag/closure tracing, source normalization, redaction, and
  adversarial ordering review pass. Rust type-check/tests/clippy must execute on
  a host with ALSA development metadata. A terminal panic/abort of the
  singleton manager task would close the broadcast channel and is not a normal
  recoverable reconnect state; startup/shutdown fault handling remains for the
  later lifecycle-close track.

### F-19 — TWAP status results do not own an exact retry attempt

- Status: addressed in Turn 18; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: TWAP child-status task dispatch, result correlation, bounded status
  retries, account-fill confirmation, and reconciliation cleanup
- Preconditions/event ordering:
  1. A TWAP child has an ambiguous placement outcome and retains its CLOID for
     status repair.
  2. A status result is duplicated, or results from separately queued calls for
     the same CLOID are delivered out of order.
  3. One result has already consumed or advanced the mutable retry counter, or
     moved the child into filled/no-fill account reconciliation.
  4. The delayed result still finds the same `status_check_cloid`; that value
     intentionally remains present while account fills confirm the outcome.
- Evidence: both immediate and delayed status tasks previously emitted only
  the TWAP ID and CLOID. Dispatch had no in-flight owner, while the handler
  accepted every non-terminal result matching that CLOID and derived its next
  action from shared `status_check_retries`. Filled and canceled results retain
  the CLOID during their separate account-fill reconciliation phase
  (`src/order_execution/twap/status/tasks.rs:13-100`,
  `src/order_execution/twap/status.rs:34-225`).
- Violated invariant: exactly one status task may own a TWAP child retry
  attempt, and exactly one matching result may consume that attempt or advance
  its reconciliation phase.
- Risk: a duplicate missing/unclear/error result can consume bounded retry
  budget more than once. More importantly, a stale missing result can rewrite
  `AwaitingNoFillConfirmation` back to `StatusUnknown` and launch another
  lookup after a later canceled result already entered account-fill proof. A
  reversed open result can also enter target-specific cancellation after a
  newer status moved to fill reconciliation. These transitions cannot place a
  new child because the CLOID gate remains closed, but they can corrupt the
  fail-closed lifecycle, issue an obsolete cancel, or terminally fail a TWAP
  using evidence that no longer owns the phase.
- Why existing checks did not cover it: Turn 7 correlates the mutation result
  to an exact slice index/retry, but status repair is a recurring read phase
  after that pending mutation has cleared. CLOID equality proves the logical
  child, not which status attempt owns current mutable state; the terminal
  guard applies only after the TWAP has already terminated.
- Implemented fix: add one runtime-only optional status attempt to `TwapOrder`.
  A shared arming helper verifies the TWAP is non-terminal, the CLOID is still
  current, and no task already owns it, then captures the current bounded retry
  count in `TwapOrderStatusLoaded`. The handler requires exact CLOID/attempt
  ownership and clears it before applying the result, so every subsequent copy
  is stale. A retry arms the next attempt synchronously. Account-fill success
  and timeout cleanup clear any outstanding ownership independently of the
  CLOID (`src/twap_state/model.rs:184-189`,
  `src/order_execution/twap/status/tasks.rs:13-100`,
  `src/order_execution/twap/status.rs:34-64`,
  `src/order_execution/twap/fills.rs:176-182`).
- Regression coverage: one test calls status dispatch twice and proves only one
  task is created; one duplicates a missing result and proves retry count,
  events, and the next armed attempt advance only once; one delivers a stale
  missing result after canceled/no-fill status and proves the deadline and
  `AwaitingNoFillConfirmation` phase are unchanged
  (`src/order_execution/twap/tests/account/status_check.rs:137-215`). Existing
  filled, canceled, rejected, retry-exhaustion, stop, and terminal-result tests
  now supply and/or assert the attempt owner. Custom `TwapOrder::Debug` exposes
  only whether an attempt exists.
- Smallest behavior-preserving fix: correlate the existing read task with the
  existing retry counter rather than adding a mutation retry, timer, exchange
  identifier, or new lifecycle policy. No attempt value is signed, sent to the
  exchange, rendered, or persisted.
- Protected behavior: deterministic child CLOIDs, IOC request construction,
  slice size/price/cadence/randomization, retry limits and delays, status REST
  endpoint, fill attribution, no-fill confirmation, stop/timeout/archive paths,
  status strings, tasks for unique results, UI, persistence, and secrets are
  unchanged.
- Residual uncertainty: constructor/producer/consumer inventory, Elm message
  ordering, retry and reconciliation cleanup paths, redacted diagnostics, and
  diff review pass. Rust type-check/tests/clippy must execute on a host with
  ALSA development metadata. TWAP unexpected-resting cancel results remain a
  separate exact-attempt correlation candidate for continued Track 6 audit.

### F-20 — TWAP unexpected-child cancel attempts lack single result ownership

- Status: addressed in Turn 19; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: TWAP unexpected-resting child cancellation dispatch, delayed retry
  triggers, result messages, bounded retry accounting, stop/terminal guards,
  and diagnostic state
- Preconditions/event ordering:
  1. An IOC child unexpectedly rests and the TWAP begins target-specific
     cancellation by CLOID (or OID when no CLOID exists).
  2. Attempt N returns an ambiguous, contradictory, or transport-unknown
     result. The handler increments shared `cancel_retries` and schedules the
     delayed trigger for attempt N+1.
  3. A duplicate result for N arrives, or the N result is delayed until N+1 is
     in flight. The pending operation still has the same target throughout the
     bounded retry sequence.
  4. Independently, a duplicate retry-due message for N+1 can arrive while the
     same counter and target still match.
- Evidence: `TwapUnexpectedCancelRetryDue` carried an attempt and compared it
  with `cancel_retries`, but dispatch did not consume or otherwise mark that
  attempt in flight. `twap_cancel_child_task` then emitted
  `TwapUnexpectedCancelResult` with only TWAP ID and target. The result handler
  accepted any message matching the long-lived pending target and incremented
  the current shared counter (`src/order_execution/twap/cancel.rs:22-110`,
  `src/order_execution/twap/helpers/cancellation.rs:43-88`).
- Violated invariant: one retry attempt may create at most one in-flight cancel
  task, and exactly one result from that attempt may consume retry budget or
  settle the pending child.
- Risk: repeated ambiguous/error results for one dispatch can consume the
  five-attempt budget without five owned exchange attempts, transition the
  TWAP to terminal error, archive its history, and clear its captured agent key
  while the unexpected resting child may remain open. A stale earlier success
  can also finish the pending operation while a newer task still owns it.
  Duplicate retry triggers can send redundant cancellation mutations. The
  mutation is deliberately target-specific and idempotent, but duplicate
  dispatch and false retry exhaustion are unnecessary lifecycle risk.
- Why existing checks did not cover it: target matching identifies the same
  logical child, not which recurring dispatch owns current mutable state. The
  retry-trigger attempt check rejects a trigger only after the counter changes;
  it does not distinguish two copies while one task is in flight. Turn 9's
  contradictory-response classifier prevents false success but does not make
  retry accounting idempotent.
- Implemented fix: add one runtime-only optional unexpected-cancel owner to
  `TwapOrder`. Both initial entry paths capture the current retry attempt when
  they create the pending cancel. A shared arming helper requires non-terminal
  state, the exact current retry count, the exact OID/CLOID target, and no
  existing owner before any cancel task dispatch. The result message carries
  that attempt; the handler requires the exact target, retry count, and owner,
  then clears ownership before applying the result. Ambiguous/error handling
  then schedules the unchanged next retry with no owner until its delayed
  trigger claims it
  (`src/twap_state/model.rs:197-201`,
  `src/order_execution/twap/cancel.rs:22-110`,
  `src/order_execution/twap/slice_result.rs:270-412`,
  `src/order_execution/twap/status.rs:198-319`).
- Regression coverage: duplicate retry-due delivery creates one task; duplicate
  ambiguous results consume one retry/event/backoff transition; a near-limit
  duplicate error cannot falsely exhaust the budget; a stale success from
  attempt zero cannot settle attempt one; and the current attempt-one success
  retains the existing terminal transition
  (`src/order_execution/twap/tests/cancel.rs:195-460`). Placement-result and
  status-result tests prove both initial cancellation paths arm attempt zero
  (`src/order_execution/twap/tests/place_result.rs:152-186`,
  `src/order_execution/twap/tests/account/status_check.rs:300-329`). Existing
  target mismatch, terminal, redaction, contradictory response, and retry
  exhaustion coverage now models exact ownership. Custom `TwapOrder::Debug`
  exposes only whether an owner exists.
- Smallest behavior-preserving fix: correlate and atomically claim the existing
  retry count rather than changing the target-specific mutation, adding a new
  retry, or introducing an exchange status policy. The attempt is not signed,
  transmitted to Hyperliquid, rendered, or persisted.
- Protected behavior: CLOID-before-OID cancel selection, signed cancellation
  payload, classification, retry maximum and delays, refresh/invalidation,
  stop and archive behavior for unique results, status/event strings, UI,
  scheduling, persistence, and secrets are unchanged.
- Residual uncertainty: field/message/call-site inventories, initial and retry
  dispatch paths, target matching, stop/terminal guards, redacted diagnostics,
  and diff review pass. Rust type-check/tests/clippy must execute on a host with
  ALSA development metadata. Fill reconciliation can mark an unexpected child
  filled before a later cancel result rewrites its child status; that reversed
  ordering is the next distinct Track 6 candidate.

### F-21 — Filled unexpected-child label depends on cancel result ordering

- Status: deferred in Turn 20; financial invariants characterized; resolving
  the label requires explicit user-visible/history semantics approval
- Severity: Medium
- Scope: TWAP unexpected-resting child status, authoritative fill
  reconciliation, live details rows, terminal advanced-order history, and
  event-order permutations
- Preconditions/event ordering:
  1. An IOC child unexpectedly rests and a target-specific cancellation is in
     flight.
  2. Account or user-stream reconciliation proves a partial or full fill for
     the exact OID, coin, and side.
  3. The fill and confirmed/terminal-not-open cancel result arrive in opposite
     orders while both refer to that same child.
  4. Each handler writes its own single-valued `TwapChildStatus`; the later
     writer therefore determines the label.
- Evidence: fill reconciliation preserves the maximum child fill, average
  price, and fee, then assigns `Filled` for any non-rejected positive fill
  (`src/twap_state/order/reconciliation.rs:54-65`). Both definitive cancel
  branches assign `UnexpectedRestingCancelled` without consulting retained
  fill metrics (`src/order_execution/twap/cancel.rs:121-153`). Live details
  render `child.status.label()`, terminal snapshots persist that label as a
  string, and history details render it directly
  (`src/order_views/twap_details/sections/activity.rs:66-93`,
  `src/advanced_order_history/snapshots.rs:88-107`,
  `src/order_views/advanced_history_details/sections.rs:174-186`).
- Violated invariant: identical exchange facts should not produce a different
  terminal child outcome label solely because two valid messages were queued
  in a different order.
- Risk: the live row and persisted audit history can say `Canceled` for a child
  with a confirmed fill in one ordering and `Filled` in the reverse ordering.
  This is presentation/history ambiguity, not lost exposure: child and
  aggregate fill quantities, average price, fee, remaining target, scheduling,
  completion, and terminal numeric history remain correct and idempotent.
- Why existing checks do not cover it: `filled_size` intentionally carries
  fill truth independently of the status enum, so all financial calculations
  are safe while last-writer label behavior remains observable. Existing fill
  and cancel tests exercised each lane separately, not both permutations.
- Characterization coverage: partial-fill permutations now prove identical
  child/aggregate fill metrics, remaining target, pending cleanup, retry state,
  next lifecycle status, and duplicate-fill idempotence while explicitly
  recording the two current labels. Full-fill permutations prove identical
  completed status, scrubbed agent key, and archived target/fill/remaining/
  average/fee metrics while recording `Canceled` versus `Filled` history rows
  (`src/order_execution/twap/tests/cancel.rs:462-570`).
- Why deferred: making fill dominate would change an existing visible
  `Canceled` row to `Filled`; making cancel dominate would change the reverse
  case; adding `Partially filled / canceled` or an equivalent combined state
  adds new UI/history copy and semantics. All three violate this campaign's
  no-UX-change authority. The persisted history string also makes the chosen
  meaning part of the user-visible audit record. No trading-safety benefit
  justifies silently selecting one.
- Approval options: explicitly choose fill-dominant, cancel-dominant, or a new
  combined outcome label, then update live/history rendering and both
  permutation expectations together. No persisted schema migration is required
  for future entries, but existing stored strings should remain untouched
  unless separately approved.
- Protected behavior: no production source changed in Turn 20. Fill matching,
  deduplication, quantities, fees, target-specific cancellation, retry and
  refresh behavior, scheduling, stop/completion/archive behavior, current
  labels/copy, persistence format, and secrets remain unchanged.
- Residual uncertainty: the characterization tests cannot execute on this host
  until ALSA development metadata is present. Event log summary ordering also
  follows message order, but those entries truthfully record both events and do
  not alter financial state; no copy normalization is proposed.

### F-22 — Terminal TWAP cancel outcomes schedule a dead retry trigger

- Status: addressed in Turn 21; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium defense-in-depth hardening; no current duplicate exchange
  mutation or financial-state corruption was confirmed
- Scope: full-fill TWAP terminalization while an exact unexpected-child cancel
  attempt is in flight, retryable cancel-result handling, delayed retry
  dispatch, account refresh, terminal history, and captured-key lifetime
- Preconditions/event ordering:
  1. An IOC child unexpectedly rests and attempt zero owns its target-specific
     cancel task.
  2. A complete account/user-fill snapshot arrives first, attributes the full
     target to that child, transitions the TWAP to `Completed`, archives its
     financial history, and clears the captured agent key.
  3. The already-dispatched cancel then returns either an ambiguous successful
     envelope or a transport-unknown error, both of which ordinarily require a
     bounded retry for an active TWAP.
- Evidence: the result handler correctly claimed the exact cancel attempt and
  incremented its retry counter, then unconditionally built a batch containing
  both the immediate account refresh and a delayed retry-due task whenever
  `retry_cancel` was set. The later retry-due handler independently rejected
  terminal state before cloning the key or dispatching cancellation
  (`src/order_execution/twap/cancel.rs:22-75,87-262`). The pre-fix adversarial
  path therefore produced two task units even though only the refresh could do
  useful work.
- Violated invariant: once authoritative reconciliation makes automation
  terminal, an in-flight result may still request authoritative read repair but
  must not originate future lifecycle/mutation retry work.
- Risk: current code did not send a duplicate cancel because the delayed
  consumer revalidated terminal state and terminal archive scrubbed the key.
  It nevertheless scheduled a guaranteed no-op timer/message and made terminal
  safety depend unnecessarily on that second guard. A future retry refactor
  could turn this producer/consumer mismatch into post-terminal exchange work.
- Why existing checks did not cover it: Turn 19 tested that a retry-due message
  cannot dispatch for an already-terminal TWAP, but it did not reverse full
  fill and retryable cancel-result delivery to prove the producer itself stops
  creating retry work.
- Implemented fix: after preserving the existing result classification,
  attempt claim, retry count, child summary, and immediate account refresh, the
  cancel-result handler checks terminal state before batching the delayed
  retry. Terminal state returns only the unchanged immediate origin-account
  refresh; active state retains the same bounded retry task and delay
  (`src/order_execution/twap/cancel.rs:247-262`).
- Regression coverage: both ambiguous-response and transport-error permutations
  full-fill the target while cancel attempt zero is owned, assert completion,
  key scrubbing, exact financial/history stability, one refresh task, and no
  armed retry; even a synthetic retry-due delivery remains a no-op
  (`src/order_execution/twap/tests/cancel.rs:572-627`). Nearby assertions prove
  all four terminal statuses reject scheduling/timer ticks, delayed terminal
  status results cannot mutate history, and repeated archive calls upsert one
  entry while keeping the key empty (`src/twap_state/tests/timing.rs:74-91`,
  `src/order_execution/twap/tests/account/status_check.rs:332-366`,
  `src/advanced_order_history/tests/twap_snapshot.rs:38-66`).
- Smallest behavior-preserving fix: suppress one delayed message/task only
  after terminal state is already authoritative. No exchange payload, target,
  CLOID/OID preference, retry limit or delay for active orders, immediate
  refresh, local status/event copy, child label, financial value, history
  schema, persistence format, or secret handling changes.
- Residual uncertainty: source/call-site tracing, redacted state inspection,
  formatting, and diff review pass. The regression tests and Rust type-check
  still require a host with ALSA development metadata. F-21 remains separately
  deferred because its visible child-label semantics are outside this finding.

### F-23 — Final-save shutdown can start a new automation mutation

- Status: addressed in Turn 22; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: main-window closure, debounced/in-flight final config persistence,
  Chase place/reprice/size-correction dispatch, TWAP slice dispatch, failed-save
  recovery, and exposure-reducing automation cleanup
- Preconditions/event ordering:
  1. A Chase has queued placement, reprice, or size correction work, or a TWAP
     slice is due.
  2. A config save is already in flight or a debounced save is due.
  3. The main window closes. `WindowClosed` requests a final save and the iced
     daemon stays alive until the blocking file write produces `ConfigSaved`.
  4. A queued timer, book update, initial-book result, or account-reconciliation
     result reaches the automation handler before the exit task terminates the
     runtime.
- Evidence: `update_window` closes the main window before calling
  `flush_pending_config_save_and_exit`; that method deliberately returns
  `Task::none()` while a save is already in flight, or starts an asynchronous
  save while setting `config_save_exit_requested`. Before Turn 22, the Chase
  place/reprice/modify and TWAP slice gates checked account loading,
  reconciliation, rate/cooldown, and lifecycle state but not that exit flag.
  Successful/no-save branches also cleared the flag before returning
  `iced::exit()` (`src/window_update.rs:29-35`,
  `src/config_persistence/save.rs:106-172`,
  `src/order_execution/chase/lifecycle.rs:23-37`,
  `src/order_execution/chase/lifecycle/place.rs:309-317`,
  `src/order_execution/chase/lifecycle/reprice.rs:151-205`,
  `src/order_execution/twap/execution/lifecycle.rs:107-121`).
- Violated invariant: after the user closes the main window and final exit owns
  the daemon, no new exposure-progressing automation mutation may begin.
  Results for mutations already sent may still reconcile, and exact
  exposure-reducing cancellation must remain available.
- Risk: a slow or stalled config write could let an unobserved Chase placement,
  modify, size correction, or TWAP IOC slice reach the exchange after the
  trading window has disappeared. The process could then exit immediately,
  leaving the user unable to monitor the new mutation and with no persisted
  live automation to resume. This is realistic continuation of automation
  across a shutdown boundary, even though the request itself retains the right
  account, key, price, size, and identifier.
- Why existing checks did not cover it: account/reconciliation/cooldown guards
  prove exchange readiness, not application-lifecycle authority. Runtime-only
  persistence prevents restart resurrection but does not stop the still-live
  daemon. Config-save tests covered durability and failure feedback, while
  Chase/TWAP tests covered busy exchange gates without a closed-window exit
  interval.
- Implemented fix: retain `config_save_exit_requested` through successful and
  immediate `iced::exit()` tasks, clearing it only when a failed final save
  explicitly returns control to the app. Add a Chase progress gate that combines
  the unchanged exchange-readiness gate with the exit fence for place, book
  reprice, reconciled modify, and size correction. Add the same exit fence to
  the TWAP slice gate. Status checks and target-specific Chase/TWAP cancellation
  continue using their existing paths and can still reduce or reconcile known
  exposure (`src/config_persistence/save.rs:106-172`,
  `src/order_execution/chase/lifecycle.rs:23-37`,
  `src/order_execution/chase/lifecycle/place.rs:309-317`,
  `src/order_execution/chase/lifecycle/reprice.rs:151-205`,
  `src/order_execution/twap/execution/lifecycle.rs:107-121`).
- Regression coverage: config-save tests prove the fence is armed while an
  existing write finishes and remains armed through both successful-save and
  no-save exit tasks; the established failed-save test proves it clears on
  recovery. Chase tests prove placement, reprice, and queued size correction
  remain unsent with their exact queued values and resume normally when the
  fence clears. TWAP proves a due slice keeps zero attempts/sends/pending op and
  then dispatches once after recovery. Separate Chase and TWAP tests prove
  cancel retries remain authorized during exit
  (`src/config_persistence/save/tests.rs:92-142`,
  `src/order_execution/chase/lifecycle/tests/place.rs:83-114`,
  `src/order_execution/chase/lifecycle/tests/reprice/direct.rs:118-138`,
  `src/order_execution/chase/lifecycle/tests/reprice/tick.rs:23-53,113-135`,
  `src/order_execution/twap/tests/gating.rs:40-78`,
  `src/order_execution/twap/tests/cancel.rs:195-214`).
- Smallest behavior-preserving fix: reuse the existing runtime-only exit flag
  at the final mutation owners. No automation is stopped or removed; no task or
  key is persisted; no active-order cancel is suppressed; and no valid order
  payload, identifier, price, size, timing rule, retry rule, history, schema,
  status text, view, or normal in-app interaction changes.
- Residual uncertainty: this batch covers autonomous Chase/TWAP progress after
  main-window closure while final exit remains owned. F-24 separately records
  the existing failed-save path that relinquishes exit after the window is
  gone. A queued one-shot/cluster/leverage intent and close-versus-config-clear
  orchestration still require Track 9 audit. Rust type-check/tests/clippy remain
  environment-blocked before Kerosene compilation.

### F-24 — Failed final save resumes automation without a main window

- Status: deferred in Turn 22; every safe resolution changes visible window,
  exit, or unsaved-config behavior and requires explicit approval
- Severity: High
- Scope: iced main-window close semantics, final config-save failure, toast/
  retry visibility, daemon lifetime, and all runtime trading automation
- Preconditions/event ordering:
  1. The main window uses iced's default `exit_on_close_request: true`.
  2. The OS/custom close path removes that window and emits `WindowClosed`.
  3. A pending final config save fails after the window is gone.
  4. `BlockExitOnError` clears `config_save_exit_requested`, queues an immediate
     future save, pushes a retry toast, and returns `Task::none()`.
- Evidence: iced 0.14 converts a close request with the default window setting
  into `window::Action::Close`, removes the window, then emits `Event::Closed`;
  Kerosene subscribes to closed—not close-requested—events. The main-window
  handler therefore starts persistence after removal. The failure branch is
  intentionally documented to stay open and tell the user to close again, but
  there is no main window to render that feedback or receive another close.
  Because the daemon does not exit when all windows are closed and the exit flag
  is cleared, subscriptions and queued automation resume
  (`src/window_chrome.rs:6-14`, `src/app_boot/windows.rs:10-27`,
  `src/subscription_state.rs:24-26`, `src/window_update.rs:29-35`,
  `src/config_persistence/save.rs:134-172`; iced 0.14 local sources
  `iced_core/src/window/settings.rs`, `iced_winit/src/lib.rs`, and
  `iced/src/daemon.rs`, inspected 2026-07-11).
- Violated invariant: aborting exit because configuration could not be saved
  must return the user to an observable, controllable trading application; it
  must not resume client-side automation in a headless daemon.
- Risk: after a disk/permission/transient write failure, Chase or TWAP can
  continue to mutate exchange state without its main monitoring/control window.
  The promised toast and “close again” recovery are not actionable. A later
  ordinary retry can save successfully but cannot exit because the exit owner
  was cleared, leaving the process alive until externally terminated.
- Why current checks do not cover it: config-save unit tests exercise the pure
  exit decision and state flags without a real window manager. The headless
  daemon behavior spans iced's automatic close action, Kerosene's closed-event
  subscription, and the later asynchronous save result.
- Why deferred: intercepting `CloseRequested` and keeping the window open until
  save completion changes close timing/appearance; reopening the main window on
  failure visibly reverses a close and must restore window state; exiting on
  failure discards the current durability guarantee; retaining a permanent
  headless fence strands the process. Selecting among those behaviors exceeds
  this campaign's no-UX/no-workflow authority.
- Approval options: (1) keep the main window open during final save and close
  only after success; (2) close immediately but reopen on save failure; or (3)
  honor close even on save failure and accept unsaved preference/layout loss.
  Whichever policy is chosen should add a real close-request/save-result window
  lifecycle test and define behavior when auxiliary windows remain.
- Protected behavior: Turn 22 does not alter the established save-error branch,
  toast copy, automatic retry due time, or window creation/closure behavior.
  F-23 only fences the interval where final exit still owns the daemon.
- Residual uncertainty: the executable GUI path cannot be smoked on this host
  because compilation stops at missing ALSA metadata. Source semantics are
  explicit, but platform-specific window-manager timing should be verified once
  a product policy is approved.

## Turn 1 — Baseline and Lifecycle Assurance Matrix

- Status: audited
- Severity: N/A; read-only startup turn
- Scope: architecture, all signed mutation call sites, baseline validation,
  initial assurance matrix
- Invariant: no production source changes before every current mutation surface
  has an evidence-backed lifecycle row.
- Protected behavior: all UI, order semantics, automation timing, persistence,
  and signing behavior remained unchanged.
- Evidence: required architecture/security/testing documents, prior lifecycle
  audit and campaign, repository-wide signing/task call-site searches, and the
  source references in the matrix above.
- Change: created this progress ledger only.
- Tests/checks: baseline commands and exact outcomes are recorded above.
- Compatibility/UX assessment: documentation-only; no runtime impact.
- Residual risk: matrix gaps and candidates F-01 through F-05 require focused
  adversarial verification. The Rust validation environment remains blocked by
  missing ALSA development metadata.
- Prior turn commit hash: `cbd48106a75fcbeb982f5e6ed2d53772bbf7123b`
- Next candidate: implement F-01 with a failing duplicate-child regression and
  CLOID-keyed idempotent NUKE aggregation, preserving all unique-result behavior
  and status text.

## Turn 2 — Idempotent NUKE Child Settlement

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium invariant hardening for a potentially High-consequence
  emergency-close ordering failure
- Scope: runtime NUKE parent accounting and the two terminal result boundaries
- Invariant: one logical NUKE child contributes at most one terminal outcome to
  its parent, regardless of duplicate direct or reconciliation-result delivery.
- Protected behavior: every unique child still produces the same aggregate
  counts, completion threshold, account-refresh decision, error flag, and
  visible status text. Order preparation, CLOID generation, signed payloads,
  exchange calls, UI, and persistence are untouched.
- Evidence: the result context already carries the child's immutable CLOID;
  prior aggregation discarded it and incremented an unkeyed counter. Source
  references and adversarial ordering are recorded under F-01 above.
- Change: replaced the unkeyed completed count with a runtime-only settled-CLOID
  set, made each outcome recorder atomically reject an already-settled child,
  derived completion from unique settlements, and retained the former safe
  counter-only `Debug` shape so stored CLOIDs cannot be formatted.
- Regression tests: added duplicate direct-result and duplicate status-result
  cases. Each proves one repeated CLOID leaves the parent at `1/2` and that a
  second unique CLOID is required for the unchanged `2/2` completion state.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene duplicate_nuke_` stopped in
    `alsa-sys` before compiling Kerosene because `pkg-config` could not find
    the system `alsa.pc` package.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
  - A pre-implementation focused-test attempt encountered the same ALSA
    boundary, so the new regression could not be observed failing on this host.
- Compatibility/UX assessment: internal runtime bookkeeping only; no normal
  success/failure copy or behavior changes and no schema/dependency changes.
- Residual risk: source parsing, formatting, call-site inspection, and the diff
  pass, but the new tests and Rust type-check must still execute once ALSA
  development metadata is available.
- Prior turn commit hash: `f8d2fa41abc4f6ef1456fcc2988b1ef8d280f315`
- Next candidate: audit F-02's account-refresh completeness and exact-resolution
  semantics before deciding whether one-shot status cleanup needs a guarded
  production change or only characterization coverage.

## Turn 3 — Scope-Complete One-Shot Refresh Reconciliation

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: the shared pending one-shot status record and successful account-load
  cleanup boundary
- Invariant: an account refresh may release an uncertain one-shot placement only
  if account-wide fills are complete and the refresh successfully fetched the
  open-order lane for the order's origin market.
- Protected behavior: exact `orderStatus` outcomes and complete covering refresh
  cleanup retain their existing status strings, blocker transitions, account
  refresh behavior, and timing. Order construction, signing, dispatch, pricing,
  sizing, reduce-only behavior, UI, and persistence are untouched.
- Evidence: best-effort bootstrap and scoped open-order behavior prove that
  `Ok(AccountData)` alone is not a completeness guarantee. Detailed source
  evidence and the failure ordering are recorded under F-02 above.
- Change: retained `symbol_key` in the runtime-only pending status record and
  added a per-symbol open-order completeness helper that distinguishes main and
  HIP-3 fetch lanes. Account-refresh cleanup evaluates that signal and fill
  completeness independently for every pending request.
- Regression tests: added incomplete-open-orders, incomplete-fills,
  wrong-HIP-3-scope, and later-covering-scope cases. Existing complete-refresh
  tests remain the characterization for unchanged normal cleanup. A focused
  account-data test proves an unrelated failed dex does not invalidate an
  origin lane with a successful fetch timestamp
  (`src/account/types/data/tests/freshness.rs:112-125`).
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene account_refresh_must_cover_one_shot_symbol_before_clearing_status_request`
    stopped in `alsa-sys` before compiling Kerosene because `pkg-config` could
    not find the system `alsa.pc` package.
  - `cargo test --package kerosene --bin kerosene incomplete_account_refresh_does_not_clear_one_shot_status_request`
    stopped at the same environment boundary.
  - `cargo test --package kerosene --bin kerosene complete_open_order_coverage_tracks_each_symbol_lane`
    stopped at the same environment boundary.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
  - A pre-implementation focused-test attempt encountered the same ALSA
    boundary, so the new regression could not be observed failing on this host.
- Compatibility/UX assessment: no visible copy, indicator, layout, or
  normal-path enabled/disabled behavior changes. Only an incomplete or
  origin-excluding fallback snapshot remains uncertain until exact status or a
  complete covering refresh arrives; treating that state as resolved was the
  confirmed safety defect.
- Residual risk: source parsing, formatting, call-site inspection, and the diff
  pass, but the new tests and Rust type-check must still execute once ALSA
  development metadata is available. A complete covering account snapshot
  remains the established fallback resolution boundary and does not rewrite the
  visible status into a false exact success.
- Prior turn commit hash: `5ea78f1c2aa9a1327d50093f1b382c37f48b0b28`
- Next candidate: implement F-03's zero-behavior-change CLOID redaction in
  `PendingOneShotStatusRequest::Debug` with focused regression coverage.

## Turn 4 — Redact Pending One-Shot CLOIDs

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium privacy hardening
- Scope: one custom `Debug` implementation and its focused regression
- Invariant: diagnostic formatting of pending order lifecycle state must not
  disclose account addresses or CLOIDs.
- Protected behavior: the pending record retains the exact CLOID internally for
  matching. All request IDs, lifecycle transitions, status strings, account
  refreshes, exchange payloads, views, and persistence behavior are untouched.
- Evidence: repository-wide inspection of explicit CLOID formatters found the
  raw exposure only in `PendingOneShotStatusRequest`; adjacent context, signing,
  TWAP, cluster, and order-status models use `<redacted>` or presence-only
  fields.
- Change: replaced the raw CLOID formatter value with `<redacted>` and tightened
  the existing test to reject the synthetic account and CLOID independently.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene pending_one_shot_status_request_debug_redacts_account_and_cloid`
    stopped in `alsa-sys` before compiling Kerosene because `pkg-config` could
    not find the system `alsa.pc` package.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
  - The pre-implementation focused-test attempt encountered the same ALSA
    boundary, so the strengthened regression could not be observed failing on
    this host.
- Compatibility/UX assessment: diagnostic-only output change; no user-visible,
  trading-semantic, timing, schema, or dependency impact.
- Residual risk: source parsing, formatting, call-site inspection, and the diff
  pass, but the focused test and Rust type-check must still execute once ALSA
  development metadata is available.
- Prior turn commit hash: `55263a56c1b598af20e3aaadb012948e20644e47`
- Next candidate: characterize F-04's wallet-cluster result correlation and
  idempotence under stale IDs, wrong profile/CLOID, duplicate direct results,
  and status-after-terminal ordering before changing production behavior.

## Turn 5 — Make Wallet-Cluster Leg Transitions Idempotent

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium invariant hardening
- Scope: shared order/close cluster leg result transitions and follow-up gating
- Invariant: a fully correlated direct result may transition a leg out of
  `Pending` once; only the resulting `Checking` phase may accept one
  `orderStatus` result; terminal states never transition again.
- Protected behavior: every valid unique result retains the existing classifier,
  status/message text, member refresh, CLOID repair, aggregate completion, and
  problem-count behavior. Cluster planning, sizing, signing, fan-out, UI,
  persistence, and member data streams are untouched.
- Evidence: handler and model tracing plus adversarial cases proved the void
  update helper could overwrite terminal legs and could not suppress follow-up
  work for mismatched results. Detailed ordering and source evidence are
  recorded under F-04 above.
- Change: replaced unconditional leg assignment with an expected-state
  transition that verifies the complete immutable origin context. Both handlers
  now stop immediately when correlation or phase validation fails.
- Regression tests: added exact-origin mismatch coverage, conflicting duplicate
  direct results for both cluster order and close executions, terminal-to-
  checking regression, duplicate ambiguity, status-before-checking, normal
  checking-to-confirmed settlement, and status-after-terminal replay.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene wallet_cluster_update::tests`
    stopped in `alsa-sys` before compiling Kerosene because `pkg-config` could
    not find the system `alsa.pc` package.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
  - The pre-implementation focused-test attempt encountered the same ALSA
    boundary, so the new regression could not be observed failing on this host.
- Compatibility/UX assessment: internal correlation and replay rejection only;
  no visible copy, normal result timing, controls, order semantics, schema, or
  dependency changes.
- Residual risk: source parsing, formatting, call-site inspection, and the diff
  pass, but the focused tests and Rust type-check must still execute once ALSA
  development metadata is available.
- Prior turn commit hash: `966de31f4ad2b8de6451d0ef04a54bcb8fde74a4`
- Next candidate: implement F-06's diagnostic-only cluster-leg message
  redaction, then resume F-05's Chase and TWAP duplicate/late-result audit.

## Turn 6 — Redact Wallet-Cluster Lifecycle Messages in Debug

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium privacy hardening
- Scope: `WalletClusterExecutionLeg::Debug` and one focused model regression
- Invariant: redacting explicit order fields is insufficient if a free-form
  lifecycle message can carry the same CLOID or exchange detail; diagnostic
  formatting must redact both paths.
- Protected behavior: the stored lifecycle message and the cluster execution
  view remain unchanged. No result classification, transition, refresh,
  request, signing, UI, schema, or persistence code changed.
- Evidence: the only non-view consumer of the stored message was its custom
  formatter, and unexpected-resting handlers can embed the CLOID in that message.
  Detailed source evidence is recorded under F-06 above.
- Change: replaced the formatter's raw message value with `<redacted>`.
- Regression test: formats a synthetic uncertain leg whose message repeats its
  CLOID, rejects that raw identifier in `Debug`, checks the explicit redaction
  marker, and verifies the stored message is intact.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene execution_leg_debug_redacts_lifecycle_message`
    stopped in `alsa-sys` before compiling Kerosene because `pkg-config` could
    not find the system `alsa.pc` package.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
  - The pre-implementation focused-test attempt encountered the same ALSA
    boundary, so the strengthened regression could not be observed failing on
    this host.
- Compatibility/UX assessment: diagnostic-only output change; no user-visible,
  trading-semantic, timing, schema, or dependency impact.
- Residual risk: source parsing, formatting, consumer inspection, and the diff
  pass, but the focused test and Rust type-check must still execute once ALSA
  development metadata is available.
- Prior turn commit hash: `47807a313004fec25e53377d1a5198b137bf1d34`
- Next candidate: resume F-05 with adversarial Chase and TWAP direct-result
  replay tests, adding per-attempt correlation only if existing phase guards do
  not already make duplicate and late delivery harmless.

## Turn 7 — Correlate Advanced-Order Direct Results to Exact Attempts

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium invariant hardening
- Scope: internal result messages and handlers for Chase placement, Chase
  modify, and TWAP child placement
- Invariant: a direct exchange result may transition only the exact runtime
  attempt whose task emitted it, even after the strategy returns to the same
  lifecycle phase or reuses the same OID/CLOID.
- Protected behavior: every result whose dispatch-time sequence matches the
  current pending attempt retains the existing classification, account refresh,
  reconciliation, fill accounting, retry, stop, archive, status text, and task
  timing. Order preparation, sizing, pricing, rounding, reduce-only behavior,
  Chase repricing, TWAP cadence/randomization, signing, UI, and persistence are
  untouched.
- Evidence: dispatch and handler tracing confirmed that all three state machines
  already own bounded runtime counters that distinguish recurring phases, but
  their direct-result messages discarded those counters. Exact ordering and
  source references are recorded under F-05 above.
- Change: captured `place_attempt`, `reprice_count`, or
  `slice_index`/`retry_count` in the corresponding `Message` and required it to
  equal current state before handling. This adds no state, identifiers,
  dependencies, schema fields, exchange requests, or mutation retries.
- Regression tests: added conflicting immediate duplicates plus late prior-
  attempt results while later attempts occupy the same phase. The Chase modify
  regression keeps OID 42 constant; TWAP independently exercises a prior slice
  index and a prior retry of the current slice.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene stale_place_result_from_prior_attempt_does_not_settle_current_attempt`
    stopped in `alsa-sys` before compiling Kerosene because `pkg-config` could
    not find the system `alsa.pc` package.
  - `cargo test --package kerosene --bin kerosene stale_modify_result_from_prior_reprice_does_not_settle_current_reprice`
    stopped at the same environment boundary.
  - `cargo test --package kerosene --bin kerosene stale_slice_result_requires_current_index_and_retry_count`
    stopped at the same environment boundary.
  - A final combined `cargo test --package kerosene --bin kerosene stale_`
    retry after the correlation guards were complete stopped at the same
    boundary.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
- Compatibility/UX assessment: runtime message correlation only; no visible
  copy, controls, success/failure behavior for current attempts, order
  semantics, schema, or dependency changes.
- Residual risk: source parsing, formatting, exhaustive call-site inspection,
  and the diff pass, but the new tests and Rust type-check must still execute on
  a host with ALSA development metadata. Existing Chase duration/reprice and
  TWAP slice/retry limits keep these runtime sequences below saturation; no
  counter or wraparound behavior was changed.
- Prior turn commit hash: `fda0d0ee3fe8491624a53e4c5bab3fb6d71a9c65`
- Next candidate: audit prepared-order-to-signed-wire parity and narrow
  structural validation (Track 2), starting with non-finite/negative-zero,
  market-type/asset identity, side, reduce-only, and order-kind preservation
  before deciding whether production hardening is warranted.

## Turn 8 — Validate Signed Order Wire Structure

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium invariant hardening
- Scope: Audit Track 2 across all prepared placement and modify surfaces, plus
  the shared signed-payload boundary
- Invariant: valid prepared fields must reach the signed action unchanged, while
  structurally invalid numeric values or an uncorrelatable placement must never
  be hashed, signed, or posted.
- Protected behavior: exact valid price/size strings, asset, side, TIF/order
  kind, reduce-only, OID/CLOID, JSON/msgpack field order, signature inputs,
  preparation policy, result handling, and user-visible behavior remain
  unchanged.
- Evidence: a repository-wide caller trace found that ticket, preset, Alfred,
  quick, HUD, close, cluster, NUKE, Chase, and TWAP placements all use
  `place_order_task`, while Move and Chase modify use `modify_order_task`; both
  converge on `build_signed_exchange_payload_with_nonce`. Ordinary intents use
  shared preparation; the three direct advanced/emergency constructors validate
  their planned numeric values before constructing prepared orders. Detailed
  evidence and the missing boundary invariant are recorded under F-07.
- Change: added one read-only validation pass at the single signed-action
  payload builder. Place/modify prices and sizes must parse as positive finite
  numbers; placements must carry the established 128-bit hexadecimal CLOID.
  Validation returns static redacted errors and does not mutate the action.
- Regression tests: added exact valid signed-action parity, invalid place price
  and size classes including negative zero, invalid modify size, and missing or
  malformed CLOID cases. Expanded the prepared-request characterization to
  assert the side, order-kind, and reduce-only fields copied by that handoff.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - The pre-implementation
    `cargo test --package kerosene --bin kerosene signed_order_payload_rejects_invalid_wire_numbers_before_signing`
    attempt stopped in `alsa-sys` before compiling Kerosene because
    `pkg-config` could not find the system `alsa.pc` package.
  - Post-implementation focused
    `cargo test --package kerosene --bin kerosene signed_order_payload_`,
    `cargo test --package kerosene --bin kerosene signed_modify_payload_`,
    and `cargo test --package kerosene --bin kerosene signing::tests::actions`
    attempts stopped at the same environment boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    the same pre-existing dependency boundary before checking Kerosene.
- Compatibility/UX assessment: no valid request or normal result path changes.
  Only structurally invalid internal actions that previously could reach signing
  now fail closed before hashing/posting; no schema, dependency, view, copy, or
  trading-policy change was introduced.
- Residual risk: source parsing, formatting, exhaustive call-site inspection,
  and diff checks pass, but the focused tests, Rust type-check, full suite, and
  clippy must still execute on a host with ALSA development metadata. Signing
  intentionally cannot revalidate symbol-to-asset or market capability because
  those identities are absent from the wire action and remain owned by the
  canonical preparation/state-machine boundaries.
- Prior turn commit hash: `2cda6c85c6f557c28283a7709e24ede3066b7fb1`
- Next candidate: audit mutation transport and ambiguity classification (Track
  3), distinguishing failures before serialization/connect/send from failures
  after a request may have reached the exchange without adding any automatic
  mutation retry.

## Turn 9 — Reconcile Contradictory Exchange Acknowledgements

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: Audit Track 3 parsed-response classification at the shared response
  model and all Chase/TWAP mutation handlers that bypass the shared one-shot
  classifier
- Invariant: an explicit error combined with a possible structured exchange
  effect is not a rejection or terminal success; exact CLOID/OID or account
  reconciliation must resolve it before local lifecycle state settles.
- Protected behavior: pure exchange rejection, ordinary resting/fill/cancel
  success, unstructured malformed response, transport failure, existing
  bounded cancel retry, pricing, sizing, automation timing, signed payloads,
  persistence, and normal user-visible status behavior remain unchanged.
- Evidence: the shared classifier already distinguished mixed inner statuses,
  but top-level errors hid their structured effects and advanced handlers
  consumed error/IOC/terminal-cancel branches before ambiguity. Detailed event
  ordering, source references, and risk are recorded under F-08 above.
- Change: added one response-analysis conflict predicate, removed the
  top-level-error exclusion from potential-effect analysis, and ordered every
  Chase/TWAP parsed-result handler so a conflict reaches its existing uncertain
  reconciliation path first. The patch adds no automatic place/modify retry,
  exchange request, state field, identifier, dependency, schema, or visible
  control.
- Regression tests: added top-level-envelope and shared-classifier cases; Chase
  place, modify, and cancel state-machine cases; and TWAP slice and unexpected-
  child-cancel cases. A separate TWAP characterization proves a
  non-conflicting filled status without an OID retains its prior fill-accounting
  path; existing pure-rejection and normal-success tests protect the other
  unchanged branches.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` and `git diff --check` passed before the ledger
    update and are rerun during final review.
  - Pre- and post-implementation
    `cargo test --package kerosene --bin kerosene conflicting_` attempts stopped
    in `alsa-sys` before compiling Kerosene because `pkg-config` could not find
    the system `alsa.pc` package.
  - `cargo test --package kerosene --bin kerosene signing::tests::responses` and
    `cargo test --package kerosene --bin kerosene execution_result_classifier_`
    stopped at the same environment boundary.
  - `cargo test --package kerosene --bin kerosene non_conflicting_fill_without_oid_preserves_existing_fill_accounting`
    stopped at the same boundary; this is the explicit unchanged-behavior
    characterization adjacent to the TWAP conflict guard.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
- Compatibility/UX assessment: valid and purely rejected responses take their
  former branches with identical status copy and task timing. Only an anomalous
  internally contradictory response remains uncertain instead of being
  consumed as definitive; this is lifecycle plumbing, not a product-policy or
  normal UX change.
- Residual risk: formatting, source analysis, exhaustive result-handler search,
  and diff review pass, but all Rust tests/type-check/clippy still require a
  host with ALSA development metadata. The transport client still deliberately
  flattens all failures after action construction into conservative
  transport-unknown strings.
- Prior turn commit hash: `710139d929d15fce575a4b1e056da8dc6410969f`
- Next candidate: finish Track 3 by auditing the exact HTTP client boundary for
  serialization/connect/send/body/parse phase provenance and redaction. Add
  typed internal context only if it improves diagnostics while every uncertain
  mutation remains fail-closed and no retry or user-facing behavior changes.

## Turn 10 — Isolate and Bound Signed Mutation Transport

- Status: implemented; executable Rust validation environment-blocked
- Severity: High transport replay prevention plus Medium diagnostic/privacy
  hardening
- Scope: completion of Audit Track 3 from pre-signing validation through HTTP
  policy, response parsing, task-message mapping, and immediate-result debug
- Invariant: one lifecycle attempt initiates at most one HTTP send; transport
  uncertainty returns to exact reconciliation; and both success and error
  results are diagnostically safe before they can enter derived `Message::Debug`.
- Protected behavior: valid signed payloads, endpoint/method/body, nonce and
  expiry, safe error copy, all outcome kinds, conservative unknown handling,
  CLOID/OID reconciliation, application-level target-cancel policy, ordinary
  status copy, controls, persistence, and normal request timing remain
  unchanged.
- Evidence: every signed wrapper and task caller was retraced. The exact phase
  decisions are recorded in the Mutation Transport Phase Audit, and the three
  confirmed boundary weaknesses are detailed under F-09 through F-11.
- Change: replaced shared generic HTTP policy with a dedicated no-redirect,
  no-retry exchange client; retained the existing 5/15/60-second limits and
  redundantly bound each request to 15 seconds; removed the unsafe unbounded
  fallback; applied one order-aware sanitizer to every signing exit; and made
  successful exchange-response debug metadata-only. Non-success HTTP envelopes
  can no longer confirm an apparently successful body. No downstream result
  type or handler branch changed because treating provably local errors less
  conservatively would alter UX without improving financial safety.
- Regression tests: added a loopback 307 replay trap, explicit per-request
  timeout inspection, secret/CLOID result-debug redaction, safe error and `Ok`
  preservation, 128-bit order-identifier redaction, and successful
  response-detail debug redaction, plus non-success HTTP success/rejection/
  conflict classification.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` and `git diff --check` passed before the ledger
    update and are rerun during final review.
  - Pre- and post-implementation
    `cargo test --package kerosene --bin kerosene exchange_client_does_not_replay_a_redirected_mutation`
    attempts stopped in `alsa-sys` before compiling Kerosene because
    `pkg-config` could not find the system `alsa.pc` package.
  - Focused `exchange_request_has_a_mutation_local_timeout`,
    `exchange_result_`, `parse_exchange_response_`,
    `sensitive_order_text_redacts_128_bit_cloid`,
    `exchange_response_error_status_redacts_sensitive_values`, and
    `exchange_response_debug_redacts_successful_order_details` test attempts
    stopped at the same environment boundary.
  - Focused `non_success_http_status_` test attempts stopped at the same
    boundary.
  - The protected existing
    `ambiguous_transport_results_require_account_refresh` test attempt also
    stopped at that boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
- Compatibility/UX assessment: the ordinary client used the same user agent
  and timeouts; valid exchange calls, explicit rejections, and all normal
  handler-visible strings/state are unchanged. Only hidden HTTP resend/redirect
  behavior, an unsafe client-build fallback, diagnostic exposure, and false
  confirmation from a contradictory non-2xx envelope are removed.
- Residual risk: formatting, exhaustive signed-call/result-handler inspection,
  phase analysis, and diff checks pass, but the new tests/type-check/clippy must
  execute on a host with ALSA development metadata. F-12 confirms that the
  separate `orderStatus` read-error lane needs the same pre-message CLOID
  protection in the next narrow batch.
- Prior turn commit hash: `501cfb097428987c0cd8f1dbb4f47779655db468`
- Next candidate: address F-12 at the `orderStatus` task boundary, then resume
  cancel/move ownership and repeated-attempt correlation in Track 4.

## Turn 11 — Redact Order-Status Result Diagnostics

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium privacy hardening
- Scope: F-12 across the `orderStatus` HTTP preview, parser, public task exits,
  result model, and every caller that maps those results into `Message`
- Invariant: exact OID/CLOID values remain available for request construction,
  correlation, and lifecycle handling, but neither the successful nor error
  result lane may expose sensitive order text through derived message debug.
- Protected behavior: status request JSON, exact expected/actual correlation,
  successful result fields, missing/open/filled/terminal classification, task
  routing and timing, retry/reconciliation policy, stored status text, normal
  user-visible status behavior, signing, persistence, and order semantics.
- Evidence: a complete caller trace found that one-shot, NUKE, cancel, move,
  Chase, TWAP, and wallet-cluster status tasks all use
  `fetch_order_status_by_cloid` or `fetch_order_status_by_oid`; before this turn,
  handler-time sanitization occurred only after the `Result` had entered a
  derived message. The concrete exposure and source references are recorded
  under F-12.
- Change: applied the existing order-aware redactor at both public status-result
  exits, redacted HTTP text before truncating it, changed OID/CLOID
  mismatch/missing errors to retain their diagnostic category without values,
  and sanitized the external status field only when formatting
  `OrderStatusResult::Debug`. No request, result, state-machine, message, or
  persistence type changed.
- Regression tests: added adversarial pre-message error formatting, a CLOID
  spanning the old preview cutoff, OID/CLOID mismatch and missing-field value
  checks, external parser-error CLOID redaction, and successful-result status
  debug redaction. Existing safe error text and exact successful result values
  are explicitly preserved.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` and `git diff --check` passed before the ledger
    update and are rerun during final review.
  - The pre-implementation
    `cargo test --package kerosene --bin kerosene order_status_result_error_is_redacted_before_message_mapping -- --exact`
    attempt stopped in `alsa-sys` before compiling Kerosene because
    `pkg-config` could not find the system `alsa.pc` package.
  - The post-implementation focused
    `cargo test --package kerosene --bin kerosene order_status_` attempt stopped
    at the same environment boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
- Compatibility/UX assessment: valid and safe status paths retain exact data,
  copy, state changes, and timing. Only anomalous identifier-bearing diagnostic
  strings and debug output are redacted; no view, control, schema, dependency,
  network policy, or trading behavior changed.
- Residual risk: formatting, source inspection, exhaustive public-fetch caller
  tracing, and diff review pass, but the new tests/type-check/clippy must execute
  on a host with ALSA development metadata. F-13 separately records raw
  identifier context fields on derived messages; F-12 does not claim those
  sibling fields are safe.
- Prior turn commit hash: `9123fcf33d61c01d3a168b80bea58185d503957c`
- Next candidate: address F-13 with typed redacted message-context values and
  whole-message debug regressions, then resume cancel/move ownership and
  repeated-attempt correlation in Track 4.

## Turn 12 — Redact Order-Identifier Message Context

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium privacy hardening
- Scope: F-13 across every direct exchange OID/CLOID field on `Message`, its
  publication and consumption boundaries, and the adjacent stopped-Chase cancel
  request diagnostic
- Invariant: transient message and planning diagnostics must not expose an
  exchange order identifier, while update handlers receive the exact same
  primitive value and lifecycle context as before.
- Protected behavior: all message variants and routes, exact OID/CLOID values,
  account/symbol/attempt context, handler signatures, state transitions,
  cancellation/modify/status requests, chart cancel-hover behavior, task
  timing, visible status/UI, signing, order semantics, and persistence.
- Evidence: a field-and-caller inventory traced all named OID/CLOID fields plus
  the positional chart cancel-hover OID. Fourteen order-route variants converge
  on `update_order`; the chart variant converges on `update_chart`. Every
  constructor was found by both variant-specific and repository-wide searches,
  and no alternate consumer was found. The concrete prior exposure and current
  source references are recorded under F-13.
- Change: introduced two message-only redacted wrappers with exact consuming
  accessors; wrapped at every publication site; unwrapped only at the order or
  chart update boundary; and made the stopped-Chase request's existing custom
  debug formatter value-neutral for OID. The patch adds no state field, request,
  retry, dependency, schema, or policy.
- Regression tests: added a single table-style whole-message formatter test for
  all fifteen variants, an exact wrapper round-trip test, and an OID assertion
  on the stopped-Chase request formatter. The existing chart cancel-click and
  TWAP retry-task tests remain the protected producer-path checks.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` and `git diff --check` passed before the ledger
    update and are rerun during final review.
  - The pre-implementation
    `cargo test --package kerosene --bin kerosene order_identifier_message_debug_redacts_oid_and_cloid_fields`
    attempt stopped in `alsa-sys` before compiling Kerosene because
    `pkg-config` could not find the system `alsa.pc` package.
  - The extended pre-implementation
    `cargo test --package kerosene --bin kerosene stopped_chase_cancel_request_debug_redacts_agent_key_and_oid`
    attempt stopped at the same environment boundary.
  - Post-implementation focused
    `cargo test --package kerosene --bin kerosene order_identifier_message_` and
    `cargo test --package kerosene --bin kerosene stopped_chase_cancel_request_debug_redacts_agent_key_and_oid`
    attempts stopped at the same boundary.
  - Protected producer-path
    `cargo test --package kerosene --bin kerosene fisheye_left_click_on_order_cancel_button_cancels_order`
    and
    `cargo test --package kerosene --bin kerosene unexpected_cancel_retry_due_task_waits_for_delay`
    attempts stopped at the same boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
- Compatibility/UX assessment: the wrappers exist only while a message is in
  transit and consume back to the original primitive before any handler logic.
  All visible and trading behavior is unchanged; only derived/custom diagnostic
  output substitutes a redaction marker for the identifier.
- Residual risk: formatting, exact-value characterization, complete variant
  producer/consumer inspection, direct-field source scans, and diff review pass,
  but the new tests/type-check/clippy must execute on a host with ALSA
  development metadata. The broader Track 9 audit still needs to inspect local
  derived planning/state enums that are not carried by `Message`; no claim is
  made about those later diagnostic surfaces here.
- Prior turn commit hash: `349f532e89d33135d3abc912d994af9daf7f91b5`
- Next candidate: resume Track 4 by auditing cancel and move ownership across
  repeated attempts on the same OID, delayed direct/status results, indicator
  disappearance, and authoritative open-order/fill refresh ordering.

## Turn 13 — Correlate Cancel and Move Attempts

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: F-14 across cancel/move dispatch, direct and status messages, pending
  runtime contexts, cancel phases, account-refresh cleanup, and shared request
  sequence allocation
- Invariant: only the immutable owner of one dispatched cancel or modify may
  consume its direct/status callback or clear its pending state; neither a
  reused OID nor a presentation indicator can identify an attempt by itself.
- Protected behavior: exact cancel/modify wire mutations, prepared asset/OID,
  side, size, reduce-only and price values, captured account/key behavior,
  response classification, normal statuses and refresh scheduling, confirmed
  local move-price projection, UI, persistence, and all retry policy.
- Preconditions/event ordering: an old cancel indicator/pending slot disappears
  or an old move context/status slot is cleared; a new attempt reuses the same
  account/symbol/OID; then the old direct or status result arrives after the new
  attempt owns that tuple. A duplicate cancel result can also arrive after its
  first result has advanced the attempt into status reconciliation.
- Evidence: the complete pre-change/direct/status call graph matched cancel by
  account (direct) or account/OID/symbol (status), and move by OID-key plus
  account (direct) or account/OID/symbol (status). Cancel target recovery could
  fall through from an expired indicator to the current account-scoped pending
  request. Account refresh cleanup did not distinguish a cancel awaiting its
  direct result from one awaiting reconciliation. F-14 records the concrete
  prior behavior and current source references.
- Change: renamed the one-shot-only runtime counter into a shared lifecycle
  request allocator that skips all live one-shot/cancel/move IDs across wrap;
  allocated an ID before each cancel/move dispatch; propagated it through both
  direct and status messages; required it in every result owner match; and made
  cancel's `AwaitingResult` to `CheckingStatus` transition explicit and
  one-way. Refresh cleanup can no longer erase a cancel whose direct result has
  not arrived. No field is persisted or sent to Hyperliquid.
- Tests/checks:
  - Added stale same-OID direct/status regressions for cancel and move, duplicate
    cancel direct-result coverage, indicator-expiry ownership coverage, an
    awaiting-direct-result refresh regression, and collision-aware allocator
    wrap coverage across all four live-state families.
  - Updated nearby cancel-status fixtures to enter the status-check phase
    explicitly and updated all constructor/message/handler call sites.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - Focused `cargo test --package kerosene --bin kerosene stale_cancel_`,
    `stale_move_`,
    `duplicate_cancel_result_does_not_override_status_reconciliation`,
    `account_refresh_does_not_clear_cancel_attempt_awaiting_direct_result`, and
    `order_lifecycle_request_allocator_skips_live_ids_across_wrap` attempts all
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - Nearby/protected `cancel_order_status_`, `move_order_status_`,
    `cancel_result_ambiguous_ack_uses_pending_request_after_indicator_expires`,
    and `move_result_success_carries_confirmed_price_into_local_snapshot` test
    attempts stopped at the same dependency boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing boundary before checking Kerosene.
- Compatibility/UX assessment: the request sequence and cancel phase are
  runtime-only plumbing. The normal successful, rejected, ambiguous, and
  transport-unknown paths publish the same status text and tasks; only stale,
  duplicate, or prematurely refreshed ownership is prevented from touching a
  different attempt.
- Residual risk: formatting, exact constructor/consumer inventory, phase and
  state-transition inspection, adversarial tests, and diff review pass, but
  the crate has not type-checked and the tests/clippy must execute on a host
  with ALSA development metadata. F-15 remains: refresh fallback must prove the
  origin symbol lane, and move reconciliation must retain/compare the expected
  target price before uncertainty is released.
- Prior turn commit hash: `32b998cf9877efe3a803a320d023450151d70349`
- Next candidate: address F-15 with symbol-lane-scoped cancel/move cleanup and
  exact expected-price evidence for an open moved order, preserving the current
  fetch cadence, wire mutation, normal statuses, and UI.

## Turn 14 — Scope Cancel and Move Refresh Evidence

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: F-15 across successful account-refresh cleanup, cancel origin-lane
  evidence, move prepared-price provenance, move snapshot classification, and
  status-phase test fixtures
- Invariant: an account snapshot may release cancel/move uncertainty only when
  it successfully fetched the exact operation's open-order lane; an open move
  additionally requires a parseable live price compared with the immutable
  prepared target, while target absence is terminal evidence.
- Protected behavior: exact cancel/modify preparation and signed payloads,
  account/key/request correlation, result/status classification and messages,
  refresh scope/cadence/generations, normal valid-snapshot cleanup, confirmed
  local move projection, indicators, retries, UI, and persistence.
- Preconditions/event ordering: a cancel or move becomes uncertain; the user
  changes the selected HIP-3 universe or the target lane fails while another
  lane succeeds; alternatively, a covering snapshot contains the target with a
  malformed or differently formatted/live price; successful account loading
  reaches the shared cleanup boundary.
- Evidence: account refresh chooses scope from the current market universe and
  a scoped snapshot can be complete for only that scope. `AccountData` already
  records per-lane fetch success, but cancel/move cleanup used only the global
  completeness boolean. Move's prepared price lived in its indicator and
  prepared request, then disappeared before status reconciliation. F-15 records
  the concrete source evidence and failure ordering.
- Change: reused `has_complete_open_orders_for_symbol` for both operations;
  retained the exact prepared price in `PendingMoveOrderContext`; copied it into
  pending status state only for ambiguous/transport-unknown outcomes; redacted
  it in custom diagnostics; and classified covering move snapshots as target
  absent, expected-price open, different-valid-price open, or insufficient.
  Only the first three preserve the existing cleanup. No request, task, polling,
  schema, dependency, or view was added. Status test fixtures now remove the
  direct-result context/indicator before arming reconciliation, matching the
  production phase boundary.
- Tests/checks:
  - Added cancel and move regressions for wrong HIP-3 scope and incomplete
    target lanes, covering cancel cleanup with a still-open target, move cleanup
    with formatted expected and different valid prices, malformed live-price
    retention after a real uncertain direct-result transition, and pure
    expected/different/absent snapshot classification.
  - Extended captured move-context and pending-status diagnostic tests to prove
    exact expected-price retention and diagnostic redaction.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - Pre- and post-implementation focused
    `cargo test --package kerosene --bin kerosene cancel_refresh_requires_complete_origin_symbol_lane`,
    `move_refresh_requires_complete_origin_symbol_lane`,
    `move_covering_refresh_requires_parseable_live_target_price`, and
    `move_covering_refresh_preserves_cleanup_for_valid_live_price` attempts all
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - Post-implementation focused
    `cancel_covering_refresh_preserves_existing_cleanup_when_target_is_open`,
    `move_snapshot_reconciliation_distinguishes_expected_different_and_absent_target`,
    `pending_move_context_reuses_captured_agent_key_for_same_account`, and
    `pending_move_status_request_debug_redacts_account_and_oid` attempts stopped
    at the same dependency boundary.
  - Nearby/protected `cancel_order_status_`, `move_order_status_`,
    `move_result_failure_keeps_local_price`,
    `account_refresh_must_cover_one_shot_symbol_before_clearing_status_request`,
    and `complete_open_order_coverage_tracks_each_symbol_lane` attempts stopped
    at the same boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing boundary before checking Kerosene.
- Compatibility/UX assessment: complete covering snapshots with an absent
  target, the expected price (including equivalent formatting), or a different
  valid current price retain the same cleanup and enabled/disabled timing. Only
  an unrelated/incomplete lane or malformed target evidence remains blocked.
  The captured price is runtime-only plumbing and never changes the order or
  visible status.
- Residual risk: source parsing, formatting, exhaustive constructor/consumer
  inspection, phase-order review, adversarial tests, and diff review pass, but
  Rust type-check/tests/clippy must execute on a host with ALSA development
  metadata. Track 5 still needs a fresh end-to-end Chase audit across scoped
  REST refreshes, partial open-order/fill lanes, and user-stream ordering.
- Prior turn commit hash: `8771aa8e87771d6ae3db312e01d67ba80962acf3`
- Next candidate: begin the remaining Track 5 audit by tracing Chase
  disappearance/replacement and fill reconciliation across selected HIP-3
  scopes, partial account snapshots, websocket lag/repair, and reversed REST
  versus user-stream delivery, preserving all repricing and archive behavior.

## Turn 15 — Scope Chase Reconciliation to Its Origin Lane

- Status: implemented; executable Rust validation environment-blocked
- Severity: Critical
- Scope: F-16 across Chase REST refresh, account/stream fill reconciliation,
  replacement dispatch, stop verification, fill completion, and archive
- Invariant: only a snapshot that successfully fetched the active Chase's
  origin-symbol open-order lane may prove prior exposure absent, authorize
  archive, or permit a replacement; account-wide fills are an independent
  evidence lane.
- Protected behavior: exact Chase prices, sizes, sides, reduce-only and asset
  fields, CLOID/OID identity, place/reprice limits, cadence, retries, result and
  status classification, normal covering-snapshot transitions, all visible
  strings, UI, persistence, and secret lifetime.
- Preconditions/event ordering: a Chase on one HIP-3 dex awaits verification
  after no-fill status, fill completion, or stop/cancel; the selected universe
  changes; an `Ok` refresh for a different dex returns globally complete fills
  and open orders for its own scope; the prior global open-order check treats
  absence from that unrelated result as absence from the Chase's lane.
- Evidence: account refresh derives its request scope from the current market
  universe, whereas `AccountData` exposes explicit per-symbol open-order
  authority. The pre-change Chase refresh, fill cleanup, and final replacement
  guard used only snapshot-wide completeness. Websocket disappearance was
  already correctly dex-gated, fill merging and Chase attribution deduplicate
  by fill identity and filter account/coin/side/OID/cutoff, and place/modify
  callbacks retain dispatch-attempt correlation. F-16 records the concrete
  boundaries and failure consequences.
- Change: compute an immutable set of covered active Chase symbols once per
  account snapshot; pass it into fill reconciliation; evaluate every Chase
  refresh/stop/archive/replacement branch against its own symbol; and replace
  the final known-OID placement guard's global check with the same per-symbol
  predicate. Uncovered state remains in its existing verification lifecycle
  and refreshes; no new mutation, retry, timer, request, state field, schema,
  dependency, view, or message was introduced.
- Tests/checks:
  - Added regressions proving a wrong-scope HIP-3 refresh cannot dispatch a
    no-fill replacement, archive a stopping Chase, or archive a filled Chase
    whose open orders remain unknown, plus an independent final-dispatch guard
    test for known prior OIDs.
  - Pre- and post-implementation focused attempts for
    `unrelated_hip3_refresh_does_not_place_chase_replacement`,
    `unrelated_hip3_refresh_does_not_archive_stopping_chase`,
    `unrelated_hip3_refresh_does_not_archive_filled_chase_with_unknown_open_orders`,
    and `chase_replacement_requires_open_order_coverage_for_its_symbol` all
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - Nearby/protected attempts for
    `no_fill_terminal_status_allows_clean_replacement`,
    `stopped_chase_clears_only_after_no_known_open_orders_remain`,
    `chase_fill_reconciliation_removes_fully_filled_chase`, and
    `chase_place_assigns_unique_cloid_per_place_attempt` stopped at the same
    dependency boundary. A broader `cargo test --package kerosene --bin
    kerosene chase` attempt did likewise.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing boundary before checking Kerosene.
- Compatibility/UX assessment: a complete covering lane retains the exact
  prior replacement, safety-cancel, and archive behavior. Only a snapshot that
  never observed the Chase's lane remains uncertain. All user-facing copy and
  normal enabled/disabled behavior are unchanged; this is runtime plumbing and
  no persisted type changed.
- Residual risk: source formatting, all call-site inventory, per-state ordering,
  fill-deduplication, websocket dex-scope, attempt-correlation, and diff review
  pass, but the crate has not type-checked and tests/clippy must execute on a
  host with ALSA development metadata. REST snapshot versus newer websocket
  delta ordering needs a separate Track 8 audit.
- Prior turn commit hash: `97c40f9fca2cdb9e9035e682b87a5ac5824c2ec5`
- Next candidate: audit whether an account REST request captured before a newer
  user-stream delta can complete afterward and replace that delta without
  scheduling a follow-up refresh. Trace request-start revision, generation,
  `account_refresh_followup_pending`, and every websocket mutation before
  deciding whether this is a finding; preserve refresh cadence and display
  behavior.

## Turn 16 — Preserve Newer User Data Across REST Completion

- Status: implemented; executable Rust validation environment-blocked
- Severity: Critical
- Scope: F-17 across connected account request context, successful REST result
  application, all account-bearing user-data lanes, initial loading, shared
  lifecycle cleanup, documentation, and context fixtures
- Invariant: a connected REST result may replace account state or reconcile an
  order only if no user-data event advanced that account after request
  dispatch; an event received before a mergeable base exists must force one
  post-event snapshot before lifecycle cleanup.
- Protected behavior: account addresses/providers/key generations, request
  generations, scopes/cadence, successful no-conflict refreshes, errors and 429
  backoff, existing explicit/lag follow-ups, display/status text, order
  preparation and signing, automation parameters, UI, persistence, and secrets.
- Preconditions/event ordering: one connected refresh starts; its response can
  represent pre-event state; a valid same-account open-order/fill/position/
  balance frame arrives and updates the local snapshot (or cannot merge during
  initial load); then the still-current REST result arrives without an explicit
  second refresh or lag signal. All old provider/address/generation checks pass.
- Evidence: ordinary mergeable user-data frames advance
  `account_data_revision`, but connected request context previously captured no
  revision. Same-generation success then replaced `account_data` and ran every
  shared cleanup consumer. When no account data existed, ordinary frames were
  skipped while loading without setting the existing follow-up flag. F-17
  records the complete call graph, false-safety consequences, and fixed source
  boundaries.
- Change: extended only `ConnectedSnapshot` context with
  `account_data_revision_at_dispatch`; captured it at both initial-connect and
  forced-refresh task creation; checked it after provider/generation/address
  ownership but before any successful-result mutation; preserved the newer
  websocket-merged state and pending uncertainty on mismatch; and started one
  coalesced post-frame refresh. An account frame received with no initial base
  now sets the existing follow-up/reconciliation bits without launching a
  parallel task. Off-account TWAP context and error handling are untouched.
- Tests/checks:
  - Added an adversarial request-before/open-order-frame/result-after regression
    that proves the live OID and revision survive, one-shot uncertainty remains,
    and exactly one newer request generation is active.
  - Added initial-fetch coverage proving an unmergeable account frame queues a
    follow-up while the original request remains the only in-flight task.
  - Pre- and post-implementation
    `cargo test --package kerosene --bin kerosene rest_result_started_before_ws_delta_does_not_overwrite_or_reconcile`
    attempts stopped in `alsa-sys` before Kerosene compilation because
    `pkg-config` could not find the system `alsa.pc` package. The post-change
    `websocket_account_delta_queues_followup_during_initial_fetch` attempt
    stopped at the same boundary.
  - Nearby/protected `queued_refresh_followup_result_clears_reconciliation_required`,
    `stale_same_account_refresh_result_does_not_overwrite_newer_snapshot`,
    `refresh_requested_mid_fetch_is_queued_and_runs_after_load`,
    `websocket_open_order_update_refreshes_only_matching_dex_lane`,
    `twap_reconciliation_result_after_switching_back_does_not_replace_connected_snapshot`,
    `account_load_error_redacts_account_error`, and
    `address_bearing_message_debug_redacts_values` attempts all stopped at the
    same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing boundary before checking Kerosene.
- Compatibility/UX assessment: a no-conflict response follows the identical
  application and cleanup path. Only a causally superseded success is withheld;
  the newer already-visible websocket state no longer flickers backward and
  existing uncertainty stays blocked until one sequential fresh result. During
  initial loading the first result retains the established display-population
  behavior but cannot settle orders before its queued follow-up. No copy,
  control, schema, or persisted value changed.
- Residual risk: formatting, constructor/consumer inventory, serialized Elm
  ordering, all websocket mutation lanes, initial/no-base behavior, cleanup
  reachability, error/backoff isolation, and diff review pass, but Rust
  type-check/tests/clippy must execute with ALSA development metadata.
  Continuous live deltas intentionally remain fail-closed and may defer
  reconciliation; requests stay sequential and the existing 429 backoff bounds
  pressure. Same-address stream-generation/reconnect provenance remains to be
  audited separately.
- Prior turn commit hash: `37b57f74b317a7f9d440557ac0e43ed618c57efa`
- Next candidate: continue Track 8 by tracing `WsUserDataStreamParams`, iced
  subscription replacement, manager reconnects, broadcast lag/closure, and
  queued `WsUserDataUpdate` messages for the same address. Determine whether
  address plus subscription identity proves stream provenance or whether a
  runtime generation is required; preserve reconnect behavior and stream UX.

## Turn 17 — Reject Frames From Replaced User-Data Recipes

- Status: implemented; executable Rust validation environment-blocked
- Severity: Critical
- Scope: F-18 across user-data recipe identity, account/detail/cluster message
  context, runtime lifecycle generations, source-address validation, recipe
  input mutation sites, reconnect/lag/closure audit, docs, and regressions
- Invariant: a queued user-data frame may change account, automation, wallet,
  or cluster freshness state only while its exact address/purpose/dex recipe
  incarnation is still requested by current application state.
- Protected behavior: websocket topics/payloads, shared-manager refcounts and
  reconnect backoff, lag signaling and REST repair, account and cluster update
  semantics for current frames, order preparation/signing, automation policy,
  stream cadence outside actual consumer replacement, UI/copy, persistence,
  and secrets.
- Preconditions/event ordering: an iced recipe pulls a valid frame; an account
  reconnect or consumer topology/scope change replaces/removes that recipe;
  cancellation occurs while the old task awaits its application-queue send;
  the old same-address frame arrives after the new state/recipe. Address-only
  routing previously accepted it.
- Evidence: iced 0.14's tracker selects cancellation only against
  `stream.next()`, then sends a won item without another cancellation select.
  Independent old/new recipes can therefore overlap for one queued item. The
  manager's ordinary socket reconnect is different: it retains one broadcast
  receiver and orders old-socket frames, disconnect, resubscription, and new
  frames within that stream. Lag emits a reconciliation signal and requests
  the existing gated reconnect. Broadcast closure requires singleton manager
  task/command teardown and is not a normal reconnect transition. F-18 records
  the concrete source and consequences.
- Change: retained each complete `WsUserDataStreamParams` in subscription
  output, added its runtime generation to hash/equality, and required exact
  current params plus normalized source at all three update routes. Account
  sessions use a monotonic generation. Detail windows use unique per-address
  generations across close/reopen and connected-address exclusion, without
  rotating other addresses; config clear removes the runtime address map while
  preserving the allocator. Selected-cluster topology/profile changes rotate
  one shared cluster generation. Visible-dex changes rotate only the consumers
  whose recipe inputs change. No value is persisted or sent to Hyperliquid.
- Tests/checks:
  - Added an account regression proving a canceled same-address recipe's queued
    lag frame cannot mark the new session loading/reconciling.
  - Added a cluster regression proving a canceled recipe's full position frame
    cannot clear `stale` or refresh the timestamp used by close-size preflight.
  - Added parameter/source, detail close/reopen isolation, config-clear address
    cleanup, production same-account reconnect rotation, selected-cluster
    member rotation, and diagnostic-redaction coverage.
  - Focused attempts for
    `queued_same_address_frame_from_replaced_account_stream_is_ignored`,
    `queued_position_frame_from_replaced_cluster_stream_cannot_mark_snapshot_fresh`,
    `reopened_wallet_detail_gets_new_generation_without_rotating_other_address`,
    and `address_bearing_message_debug_redacts_values` all stopped in
    `alsa-sys` before Kerosene compilation because `pkg-config` could not find
    the system `alsa.pc` package. A broader `queued_` attempt stopped at the
    same boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: this subscription change does not justify
    launching the app against an unknown local credential/live-trading config,
    and the same ALSA dependency blocks compilation first.
- Compatibility/UX assessment: every current frame follows its identical prior
  handler. Rotations coincide with a recipe input/session change that already
  removes or replaces that consumer, except same-address reconnect where the
  explicit reconnect now creates the required ownership boundary and its
  existing full REST refresh remains authoritative. No view, string, control,
  schema, stored config, order field, retry, or timer changed.
- Residual risk: formatting, message/call-site inventory, every production
  recipe-input mutation, source normalization, generation lifecycle, manager
  ordering, lag and closure, redaction, and full diff review pass. The crate
  has not type-checked; focused/full tests and clippy must execute on a host
  with ALSA development metadata. Terminal singleton-manager task failure is
  reserved for the later startup/shutdown fault-handling track.
- Prior turn commit hash: `8c4bd540d135796c2aa3a20f6757f18848047709`
- Next candidate: begin the remaining Track 6 TWAP audit by tracing scheduler
  ticks, pending child creation, deterministic CLOID/retry identity, reconnect
  and delayed result/status ordering, unexpected resting-child cancellation,
  fill attribution, stop, completion, and archive. Preserve slice sizes,
  cadence, price bounds, retry policy, and all visible lifecycle behavior.

## Turn 18 — Claim TWAP Status Results by Exact Attempt

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: F-19 across TWAP status-task dispatch, message context, retry
  ownership, fill-reconciliation cleanup, diagnostics, docs, and adversarial
  ordering regressions
- Invariant: at most one status lookup owns the current TWAP child CLOID/retry
  attempt, and only its exact result may mutate retry or reconciliation state.
- Protected behavior: deterministic child CLOIDs, IOC placement/cancel request
  construction, slice sizing, pricing, cadence, randomization, book gates,
  retry limits/delays, account-fill confirmation, stop/timeout/archive paths,
  order status text, UI, persistence, and secrets.
- Audit evidence: scheduler ticks synchronously claim `pending_op` before
  returning a place task; direct placement results require exact slice index
  and retry count; child CLOIDs remain deterministic; account fill attribution
  requires OID plus exact coin/side and aggregates by per-child maxima/sums;
  stop and deadline paths cannot schedule while pending/status reconciliation
  remains. Status repair was the uncovered recurring lane: its tasks/results
  carried only TWAP ID and CLOID, while retries and later account-fill proof
  reused mutable state under that same CLOID. F-19 records the exact ordering
  failure and consequence.
- Change: added a runtime-only optional armed status attempt. Both immediate and
  delayed status helpers claim that owner before creating a task and refuse a
  second owner. `TwapOrderStatusLoaded` carries the attempt; the handler
  requires and consumes exact CLOID/attempt ownership before applying any
  result. Retry scheduling synchronously arms the next existing retry count.
  Account-fill resolution and timeout cleanup clear ownership defensively.
  Custom debug output exposes only a boolean presence value.
- Tests/checks:
  - Added regressions proving duplicate dispatch creates one task, a duplicate
    missing result cannot consume retry budget/events twice, and a stale
    missing result cannot overwrite canceled/no-fill account reconciliation.
  - Updated filled, canceled, rejected, retry-exhaustion, stop, terminal,
    account-origin, and debug tests for exact attempt ownership and cleanup.
  - Focused attempts for
    `duplicate_status_dispatch_keeps_one_in_flight_owner`,
    `duplicate_status_result_cannot_consume_retry_budget_twice`,
    `stale_missing_status_cannot_override_no_fill_confirmation_phase`,
    `canceled_status_check_waits_for_account_fill_confirmation`,
    `running_twap_reconciliation_clears_status_metadata_after_fill`, and
    `duplicate_slice_result_cannot_consume_a_settled_attempt` all stopped in
    `alsa-sys` before Kerosene compilation because `pkg-config` could not find
    the system `alsa.pc` package.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: this internal state/message change does not
    justify launching an unknown local credential/live-trading configuration,
    and compilation is already blocked by ALSA metadata.
- Compatibility/UX assessment: every uniquely owned status result follows the
  identical classifier, retry, refresh, cancel, finish, and archive code. The
  attempt is local correlation only; it is not signed, sent to Hyperliquid,
  persisted, rendered, or used to alter timing. Only duplicate dispatches and
  duplicate/delayed results that do not own current state are ignored. No
  visible string, control, schema, order field, retry, timer, or normal task
  changed.
- Residual risk: formatting, field initialization, message producer/consumer
  inventory, status-result branches, fill/timeout cleanup, redacted debug,
  protected scheduler behavior, and full diff review pass. The crate has not
  type-checked; focused/full tests and clippy must execute on a host with ALSA
  development metadata. The target-correlated unexpected-child cancel retry
  trigger carries an attempt, but the direct cancel result does not; its
  duplicate/reversed-result behavior remains the next Track 6 candidate.
- Prior turn commit hash: `2cbd350d38f08989f21e3305e1f2b0355c2afba0`
- Next candidate: continue Track 6 by adversarially ordering TWAP unexpected-
  child cancel retry triggers and direct results for the same OID/CLOID. Prove
  whether target identity plus current retry state is sufficient, or add the
  smallest runtime attempt ownership needed to prevent an older failure or
  success from settling a newer retry; preserve the target-specific cancel,
  retry limit/delay, refresh, stop, and visible behavior.

## Turn 19 — Claim TWAP Unexpected-Cancel Attempts

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: F-20 across TWAP unexpected-resting cancellation entry, delayed retry
  dispatch, result context, bounded retry ownership, terminal guards,
  diagnostics, docs, and adversarial regressions
- Invariant: one target-specific cancellation retry attempt creates at most one
  in-flight task, and exactly one matching result may consume its retry budget
  or settle the pending child.
- Protected behavior: cancel-by-CLOID preference and OID fallback, signed
  request values, response classification, five-attempt limit, backoff delays,
  account refresh and balance invalidation, stop/archive handling, all status
  and event strings, scheduling, UI, persistence, and secrets.
- Preconditions/event ordering: an unexpected-resting cancel returns an
  ambiguous/error result and schedules the next retry; the direct result is
  duplicated or delayed past the next dispatch, or the same retry-due message
  is delivered twice while its target and shared counter still match. Target
  checks previously accepted the former and counter checks dispatched both
  copies of the latter.
- Evidence: the retry trigger carried `attempt` but did not claim in-flight
  ownership. Every cancel task result discarded that attempt, and the handler
  mutated shared `cancel_retries` using only the long-lived OID/CLOID target.
  Repeating one failure enough times could reach terminal error without the
  corresponding number of owned attempts. F-20 records the full call graph,
  ordering, consequence, and protected semantics.
- Change: added a runtime-only optional in-flight cancel attempt. A shared
  arming method atomically checks non-terminal state, current retry count,
  exact target, and absence of another owner. Both initial cancellation paths
  and delayed retry dispatch use it. `TwapUnexpectedCancelResult` now carries
  the captured attempt; its handler requires the exact target/retry/owner and
  consumes ownership before executing the unchanged classifier. Scheduled
  backoff deliberately has no owner until the due message dispatches. Debug
  output retains only a boolean presence field.
- Tests/checks:
  - Added regressions proving a duplicated retry trigger dispatches once, a
    duplicated ambiguous result consumes one retry/event/backoff transition,
    a near-limit duplicate error cannot falsely exhaust the retry budget, and a
    stale attempt-zero success cannot settle in-flight attempt one.
  - Added a current-attempt retry success characterization and asserted that
    both direct resting placement and open status reconciliation arm the
    initial cancellation owner.
  - Updated existing success, ambiguity, contradiction, transport redaction,
    exhaustion, target mismatch, stale trigger, and terminal trigger cases for
    exact ownership.
  - Focused attempts for
    `duplicate_unexpected_cancel_retry_due_dispatches_once`,
    `duplicate_unexpected_cancel_result_cannot_consume_retry_budget_twice`,
    `duplicate_prior_attempt_error_cannot_falsely_exhaust_cancel_retries`,
    `stale_unexpected_cancel_result_cannot_settle_newer_attempt`,
    `current_unexpected_cancel_retry_result_settles_attempt`,
    `unexpected_resting_slice_arms_exact_cancel_attempt`,
    `open_status_check_after_stop_keeps_twap_stopping_and_requests_cancel`, and
    `exhausted_transport_unexpected_child_cancel_redacts_error_event` all
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - The pre-edit focused
    `unexpected_cancel_retry_due_ignores_stale_attempt` attempt and baseline
    `cargo check` stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - Post-edit `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: this runtime result-correlation change does
    not justify launching an unknown local credential/live-trading setup, and
    compilation is already blocked by ALSA metadata.
- Compatibility/UX assessment: every uniquely owned initial or retry result
  follows the identical target-specific request, classifier, backoff, refresh,
  finish, and terminal path. The new attempt is local correlation only. Only a
  duplicate dispatch or a duplicate/delayed result that does not own current
  state is ignored. No visible string, control, schema, payload, retry, timer,
  or normal interaction changed.
- Residual risk: formatting, all state initializers, message producers and the
  sole consumer, both initial entry paths, retry dispatch, result branches,
  stop/terminal behavior, redacted diagnostics, and full diff review pass. The
  crate has not type-checked; focused/full tests and clippy must execute on a
  host with ALSA development metadata. A fill can reconcile before the cancel
  result and the latter currently rewrites the child status; that separate
  monotonic fill-state ordering requires the next Track 6 audit.
- Prior turn commit hash: `692cc46c6a6a517a89302ff6aaed05aaa6b00eae`
- Next candidate: continue Track 6 by ordering account/user-stream fill
  reconciliation before and after unexpected-child cancel results. Verify that
  a confirmed child fill and aggregate fill accounting remain monotonic while
  the resting remainder still reaches the identical cancel/finish path; do not
  change cancellation, partial-fill, scheduling, or visible policy.

## Turn 20 — Characterize TWAP Fill/Cancel Ordering

- Status: audited; financial invariants characterized; F-21 deferred for an
  explicit UX/history semantics decision; executable Rust validation
  environment-blocked
- Severity: Medium for F-21; no Critical/High financial-state defect confirmed
- Scope: TWAP authoritative account/user-stream fills versus confirmed
  unexpected-child cancellation, partial and full fill permutations, repeated
  fill delivery, scheduling/completion, key scrubbing, and terminal history
- Invariant: fill quantity, average price, fee, remaining target, next
  lifecycle state, completion, and archived financial metrics must be
  independent of whether fill reconciliation or cancel acknowledgement arrives
  first.
- Protected behavior: exact OID/coin/side fill attribution, stable fill
  deduplication, child/aggregate max-and-sum accounting, target-specific cancel,
  retry/refresh/stop behavior, next-slice cadence and sizing, current child
  labels and event copy, terminal history format, persistence, and secrets.
- Preconditions/event ordering: an IOC child rests; the cancel task is already
  owned; the account/user stream proves a partial or full fill; the confirmed
  cancel result and fill application are handled in both possible orders. A
  repeated fill snapshot then tests idempotence after both paths converge.
- Evidence: fill calculations depend on retained per-child metrics, not the
  child status label. Cancel success changes pending/retry state and the child
  label but never resets fill metrics. Scheduling derives its next size from
  aggregate remaining target. Terminal snapshots separately copy numeric fill
  metrics and the visible label. F-21 records the exact source and rendering
  boundaries.
- Change: added two adversarial characterization tests only. The partial test
  proves both orders converge on identical `0.25 / 0.75` child/aggregate
  accounting, price, fee, waiting state, pending cleanup, and retry state, then
  reapplies the same fill without double counting. The full test proves both
  orders converge on completion, zero remainder, key scrubbing, and identical
  archived target/fill/average/fee values. It explicitly captures the current
  order-dependent `Canceled`/`Filled` child labels. No production source or
  documentation contract changed.
- Tests/checks:
  - Pre-edit attempts for
    `confirmed_unexpected_child_cancel_clears_pending_cancel`,
    `running_twap_reconciliation_clears_status_metadata_after_fill`, and
    baseline `cargo check` stopped in `alsa-sys` before Kerosene compilation
    because `pkg-config` could not find the system `alsa.pc` package.
  - Focused attempts for
    `partial_fill_accounting_is_monotonic_across_cancel_result_order`,
    `terminal_fill_history_keeps_financial_metrics_across_cancel_result_order`,
    `twap_fill_reconciliation_deduplicates_fills_by_stable_identity`, and
    `current_unexpected_cancel_retry_result_settles_attempt` stopped at the same
    dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - Post-edit `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: this turn changes tests and the audit ledger
    only, and compilation is already blocked by ALSA metadata.
- Compatibility/UX assessment: tests and ledger only. Current live/history
  labels, event ordering, order values, cancellation behavior, timing, views,
  schema, stored data, and secrets are untouched. The explicit deferral avoids
  silently selecting a new visible meaning for a partially filled and canceled
  child.
- Residual risk: source and view/history tracing plus the new permutations
  establish the intended financial invariants, but the crate has not
  type-checked and the tests cannot execute without ALSA metadata. F-21 remains
  an approved-decision dependency, not an active production-safety gap.
- Prior turn commit hash: `351f90fe1b36316d9e0d312796c121683e472625`
- Next candidate: finish Track 6 terminalization by adversarially ordering full
  fill completion, stop requests, successful/failed in-flight status and cancel
  results, retry-due messages, repeated archive upserts, and captured-key
  scrubbing. Prove no terminal TWAP can schedule or dispatch new exchange work
  and that delayed results cannot corrupt its financial history.

## Turn 21 — Suppress Post-Terminal TWAP Cancel Retry Work

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium defense-in-depth hardening; no current financial-state
  corruption or duplicate exchange mutation was confirmed
- Scope: remaining Track 6 terminalization audit across full-fill completion,
  stop/in-flight resolution, delayed status and cancel results, cancel retry-due
  delivery, scheduler/timer eligibility, repeated history upserts, and captured
  agent-key scrubbing
- Invariant: once authoritative fills or lifecycle resolution make a TWAP
  terminal, it cannot schedule or dispatch new exchange work; delayed results
  may retain required read reconciliation but cannot regress terminal financial
  history or extend signing-key lifetime.
- Protected behavior: active-order cancel target selection, signed payload,
  retry maximum/backoff, result classification, retry counters, pending target,
  child status/summary, immediate origin-account refresh, stop handling, fill
  attribution, slice sizes/cadence/randomization/price gates, status/event copy,
  advanced-history values/schema/copy, views, persistence, and secret storage.
- Preconditions/event ordering: an unexpected-resting child has exact cancel
  attempt zero in flight; a complete account fill arrives first and completes
  the target; terminal archive persists the numeric snapshot and clears the
  key; then an ambiguous exchange response or transport error arrives and asks
  for the normal active-order cancel retry.
- Evidence: terminal status-result handlers, status and cancel arming helpers,
  the scheduler predicates, timer subscription, book handlers, finish/timeout
  paths, and account-fill reconciliation all reject or preserve terminal state.
  Repeated terminal archive calls use a stable ID upsert and re-clear the key.
  The one producer mismatch was in the cancel-result retry branch: it batched a
  delayed retry trigger even though the trigger consumer was guaranteed to
  reject terminal state. F-22 records exact source evidence and risk.
- Change: added a terminal check at the cancel-result retry-task boundary. A
  retryable result after full-fill completion now returns the same immediate
  origin-account refresh without the delayed retry trigger. Active TWAPs still
  receive the exact existing retry delay and task. Added adversarial coverage
  for both retry-producing result classes, immutable serialized terminal
  history, financial metrics, manual retry-due rejection, all terminal scheduler
  statuses, delayed terminal status-result rejection, repeated history upsert,
  and key scrubbing. Updated the internal trading architecture documentation.
- Tests/checks:
  - The pre-edit focused attempt for
    `unexpected_cancel_retry_due_ignores_terminal_twap` stopped in `alsa-sys`
    before Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` package.
  - The post-edit focused attempt for
    `terminal_fill_cancel_retry_outcomes_refresh_without_scheduling_retry`
    stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: startup, window, and subscription plumbing
    are unchanged, and compilation is already blocked by ALSA metadata.
- Compatibility/UX assessment: the only runtime difference is removal of a
  delayed retry message after the TWAP is already terminal; that message
  previously woke and no-op'd. Immediate reconciliation timing and every
  user-visible/state/persistence value are preserved. No schema, stored value,
  interaction, order payload, order timing, or copy changed.
- Residual risk: source tracing and the added assertions establish the
  terminal contract, but the crate has not type-checked and tests cannot execute
  on this host without ALSA development metadata. F-21 remains deliberately
  deferred for explicit visible/history semantics approval. No overall campaign
  completion claim is made.
- Prior turn commit hash: `7ce27516fadc2a43306e536993fe8ed680893e4f`
- Next candidate: begin Track 9 restart, shutdown, and secret-lifetime audit.
  Trace active/in-flight Chase and TWAP persistence exclusions plus disconnect,
  profile deletion, clear-config, and shutdown cleanup; verify task contexts,
  uncertainty, keys, and redacted diagnostics cannot survive or leak across
  those boundaries.

## Turn 22 — Fence Automation Progress During Final Exit

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: first Track 9 batch covering config/boot exclusion of live order state,
  main-window close plus final-save lifecycle, Chase place/reprice/size
  correction, TWAP slice dispatch, save-failure recovery, and cancellation
  cleanup during exit
- Invariant: after main-window closure transfers ownership to final persistence
  and process exit, no new exposure-progressing automation mutation may begin.
  Already-sent results must still reconcile, known exposure may still be
  canceled, and a failed save must not corrupt the exact queued automation
  state; F-24 separately owns whether that state may resume without a window.
- Protected behavior: config schema/defaults and terminal-history persistence;
  normal Chase/TWAP lifecycle, prices, sizes, cadence, randomization, limits,
  CLOIDs/OIDs, retry/cooldown rules, account and symbol gates, status/cancel
  reconciliation, active-order cleanup, save failure feedback, all UI/copy, and
  key storage/redaction.
- Preconditions/event ordering: live Chase/TWAP state is runtime-only; a place,
  modify, size correction, or slice becomes due; a config save is pending; the
  main window closes; the daemon remains alive awaiting the save; and an
  automation task/message arrives before `iced::exit()` executes.
- Evidence: `KeroseneConfig` has only terminal advanced-order history, config
  snapshots explicitly blank secret fields and omit every active/pending order
  owner, and boot initializes pending one-shot/cancel/move/nuke/leverage,
  Chase, and TWAP state empty. Account disconnect/switch, saved-profile delete,
  and config clear already block pending trading activity/active automation or
  defer destructive runtime reset. F-23 identifies the distinct live final-save
  window in which progress gates previously ignored `config_save_exit_requested`.
- Change: documented the existing exit flag as the runtime owner/fence and keep
  it armed through both successful and immediate exit tasks. Added a Chase
  progress predicate for placement, book reprice, reconciled modify, and size
  correction while retaining the broader readiness predicate for status/cancel
  cleanup. Added the exit condition to the TWAP slice dispatch gate. Queue and
  pending state are not cleared, so the existing failed-save path clears the
  flag and normal gates resume exact work. Updated internal architecture docs.
- Tests/checks:
  - The pre-edit focused attempt for
    `chase_reprice_tick_runs_queued_size_correction` stopped in `alsa-sys`
    before Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` package.
  - Focused post-edit attempts for
    `chase_place_waits_during_pending_exit_and_resumes_if_exit_aborts`,
    `due_twap_slice_waits_during_pending_exit_and_resumes_if_exit_aborts`, and
    `successful_exit_keeps_automation_fence_armed_until_runtime_exits` stopped
    at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test could not be run because compilation is already blocked
    by ALSA metadata; runtime close/exit behavior therefore remains executable-
    validation debt on a provisioned host.
- Compatibility/UX assessment: the window is already closed when the fence is
  active. Only a not-yet-dispatched Chase/TWAP progress mutation waits; existing
  reconciliation and cancellation keep their behavior. A failed save resets
  the flag and exact queued values can resume under the existing policy; F-24
  documents that policy's headless risk without changing it. No visible
  control, feedback, normal interaction timing, order payload/strategy,
  persisted value, schema, secret, or history meaning changed.
- Residual risk: Rust execution remains blocked. Track 9 still needs explicit
  audit of queued one-shot/NUKE/cluster/leverage intent after close,
  close-versus-config-clear completion, remaining disconnect/profile secret
  lifetimes, and repository-wide order/account diagnostic redaction. F-24's
  failed-save/headless policy requires approval. No overall completion claim is
  made.
- Prior turn commit hash: `10d20f18d7755010d3f145e88a5b70442de67bec`
- Next candidate: continue Track 9 at the root close boundary. Adversarially
  order queued one-shot, cancel/move, close/NUKE, wallet-cluster, and leverage
  intent messages against `WindowClosed`; inspect config-clear completion after
  a main-window close; then prove no exposure-increasing dispatch or headless
  runtime survives the successful exit owner while preserving result
  reconciliation and cleanup cancellation. Keep F-24 deferred until its visible
  failed-save policy is approved.

## Deferred Findings

- F-21: the live and persisted child label for a filled unexpected-resting
  order depends on fill-versus-cancel delivery order. Financial state is safe,
  but choosing fill-dominant, cancel-dominant, or combined visible semantics
  requires explicit approval. Existing stored history strings remain unchanged.
- F-24: a failed final config save clears exit ownership after iced has already
  removed the main window, so automation can resume headlessly. Fixing it
  requires choosing delayed close, reopen-on-error, or exit-with-unsaved-config
  behavior; current window/save-error UX is unchanged pending approval.

## Validation Summary

- Passing this turn: `cargo fmt`, `cargo fmt -- --check`, `git diff --check`.
- Environment-blocked this turn: focused final-exit flag, Chase place, queued
  Chase correction, and TWAP slice fence/resumption tests; `cargo check`; full
  `cargo test`; and strict clippy at `alsa-sys` system dependency discovery,
  before Kerosene was compiled.
- No live exchange mutation or credential-bearing operation was run.

## Residual Risk

- The remaining audit tracks are incomplete; no overall safety-completion claim
  is made.
- F-01 through F-20 and F-22/F-23 have source fixes and regression coverage but
  await executable validation on a host with ALSA development metadata.
- F-21 is explicitly deferred for a visible/history semantics decision; its
  financial invariants have characterization coverage.
- F-24 is explicitly deferred for a main-window/final-save failure policy; its
  successful-exit automation interval is independently fenced by F-23.
- TWAP terminalization and advanced-automation final-exit fencing are
  source-audited with focused coverage but cannot be executed on this host.
  Remaining shutdown/config-clear/one-shot paths, local planning/state
  diagnostic redaction, and the rest of Track 9 require completion before a
  final verdict.
