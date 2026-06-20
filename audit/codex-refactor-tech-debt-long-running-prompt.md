# Codex Long-Running Goal Prompt — Safe Tech-Debt Refactor

Use this prompt as the goal for a long-running Codex coding agent working in the
Kerosene repository. This is a production-minded refactor and maintainability
campaign, not a rewrite. Optimize for future developer velocity while preserving
all current user-facing behavior, persisted data compatibility, trading safety,
and functional correctness.

## Mission

Improve Kerosene's codebase quality, maintainability, and best-practice
alignment by eliminating real technical debt in small, verified batches.

The required outcome is a cleaner, easier-to-work-on codebase with no user
experience regression whatsoever. The only acceptable user-visible changes are:

- bug fixes that prevent incorrect behavior,
- clearer error handling for genuinely broken states,
- performance improvements that preserve semantics, and
- safety hardening that fails closed in trading/security-sensitive paths.

If a proposed refactor might alter visible behavior, persisted config, keyboard
or pointer interaction, order semantics, account selection, layout restore,
notifications, or user data, do not implement it until you have an explicit
compatibility story and focused tests proving no regression.

## Repository Context

Kerosene is a Rust 2024 desktop trading terminal for Hyperliquid built with
`iced` 0.14. It uses an Elm-style state/message/update/view architecture with
live market data, charting, account/wallet state, order placement, client-side
Chase/TWAP automation, settings, persistence, optional integrations, and secret
storage.

This is trading software. Correctness, privacy, and backwards compatibility are
hard requirements. Never print, snapshot, log, commit, or expose private keys,
API keys, bearer tokens, encrypted secret blobs, wallet-sensitive material, or
generated credentials.

## Non-Negotiable Constraints

- Read `AGENTS.md` before doing any work and follow it over this prompt when it
  is more specific.
- Inspect `git status --short` before editing. Preserve unrelated dirty files.
  Never revert, reformat, or rewrite user changes unless explicitly asked.
- Commit each completed, validated change batch with a clear message explaining
  what changed and why. Do not push, tag, release, amend/squash existing
  commits, or rewrite history.
- Prefer small, behavior-preserving patches over broad rewrites.
- Avoid style churn, format-only diffs, file moves, mass renames, and broad
  abstraction passes unless they directly remove proven risk.
- Do not add dependencies unless existing dependencies and the standard library
  are clearly insufficient; document why if you do.
- Do not change public/persisted config schemas, defaults, serialized field
  names, layout formats, saved IDs, or credential storage formats without
  backwards-compatible serde/default tests.
- Do not change trading behavior, order planning, signing, nonce allocation,
  stale-account checks, account switching, or automation lifecycle without
  focused regression tests.
- Do not change UI layout, copy, shortcuts, defaults, interaction priority, or
  visual semantics unless the change is a verified bug fix or performance win.
- If unsure whether a change is behavior-preserving, leave the code unchanged
  and record the opportunity in the audit notes.

## Required Startup Procedure

1. Read:
   - `AGENTS.md`
   - `Cargo.toml`
   - `audit/codebase-cleanup-long-running-prompt.md`
   - `audit/codebase-cleanup-progress.md` if present, to avoid repeating work
2. Run:
   - `git status --short`
   - `git branch --show-current`
3. Build a short map of the areas you intend to touch before editing. For every
   symbol you plan to change, trace its definition and usages first.
4. Create or update `audit/codex-refactor-tech-debt-progress.md` as the living
   work log for this campaign.
5. Start with a read-only discovery pass. Produce a ranked backlog before the
   first source edit.

## Long-Running Work Loop

Repeat this loop until the time budget is exhausted or the highest-value safe
cleanup batches are complete:

1. Discover one narrow improvement opportunity with evidence.
2. Classify risk and expected value.
3. Select a batch small enough to review easily, ideally one feature boundary or
   one repeated pattern at a time.
