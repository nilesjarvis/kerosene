# Codex Refactor Tech Debt Report

Date: 2026-06-20

Source prompt: `audit/codex-refactor-tech-debt-long-running-prompt.md`

## Summary

This campaign completed six small, validated refactor batches focused on
debug-output hygiene and one strict-clippy blocker in journal snapshot request
construction. The changes reduce accidental full-payload Debug output for raw
API helpers, add bounded Debug summaries for exchange symbols carried through
messages, and restore a green strict clippy baseline.

No user-facing behavior, persisted schema, layout format, credential storage,
trading flow, signing flow, subscription behavior, or UI semantics were
intentionally changed.

## Completed Batches

- `386866d Summarize exchange symbol debug output`
  - Added bounded Debug output for `ExchangeSymbolsPayload`, `ExchangeSymbol`,
    and `OutcomeSymbolInfo`.
  - Added API/message tests proving outcome labels, descriptions, and
    thresholds are not dumped through `Message::SymbolsLoaded`.
- `31cd53a Drop HYPE ETF raw response debug derives`
  - Removed unused `Debug` derives from private BHYP, THYP, and Farside raw
    response structs.
- `4f7fe2d Drop HyperDash raw GraphQL debug derives`
  - Removed unused `Debug` derives from private HyperDash GraphQL response
    wrappers while preserving public/redacted model Debug behavior.
- `aa14972 Drop outcome metadata helper debug derives`
  - Removed unused `Debug` derives from private outcome metadata rows and
    question helper data.
- `07a00fb Record refactor milestone validation`
  - Recorded the first full-test pass and the pre-existing strict-clippy
    journal snapshot blocker.
- `63d4982 Refactor journal snapshot request inputs`
  - Derived `Default` for `JournalSnapshotCoverage`.
  - Introduced non-`Debug` `JournalSnapshotRequestSettings` to carry
    account/provider/timing metadata into snapshot constructors.
  - Removed long constructor argument lists and cleared strict clippy.
- `0896653 Drop SEC raw response debug derives`
  - Removed unused `Debug` derives from private SEC raw response helpers while
    preserving public `SecEarningsEvent` Debug output.

## Validation

Focused validation was run per batch and recorded in
`audit/codex-refactor-tech-debt-progress.md`.

Final broad validation:

- `cargo clippy --all-targets --all-features -- -D warnings` passed.
- `cargo test` passed: 3304 passed, 0 failed, 3 ignored.
- Doc-tests passed with 0 tests.
- `cargo fmt -- --check` and `git diff --check` passed on the final code
  batches before commit.

The Linux GUI smoke test was not run because this campaign did not touch
startup, window routing, shell, canvas behavior, or platform integration code.

## Compatibility Statement

- Persisted config/layout schemas: unchanged.
- Secret storage and keychain/encrypted payload formats: unchanged.
- Trading/order/signing/nonce behavior: unchanged.
- Account switching and automation lifecycle behavior: unchanged.
- UI layout, copy, hotkeys, pane defaults, and visual semantics: unchanged.
- Network request/response parsing semantics: unchanged for the touched API
  modules.
- Debug output: intentionally narrower for selected raw/private payloads and
  summarized for exchange symbol payloads.

## Residual Risks

- Future developers who need ad hoc Debug output for the private raw response
  helpers will need to add explicit summarized or redacted implementations.
- Public model Debug derives remain in several market-data/value types. Many are
  harmless value objects or already covered by custom redaction tests, but they
  should be reviewed deliberately rather than removed mechanically.
- The remaining private `CandleRequest` Debug derives are low-value candidates:
  they do not include bearer tokens and are request payloads rather than raw
  responses. They were left unchanged to avoid churn.

## Next Recommended Cleanup

Continue with a ranked, read-only scan before editing:

1. Re-rank remaining public API/model `Debug` derives and separate harmless
   value models from payloads that need summarized or custom-redacted output.
2. Revisit persisted config/default candidates from the startup backlog and
   only change fields with a concrete backwards-compatibility test plan.
3. Run a production-only unwrap/expect scan with better filtering for inline
   tests, then fix only non-test paths where the fallback behavior is obvious
   and covered by focused tests.
