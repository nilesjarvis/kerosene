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

## Batch 2 — Drop HYPE ETF Raw Response Debug Derives

- Status: validated
- Scope: `src/api/hype_etfs/bhyp.rs`, `src/api/hype_etfs/thyp.rs`, and
  `src/api/hype_etfs/farside.rs`.
- Motivation: remove unnecessary derived `Debug` from private API response
  carriers after prior work already redacted the public HYPE ETF state Debug
  output. This keeps accidental diagnostic dumps smaller without changing
  parsing or mapped state.
- Behavior invariant: BHYP, THYP, and Farside response deserialization,
  validation, ETF fund mapping, daily-flow calculation, and UI state updates
  remain unchanged. Only the private raw response structs stop implementing
  `Debug`.
- Evidence: private raw response structs derive `Debug` in
  `src/api/hype_etfs/bhyp.rs`, `src/api/hype_etfs/thyp.rs`, and
  `src/api/hype_etfs/farside.rs`; `rg` found no formatting or trait-bound
  dependence on those Debug impls.
- Change summary: removed `Debug` from the private BHYP, THYP, and Farside
  response/row structs while keeping their `Deserialize` implementations and
  mapping logic unchanged.
- Tests/checks run:
  - `cargo fmt` passed.
  - `rg -n "derive\\([^\\n]*Debug|Debug" src/api/hype_etfs src/api/hype_etfs.rs`
    returned no matches.
  - `cargo test --package kerosene --bin kerosene api::hype_etfs` passed (23
    tests).
  - `cargo test --package kerosene --bin kerosene market_update::hype_etfs::tests`
    passed (6 tests).
  - `cargo check` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed.
- Compatibility impact: expected none for persisted data, UI, trading behavior,
  and secrets. Response parsing and mapped state are unchanged.
- Residual risk: downstream code cannot format these private raw response rows
  with `Debug`; current search and compile checks show no such dependency.
- Next candidate: continue scanning HyperDash raw response Debug derives or
  re-rank strict persisted enum candidates.

## Batch 3 — Drop HyperDash Raw GraphQL Debug Derives

- Status: validated
- Scope: `src/hyperdash_api/heatmap/parsing.rs`,
  `src/hyperdash_api/liquidation_levels.rs`, and
  `src/hyperdash_api/positioning/response.rs`.
- Motivation: continue the raw-response cleanup by removing derived `Debug`
  from private GraphQL response wrappers that are only deserialized and mapped
  into public model/state types.
- Behavior invariant: HyperDash heatmap parsing, liquidation-level parsing,
  positioning/perp-delta parsing, error classification, response size bounds,
  and UI update behavior remain unchanged. Public model Debug implementations
  are not changed in this batch.
- Evidence: private `Gql*` wrapper structs in the scoped files derive `Debug`;
  `rg` found no formatting or trait-bound dependence on those private Debug
  impls.
- Change summary: removed `Debug` from private HyperDash heatmap,
  liquidation-level, and positioning GraphQL response wrappers while leaving
  public model/value Debug behavior unchanged.
- Tests/checks run:
  - `cargo fmt` passed.
  - `rg -n "derive\\([^\\n]*Debug|#\\[derive\\([^\\]]*Debug" src/hyperdash_api/heatmap/parsing.rs src/hyperdash_api/liquidation_levels.rs src/hyperdash_api/positioning/response.rs`
    returned no matches.
  - `cargo test --package kerosene --bin kerosene hyperdash_api::heatmap`
    passed (6 tests).
  - `cargo test --package kerosene --bin kerosene hyperdash_api::liquidation_levels`
    passed (2 tests).
  - `cargo test --package kerosene --bin kerosene hyperdash_api::positioning`
    passed (9 tests).
  - `cargo test --package kerosene --bin kerosene market_update::positioning_info::tests`
    passed (13 tests).
  - `cargo check` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed.
- Compatibility impact: expected none for persisted data, UI, trading behavior,
  and secrets. Response parsing and public model state are unchanged.
