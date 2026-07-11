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
  phases; move keeps the sequence in both its captured-key context and status
  request. The sequence is local correlation only and does not change the
  signed exchange action or add mutation retries (`src/order_update/results.rs:88-232`,
  `src/order_update/results.rs:359-459`, `src/order_update/move_order.rs:12-160`).
- Account task results carry read-provider and request-generation context;
  user-data events are address-scoped, and websocket lag forces reconciliation
  (`src/account_update/connection/refresh.rs:94-140`,
  `src/account_update/stream.rs:45-78`).
- Chase and TWAP retain captured account/key identity and explicit lifecycle or
  pending-operation state. Their active state and in-flight requests are
  runtime-only.
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
| Cancel by OID | Connected account + symbol + OID + runtime request sequence; one request owns awaiting-result/checking-status phases | Request sequence + account + OID + symbol; indicator is presentation only | Target OID; runtime sequence is correlation only | Shared classifier plus confirmed-cancel predicate | `orderStatus` by OID + open-order/account refresh | Exact request/phase/account match for direct result; exact request/account/OID/symbol match for status | Confirmed/terminal result removes matching local order; only status-check phase is refresh-reconcilable | `order_execution/position_actions/cancel.rs` tests, cancel result/status duplicate and stale-attempt tests | F-14 addressed in Turn 13; F-15 must require refresh coverage of the origin symbol lane |
| Move/modify | Connected account + symbol + OID + runtime request sequence; original key captured in `PendingMoveOrderContext` | Request sequence + account + symbol + OID; indicator ID is presentation/local-price provenance | Target OID; runtime sequence is correlation only | Shared classifier plus confirmed-modify predicate | `orderStatus` by OID + account refresh to confirm price | Exact request/account/move-key context for direct result; exact request/account/OID/symbol for status | Removes only the matching move context/indicator; terminal status or refresh can clear status uncertainty | `order_execution/quick_order/move_order/tests/**`, `order_update/move_order.rs` duplicate/stale-attempt tests | F-14 addressed in Turn 13; F-15 must retain expected price and require target-lane refresh evidence before cleanup |
| Chase place/replacement | `ChaseOrder` captures ID, account, agent key, symbol, side, sizes, start time, lifecycle | Chase ID + lifecycle + dispatch-time place attempt; current CLOID is checked by status path | CLOID hashes account + chase ID + start + attempt | Chase-specific strict response analysis | CLOID status + account refresh + open-order/fill stream reconciliation | Exact place-attempt equality + `expects_place_result`; current account, symbol identity, prior-exposure, and reconciliation gates | Moves to verification/resting/stop/archive; late stopped placement is cancelled | Chase lifecycle/place/result/status tests, duplicate/late direct-result regressions, and account stream Chase tests | F-05 addressed in Turn 7; executable regression validation remains environment-blocked by missing ALSA metadata |
| Chase modify | Chase ID + captured account/key + current OID + lifecycle + desired price | Chase ID + OID + dispatch-time reprice count + `expects_modify_result` | Target OID; no separate exchange idempotency key (runtime sequence is correlation only) | `is_confirmed_modify_result` and Chase-specific error handling | OID status + account refresh + open-order/fill stream | Exact reprice-count/lifecycle/OID match; account and symbol/reconciliation checks before dispatch | Verification/resting/stop flow; terminal Chase archived | `order_update/chase/modify/tests/**`, including duplicate/late direct-result regressions, and Chase reprice tests | F-05 addressed in Turn 7; executable regression validation remains environment-blocked by missing ALSA metadata |
| Chase cancel | Chase ID + captured account/key + OID + stopping phase | Chase ID + OID + `expects_cancel_result` | Target OID; bounded retry treats terminal-not-open responses specially | Confirmed-cancel predicate plus Chase cancel classification | OID status + account refresh/open-order disappearance | Exact stopping phase/OID; disconnected account is reconciled at origin scope | Verifying-cancel then archive; bounded manual-check terminal | `order_update/chase/cancel/tests.rs`, Chase stop/status tests | F-08 addressed in Turn 9; retry idempotence depends on target-specific cancel semantics |
| TWAP child place | `TwapOrder` captures ID/account/key/symbol/plan; `TwapPendingSlice` captures index/size/price/CLOID/retry | TWAP ID + dispatch-time slice index/retry count + current `pending_op`; status path adds exact CLOID | Deterministic child CLOID (runtime index/retry tuple is correlation only) | TWAP-specific IOC/fill/resting/transport classification | CLOID status + scoped account-fill refresh + reconciliation deadline | Exact pending index/retry equality, current account for dispatch, status CLOID, and terminal checks | Finishes attempt once; child status/fills updated; terminal TWAP archived | `order_execution/twap/tests/**`, including duplicate/late slice-result regressions, and `twap_state/tests/**` | F-05 addressed in Turn 7; executable regression validation remains environment-blocked by missing ALSA metadata |
| TWAP unexpected-child cancel | TWAP ID + captured key + OID/CLOID target + retry attempt | Pending cancel target matches OID or CLOID; retry message includes attempt | Target-specific cancel by CLOID preferred, else OID | Confirmed-cancel/terminal-not-open/error handling | Immediate origin-account refresh; child status and later fills | Exact pending target, retry count, and terminal-state checks | Clears pending cancel and finishes attempt, or bounded error terminal | `order_execution/twap/tests/cancel.rs`, status/account tests | F-08 addressed in Turn 9; contradictory acknowledgements retain the existing bounded target-specific retry path |
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