4. State the intended invariant: what must remain behaviorally identical.
5. Implement the smallest viable change.
6. Add or update focused tests when the code touches calculations, parsing,
   routing, persistence, order planning, signing, risk filtering, chart geometry,
   async state machines, or safety-sensitive logic.
7. Run the narrowest meaningful validation first, then broader validation when
   the touched code is shared or high risk.
8. Record what changed, why it is safe, validation output, and residual risk in
   `audit/codex-refactor-tech-debt-progress.md`.
9. Re-check `git status --short` and review the diff. Stage only the files for
   the current batch, including the progress log when updated. Never stage
   unrelated dirty files.
10. Commit the batch before starting the next one. Use an atomic commit message
    that explains the cleanup, safety fix, or test improvement, for example
    `Refactor chart candle fetch state guards` or `Add config default regression
    coverage`. If validation is blocked or the batch is intentionally deferred,
    do not commit partial work; record the blocker or deferral in the progress
    log instead.

## High-Value Cleanup Targets

Prioritize debt that has concrete future cost or production risk:

### Architecture and Boundaries

- Large update branches that can be split into existing feature update helpers.
- Cross-feature leakage where state, update, view, and subscription ownership is
  unclear.
- Duplicate model/update/view logic that can be consolidated without changing
  behavior.
- Helpers that clarify invariants and reduce call-site mistakes.

### Error Handling and Safety

- `unwrap`, `expect`, `panic`, `todo!`, `unimplemented!`, `dbg!`, `println!`, or
  `eprintln!` in production paths.
- Stringly typed errors where local typed errors or clearer context would reduce
  incorrect caller behavior.
- Silent failure paths that hide persistence, networking, account, or order
  problems.
- Invariants that are implicit and should be represented by types or focused
  guard helpers.

### Persistence and Compatibility

- Missing serde defaults on persisted fields.
- Config migrations, backup recovery, clear/reset flows, and layout restore
  behavior that lack tests.
- Duplicated persistence normalization, redaction, or fallback logic.
- Any path that might drop user data, credentials, layouts, or preferences.

### Trading, Account, and Order Safety

- Stale account/profile behavior around order placement, cancellation, Chase,
  TWAP, and account switching.
- Order result classification, reconciliation, and ambiguous transport-failure
  handling.
- Signing and nonce helpers whose invariants are not tested.
- UI action paths that could accidentally act on the wrong account, symbol, or
  order because stale row data is reused.

### Runtime, Subscriptions, and Resources

- Duplicate subscription stacks that can share safe helpers without coupling
  unrelated behavior.
- Unbounded queues, stale channels, leaked tasks, or reconnect paths without
  clear shutdown/replay behavior.
- Timer or websocket subscriptions whose identity stability is fragile.
- Task-owned secrets that live longer than needed or appear in debug output.

### Views and Developer Ergonomics

- View helpers that mix formatting, state derivation, and widget construction in
  ways that cause repeated bugs.
- Repeated display-name, size-formatting, or status-formatting logic that can be
  centralized behind existing canonical helpers.
- Tests that are difficult to target because logic is embedded in views instead
  of pure helpers.

### Performance Without UX Change

- Avoidable cloning, repeated allocation, or O(n) scans in hot UI/render/update
  paths, only when you can prove equivalent behavior.
- Chart, order book, feed, and subscription paths where bounded work or cache
  invalidation can be made clearer and safer.
- Performance changes must preserve visual output, event ordering, precision,
  and fallback behavior unless tests/documentation justify the difference.

## Explicitly Avoid

- Aesthetic rewrites.
- New architecture for its own sake.
- Replacing working iced patterns with unfamiliar abstractions.
- Broad module moves or renames.
- Public type/field renames that make diffs hard to review.
- Changing user-visible labels, button order, pane defaults, layout sizing,
  theme colors, shortcut behavior, or interaction priority.
- Changing retry/backoff/order behavior without a correctness rationale and
  regression tests.
- Removing code as "dead" without searching all references, config/layout
  restore paths, message routing, and tests.
