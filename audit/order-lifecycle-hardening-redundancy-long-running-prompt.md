# Kerosene Order Lifecycle Hardening and Prudent Redundancy — Long-Running Goal

Use this entire document as the prompt for a long-running Codex goal in the
Kerosene repository. This is an audit-and-implementation campaign for internal
trading plumbing. It is not a redesign, a feature pass, or a broad refactor.

## Mission

Audit the complete order lifecycle and incrementally harden its internal
correctness, correlation, reconciliation, and recovery paths. Add prudent,
independent safeguards where they materially reduce the chance of a wrong,
duplicate, stale, untracked, or falsely settled exchange mutation.

Preserve all current user-visible behavior and all intended trading semantics.
The goal is stronger plumbing beneath the existing product:

- no UI, workflow, copy, layout, shortcut, sound, or notification changes;
- no changes to order sizing, pricing, rounding, slippage, time-in-force,
  reduce-only behavior, supported markets, or automation strategy;
- no changes to normal success/failure feedback or interaction timing;
- no persisted schema, default, secret-storage, or layout compatibility changes;
- no new trading features and no broad architecture rewrite.

If a confirmed safety defect cannot be fixed without an observable product or
trading-semantics change, document it precisely and defer it for explicit user
approval. Do not quietly redefine existing behavior in the name of hardening.

## Why the Current Architecture Makes Sense

The current structure is a sound base for incremental hardening. Preserve these
boundaries unless a verified defect proves a small change is necessary:

- iced's Elm flow keeps intent and asynchronous results explicit:
  `Message -> update route -> feature update -> Task<Message>`.
- `src/order_execution/core.rs` is the canonical prepared-order boundary for
  market capability, symbol identity, sizing, price, reduce-only, CLOID, and
  place/cancel/modify preparation.
- `src/signing/` is the only signed Hyperliquid mutation boundary.
- `src/order_update/results.rs` classifies acknowledgements and owns much of the
  one-shot placement/cancel reconciliation state.
- `src/account_update/` and `src/ws/user_streams*` apply authoritative account,
  order, and fill data with completeness/revision/generation tracking.
- Chase and TWAP are explicit client-side state machines under
  `src/order_execution/chase*`, `src/order_update/chase*`,
  `src/order_execution/twap*`, and `src/twap_state*`.
- Wallet-cluster fan-out has a separate update route but reuses the prepared
  order and signing boundaries.
- Active automation and in-flight tasks are runtime state, not persisted as
  live work after restart.

Order lifecycle logic is necessarily distributed across intent handling,
signing tasks, result messages, account refreshes, and user-data streams. The
main risk is therefore at handoffs between those boundaries, not the absence of
one giant order manager. Harden the handoffs; do not collapse the architecture
into a monolith.

Treat this architecture summary as a starting hypothesis. Verify it against the
current checkout before relying on it, because this goal may span later commits.

## Definition of Prudent Redundancy

Redundancy is desirable only when the checks are independent enough to catch a
different failure mode and have a single clear owner. Good examples include:

- canonical intent/preparation validation plus narrow wire-boundary structural
  validation before signing;
- immutable origin context on dispatch plus result-time correlation against the
  current account/request generation;
- local exchange-response classification plus authoritative `orderStatus`,
  account refresh, open-order, and fill reconciliation after an uncertain
  result;
- websocket-driven reconciliation plus a bounded REST repair path after lag,
  reconnect, partial data, or ambiguous transport failure;
- explicit terminal-state cleanup plus idempotence tests for duplicate and
  out-of-order messages;
- runtime state guards plus tests proving account/profile/provider/key changes
  cannot retarget an already dispatched action;
- redacted domain types plus a second redaction boundary before external error
  text reaches diagnostics or UI state.

Prudent redundancy does **not** mean:

- submitting the same exchange mutation through two paths;
- automatically retrying a place, modify, cancel, or leverage action whose
  exchange outcome is unknown;
- maintaining two competing sources of local truth;
- copying market, sizing, or reduce-only policy into every order surface;
- polling indefinitely without a bounded state machine;
- treating a refresh request as proof that reconciliation completed;
- adding speculative abstractions, dependencies, or fallback behavior.