- Status: confirmed and queued for the next narrow Track 4 turn
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
  (`src/account/types/data.rs:198-213`). However, shared cancel/move cleanup
  checks only the snapshot-wide `open_orders_complete` flag and account before
  dropping either request (`src/order_update/results.rs:711-735`). The move
  status request retains no expected target price
  (`src/order_update/results.rs:186-232`).
- Violated invariant: uncertainty may be released only by authoritative data
  that covers the operation's origin lane and distinguishes the relevant
  terminal/open state; an open move additionally needs evidence of whether its
  expected price committed.
- Risk: a complete but unrelated HIP-3 lane can unblock a cancel or move whose
  exposure remains unknown. A same-lane move refresh can also erase the status
  request without comparing the live order price to the dispatched target,
  allowing later actions to proceed without establishing which modification
  won.
- Planned remediation/tests: retain the move's exact prepared target price in
  its runtime-only reconciliation context; require
  `has_complete_open_orders_for_symbol` for both operations; settle cancel from
  target OID presence/absence; and settle move only from terminal absence or a
  parsed live price that can be compared to the expected target under existing
  numeric semantics. Add selected-dex-switch, unrelated-lane, old-price,
  expected-price, and terminal-disappearance regressions. Do not alter fetch
  scope, request cadence, wire payloads, or visible normal-path behavior.

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

## Deferred Findings

- None yet. Candidates are not deferred findings.

## Validation Summary

- Passing this turn: `cargo fmt`, `cargo fmt -- --check`, `git diff --check`.
- Environment-blocked this turn: focused stale/duplicate cancel and move
  attempt tests, allocator-wrap and awaiting-result phase tests, nearby
  cancel/move status tests, protected indicator-expiry/confirmed-price tests,
  `cargo check`, full `cargo test`, and strict clippy at `alsa-sys` system
  dependency discovery, before Kerosene was compiled.
- No live exchange mutation or credential-bearing operation was run.

## Residual Risk

- The remaining audit tracks are incomplete; no overall safety-completion claim
  is made.
- F-01 through F-14 have source fixes and regression coverage but await
  executable validation on a host with ALSA development metadata.
- F-15 cancel/move refresh sufficiency, broader Chase/TWAP and account-stream
  ordering, restart/shutdown cleanup, local planning/state diagnostic
  redaction, and the remaining tracks require completion before a final verdict.
