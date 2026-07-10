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
  CLOID, modify, and leverage update (`src/signing/actions.rs:25-32`,
  `src/signing/client.rs:163-221`).
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
- Order-status REST parsing validates the returned OID/CLOID for concrete order
  bodies before handing a result to lifecycle code
  (`src/api/order_status/parsing.rs:10-61`).
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
| Ticket place | Submission snapshot; captured account/key; `OneShotPlacementContext` with account, surface, symbol, kind | Indicator ID + placement context; request ID after uncertain result | Unique one-shot CLOID | `classify_execution_result` | `orderStatus` by CLOID + connected-account refresh; fills consume market projections | Serialized pending gate; snapshot equality; current-account and pending-request match | Clears global action/indicator; exact status or later refresh removes status request | `order_execution/submit/tests.rs`, `order_execution/core.rs` tests, `order_update/results/tests.rs` | Audit successful-refresh removal of unresolved CLOID requests (F-02) |
| Preset place | Preset preflight then the shared ticket fields/context with `OrderSurface::Preset` | Same as ticket | Unique one-shot CLOID | Shared classifier | Shared one-shot reconciliation | Pending/reconciliation gates run before preflight and again on submit | Shared ticket/result cleanup | `order_update/presets.rs` tests | No confirmed gap; retain double preflight as deliberate queued-event defense |
| Alfred place | Parsed draft preflight; then current form and captured signing context | Same ticket result message/context | Unique one-shot CLOID | Shared classifier | Shared one-shot reconciliation | Alfred preflight plus shared submit gates | Shared ticket/result cleanup | `alfred_update/submit.rs` tests and ticket tests | Verify command-to-form handoff in the later origin-identity track |
| Quick place | Chart ID/surface snapshot and recovery data; captured account; one-shot context | Indicator ID + CLOID context; optional form recovery | Unique one-shot CLOID | Shared classifier | Shared CLOID status + account refresh | Chart/surface/symbol and percentage provenance checks; current-account match | Clears global action/indicator; rejection may restore matching form | `order_execution/quick_order/submit/tests.rs`, `order_update/quick_order/form/tests.rs` | No confirmed gap |
| HUD place | `HudOrderRequest` captures chart/surface/symbol/side; account context; one-shot context | Market: global action + CLOID. Limit: HUD in-flight ID + indicator + CLOID | Unique one-shot CLOID | Shared classifier | Shared CLOID status + account refresh | Chart/surface/symbol/arm checks; per-account limit tracker; current-account match | Market clears global action; limit finishes its tracker entry; both clear indicator | `order_update/hud.rs` tests, `order_execution/hud.rs` tests | Per-tracker ID wraps without collision handling; practically remote, audit with other allocators |
| Close-position place (UI or Alfred) | Fresh connected-account position snapshot; account/key; coin/fraction; one-shot context | Global close action + indicator + CLOID context | Unique one-shot CLOID | Shared classifier | Shared CLOID status + account refresh | Pending/reconciliation/freshness/completeness gates; current-account match | Clears close action/indicator; shared one-shot terminal handling | `order_execution/position_actions/close/tests/**`, result tests | No confirmed gap |
| NUKE child place (UI or Alfred) | Parent execution ID; connected account; planner output; per-child one-shot context | Execution ID + child CLOID in the result context and aggregate settlement set | Unique one-shot CLOID per child; the first terminal transition claims it | Shared classifier | Uncertain child gets CLOID status; parent refreshes after aggregate completion | Current-account and execution-ID checks; duplicate settlement is a no-op | CLOID-keyed confirmed/failed/uncertain totals; parent removed after the unique settled-child count reaches total | `order_execution/position_actions/nuke/tests/**`, direct/status duplicate regressions in `order_update/results/tests.rs` | F-01 addressed in Turn 2; executable regression validation remains environment-blocked by missing ALSA metadata |
| Cancel by OID | Connected account + symbol + OID; durable pending cancel status context | Account + OID + symbol; indicator is presentation only | Target OID (no separate request token) | Shared classifier plus confirmed-cancel predicate | `orderStatus` by OID + open-order/account refresh | Same-account check and exact pending status tuple | Confirmed/terminal result removes matching local order; complete open-order refresh clears uncertainty | `order_execution/position_actions/cancel.rs` tests, cancel result tests | Audit whether every `Ok` refresh used for cleanup has complete open orders |
| Move/modify | Connected account + symbol + OID; original key captured in `PendingMoveOrderContext` | Account + symbol + OID; indicator ID | Target OID (no separate request token) | Shared classifier plus confirmed-modify predicate | `orderStatus` by OID + account refresh to confirm price | Exact pending move key/context; account match; status tuple match | Removes move context/indicator; terminal status or complete open-order refresh clears uncertainty | `order_execution/quick_order/move_order/tests/**`, `order_update/move_order.rs` tests | Same OID can be modified repeatedly; audit per-attempt correlation after prior completion |
| Chase place/replacement | `ChaseOrder` captures ID, account, agent key, symbol, side, sizes, start time, lifecycle | Chase ID + lifecycle; current CLOID is checked by status path | CLOID hashes account + chase ID + start + attempt | Chase-specific strict response analysis | CLOID status + account refresh + open-order/fill stream reconciliation | `expects_place_result`, current account, symbol identity, prior-exposure and reconciliation gates | Moves to verification/resting/stop/archive; late stopped placement is cancelled | Chase lifecycle/place/result/status tests and account stream Chase tests | Direct place-result message lacks the attempt CLOID; audit duplicate/late result behavior (F-05) |
| Chase modify | Chase ID + captured account/key + current OID + lifecycle + desired price | Chase ID + OID + `expects_modify_result` | Target OID; no per-modify request token | `is_confirmed_modify_result` and Chase-specific error handling | OID status + account refresh + open-order/fill stream | Lifecycle/OID match; account and symbol/reconciliation checks before dispatch | Verification/resting/stop flow; terminal Chase archived | `order_update/chase/modify/tests/**`, Chase reprice tests | Repeated modifies can reuse OID; audit per-attempt result correlation (F-05) |
| Chase cancel | Chase ID + captured account/key + OID + stopping phase | Chase ID + OID + `expects_cancel_result` | Target OID; bounded retry treats terminal-not-open responses specially | Confirmed-cancel predicate plus Chase cancel classification | OID status + account refresh/open-order disappearance | Exact stopping phase/OID; disconnected account is reconciled at origin scope | Verifying-cancel then archive; bounded manual-check terminal | `order_update/chase/cancel/tests.rs`, Chase stop/status tests | No confirmed gap; retry idempotence depends on target-specific cancel semantics |
| TWAP child place | `TwapOrder` captures ID/account/key/symbol/plan; `TwapPendingSlice` captures index/size/price/CLOID | TWAP ID + current `pending_op`; status path adds exact CLOID | Deterministic child CLOID | TWAP-specific IOC/fill/resting/transport classification | CLOID status + scoped account-fill refresh + reconciliation deadline | Pending-op state, current account for dispatch, status CLOID, terminal checks | Finishes attempt once; child status/fills updated; terminal TWAP archived | `order_execution/twap/tests/**`, `twap_state/tests/**` | Direct slice-result message carries only TWAP ID; audit duplicate/late result behavior (F-05) |
| TWAP unexpected-child cancel | TWAP ID + captured key + OID/CLOID target + retry attempt | Pending cancel target matches OID or CLOID; retry message includes attempt | Target-specific cancel by CLOID preferred, else OID | Confirmed-cancel/terminal-not-open/error handling | Immediate origin-account refresh; child status and later fills | Exact pending target, retry count, and terminal-state checks | Clears pending cancel and finishes attempt, or bounded error terminal | `order_execution/twap/tests/cancel.rs`, status/account tests | No confirmed gap |
| Wallet-cluster order child | Execution ID + profile secret ID + member address/key + one-shot context | Execution ID + profile secret ID + CLOID | Unique one-shot CLOID per member leg | Shared classifier | CLOID status + member refresh + member user stream | Leg lookup uses execution/profile/CLOID; pending executions are not evicted | Leg becomes confirmed/failed/uncertain; execution complete when every leg terminal | Cluster planning/member tests; shared core/signing tests | Result/status transitions lack focused tests and can overwrite an already-terminal leg (F-04) |
| Wallet-cluster close child | Same as cluster order, plus fresh per-member position snapshot and reduce-only plan | Execution ID + profile secret ID + CLOID | Unique one-shot CLOID per member leg | Shared classifier | CLOID status + member refresh + member stream | Freshness/side/position checks; exact leg lookup | Same leg/execution terminal handling | Cluster close sizing/freshness tests; shared result tests | Same focused result/idempotence gap as cluster orders (F-04) |
| Leverage update | `PendingLeverageUpdateContext` captures account, symbol, asset, dex, margin mode, leverage | Full pending-context equality | None; mutation is not blindly retried | Confirmed-default predicate; other non-error bodies are uncertain | Scoped account refresh for outcomes that may have committed | Pending-context equality + current-account match | Pending context cleared once; matching form updated only on confirmed default | `order_update/leverage.rs` tests, signing action/response tests | No exact mutation status endpoint; verify refresh completeness is sufficient in transport audit |

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
  `src/order_update/results.rs:463-515`,
  `src/order_update/results.rs:789-862`).
