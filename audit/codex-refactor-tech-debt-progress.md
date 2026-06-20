# Codex Refactor Tech-Debt Progress

Started: 2026-06-20

Source prompt: `audit/codex-refactor-tech-debt-long-running-prompt.md`

## Startup — 2026-06-20

- Branch: `main`
- Initial worktree: two untracked directories, `portfolio-widget-handoff/` and
  `telegram-feed-handoff/`. They appear unrelated to this campaign and must not
  be staged or modified.
- Required context read:
  - `AGENTS.md`
  - `Cargo.toml`
  - `audit/codebase-cleanup-long-running-prompt.md`
  - `audit/codebase-cleanup-progress.md`
  - `audit/codex-refactor-tech-debt-long-running-prompt.md`
- Prior progress summary:
  - Previous cleanup work has already addressed many high-risk persistence,
    secret-storage, account-switching, order-safety, websocket-lag, and Debug
    redaction findings.
  - The latest prior residual recommendation was to continue scanning public API
    payloads and metadata structs that still derive full `Debug`.

## Read-Only Discovery Backlog

1. Medium/Low privacy and operability: exchange-symbol fetch payloads still
   derive full `Debug`, including every symbol and full outcome-market metadata.
   Evidence: `src/api/exchange_symbols.rs:19`,
   `src/api/exchange_symbols/model.rs:14`, and
   `src/api/exchange_symbols/model.rs:50`. These payloads flow through
   `Message::SymbolsLoaded` at `src/message.rs:1082`.
2. Low maintainability: several API/private response rows still derive `Debug`
   even though they are only deserialization carriers. Evidence includes
   `src/api/hype_etfs/bhyp.rs`, `src/api/hype_etfs/thyp.rs`, and
   `src/hyperdash_api/*`. Re-rank before editing because recent batches already
   covered HYPE ETF state-level Debug output.
3. Low persistence compatibility: some persisted config structs still use direct
   `Deserialize`, but many have custom tolerant wire wrappers or custom field
   deserializers. Evidence: `src/config/live_watchlist.rs`,
   `src/config/fonts.rs`, and `src/config/schema/candles.rs`. Any change here
   needs exact schema tests and should not duplicate prior enum-tolerance work.
4. Deferred after inspection: apparent `Instant::now()` calls in calendar and
   HYPE unstaking queue views are test-only; production view paths already use
   `status_bar_now`/`status_bar_now_ms`.
5. Deferred after inspection: apparent production `unwrap()` in move-order
   result handling is inside an inline test module, not runtime code.

## Batch 1 — Summarize Exchange Symbol Debug Output

- Status: validated
- Scope: `src/api/exchange_symbols.rs`, `src/api/exchange_symbols/model.rs`,
  `src/api/exchange_symbols/tests.rs`, and `src/message.rs`.
- Motivation: reduce full public market metadata dumps in message/result Debug
  output while preserving useful diagnostics and all runtime behavior.
- Behavior invariant: symbol fetch parsing, serialization, deserialization,
  equality, sorting, cache shape, market resolution, orderability, and UI labels
  remain unchanged. Only `Debug` formatting changes.
- Evidence: `ExchangeSymbolsPayload`, `OutcomeSymbolInfo`, and
  `ExchangeSymbol` currently derive full `Debug`; `Message::SymbolsLoaded`
  carries the payload result.
- Change summary: replaced full derived `Debug` for exchange-symbol payloads and
  outcome metadata with bounded summary formatters; added focused API and
  message-level tests that verify counts/status flags remain visible while raw
  outcome names, descriptions, and threshold values are not printed.
- Tests/checks run:
  - `cargo fmt` passed.
  - `cargo test --package kerosene --bin kerosene api::exchange_symbols` passed
    (16 tests).
  - `cargo test --package kerosene --bin kerosene symbols_loaded_message_debug_summarizes_exchange_metadata`
    passed (1 test).
  - `cargo test --package kerosene --bin kerosene message::tests` passed (8
    tests).
  - `cargo check` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed.
- Compatibility impact: no persisted schema, serialized field, UI, trading,
  order, account, or secret-storage behavior changed. Runtime data remains fully
  available to parsers and update/view code; only diagnostic formatting is
  bounded.
- Residual risk: direct `Debug` on individual exchange symbols now omits full
  display names, keywords, and outcome text. This is intentional for bounded
  diagnostics; tests cover the new summary fields.
- Next candidate: continue scanning lower-risk API payload Debug derives or
  re-rank strict persisted enum candidates.
