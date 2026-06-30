# Trading Safety and Order Lifecycle Hardening - Long-Running Goal Prompt

Use this prompt to drive a careful, production-minded Codex goal over the
Kerosene codebase. Treat this as a long-running implementation and audit goal,
not a broad refactor.

## Objective

Audit and harden Kerosene's order lifecycle so the app cannot place, display,
retry, cancel, automate, or reconcile an order using stale, ambiguous, or
unintended state.

The highest-value outcome is reducing the risk of wrong-account orders,
duplicate submissions, stale optimistic state, automation continuing after its
assumptions changed, or secret material leaking through diagnostics.

## Repository Context

Kerosene is a Rust 2024 desktop trading terminal for Hyperliquid built with
iced. It uses an Elm-style state/message/update/view architecture and handles
live market data, account state, wallet state, order execution, signing,
client-side Chase/TWAP automation, persistence, and secrets.

This is trading software. Treat correctness, privacy, and backwards
compatibility as production requirements. Never print, snapshot, commit, log,
or otherwise expose private keys, API keys, bearer tokens, wallet-sensitive
material, generated credentials, Telegram credentials, or encrypted secret
blobs.

Before doing any work:

- Read `AGENTS.md`.
- Inspect the current git status.
- Identify unrelated dirty files and do not revert, rewrite, format, stage, or
  commit them.
- Prefer `rg`/`rg --files` for search.
- Use `apply_patch` for manual edits.
- Run `cargo fmt` after Rust edits.
- Commit after each completed change locally, using a narrowly scoped commit
  that includes only files changed for that batch.

## Scope

Primary scope:

- Order construction, signing, submission, cancellation, and response parsing.
- Order update flow, pending indicators, optimistic UI state, and reconciliation
  with exchange/user-data state.
- Account, wallet, agent key, provider, and API-key generation invalidation when
  those values affect orders.
- Chase and TWAP automation lifecycle, especially pause/stop/cancel behavior
  when account, market, connection, or validation assumptions change.
- Websocket reconnect and user-data handling where it affects order status,
  fills, cancels, positions, or stale pending state.
- Secret and sensitive-value redaction in messages, debug output, errors,
  toasts, tests, and persisted snapshots when directly connected to order or
  account flows.
- Focused tests for the above.

Secondary scope, only when directly required by a primary finding:

- Config/schema/default handling for order, account, wallet, or automation
  state.
- Shared helpers used by order/account/signing flows.
- Minimal docs updates for release-impacting behavior or new safety semantics.

Out of scope unless explicitly approved:

- Broad UI redesigns.
- Charting, screener, feed, layout, packaging, or theme work unrelated to order
  safety.
- Large module moves or naming churn.
- New dependencies for small helpers.
- Behavior changes that only clean up style.

Do not break anything outside this scope. If a fix requires touching a shared
module, preserve existing behavior for unrelated callers and add targeted tests
around both the fixed path and the affected shared contract.

## Operating Rules

Work in small, reviewable batches. Each batch should have one clear purpose:
one bug fix, one test coverage gap, one defensive-state improvement, or one
small consolidation that directly reduces order-safety risk.

After each batch:

1. Run the narrowest meaningful validation.
2. Run `cargo fmt` if Rust code changed.
3. Inspect `git diff` and `git status`.
4. Commit locally with a concise message, staging only the files from that
   batch.
5. Record any validation gaps or residual risk in the progress note.

Do not create a giant mixed commit. Do not commit unrelated pre-existing dirty
work. If unrelated changes are in the same file, read them carefully and work
around them; stage only the intended hunks when practical.

## Subagent Guidance

Use subagents where prudent, especially for parallel read-only investigation.
Keep the main agent responsible for synthesis, final decisions, edits, tests,
and commits.

Recommended subagent tracks:

1. Signing and exchange response safety
   - Review nonce allocation, action expiry, signature construction, request
     payloads, response parsing, and ambiguous send failure handling.
   - Identify cases where a timeout, partial response, malformed response, or
     retry could place or report the wrong order.

2. Order update and optimistic state
   - Trace order placement, cancellation, pending indicators, error handling,
     fills, and reconciliation through messages and update routes.
   - Look for stale pending state, missing terminal states, duplicate submits,
     and UI state that can imply success before confirmation.

3. Account, wallet, provider, and key invalidation
   - Verify that account/profile/provider/API-key changes invalidate pending
     order assumptions, user-data streams, cached balances, and automation state.
   - Search for generation counters or equivalent guards and report gaps.

4. Chase and TWAP automation
   - Audit start/stop/pause/cancel transitions, market/account changes,
     disconnected states, rejected order handling, and stale price/size inputs.
   - Prioritize cases where automation can continue after the user-visible
     assumptions are no longer true.

5. Websocket and user-data reconciliation
   - Check reconnect behavior, subscription identity, stale stream handling, and
     lag recovery for order/fill/account messages.
   - Identify whether dropped or delayed user-data can leave unsafe local state.

6. Secrets and diagnostics
   - Search for sensitive order/account/signing data reaching `Debug`, logs,
     errors, toasts, snapshots, screenshots, test fixtures, or persistence.
   - Confirm redaction wrappers are used consistently.