When two authoritative-looking signals disagree, keep the internal state
uncertain and reconcile it. Never convert disagreement into false success.

## Non-Negotiable Safety and Compatibility Constraints

- Read and follow `AGENTS.md` before every implementation campaign. If it is
  more specific than this prompt, it wins.
- Never place a live order or call a mutating exchange endpoint during
  validation. Use unit tests, fixtures, pure helpers, and mocked/local inputs.
- Never use real wallets, agent keys, API keys, addresses, signatures, CLOIDs,
  order payloads, Telegram credentials, or production account data in tests or
  documentation.
- Never print, log, snapshot, persist, or commit secret material. Preserve
  `Zeroizing`, redacted message wrappers, and custom redacted `Debug`
  implementations.
- Do not weaken a safety check to make a test pass or to preserve concurrency.
- Do not add blind mutation retries. A retry is allowed only if the exact
  operation is provably idempotent at the exchange boundary and the proof is
  documented and tested. Otherwise reconcile by CLOID/OID/account state.
- Do not change visible status strings, toasts, controls, pending indicators,
  animations, sounds, or disabled/enabled behavior. If plumbing needs a new
  internal state, keep its rendering identical.
- Do not change order price, quantity, denomination, precision, fee, side,
  reduce-only, trigger, slippage, time-in-force, scheduling, repricing, or
  cancellation semantics.
- Do not change config/layout wire formats, defaults, migrations, or persisted
  active-order behavior. If a schema change seems necessary, defer it.
- Do not add a crate unless existing dependencies and the standard library are
  demonstrably insufficient. Prefer no dependency changes for this goal.
- Do not perform broad file moves, renames, formatting churn, or cleanup that is
  not required by a confirmed order-lifecycle risk.
- Do not fix unrelated failures or dirty files. Preserve user work exactly.
- If exchange behavior is not provable from code and fixtures, consult only
  authoritative current documentation, record the source and date, and do not
  implement a guess.

## Protected Behavior Contract

Before each source change, state the behavior that must remain unchanged and add
or identify tests that protect it. At minimum, preserve:

- every existing order-entry surface and its current market capability;
- exact order preparation for valid inputs;
- valid order wire payloads and signing semantics;
- current CLOID/OID use and uniqueness properties;
- current account-selection and profile behavior;
- current successful and rejected result handling;
- Chase placement, repricing, fill accounting, stop, and archive semantics;
- TWAP slice planning, timing, randomization, price gates, fill accounting,
  stop, and archive semantics;
- wallet-cluster fan-out sizing and per-member execution semantics;
- spot/perpetual/outcome distinctions and spot metadata/balance-completeness
  requirements;
- no live automation restoration after restart;
- current user-visible UI and status behavior.

For behavior-sensitive code, prefer a characterization/regression test before
the implementation patch. If the existing behavior cannot be stated precisely,
continue the audit instead of editing.

## Required Startup Procedure

At the beginning of the goal, and again after any long interruption:

1. Read `AGENTS.md` and `docs/README.md`.
2. Read:
   - `docs/architecture/elm-runtime.md`
   - `docs/architecture/state-map.md`
   - `docs/architecture/subscriptions-and-tasks.md`
   - `docs/components/trading-and-order-execution.md`
   - `docs/components/account-wallet-portfolio.md`
   - `docs/operations/security-and-secrets.md`
   - `docs/operations/testing-validation-packaging.md`
3. Read the prior lifecycle work so it is verified rather than repeated:
   - `docs/order-lifecycle-refactor-audit.md`
   - `audit/order-lifecycle-safety-long-running-prompt.md`
   - `audit/order-lifecycle-safety-progress.md`
4. Run `git status --short --branch`, `git branch --show-current`, and inspect
   recent order/account/signing commits. Record the starting HEAD and every
   pre-existing dirty path.
5. Create or update
   `audit/order-lifecycle-hardening-redundancy-progress.md` as the durable goal
   ledger. Never overwrite useful history in the prior progress file.