- Fixing unrelated clippy/test failures outside the selected batch unless they
  block validation and are clearly caused by the current change.

## Suggested Discovery Commands

Use these as starting points; refine them based on what you find:

```bash
git status --short
rg -n "unwrap\(|expect\(|panic!\(|todo!\(|unimplemented!\(|dbg!\(|println!\(|eprintln!\(" src tests
rg -n "api_key|agent_key|bearer|token|password|secret|private|wallet" src tests
rg -n "serde\(|default|skip_serializing|rename" src/config* src/layout* src/*persistence* src
rg -n "Message::|Task::perform|subscription|Subscription|stream|broadcast|mpsc" src
rg -n "clone\(|to_string\(|collect::<Vec|HashMap|BTreeMap" src/chart* src/*views* src/*update* src/ws* src/order*
```

Do not turn search hits into mechanical edits. Each change needs local reasoning,
usage tracing, and validation.

## Validation Ladder

Always run validation appropriate to the touched area:

1. Formatting:
   - `cargo fmt`
   - or `cargo fmt -- --check` when verifying no more formatting is needed
2. Focused tests for changed modules, for example:
   - `cargo test config::`
   - `cargo test secret_storage::`
   - `cargo test order_execution::`
   - `cargo test order_update::`
   - `cargo test account_state::`
   - `cargo test subscription_state::`
   - `cargo test ws::`
3. Type-check:
   - `cargo check`
4. Broader tests when practical or when shared/high-risk code changed:
   - `cargo test`
5. Strict lint at meaningful milestones:
   - `cargo clippy --all-targets --all-features -- -D warnings`
6. GUI smoke test for changes that could affect startup/window construction:
   - `timeout 20s xvfb-run -a cargo run`

If a validation command fails, determine whether it is caused by your changes.
Fix caused failures. If the failure appears pre-existing or unrelated, record the
exact command and failure summary in the progress log and do not paper over it.

## Progress Log Format

Maintain `audit/codex-refactor-tech-debt-progress.md` with entries like:

```markdown
## Batch N — Short Title

- Status: planned | implemented | validated | deferred
- Scope: files/modules touched
- Motivation: concrete debt/risk removed
- Behavior invariant: what must remain unchanged
- Evidence: file:line references from the pre-change code
- Change summary: what was edited
- Tests/checks run: exact commands and result
- Compatibility impact: persisted data, UI, trading behavior, secrets
- Residual risk: what remains and why
- Next candidate: highest-value follow-up
```

## Final Report Format

When ending the long-running goal, create or update
`audit/codex-refactor-tech-debt-report.md` with:

1. Executive summary.
2. Batches completed, in order.
3. Changed files grouped by feature area.
4. Validation commands run and final results.
5. Behavior and compatibility statement.
6. Known residual risks or deferred opportunities.
7. Recommended next cleanup batch.

## Batch Selection Heuristics

Use this priority order:

1. Critical safety/privacy/data-loss risks.
2. High-confidence behavior-preserving cleanups that reduce repeated mistakes.
3. Missing tests around high-risk existing behavior.
4. Local simplifications that remove duplication or clarify invariants.
5. Performance wins in hot paths with clear equivalence.
6. Low-risk developer-experience improvements.

Do not spend time on low-value cleanup while higher-risk findings remain.

## Codex Execution Contract

You are expected to leave a clean, reviewable diff. Before stopping, ensure:

- `git status --short` is understood and documented.
- Each completed change batch is committed separately with an explanatory
  message, and no unrelated dirty files are included in any commit.
- Every source edit has a rationale tied to debt reduction or safety.
- Every behavior-sensitive edit has focused validation.
- No secrets or generated credentials were printed or added to files.
- The progress log reflects exactly what happened.
- The final response includes changed paths, validations run, known blockers,
  and the next recommended batch.

If you cannot safely make progress without risking UX or functionality, stop and
write a ranked findings/backlog entry instead of forcing a refactor.