- Residual risk: downstream code cannot format these private raw GraphQL
  wrappers with `Debug`; current search and compile checks show no such
  dependency.
- Next candidate: re-rank remaining public API/model Debug derives versus
  strict persisted enum candidates.

## Batch 4 — Drop Outcome Metadata Helper Debug Derives

- Status: validated
- Scope: `src/api/exchange_symbols/outcomes.rs` and
  `src/api/exchange_symbols/outcomes/questions.rs`.
- Motivation: continue the private metadata cleanup for outcome-market symbol
  loading. Raw outcome entries and derived question helpers carry full names,
  descriptions, and threshold strings that do not need ad hoc Debug output now
  that the public exchange-symbol Debug output is summarized.
- Behavior invariant: outcome metadata deserialization, question-to-outcome
  mapping, symbol construction, keywords, display labels, orderability, and
  quote-token handling remain unchanged. `Clone` is preserved where the helper
  map needs it.
- Evidence: `OutcomeMetaResponse`, raw outcome/question rows, and
  `OutcomeQuestionInfo` derive `Debug`; `rg` found no formatting or trait-bound
  dependence on those Debug impls.
- Change summary: removed `Debug` from raw outcome metadata response rows and
  the internal question-info helper while preserving `Clone` and `Deserialize`
  where required by parsing and map construction.
- Tests/checks run:
  - `cargo fmt` passed.
  - `rg -n "derive\\([^\\n]*Debug|#\\[derive\\([^\\]]*Debug|Debug" src/api/exchange_symbols/outcomes.rs src/api/exchange_symbols/outcomes/questions.rs`
    returned no matches.
  - `cargo test --package kerosene --bin kerosene api::exchange_symbols` passed
    (16 tests).
  - `cargo check` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed.
- Compatibility impact: expected none for persisted data, UI, trading behavior,
  and secrets. Outcome symbol construction and display metadata are unchanged.
- Residual risk: downstream code cannot format these private metadata helpers
  with `Debug`; current search and compile checks show no such dependency.
- Next candidate: re-rank remaining public API/model Debug derives versus
  strict persisted enum candidates.

## Broad Validation — 2026-06-20

- Status: partial
- Scope: milestone validation after Batches 1-4.
- Tests/checks run:
  - `cargo test` passed: 3304 passed, 0 failed, 3 ignored.
  - `cargo clippy --all-targets --all-features -- -D warnings` failed with
    pre-existing journal findings in `src/journal/snapshot.rs`:
    `clippy::derivable_impls` on `JournalSnapshotCoverage` and
    `clippy::too_many_arguments` on `initial_snapshot_request` and
    `live_position_snapshot_request`.
- Compatibility impact: no code changed for this validation entry.
- Residual risk: strict clippy remains red for journal snapshot issues outside
  the Debug-output batches. Re-rank separately before changing because the
  functions are journal snapshot API boundaries and need focused tests.
- Next candidate: either address the journal snapshot clippy findings as a
  scoped refactor batch, or continue the public API/model Debug derive scan.

## Batch 5 — Refactor Journal Snapshot Request Inputs

- Status: validated
- Scope: `src/journal/snapshot.rs`, `src/journal.rs`, and
  `src/journal_update.rs`.
- Motivation: clear the strict clippy findings recorded during broad
  validation while keeping journal snapshot behavior and redaction boundaries
  unchanged.
- Behavior invariant: fill-based, live-position, pinned-timeframe, retry,
  provider-generation, account-key, address, coverage, and now-time semantics
  remain unchanged.
- Evidence: `JournalSnapshotCoverage` had a manual `Default` implementation
  matching the `TwoX` variant, and the snapshot constructors accepted repeated
  account/provider/timing fields. The fields are metadata for request
  construction rather than independent behavior switches.