- Regression coverage: direct exchange results and `orderStatus` results each
  deliver the same child twice in a two-child execution, assert the unchanged
  `1/2` progress text and pending parent, then settle a distinct CLOID
  (`src/order_update/results/tests.rs:924-1001`). The direct regression also
  proves `PendingNukeExecution` debug output does not expose its retained CLOID.
- Protected behavior: unique child outcomes retain the existing confirmed,
  failed, uncertain, skipped, refresh, error-state, and status-text behavior.
  The change does not affect request construction, signing, dispatch, order
  semantics, views, persistence, or user interaction timing.

### F-02 — Successful refresh may clear unresolved one-shot status state too broadly

- Status: candidate; completeness and intended-resolution semantics still need
  a focused test before classification
- Provisional severity: High
- Evidence:
  - `apply_account_data_loaded` clears all pending one-shot status requests for
    the account after a successful non-follow-up refresh
    (`src/account_update/connection/refresh.rs:145-187`).
  - `clear_pending_one_shot_status_request_for_account` does not inspect CLOID,
    fills, open-order completeness, or status (`src/order_update/results.rs:571-577`).
  - Cancel/move cleanup, by contrast, explicitly requires complete open orders
    (`src/order_update/results.rs:579-604`).
- Risk hypothesis: a partial-but-successful account snapshot can remove the
  blocker for an ambiguous placement without proving whether that placement
  filled, rested, or failed.
