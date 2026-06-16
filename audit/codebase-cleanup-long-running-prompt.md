# Long-Running Codebase Cleanup Prompt

Use this prompt to drive a careful, production-minded audit and cleanup of the
Kerosene codebase. Treat this as a long-running goal, not a single sweeping
refactor.

## Objective

Audit and improve the codebase for best practices, prudence, maintainability,
security, and consolidation while preserving user-visible behavior and persisted
data compatibility. Prioritize changes that reduce operational risk, remove
meaningful duplication, clarify ownership boundaries, and make future feature
work safer.

## Repository Context

Kerosene is a Rust 2024 desktop trading terminal built with iced. It uses an
Elm-style state/message/update/view architecture and handles trading,
wallet/account state, order execution, charting, feeds, settings, persistence,
and secrets.

This is trading software. Treat correctness, privacy, and backwards
compatibility as production requirements. Never print, snapshot, commit, or
otherwise expose private keys, API keys, bearer tokens, encrypted secret blobs,
wallet-sensitive material, or generated credentials.

Before doing any work:

- Read `AGENTS.md`.
- Inspect the current git status.
- Identify unrelated dirty files and do not revert or rewrite them.
- Prefer `rg`/`rg --files` for search.
- Use `apply_patch` for manual edits.
- Run `cargo fmt` after Rust edits.
- Do not commit unless explicitly asked.

## Operating Principles

Make behavior-preserving improvements first. Avoid style churn, broad file
moves, large abstractions, and format-only rewrites unless they clearly reduce
risk or unlock a specific cleanup.

Use existing local patterns over new frameworks or new dependencies. Do not add
dependencies unless the standard library and existing crates are clearly
insufficient.

Keep changes close to feature boundaries:

- State belongs in feature state/model modules before adding top-level fields.
- Messages route through `src/app_update/routing.rs`.
- Views stay pure and do not perform I/O, mutate state, spawn tasks, or read
  the clock.
- Subscriptions belong in `subscription_state.rs` or child modules.
- Persisted wire types belong in config modules and require compatibility tests.

For any change touching signing, orders, persistence, secrets, or account
switching, add focused regression tests or explain why a test is not practical.

## Subagent Audit Plan

Use subagents when available. Keep them read-only unless explicitly assigning a
small implementation task. Ask each subagent for concrete findings with file and
line references, severity, and suggested fixes.

Recommended independent audit tracks:

1. Architecture and consolidation
   - Look for duplicated model/update/view logic.
   - Find modules with unclear ownership or cross-feature leakage.
   - Identify helpers that should be shared because they remove real
     complexity, not just because code looks similar.

2. Persistence and compatibility
   - Review config schemas, defaults, migrations, backups, and clear/reset
     flows.
   - Check that saved layouts, pane config, credentials, account profiles, and
     feature preferences remain backwards compatible.
   - Verify corrupt-primary/backup behavior and failure messages.

3. Security and secrets
   - Search for secret-bearing strings, `Debug` derives, logs, toasts, errors,
     tests, and serialization paths.
   - Confirm `SensitiveString`, `Zeroizing`, or equivalent patterns are used
     consistently.
   - Check keychain/encrypted-config transitions, deletion semantics, and
     migration warnings.

4. Trading correctness and order safety
   - Review signing, order request construction, stale-account behavior,
     cancellation, Chase/TWAP automation, and response handling.
   - Prioritize bugs that could place the wrong order, use the wrong account, or
     continue automation after state changes.

5. Runtime, subscriptions, and async tasks
   - Check websocket lifecycle, task cancellation, reconnect behavior, timer
     subscriptions, and shared manager registries.
   - Look for unbounded growth, duplicate subscriptions, stale channels, and
     task-owned secrets that linger longer than needed.

6. Tests and validation gaps
   - Identify high-risk code with no focused tests.
   - Prefer small regression tests near the module under test.
   - Mark slow, flaky, or manual-only validation separately from normal tests.