6. Establish a validation baseline before editing source. Run the narrowest
   order/account/signing tests that work in the environment, then `cargo check`.
   Record exact commands and results, including pre-existing failures.
7. Perform a read-only architecture and lifecycle map before the first source
   change. Do not treat the older audit's “no remaining findings” statement as
   proof about the current checkout.

## Current Architecture Map to Verify

Trace definitions and all callers rather than relying only on this list.

### Intent, Preparation, and Dispatch

- `src/message.rs`
- `src/app_update.rs`
- `src/app_update/routing.rs`
- `src/order_update.rs` and `src/order_update/`
- `src/order_execution.rs` and `src/order_execution/`
- `src/order_execution/core.rs`
- `src/order_pending_indicators.rs`
- `src/wallet_cluster_update.rs`
- Alfred trading/position-close paths that create shared order intents

Inventory every mutation surface:

- ticket and presets;
- chart quick order and HUD;
- cancel and drag-to-move;
- close position and NUKE;
- Chase and adopted resting Chase orders;
- TWAP child orders and unexpected-child cancellation;
- wallet-cluster open and close fan-out;
- leverage updates;
- any newer surface found by searching all signing/order-task callers.

### Signing and Transport

- `src/signing.rs` and `src/signing/`
- request builders and wire models;
- nonce allocation and action expiry;
- action hashing and EIP-712 signing;
- `/exchange` transport and response parsing;
- redacted request/response/error formatting.

### Result Classification and Reconciliation

- `src/order_update/results.rs`
- one-shot CLOID status requests;
- cancel and move OID status requests;
- pending order indicators and pending action state;
- account refresh request contexts/generations;
- account completeness and revision state;
- websocket open-order/fill/spot-state application;
- reconnect, lag, partial-frame, and repair behavior;
- advanced-order history terminal snapshots.

### Automation

- `src/order_execution/chase*`
- `src/order_update/chase*`
- `src/account_update/stream/chase*`
- `src/subscription_state/market/chase.rs`
- `src/order_execution/twap*`
- `src/twap_state*`
- `src/subscription_state/market/twap.rs`

### Account, Identity, and Lifecycle Invalidation

- `src/app_state.rs`
- `src/account_state*`
- `src/account_update/connection*`
- `src/account_update/stream*`
- account/profile/key/provider changes;
- `account_data_revision`, `spot_balances_revision`, request generations,
  completeness flags, and reconciliation-required state;
- startup, disconnect, reconnect, clear-config, and window/application close.

## First Deliverable: Lifecycle Assurance Matrix

Before implementation, put a matrix in the progress ledger with one row per
exchange mutation path and these columns:

| Surface/operation | Immutable origin identity | Correlation key | Idempotency key | Immediate-result classifier | Authoritative reconciliation | Stale-result guard | Terminal cleanup | Existing tests | Gaps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |

Include, at minimum, ticket, preset, quick, HUD, close, NUKE child, cancel,
move/modify, Chase place/modify/cancel, TWAP child/cancel, cluster child, cluster
close child, and leverage update.

For each row, trace this full chain:

```text
user or automation intent
  -> captured account/profile/symbol/order identity
  -> canonical preparation
  -> signed request construction
  -> mutation dispatch
  -> result Message with correlation context
  -> response classification
  -> local pending-state transition
  -> authoritative REST/user-stream reconciliation when required
  -> exactly one terminal cleanup/archive path
```

The matrix is evidence, not a checklist to mark optimistically. Use concrete
file and symbol references. Mark unknowns explicitly.

## Audit Tracks

Audit all tracks, but implement one narrow confirmed fix per turn.

### 1. Origin Identity and Stale Results

- Verify every task result carries enough immutable context to identify the
  originating account/profile, operation, symbol/asset, order/CLOID/OID, and
  logical request or execution.
- Check whether account, profile, key, provider, metadata, balance revision, or
  request generation can change between dispatch and result application.
- Ensure a stale result cannot mutate the current account, clear another
  request's pending state, archive the wrong automation, or start a follow-up
  exchange mutation.
