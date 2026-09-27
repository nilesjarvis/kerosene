# Code simplification progress

## Objective and working rules

Review the entire codebase for simpler code, meaningful consolidation, prudent
module splits, and low-risk performance improvements. Preserve functionality,
persisted formats, privacy, and trading behavior. The goal remains active;
the batches below do not represent completion of the repository-wide review.

The worktree was clean at the start, on `bb07d8a7`. The user requested a commit
after each turn. Keep completed changes and validation in those commits.

## Review coverage

Initial inventory: 1,910 files under `src`, approximately 332,569 lines including
tests and fixtures. Read the architecture, runtime, and validation guides and
scanned Rust sources for repeated blocks and larger modules. This scan locates
candidates; it does not establish that every module has been reviewed.

| Area | Current coverage |
| --- | --- |
| Startup and layout restoration | Chart, comparison-chart, positioning, order-book, and default-watchlist initialization reviewed and consolidated. Session-data restoration inspected and left explicit because its fallback policies differ and its model construction is already shared. Remaining pane/layout flows need review. |
| Charting and canvas | Instance construction, persisted chart settings, annotation loading, and comparison settings reviewed. Rendering and interactions remain. |
| Market data | Mid-price update visibility filtering reviewed; unnecessary catalog copies removed. APIs, caching, symbol lifecycle, books, and other widgets remain. |
| Wallets and account state | Wallet detail and cluster read-result/websocket filters reviewed. Account picker/setup routes traced; unreachable legacy credential-editing handlers removed. Active Add Account, connection, and switching safety boundaries inspected and retained. Account user-stream handling and risk scrubbing inspected for copies but unchanged. Broader account and portfolio flows remain. |
| Orders, signing, Chase, TWAP | Initial duplication scan only; substantive review remains. |
| Config, persistence, secrets | Chart snapshot/config boundaries reviewed, schema unchanged. Remaining persistence/security code needs review. |
| Subscriptions and transport | Initial duplication scan only; substantive review remains. |
| Feeds, integrations, assistant | Initial size/duplication scan only; substantive review remains. |
| Views, settings, commands, app shell | Architecture mapped; default live-watchlist Add Widget creation reviewed and shared with restoration. Substantive review of other surfaces remains. |
| Tests, scripts, packaging, assets | Validation documentation read; remaining source/tooling review remains. |

## 2026-09-27: chart restoration

- `app_boot/chart_instances.rs` and `layout_persistence/instances.rs` duplicated
  persisted chart settings, quick-trade validation, annotations, and comparison
  settings. Future additions could easily update only one restoration path.
- Added `ChartInstance::from_config` in `chart_state/model/restoration.rs` and
  `SpaghettiChartInstance::from_config` in `spaghetti_state.rs`, used by both
  restoration paths.
- Symbol resolution, muted/hidden-symbol handling, legacy `@0` deferral, cache
  loading, request generations, and session normalization stay in their callers.
  Startup and runtime layout changes intentionally differ in those areas.
- Kept filtering before the quick-action limit and allocated annotation IDs only
  for valid annotations, preserving their order and the mirrored canvas state.
- Moved the layout instance tests to `layout_persistence/instances/tests.rs`.
  Added two integration tests that exercise both restoration paths and compare
  persisted snapshots, including non-default settings, invalid entries, dense
  annotation IDs, comparison modes, and watchlist association rules.
- Updated the charting component guide with the shared ownership boundary.
- No config/schema, message, subscription, dependency, or asset changes.

## 2026-09-27: avoid copies during read-only visibility filtering

- Five update sites copied `exchange_symbols`, `muted_tickers`, and
  `market_universe` solely for filtering while mutating unrelated state fields.
  Full symbol metadata contains owned strings and outcome metadata, so these
  copies are unnecessary work, especially on frequent mid-price events.
- Borrow those fields directly in `market_state/mids.rs`,
  `wallet_state/details.rs`, `wallet_update/details.rs`, and both the REST and
  websocket paths in `wallet_cluster_update.rs`.
- No filtering, account/source checks, timestamps, position freshness rules, or
  task scheduling changed. The Rust borrow checker verifies that the borrowed
  catalog cannot change while the filtering uses it.
- Account user-stream and risk-scrub copies were left alone: they span methods
  that borrow the full terminal and need a separate ownership review.

## Validation for the first turn (`d886a9ae`)

This environment has ALSA development files in a local dependency prefix. Plain
`cargo check --locked -j 2` initially failed to locate `alsa.pc`. Cargo commands
work with these environment variables:

```sh
PKG_CONFIG_PATH=/home/thread/.local/kerosene-deps/usr/lib/x86_64-linux-gnu/pkgconfig
LIBRARY_PATH=/home/thread/.local/kerosene-deps/usr/lib/x86_64-linux-gnu
```

- Baseline `cargo check --locked -j 2`: passed with that prefix. Existing
  dead-code warning for `SaveCredentials`, `WalletKeyInputChanged`, and
  `WalletAddressInputChanged` in `message.rs`.
- Baseline `cargo test --locked -j 2 --package kerosene --bin kerosene
  app_boot::chart_instances`: 3 passed.
- New `chart_restoration` tests: 2 passed.
- `cargo test --locked -j 2 --package kerosene --bin kerosene
  layout_persistence::`: 13 passed.
- `cargo check --locked -j 2` after the production changes: passed, with the
  same pre-existing warning.
- `cargo test --locked -j 2` after both batches and the test-module move:
  **4,296 passed, 0 failed, 6 ignored**; doc-tests passed (0 tests).