- Change summary: derived `Default` for `JournalSnapshotCoverage` with `TwoX`
  as the default variant; introduced `JournalSnapshotRequestSettings` to carry
  account/provider/timing metadata; routed all snapshot constructors through the
  settings value; and updated journal update/tests to use the lower-arity API.
  `JournalSnapshotRequestSettings` intentionally does not implement `Debug`
  because it carries account identifiers and wallet addresses before request
  redaction.
- Tests/checks run:
  - `cargo fmt` passed.
  - `cargo test --package kerosene --bin kerosene journal::snapshot` passed
    (11 tests).
  - `cargo test --package kerosene --bin kerosene journal_update::tests` passed
    (15 tests).
  - `cargo check` passed.
  - `cargo clippy --all-targets --all-features -- -D warnings` passed.
  - `cargo test` passed: 3304 passed, 0 failed, 3 ignored.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed.
- Compatibility impact: expected none for persisted data, UI, trading behavior,
  and secrets. The refactor changes internal Rust call shapes only; snapshot
  request fields and redacted `JournalTradeSnapshotRequest` Debug output are
  unchanged.
- Residual risk: downstream code outside the crate cannot use the old long
  constructor signatures. This is expected for the binary crate boundary and
  compile checks covered all local call sites.
- Next candidate: resume the ranked public API/model Debug derive scan and
  separate true leak risk from already-redacted model Debug implementations.

## Batch 6 — Drop SEC Raw Response Debug Derives

- Status: validated
- Scope: `src/api/sec.rs`.
- Motivation: continue removing unused full-payload Debug implementations from
  private deserialize-only API response structs. SEC raw submissions can include
  large company filing vectors, and local code only needs them for parsing.
- Behavior invariant: SEC ticker lookup, company submission parsing, earnings
  event extraction, sorting, chart-marker conversion, and error handling remain
  unchanged.
- Evidence: `SecTickerEntry`, `SecCompanySubmissions`, `SecCompanyFilings`, and
  `SecRecentFilings` were private raw response helpers deriving `Debug`; `rg`
  found no formatting or trait-bound dependence on those Debug impls. The
  public `SecEarningsEvent` keeps `Debug` because chart state/messages can carry
  event values.
- Change summary: removed `Debug` from the private SEC raw response helpers
  while preserving `Clone`, `Default`, and `Deserialize` where the parser needs
  them.
- Tests/checks run:
  - `cargo fmt` passed.
  - `cargo test --package kerosene --bin kerosene api::sec` passed (3 tests).
  - `cargo test --package kerosene --bin kerosene chart_update::earnings`
    passed (11 tests).
  - `cargo check` passed.
  - `rg -n "#\\[derive\\([^\\]]*Debug|Debug" src/api/sec.rs` now only reports
    public `SecEarningsEvent`.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed.
- Compatibility impact: expected none for persisted data, UI, trading behavior,
  and secrets. SEC parsing output and public event shape are unchanged.
- Residual risk: downstream code cannot format these private raw SEC helper
  structs with `Debug`; current search and compile checks show no such
  dependency.
- Next candidate: continue the ranked Debug derive scan, prioritizing private
  raw response helpers before public model/value types that already have
  redaction or harmless summary output.

## Final Validation and Report — 2026-06-20

- Status: validated
- Scope: all committed refactor batches in this campaign.
- Tests/checks run:
  - `cargo clippy --all-targets --all-features -- -D warnings` passed.
  - `cargo test` passed: 3304 passed, 0 failed, 3 ignored; doc-tests passed
    with 0 tests.
- Compatibility impact: no persisted config, layout, secret storage, trading,
  signing, order planning, subscription, or UI behavior changes were made. The
  only Rust API shape change was internal to journal snapshot request
  construction and was covered by focused tests plus full test/clippy runs.
- Stopping rationale: the remaining obvious API `Debug` derives are public value
  models, custom-redacted implementations, or lower-value private request
  payloads. Continuing mechanically would risk churn without a stronger safety
  payoff.
- Final report: `audit/codex-refactor-tech-debt-report.md`.