- Audit ID allocation, wraparound, reuse after state reset, and concurrent
  in-flight operations. Do not assume a map key alone is sufficient.

### 2. Prepared Order and Wire Boundary

- Confirm every placement surface reaches the canonical prepared-order boundary
  or has a documented, tested reason not to.
- Compare prepared values with signed wire values without changing valid
  payloads.
- Look for lossy conversions, non-finite numbers, negative zero, precision
  drift, asset/symbol alias mismatch, side inversion, reduce-only loss, market
  type confusion, or UI strategy kinds reaching wire-only types.
- Add only narrow structural validation at the signing edge. Do not duplicate
  pricing/sizing policy there.

### 3. Mutation Transport and Ambiguous Outcomes

- Trace failures before connect, before send, after partial/full send, on
  timeout, on malformed HTTP/JSON, and on mixed exchange statuses.
- Verify confirmed, rejected, ambiguous, and transport-unknown outcomes cannot
  collapse into one another.
- Verify mutation errors are redacted before entering `Message`, debug output,
  progress notes, tests, or user-visible state.
- Never retry an unknown mutation automatically. Prefer exact CLOID/OID status
  checks and authoritative account reconciliation.

### 4. One-Shot, Cancel, Move, Close, and NUKE

- Verify one-shot CLOIDs are unique under same-millisecond concurrency and do
  not expose account/order data.
- Verify unexpected resting responses for non-resting order kinds remain
  uncertain until reconciled.
- Check duplicate/out-of-order result and status messages for idempotence.
- Ensure a complete refresh is required before it clears uncertainty; a partial
  or failed refresh must not do so.
- Check cancel and move status ownership when indicators disappear, an order
  vanishes, or a later websocket frame arrives.
- Verify NUKE fan-out accounts for every child once and never reports aggregate
  completion while a child is unknown.

### 5. Chase State Machine

- Trace placement, verification, reprice, modify, fill reconciliation, stop,
  cancellation, retry scheduling, disappearance, and archive transitions.
- Verify no reprice or replacement can occur while prior exchange exposure is
  unknown.
- Verify duplicate fills/open-order frames and REST refreshes cannot double
  count fills or resurrect terminal state.
- Verify account/key/market/metadata/reconciliation changes cannot retarget or
  continue a Chase under different assumptions.
- Preserve all repricing, limits, timing, and visible lifecycle behavior.

### 6. TWAP State Machine

- Trace scheduling, pending child creation, CLOID generation, IOC result
  handling, unexpected resting cancellation, status retry, fill reconciliation,
  stop, timeout, completion, and archive transitions.
- Verify a tick, reconnect, duplicate result, or delayed status response cannot
  dispatch the same logical slice twice.
- Verify fills are attributed once and terminal state waits for all required
  reconciliation.
- Preserve slice sizes, cadence, randomization, price bounds, retry policy, and
  visible lifecycle behavior.

### 7. Wallet-Cluster Fan-Out

- Treat every member leg as an independently correlated financial mutation.
- Verify profile ID, address, agent key, prepared request, CLOID, execution ID,
  and result/status message cannot cross between members or executions.
- Verify partial success, rejection, ambiguity, member refresh failure, and
  window/config changes do not lose or falsely settle a leg.
- Preserve cluster sizing, status rendering, and execution history semantics.

### 8. Account and User-Data Reconciliation

- Verify websocket frames are scoped to the correct source address and stream
  generation before changing trading state.
- Verify reconnect, lag, channel closure, partial snapshots, and provider
  changes mark the correct completeness/reconciliation state.
- Check ordering between REST refreshes and websocket deltas so older data
  cannot overwrite newer authoritative state.
- Verify open-order, fill, position, and spot-balance completeness are treated
  independently where the exchange delivers them independently.
- Confirm pending uncertainty is cleared only by data sufficient to resolve the
  exact operation.

### 9. Restart, Shutdown, and Secret Lifetime

- Confirm in-flight mutations and active Chase/TWAP state are not accidentally
  persisted or resumed after restart.
- Inspect disconnect, profile deletion, clear-config, and shutdown cleanup for
  stranded task context or incorrectly cleared uncertainty.