- `cargo fmt -- --check` and `git diff --check`: passed.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  failed on the three pre-existing unused message variants noted above.
- Diagnostic Clippy run with `-D warnings -A dead_code`: passed. This is not a
  strict-Clippy pass; no lint allowances were added to the source.
- `cargo build --locked -j 2`: passed with the same existing warning.
- Headless smoke: ran the built binary with `--test` under `xvfb-run -a` and a
  20-second timeout. `xwininfo -root -tree` confirmed a 1600x960 **Kerosene
  Trading Terminal** window. Exit 124 was the expected timeout; no panic.
  The only runtime output was an Xvfb EGL/DRI3 acceleration warning, so this
  validates startup, not hardware GPU rendering. In-memory test mode kept
  personal configuration out of the smoke run.

## 2026-09-27: remaining market-pane initialization

- Positioning and order-book startup/layout paths repeated settings assignment
  and reconstruction of instances missing from saved widget lists. Added
  `PositioningInfoInstance::from_config` and `OrderBookInstance::from_config`,
  and shared each feature's pane-recovery loop in its existing update module.
- Kept symbol fallback/canonicalization, tick-size validation, request reset,
  and fetch scheduling at the original call sites. Recovery still includes
  closed canvas workspaces, retains existing instances, and advances IDs only
  when it creates missing instances.
- Default watchlist creation was repeated in boot, layout restoration, and
  Add Widget. Moved it to `market_update/live_watchlist/panes.rs` and shared
  preset selection and visibility filtering. The Add Widget message delegates
  to a feature helper that preserves workspace targeting and split-failure
  cleanup. Saved watchlist symbol resolution remains unchanged.
- Added four integration tests in `layout_persistence/tests.rs`, covering
  snapshots from both restoration paths, all order-book display modes,
  deprecated positioning sorts, muted-symbol fallback, tick/height clamping,
  missing canvas instances, stale-instance/request removal, ID allocation,
  preservation of existing runtime state, and default watchlists at boot,
  layout load, and Add Widget. Updated the market-data component guide.
- Session-data restoration already delegates model construction and differs in
  startup/runtime symbol fallback. Left its small orchestration loops explicit.
- No schema, message routing, subscription, dependency, or trading changes.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene positioning`:
  60 passed after the positioning/order-book extraction.
- `cargo test --locked -j 2 --package kerosene --bin kerosene
  layout_persistence::tests`: all 4 new tests passed after watchlist extraction.
- `cargo test --locked -j 2`: **4,300 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo fmt -- --check` and `git diff --check`: passed.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  failed only on the same three pre-existing unused account message variants.
- `cargo build --locked -j 2`: passed with the existing unused-message warning.
- Headless startup smoke: `timeout 20s xvfb-run -a` running the built binary
  with `--test` opened the 1600x960 Kerosene window, confirmed by `xwininfo`.
  Expected timeout 124, no panic, and only the same EGL/DRI3 acceleration
  warning as the first turn. This checks startup with in-memory configuration,
  not hardware GPU rendering.

## 2026-09-27: remove retired account-editing routes

- Traced the three compiler-reported unused messages through the account views,
  root routing, account dispatch, and credential persistence. No production
  code constructs `SaveCredentials`, `WalletKeyInputChanged`, or
  `WalletAddressInputChanged`; account setup uses the dedicated Add Account
  window and its draft messages.
- Removed those variants, routes, legacy profile-editing handlers, and 22 tests
  that only exercised the removed handlers. Kept the live connection/rebinding,
  switching, deferred migration, and shared credential-persistence paths and
  their tests. Active wallet input fields are still needed by those flows.
  `SensitiveString::into_zeroizing` is now only needed by test fixtures and is
  compiled under `cfg(test)`, alongside its existing test-only `clear` helper.
- Moved the Add Account tests into `account_update/add_window/tests.rs` without
  changing that module's production logic. Added a test through the root update
  route proving draft edits and cancellation preserve the active signer and
  stored credentials. Extended the locked-store failure test to check the
  signer, active inputs, retained draft, and encrypted payload. Added coverage
  for all Add Account draft/lifecycle routes; existing redaction tests still
  cover the active address and key messages.
- Updated the account component guide and a stale connection comment. No
  persisted format, subscription, dependency, or user-facing behavior changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene account_update::`:
  **160 passed**, including connection/rebinding and Add Account safety tests.
- `cargo test --locked -j 2`: **4,280 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests). The count decreased by 20: 22 tests of removed handlers
  were deleted and 2 active-flow/routing tests were added.
- Compared the extracted Add Account production module and all 10 retained
  profile methods against the previous commit: their implementations match.
- Strict Clippy initially identified the now-test-only `SensitiveString`
  conversion after the old handlers were removed; scoped it to test builds.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  **passed**, with no lint allowances.
- `cargo fmt -- --check`, `git diff --check`, and
  `cargo build --locked -j 2`: passed.
- Headless startup smoke with the built binary and `--test`: the 1600x960
  Kerosene window opened under Xvfb, confirmed by `xwininfo`. Expected timeout
  124 after 20 seconds, no panic, and only the existing EGL/DRI3 warning. This
  checks startup with in-memory configuration, not hardware GPU rendering.

## Next candidates

1. Review repeated account-picker/layout-switcher styling and comparison-chart
   axis drawing. Similar appearance alone is not sufficient reason to share a
   helper; preserve layout, coordinate, and interaction differences.
   The three active-row style closures in
   `account_views/summary/layout_switcher.rs` and
   `account_views/picker/dropdown/option_row.rs` were inspected and match; the
   existing `helpers/ui/buttons.rs` styles have different state/color behavior.
2. Continue across the unreviewed areas in the coverage table. Large files often
   include inline tests, so distinguish production complexity from file length.