- Next evidence needed: construct an `AccountData` result with incomplete order
  or fill lanes and verify whether the fetch contract permits it to reach this
  cleanup path. Determine whether any complete snapshot can resolve a CLOID
  without the separate status response.
- Compatibility note: any fix that changes visible blocking behavior needs
  explicit comparison against the protected UX contract.

### F-03 — Pending one-shot debug output exposes the CLOID

- Status: confirmed, not yet implemented
- Severity: Medium privacy hardening
- Evidence: `PendingOneShotStatusRequest::fmt` redacts the address but formats
  the full CLOID (`src/order_update/results.rs:41-48`), while
  `OneShotPlacementContext` and `PlaceOrderRequest` deliberately avoid exposing
  it.
- Risk: diagnostic formatting can reveal a stable order correlation identifier
  derived from account and order inputs.
- Smallest fix: emit `has_cloid` or `<redacted>` and extend the existing debug
  regression test. This has no runtime or UX effect.

### F-04 — Wallet-cluster result transitions lack focused correlation/idempotence coverage

- Status: confirmed test gap; production defect not yet proven
- Severity: Medium
- Evidence:
  - Result/status handlers update a leg by execution ID, profile secret ID, and
    CLOID (`src/wallet_cluster_update.rs:1095-1239`).
  - The update overwrites any existing terminal state and does not report a
    missing or already-terminal match.
  - Current wallet-cluster tests cover sizing, member selection, persistence,
    and pending-history retention, but not exchange result/status transitions.
- Next evidence needed: characterize stale execution IDs, wrong profile/CLOID,
  duplicate placement results, and status-after-terminal ordering before
  deciding whether production guards are needed.

### F-05 — Advanced place/modify result messages rely on lifecycle state rather than per-attempt tokens

- Status: audit candidate
- Provisional severity: Medium
- Evidence:
  - `ChasePlaceResult` carries Chase ID but not current CLOID/attempt;
    `ChaseModifyResult` carries Chase ID and OID, which may remain stable across
    modifies (`src/message.rs:1212-1220`).
  - `TwapSliceResult` carries TWAP ID but not the pending child CLOID/index
    (`src/message.rs:1170-1173`).
  - Handlers use explicit lifecycle/pending-op state, and current code appears
    to serialize each strategy's exchange mutations.
- Next evidence needed: adversarial duplicate and late-result tests. If the
  state machines already make replay harmless, close this candidate with test
  evidence rather than adding fields.

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

## Deferred Findings

- None yet. Candidates are not deferred findings.

## Validation Summary

- Passing this turn: `cargo fmt`, `cargo fmt -- --check`, `git diff --check`.
- Environment-blocked this turn: the focused `duplicate_nuke_` Rust tests and
  `cargo check` at `alsa-sys` system dependency discovery, before Kerosene was
  compiled.
- No live exchange mutation or credential-bearing operation was run.

## Residual Risk

- The remaining audit tracks are incomplete; no overall safety-completion claim
  is made.
- F-01 has a source fix and regression coverage but awaits executable validation
  on a host with ALSA development metadata. F-02 through F-05 remain open as
  described above.
- Signing wire construction, response classification, Chase/TWAP correlation,
  cluster result handling, account refresh completeness, restart cleanup, and
  redaction require further track-by-track completion before a final verdict.