Subagent reports must include concrete file and line references, severity,
failure mode, and suggested fix. The main agent should dedupe findings, verify
claims directly before editing, and avoid implementing speculative issues.

## Initial Read-Only Pass

Start by mapping the current flow before editing:

- Entry and routing:
  - `src/message.rs`
  - `src/app_update.rs`
  - `src/app_update/routing.rs`
- Order and signing:
  - `src/order_execution*`
  - `src/order_update*`
  - `src/order_views*`
  - `src/order_pending_indicators*`
  - `src/signing*`
  - `src/advanced_order_history*`
- Automation:
  - `src/twap_state*`
  - Chase-related modules under advanced/order state and update paths.
- Account and wallet:
  - `src/account*`
  - `src/account_state*`
  - `src/account_update*`
  - `src/wallet_state*`
  - `src/wallet_update*`
- Transport and subscriptions:
  - `src/ws*`
  - `src/subscription_state*`
  - User-data stream builders and consumers.
- Persistence and secrets where relevant:
  - `src/config*`
  - `src/config_persistence*`
  - `src/config/secrets*`

Search for:

- `unwrap`, `expect`, `dbg!`, `println!`, `eprintln!`, `todo!`,
  `unimplemented!`
- `order`, `cancel`, `cloid`, `nonce`, `expires`, `signature`, `agent`,
  `private`, `api_key`, `secret`, `wallet`, `profile`, `generation`
- Broad `Debug` derives or formatted errors near secret-bearing or
  order-bearing types.

Produce a ranked plan before implementation. Classify each finding:

- Critical: could place a wrong or duplicate order, leak secret material,
  corrupt user data, or break startup for existing users.
- High: likely stale or misleading trading state under realistic reconnect,
  account-switch, rate-limit, or automation conditions.
- Medium: maintainability or testability issue with a clear order-safety risk.
- Low: local clarity issue. Do not spend time on Low findings while higher-risk
  findings remain.

## Implementation Priorities

Prefer fixes that make unsafe states unrepresentable or explicitly invalidated:

- Tie order actions to account/profile/provider/API-key generations where
  appropriate.
- Clear or mark pending state stale on account/provider/key/user-data generation
  changes.
- Make ambiguous submission results visible and non-retryable unless there is a
  deliberate idempotency mechanism.
- Require Chase/TWAP automation to stop or revalidate when account, market,
  connection, balance, price source, or validation inputs change.
- Parse exchange responses strictly enough to avoid false success.
- Ensure cancellation and rejection paths reach terminal local states.
- Redact sensitive inputs before they enter `Message`, logs, errors, snapshots,
  or debug output.

Avoid:

- Rewriting working order flow wholesale.
- Moving large modules just to improve aesthetics.
- Adding dependencies before proving existing tools are insufficient.
- Changing persisted wire formats without compatibility tests.
- Broad formatting in unrelated files.

## Testing and Validation

For each change, add or update focused tests close to the changed behavior when
practical. For bug fixes, prefer a regression test that fails without the fix.

Useful targets:

- Message routing: `src/app_update/routing/tests/**`
- Orders/signing: `src/order_execution/**/tests`, `src/order_update/**/tests`,
  `src/signing/**/tests`
- Automation: `src/twap_state/tests/**` and nearby Chase-related tests.
- Account/wallet: `src/account/**/tests`, `src/account_update/**/tests`,
  `src/wallet_*`
- Websocket/user-data: `src/ws/**/tests`, `src/subscription_state/**/tests`
- Config/secrets: `src/config/tests/**`, `src/config_persistence/**/tests*`

Validation ladder:

1. Focused test for the edited behavior.
2. `cargo fmt`
3. `cargo check`
4. Broader `cargo test` when the change touches shared state, persistence,
   signing, routing, subscriptions, or automation.
5. `cargo clippy --all-targets --all-features -- -D warnings` when practical
   before ending a major work session.

If validation fails because of unrelated pre-existing worktree changes, report
the exact command and failure, and do not repair unrelated files unless asked.

## Deliverables

Maintain a living progress note, for example
`audit/order-lifecycle-safety-progress.md`, with:

- Current git branch and starting dirty-file summary.
- Subagent tracks launched and completed.
- Ranked findings with severity, evidence, failure mode, and decision.
- Change batches completed, commit hashes, files touched, and tests run.
- Deferred findings and why they were deferred.
- Known residual risks.

The goal is complete only when:

- The highest-risk confirmed findings in scope are fixed or explicitly deferred
  with rationale.
- Each implemented fix has targeted validation or a clear explanation for why
  validation was not practical.
- Local commits exist for each completed change batch.
- The final worktree status is reported, including any unrelated dirty files
  that were present before the goal started.
- No unrelated feature area was changed without an order-safety reason.

## First Goal Message

Use this as the initial Codex goal request:

> Run a long-running Kerosene trading safety and order-lifecycle hardening goal.
> Read `AGENTS.md`, inspect git status, preserve unrelated dirty files, and do
> not break anything outside the order/account/signing/automation/user-data
> scope. Use subagents where prudent for read-only audits, synthesize findings
> yourself, implement small verified batches, and commit locally after each
> completed change with only that batch staged. Prioritize bugs that could place
> wrong or duplicate orders, leave stale pending state, continue Chase/TWAP after
> assumptions changed, misparse exchange responses, or leak sensitive material.