- Audit `Debug`, error, toast, snapshot, test fixture, and progress-log paths for
  account/order/secret leakage. Preserve the repository's deliberate redaction
  policy even if a value is not traditionally considered a secret.

## Finding Standard and Priority

Do not implement a suspicion. Every finding must include:

- severity;
- exact preconditions and event ordering;
- affected order surface(s);
- concrete file and symbol evidence;
- the invariant that is violated;
- financial/user risk;
- why current checks do not already cover it;
- the smallest behavior-preserving fix;
- a regression test that fails before the fix, when practical;
- any residual uncertainty.

Severity:

- **Critical:** can place a wrong or duplicate mutation, use the wrong account or
  key, leak signing secrets, or falsely declare unknown exchange exposure safe.
- **High:** can continue automation, clear uncertainty, double-count fills, lose
  a child operation, or settle aggregate state incorrectly under realistic
  failure/reconnect ordering.
- **Medium:** a concrete missing invariant or test gap that makes a future
  Critical/High regression plausible.
- **Low:** style, naming, speculative consolidation, or local clarity. Record
  briefly; do not implement during this goal.

Prioritize confirmed Critical/High findings, then high-confidence Medium
hardening. Prefer adding a missing regression test over speculative production
code.

## Long-Running Turn Loop

Each goal turn must be self-contained and end in a local commit. Repeat:

1. Read the progress ledger and inspect `git status --short --branch`.
2. Reconcile the current HEAD with the last recorded goal commit and identify
   any new user/unrelated changes.
3. Select exactly one cohesive audit or implementation batch small enough to
   finish, validate, review, and commit in this turn.
4. State the invariant and protected behavior before editing.
5. Trace definitions and all call sites. Add a regression/characterization test
   first when practical.
6. Implement the smallest patch. Keep views and user-facing strings untouched.
7. Run the narrowest meaningful validation, then broader checks proportional to
   risk.
8. Review `git diff`, `git diff --check`, and `git status --short`. Confirm no
   secret, unrelated file, generated artifact, or broad formatting change is
   included.
9. Update the progress ledger with evidence, changes, exact validation results,
   compatibility assessment, residual risk, and the next candidate.
10. Stage only this turn's files and commit once with a narrow imperative
    message. Do not push, tag, amend, squash, rebase, or rewrite history.
11. Confirm the commit exists and report its hash plus final worktree status.

### Mandatory Per-Turn Commit Rule

- End **every** goal turn with exactly one non-empty local commit for that
  turn's work.
- Never carry goal-owned uncommitted edits into the next turn.
- A read-only/audit turn must update and commit the progress ledger.
- If an implementation cannot be completed or validated safely in the turn,
  remove only that turn's incomplete goal-owned source edits, record the
  finding/blocker in the ledger, and commit the ledger only.
- Never commit knowingly broken source merely to satisfy the cadence.
- Never stage or commit pre-existing or concurrent user changes. If safe
  hunk-level separation is not possible, leave that source change unimplemented,
  document the collision, and commit only non-conflicting goal records.
- Do not create empty commits. The progress ledger guarantees a meaningful
  auditable commit even on investigation-only turns.

The progress ledger cannot contain the hash of the same commit that adds the
entry without amending. Do not amend. Report the new hash at turn end and record
it in the next turn's ledger update or derive the final list from `git log`.

## Validation Requirements

Use focused tests closest to the changed behavior. Relevant areas include:

- `src/order_execution/**/tests*`
- `src/order_update/**/tests*`
- `src/signing/**/tests*`
- `src/twap_state/tests/**`
- `src/account_update/stream/tests/**`
- `src/account_update/connection*` tests
- `src/advanced_order_history/tests/**`
- wallet-cluster tests near `src/wallet_cluster_update.rs` and
  `src/wallet_cluster_state*`
- `src/app_update/routing/tests/**` when message routing changes

Required ladder for Rust changes:

1. focused regression tests for the exact invariant;
2. nearby module/surface tests;
3. `cargo fmt -- --check` after formatting with `cargo fmt`;
4. `cargo check`;
5. `cargo test` when shared preparation, signing, account reconciliation,
   routing, Chase/TWAP, or wallet-cluster plumbing changes;
6. `cargo clippy --all-targets --all-features -- -D warnings` at major
   milestones and before completion;
7. `timeout 20s xvfb-run -a cargo run` only when startup/subscription/window
   plumbing could be affected, and never with real credentials or a live
   trading setup.

Also run `git diff --check` before every commit. If an environment dependency
blocks validation, record the exact command, exit cause, and which claims remain
unverified. Do not call a blocked test “passed,” and do not broaden scope to fix
the machine.

### Required Adversarial Test Themes

Add tests where coverage is missing and a concrete invariant is at risk:

- duplicate result, status, fill, and open-order messages;
- reversed/out-of-order result and reconciliation messages;
- result after account/profile/provider/key/request-generation change;
- timeout or parse failure after a mutation may have been sent;
- mixed/malformed exchange statuses and missing OID/CLOID fields;
- REST refresh failure, partial completeness, and stale refresh completion;
- websocket lag/reconnect plus REST repair ordering;
- same-millisecond CLOID/nonce allocation and concurrent dispatch;
- unknown mutation followed by open, filled, canceled, rejected, or not-found
  status;
- stop/disconnect while Chase/TWAP has an in-flight child;
- unexpected resting IOC/market child;
- NUKE and cluster partial success with one unknown child;
- redaction of secrets and sensitive account/order values in `Debug` and errors.

Do not add tests that merely mirror implementation details. Assert lifecycle
invariants and externally stable behavior.

## Progress Ledger Format

Maintain `audit/order-lifecycle-hardening-redundancy-progress.md` with:

```markdown
# Order Lifecycle Hardening and Redundancy Progress

## Baseline
- Starting HEAD:
- Branch:
- Pre-existing dirty paths:
- Baseline validation:
- Architecture snapshot date:

## Lifecycle Assurance Matrix
<!-- one row per exchange mutation path -->

## Turn N — Short title
- Status: audited | implemented | validated | deferred | blocked
- Severity:
- Scope:
- Invariant:
- Protected behavior:
- Preconditions/event ordering:
- Evidence:
- Change:
- Tests/checks:
- Compatibility/UX assessment:
- Residual risk:
- Prior turn commit hash:
- Next candidate:

## Deferred Findings

## Validation Summary

## Residual Risk
```

Keep it factual. Never paste raw payloads, addresses, CLOIDs, signatures, keys,
or unredacted external error bodies into the ledger.

## Final Report

When all tracks are audited and the safe high-value batches are complete, write
`audit/order-lifecycle-hardening-redundancy-report.md` containing:

1. executive safety verdict;
2. verified current lifecycle architecture;
3. the final lifecycle assurance matrix;
4. findings by severity with implemented/deferred status;
5. commits in chronological order, derived from git history;
6. tests and checks run, with exact outcomes;
7. proof that protected trading and UX behavior remained unchanged;
8. deferred items requiring a product/behavior decision;
9. known residual risks and recommended future audit cadence.

Commit the final report and final ledger update as the last goal turn's single
commit.

## Completion Criteria

The goal is complete only when:

- every discovered exchange mutation surface appears in the assurance matrix;
- all audit tracks have been inspected against the current checkout;
- every confirmed Critical/High finding is either fixed with focused validation
  or explicitly deferred because a safe fix requires user-visible/product
  semantics approval;
- implemented changes are narrow plumbing changes with no intentional UX,
  trading-policy, or persisted-data changes;
- duplicate, out-of-order, stale-result, ambiguous-transport, partial-refresh,
  and terminal-cleanup risks have focused coverage where applicable;
- no automatic retry can duplicate an unknown exchange mutation;
- no secret or sensitive account/order material was exposed;
- every goal turn has one scoped local commit and no goal-owned uncommitted
  source changes remain;
- the final report and worktree status are accurate.

If no safe implementation findings remain, do not manufacture changes. Complete
the matrix, tests where justified, progress ledger, and final report, with one
commit per turn as required.