## Audit Method

Build a concise map before editing:

- Entry points: `src/main.rs`, `src/app_state.rs`, `src/message.rs`,
  `src/app_update.rs`, `src/app_update/routing.rs`.
- Persistence and secrets: `src/config*`, `src/secret_storage*`,
  `src/config_persistence*`.
- Account and wallet flows: `src/account*`, `src/wallet_*`.
- Orders and signing: `src/order_*`, `src/signing*`, `src/twap_state*`.
- Subscriptions and transport: `src/subscription_state*`, `src/ws*`,
  `src/api*`.
- Views only after understanding the state/update path they render.

Classify findings using this scale:

- Critical: can leak secrets, place incorrect trades, corrupt user data, or
  break startup for existing users.
- High: likely production failure, data loss risk, broken migration, stuck task,
  or repeated user-facing error.
- Medium: maintainability issue with clear future cost or testable bug risk.
- Low: local cleanup, naming, small duplication, or test clarity.

Do not implement low-value cleanup while higher-risk findings remain unresolved.

## Implementation Strategy

Work in small batches. For each batch:

1. State the scope and why it matters.
2. Edit only the files required for that scope.
3. Add or update focused tests.
4. Run the narrowest meaningful validation first.
5. Run broader validation when the touched area is shared or high risk.
6. Summarize residual risk and next recommended batch.

Prefer these cleanup types:

- Remove duplicated state normalization or persistence code.
- Consolidate repeated secret redaction and zeroization handling.
- Clarify fallible operations with typed errors or better context.
- Split large update branches into feature helpers following existing routing.
- Replace ad hoc parsing with structured config/API parsing.
- Add tests around migrations, defaults, and compatibility.
- Remove dead code only after confirming it is unreachable with search and
  tests.

Avoid these without explicit justification:

- Renaming public or persisted fields.
- Changing config schema defaults without serialization tests.
- Moving many files in one batch.
- Broad UI redesign.
- Rewriting working async/websocket flows for aesthetics.
- Introducing new global state.
- Adding dependencies for small helpers.

## Validation Expectations

Use a validation ladder:

1. `cargo fmt`
2. Focused tests for changed modules, for example:
   - `cargo test config::`
   - `cargo test secret`
   - `cargo test order_execution::`
   - `cargo test account_state::`
3. `cargo check`
4. `cargo test`
5. `cargo clippy --all-targets --all-features -- -D warnings`

If validation fails because of unrelated pre-existing worktree changes, report
the exact file and lint/test failure and do not repair unrelated files unless
asked.

For UI-only changes, at minimum run `cargo check`. For persistence, secrets,
orders, signing, account switching, or subscription changes, run focused tests
and broader tests when practical.

## Deliverables

Maintain a living audit note as work proceeds. Each entry should include:

- Finding title and severity.
- Evidence with file references.
- Risk or failure mode.
- Chosen fix or reason for deferral.
- Tests or checks run.
- Backwards compatibility impact.

At the end of each work session, report:

- What changed.
- What was validated.
- What remains dirty in the worktree and whether it was pre-existing.
- Any known residual risk.
- The next highest-value cleanup batch.

## First Pass Checklist

Start with a read-only pass and produce a ranked plan before editing:

- Search for `unwrap`, `expect`, `dbg!`, `println!`, `eprintln!`, `todo!`,
  `unimplemented!`, and broad `Debug` derives on secret-bearing types.
- Search for `agent_key`, `api_key`, `bearer`, `token`, `password`,
  `secret`, `wallet`, and `private`.
- Review config serialization tests for every touched persisted type.
- Review every path that clears credentials or deletes accounts.
- Review order placement and cancellation paths for stale account/profile
  handling.
- Review websocket manager registries for raw secret keys and task shutdown.
- Review backup/write paths for partial writes, permissions, and sanitized
  backups.
- Review routing tests for newly added or changed message variants.

Only after that plan is ranked should implementation begin.
