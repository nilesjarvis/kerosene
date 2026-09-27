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
| Charting and canvas | Instance construction, persisted chart settings, annotation loading, and comparison settings reviewed. Comparison-chart rendering contexts and axes reviewed and consolidated; crosshair formatting and series calculations inspected but retained. Metadata-driven chart identity reconciliation reviewed and separated from symbol refresh orchestration. Most chart rendering and interactions remain. |
| Market data | Mid-price update visibility filtering reviewed; unnecessary catalog copies removed. Persistent API cache reviewed and split into candle policy, queued writes, and storage, with write coalescing simplified. Public shared reads and read admission inspected and retained. API exports, order-book reads, chart asset-context reads, watchlist context requests/parsing, and exchange statistics reviewed. Spot chart context lookup indexed once per response; differing parser and partial-result policies retained. Symbol metadata orchestration, perpetual/spot parsers, DEX registry parsing, and listings parsers reviewed; unnecessary metadata copies removed. Symbol refresh, legacy spot migration, label updates, and search context results reviewed and split; shared watchlist alias rewriting and removed intermediate copies. Symbol search planning, filtering, sorting, DEX listing/ranking, and volume lookup reviewed; ranking work moved out of comparisons. Remaining API requests, outcome metadata internals, books, and other widgets need review. |
| Wallets and account state | Wallet detail and cluster read-result/websocket filters reviewed. Account picker/setup routes traced; unreachable legacy credential-editing handlers removed. Active Add Account, connection, and switching safety boundaries inspected and retained. Account user-stream handling and risk scrubbing inspected for copies but unchanged. Broader account and portfolio flows remain. |
| Orders, signing, Chase, TWAP | Chase/TWAP market-subscription assembly reviewed and shared with order-book panes; lifecycle eligibility filters and event mappings retained. Order execution, signing, and automation state-machine review remains. |
| Config, persistence, secrets | Chart snapshot/config boundaries reviewed, schema unchanged. Remaining persistence/security code needs review. |
| Subscriptions and transport | Subscription assembly reviewed across market, user data, Hydromancer, Telegram, timer/input, and window families. Shared selected-provider book setup and reduced symbol copies; remaining eligibility/identity differences retained. Market adapters and user-data routing/dispatch inspected. Shared reconnect-before-notify behavior and snapshot timing, split Hydromancer adapters, and reduced owned payload copies. Native manager lifecycle/commands, both managers' subscription reference counts/coalescers, and Hydromancer registry/session state inspected; provider-specific lifecycle and routing retained. Remaining integration stream internals still need review. |
| Feeds, integrations, assistant | Initial size/duplication scan only; substantive review remains. |
| Views, settings, commands, app shell | Architecture mapped; default live-watchlist Add Widget creation reviewed and shared with restoration. Account/layout-picker selection styles reviewed and consolidated. Substantive review of other surfaces remains. |
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

## 2026-09-27: share picker styling and comparison-chart axes

- Replaced three identical account/layout-picker style closures with
  `helpers::selected_row_button_style`. Preserved hover precedence, selected
  borders, transparent unselected backgrounds, and disabled/pressed styling.
  Button contents, layout, and messages remain at their original call sites.
- Replaced the two almost-identical comparison rendering context types with
  one `RenderContext`, assembled once before selecting the rendering mode.
  The normalized reference timestamp remains an explicit mode-specific input.
- Shared the value grid, axis border, relative-time axis, and dashed session
  marker in `spaghetti/axes.rs`. The normalized zero-percent line still renders
  between its grid and border. Ratio formatting and normalization/ratio data
  calculations are unchanged; both modes retain their original drawing order.
- Added software-renderer tests spanning normalized lines, ratio lines, and
  ratio candles at three widths, two themes, grid/dotted backgrounds, and
  ordinary/anchored/panned viewports. They check the shared time axis and the
  single-loaded-series fallback to normalized mode. Optional preview output
  uses synthetic data only and supports direct before/after image comparisons.
- Updated the charting component guide. No schema, messages, subscriptions,
  dependencies, account behavior, or order behavior changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene spaghetti::`:
  **40 passed**, including both new software-renderer tests.
- Ran the same two rendering tests against parent commit `3fe76603` in a
  temporary detached checkout: both passed. All **108** synthetic preview PNGs
  from the old and new implementations were **byte-for-byte identical**,
  verified with SHA-256. This covers the stated rendering matrix on tiny-skia;
  it is not a hardware GPU comparison. Removed the temporary checkout after
  verification. Preview files remain outside the repository under
  `/tmp/kerosene-comparison-before` and `/tmp/kerosene-comparison-after`.
- Compared the three original picker style bodies to the shared helper after
  normalizing whitespace and the boolean parameter name: all match.
- `cargo test --locked -j 2`: **4,282 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`,
  `cargo fmt -- --check`, and `git diff --check`: passed.
- `cargo build --locked -j 2`: passed. Headless startup with the built binary
  and `--test` opened the 1600x960 Kerosene window under Xvfb, confirmed by
  `xwininfo`. Expected timeout 124 after 20 seconds, no panic, and only the
  existing EGL/DRI3 acceleration warning.

The staged cleanup was included in concurrent release commit `5d9aebe1`, which
also changed the package version from 0.2.1 to 0.3.0 during final validation.
The commands above ran before that metadata change. Final smoke results were
recorded in a separate documentation commit; the release commit was preserved.

## 2026-09-27: simplify market-subscription assembly

- Shared the identical selected-provider book setup used by order-book panes,
  Chase, and TWAP in `market_book_subscription`. Canonical precision and source
  context remain in the shared setup; each consumer retains its eligibility
  filters, timers, and distinct message-mapping function. Keeping those
  function items distinct is necessary for iced subscription identity.
- Moved order-book pane assembly and its event mappings from `market.rs` into
  the existing `market/order_book.rs` feature module. Both moved event-mapping
  bodies and the Chase/TWAP event-mapping bodies match the previous commit.
- Borrowed chart symbol/timeframe keys while deduplicating in `BTreeMap`, then
  created owned strings only for the resulting stream recipes. The ordering,
  minimum chart ID, primary/secondary eligibility, and recipe tuple types are
  preserved. Order-book mode selection now borrows its symbol, and positioning
  subscriptions consume the strings they already own.
- Kept selected-provider and explicit Hydromancer-keyed source rules distinct.
  Native chart funding streams, one-second candles, real-time position PnL,
  and transport fallback generation handling retain their existing behavior.
- Added three tests covering exact recipe identities, provider/key selection,
  canonical precision with and without fresh prices, consumer separation when
  IDs coincide, automation eligibility/timers, chart deduplication/minimum IDs,
  secondary symbols, and the one-second candle source scope. Existing event
  mapping and stale-source tests remain. Updated the subscription guide.
- No schema, dependency versions, messages, network protocol, signing, or
  order-execution changes.

Validation (using the local ALSA prefix documented above):

- The first `--locked` run identified that the tracked `Cargo.lock`
  still held package version 0.2.1 after the concurrent 0.3.0 release bump.
  Let Cargo refresh it with `--offline`; diff verified that only the root
  package version changed. Include that one-line synchronization so locked
  builds work from the committed tree; no dependency versions changed.
- The initial focused run exposed a missing freshness timestamp in the new
  mid-price fixture. Added the timestamp so both precision paths are exercised.
- `cargo test --locked -j 2 --package kerosene --bin kerosene
  subscription_state::market::tests`: **23 passed, 1 ignored**, no failures.
- `cargo test --locked -j 2`: **4,285 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo fmt -- --check` and `git diff --check`: passed.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo build --locked -j 2`: passed. Headless startup with `--test` opened the
  1600x960 Kerosene window under Xvfb, confirmed by `xwininfo`. Expected timeout
  124 after 20 seconds, no panic, and only the existing EGL/DRI3 acceleration
  warning. This verifies startup with in-memory configuration.

## 2026-09-27: separate API cache responsibilities and streamline queued writes

- Split `api_cache.rs` into candle policy in `api_cache/candles.rs`, queued
  writes in `api_cache/writer.rs`, and JSON envelopes, paths, and atomic file
  replacement in `api_cache/storage.rs`. Exchange metadata and watchlist
  freshness remain in the root module. Public entry points retain their paths,
  including the atomic writer used by listings persistence.
- Replaced repeated searches through later jobs with one reverse pass and a
  set of superseding target paths. This changes worst-case quadratic scanning
  to expected linear work in the batch size and avoids constructing target
  paths for merge-only jobs. No wall-clock speedup is claimed.
- Retained the exact coalescing rule: a later save or removal supersedes an
  earlier plain save for the same target; a merge does not. Merges and removals
  always run, and retained jobs execute in their original forward order.
  Best-effort error handling, the single writer thread, and inline fallback
  after a thread-spawn failure are unchanged.
- Kept candle closure, coverage, freshness, interval-gap, provider/key, and
  length policies unchanged, as well as cache schemas, namespaces, timestamps,
  path encoding, temporary-file permissions, fsync, and replacement behavior.
  Compared all 44 existing function bodies after normalizing whitespace:
  only `writes_to_run` changed; the other 43 match the previous commit.
- Moved all 13 existing tests beside their respective modules. Added five
  tests covering all 37,449 job sequences of length zero through five across
  four job kinds and two targets, actual save/merge/removal order, continued
  writes after a failed job, envelope metadata rejection, and atomic replacement
  cleanup. Fixtures and temporary files use synthetic data only.
- Updated the charting component guide. No schema, messages, subscriptions,
  dependencies, or trading behavior changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene api_cache::`:
  **18 passed**, no failures.
- Strict Clippy initially flagged a modulo comparison in a new test fixture;
  replaced it with indexing into the fixture's two symbols.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo test --locked -j 2`: **4,290 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo fmt -- --check` and `git diff --check`: passed.
- `cargo build --locked -j 2`: passed. Headless startup with `--test` opened the
  1600x960 Kerosene window under Xvfb, confirmed by `xwininfo`. Expected timeout
  124 after 20 seconds, no panic, and only the existing EGL/DRI3 acceleration
  warning. This verifies startup with in-memory configuration; hardware GPU
  rendering and other platforms were not exercised.

## 2026-09-27: simplify Hydromancer market payload handling

- Reviewed the remaining subscription assembly modules. Kept the short timer,
  input, integration, account, wallet-detail, and cluster branches explicit:
  their visibility, eligibility, key, and consumer-identity rules differ.
  Corrected the subscription guide's user-data and window-event descriptions
  to match the existing code, including cluster streams and unhandled `NoOp`.
- Inspected native market adapters, public shared reads, and read admission.
  Retained their provider-specific lag/watchdog, cancellation, expiry, priority,
  and budget behavior. Manager lifecycle internals still need a separate review.
- Split Hydromancer market adapters into `books.rs`, `asset_context.rs`, and
  `candles.rs`, following the native stream layout. The root retains shared
  authentication-fallback policy and stable public entry points. Moved all four
  existing tests into adjacent test files.
- Consolidated the common direct-item, wrapped-item, and data-batch selection
  in `payloads.rs`. Selectors return slices borrowed from the incoming frame,
  eliminating per-message vectors and the early copy of every candle in a batch.
  Matching candles still clone once for deserialization, after routing filters.
  Legacy `books` and singular `candle` fallbacks remain distinct. Field-presence
  rules and empty-batch precedence are unchanged.
- Removed a book fallback match that reconstructed both event variants with
  exactly the same fields. Events now pass through directly, preserving the
  fallback's source generation and downstream failure handling.
- Compared existing production function bodies after normalizing whitespace
  and the moved reconnect-helper import: 11 of 15 match the previous commit.
  Only the three selectors and the identity-only book fallback match changed.
  Subscription payloads, topics, guards, buffer sizes, reconnects, one-second
  candle restrictions, watchdogs, event mapping, and validation are retained.
- Added two fixture-matrix tests covering all three selectors, accepted and
  rejected shapes, mixed/malformed batches, null fields, wrapper precedence,
  and pointer identity proving selected values borrow the source frame.
- No schema, dependencies, message routes, subscription parameters, external
  protocol, or order-execution changes.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene ws::`:
  **557 passed**, no failures. This filter includes the websocket tests plus
  other module paths containing `ws::`, such as view tests.
- Removed an unused reconnect-helper import from the candle module after the
  initial extraction; candle lag intentionally does not reconnect the manager.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo test --locked -j 2`: **4,292 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo fmt -- --check` and `git diff --check`: passed.
- Validation used synthetic/local fixtures and local websocket integration
  tests; no live provider credentials or live market connections were used.

## 2026-09-27: share websocket recovery and avoid owned-payload copies

- Reviewed user-data routing, parsing, subscription construction, and receive
  handling, along with the native manager's connect/read/backoff loop and
  both providers' subscription reference counts. Kept provider-specific manager
  loops separate: idle shutdown, key rotation, session resumption, command
  handling while connecting, and backoff policies differ.
- Replaced three identical reconnect-before-notify helpers with
  `ws/recovery.rs`. Callers supply their existing reconnect operation, event,
  output callback, and pause. Reconnect requests still occur only when the
  future is polled, before notification; either failure short-circuits, and
  the pause occurs only after successful notification. Provider reconnect
  gates and all callers' pause durations are unchanged.
- Shared user-data action execution in `UserStreamReceiveAction::emit`.
  Parsed frames and broadcast lag now select an action and pause before using
  one dispatch path. Targeted malformed spot state still reconciles with no
  pause; broadcast lag retains its two-second pause. Ordinary updates and
  ignored messages neither reconnect nor pause. Address checks, mids opt-out,
  payload parsing, and subscription identity are unchanged.
- Native text-frame parsing takes its owned `data` value instead of cloning
  the entire JSON subtree. Final unsubscribe handling in both managers moves
  the removed entry's payload instead of cloning it before removal. Preserved
  null/scalar payload handling, pong precedence, reference counts, payload
  matching, entry ordering, and native unsubscribe-method rewriting.
- Moved the nine existing user-stream tests to `user_streams/tests.rs`. Added
  three tests: recovery operation ordering/failures/pause with lazy polling;
  a 24-case action-dispatch matrix with live/closed local command channels;
  and frame payload shapes, malformed envelopes, and pong precedence.
  Existing reconnect-gate, private routing, redaction, reference-count, and
  local websocket lifecycle tests remain.
- Updated the subscription guide. No dependencies, wire formats, persisted
  schema, message routes, account eligibility, or trading behavior changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene ws::`:
  **560 passed**, no failures. The filter includes websocket tests and other
  module paths containing `ws::`, including view tests.
- Compared the three original recovery bodies with the shared helper after
  renaming the request operation and event parameter: all match.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo test --locked -j 2`: **4,295 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo fmt -- --check` and `git diff --check`: passed.
- New recovery tests explicitly poll futures using local command channels;
  they do not wait for the test pause or require live provider credentials.

## 2026-09-27: share snapshot coalescing and retain provider routing

- Reviewed both websocket coalescers and their tests, Hydromancer session
  transitions, and manager registry creation/replacement/eviction. Kept
  registry task IDs, closed-channel checks, key scoping, lock handling, and
  best-effort shutdown behavior explicit and unchanged.
- Shared the duplicate snapshot pacing in `ws/coalescer.rs`. Provider adapters
  still build their own keys and messages; only timing, latest-snapshot storage,
  pruning, and emission are shared. Both providers retain their 16 ms default.
- Preserved immediate first emission, fixed pending deadlines, replacement by
  the latest snapshot, removal of older pending data before an immediate send,
  independent coin/precision slots, passthrough channels, flush-on-disconnect,
  and recording emission time even when no broadcast receiver is present.
  Native missing-coin keys and Hydromancer batch/ambiguous-frame rules remain
  different. The native adapter now retains the complete routed message while
  pending, instead of reconstructing it from its key when flushing.
- Changed Hydromancer batch inspection to borrow the source slice. Only valid
  multi-item batches clone their items for splitting; empty, single-item, and
  ambiguous batches keep their original frame without a speculative batch copy.
  Data-wrapper precedence, item order, recursive splitting, and precision
  extraction are unchanged.
- Merged identical Connected/Reconnected session-ready branches. Cursor and
  session-ID assignment, subscription replay, pong behavior, and redaction are
  unchanged; existing tests exercise both session transitions independently.
- Retained all 18 original coalescer tests, adapting only the two history-size
  inspections to the shared implementation. Added seven tests covering fixed
  deadlines, partial/full flushes, pruning with pending data, immediate updates,
  zero intervals, failed broadcasts, borrowed/unsplit batch frames, batched
  precision variants, and native missing-coin behavior. New timing tests use
  controlled deadlines rather than sleeps.
- Updated the subscription guide. No dependencies, wire formats, saved schema,
  subscription identities, message routes, or trading behavior changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene coalescer`:
  **24 passed**, no failures, before the final native missing-coin case was added.
- Compared the original Hydromancer timing bodies with the shared core after
  replacing the concrete key type and key-expression parameter: submit,
  history pruning, next-due calculation, and both flush methods match.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- `cargo test --locked -j 2`: **4,302 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests). This includes all 25 coalescer tests and the existing
  Connected/Reconnected session tests.
- Validation used synthetic messages and local websocket integration tests;
  no live provider credentials were required.

## 2026-09-27: index batched spot chart contexts once per response

- Reviewed API exports, order-book requests/parsing, chart asset-context reads,
  watchlist context requests/parsing, exchange statistics, and the chart batch
  request scheduler. Kept chart/watchlist parsers separate because their alias
  precedence and duplicate handling differ. Exchange statistics require complete
  family totals, while watchlists allow healthy partial results; retained those
  policies.
- Moved spot chart context fetching and parsing into
  `api/chart_asset_context/spot.rs`, retaining the existing crate-facing fetch
  path. The parent module keeps perpetual context request construction/parsing;
  its function bodies are unchanged. Moved existing tests beside their parsers.
- Build borrowed context and universe-symbol indexes once per response, instead
  of rebuilding the full context map and scanning the universe for every requested
  symbol. For U universe rows, C contexts, and S requested symbols, expected
  lookup work is now O(U + C + S), previously O(S * (U + C)); this is a source-level
  complexity improvement, not a measured runtime claim. Only selected contexts
  are still cloned for deserialization.
- Preserved first matching universe row, last duplicate context coin, requested
  symbol before pair-name lookup, guarded positional fallback, rejection of
  malformed selected contexts without another fallback, request order, duplicate
  removal, and exact schema-error messages. The single-symbol wrapper now consumes
  the returned vector directly instead of draining it.
- Retained all ten existing tests and added three batch cases covering duplicate
  universe entries, alias/index collisions, invalid index types, noncanonical
  symbols, duplicate context coins, mixed keyed/unkeyed contexts, malformed and
  missing contexts, output order, and schema validation.
- Updated the charting guide. Request construction, shared-read caching,
  subscriptions, provider routing, persisted schema, and trading behavior are
  unchanged; no dependencies were added.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene api::`:
  **266 passed, 0 failed, 1 ignored**. This filter also matches other module
  paths containing `api::`, including provider and wallet database tests.
- Compared the perpetual fetch/parser bodies and the spot positional-match
  helper with their original bodies: unchanged.
- `cargo test --locked -j 2`: **4,305 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- New parser tests use synthetic JSON responses; no live provider credentials
  or market-data requests were required.

## 2026-09-27: borrow symbol metadata during parsing

- Reviewed symbol-fetch orchestration, cache provenance, perpetual/spot metadata
  parsers, DEX registry parsing, listings metadata parsers, and their tests.
  Kept the separate market-family failure policies, strict spot validation,
  and listings' inactive-market tracking. Symbol update dispatch and retained
  metadata handling were traced; the rest of symbol lifecycle still needs review.
- The perpetual annotation index now borrows coin keys and payloads from the
  response, avoiding a copy of every annotation subtree. Retained last-duplicate
  precedence, malformed-pair skipping, and default fields when annotations are
  missing or invalid. Symbol names remain borrowed until the owned output is built.
- Spot metadata deserializes directly from the borrowed JSON value instead of
  cloning the entire response first. Pair construction borrows base/quote token
  metadata; only the output ticker is cloned. Quote names and full names used to
  derive display labels and search keywords no longer need intermediate copies.
- Moved the eight perpetual parser tests to `perps/tests.rs` without changing
  their bodies. Added five tests for duplicate/malformed annotations, absent
  annotation arrays, token reuse across quotes, trimmed and case-preserving
  labels, Unicode keyword conversion, precision/asset identity, schema-error
  text, and atomic failures with previously loaded symbols. Covered USDC-base
  validation and skip ordering, including otherwise overflowing asset indices.
- Production changes affect ownership only. Asset-index calculations, collateral
  identity, leverage/precision defaults, validation order, partial-result policy,
  cache verification, request routing, and persisted formats are unchanged.

Validation (using the local ALSA prefix documented above):

- Added the new tests before changing production parser bodies and checked those
  bodies against HEAD. Fixed the new overflow fixture to use `u32::MAX` rather
  than an inferred `i32` literal; no production code was involved in that failure.
- `cargo test --locked -j 2 --package kerosene --bin kerosene
  api::exchange_symbols::`: **66 passed** both before and after the ownership
  changes, including all five new tests.
- Verified that all eight original perpetual parser test bodies were retained.
- `cargo test --locked -j 2`: **4,310 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- New tests use synthetic metadata only; no live provider credentials or
  network requests were required for them.

## 2026-09-27: separate symbol lifecycle responsibilities

- Reviewed all production methods and 30 inline tests in
  `market_update/symbols.rs`, context-result helpers/tests, symbol resolution and
  filter helpers, preset snapshot synchronization, and live-watchlist refresh
  scheduling. Retained family-specific recovery and selection rules.
- Reduced the 1,965-line symbol module to 108 lines of message dispatch and
  selection controls. Moved metadata refresh orchestration to `refresh.rs`,
  legacy spot migration to `migration.rs`, chart identity reconciliation to
  `charts.rs`, and search-context result handling into the existing `contexts.rs`.
  The 30 existing tests and eight fixtures/helpers now live in `tests.rs` with
  context, migration, and refresh test groups. Methods remain scoped to this
  feature; message routes and crate-facing refresh entry points are unchanged.
- Extracted chart reconciliation from the metadata success branch at the same
  point in the operation sequence. Preserved primary/secondary alias collision
  handling, quick-order/HUD resets, macro-cache clearing, request context and
  generation capture, task ordering, and persistence. Chart-key resolution now
  borrows both keys rather than cloning them.
- Shared preset/live-watchlist alias rewriting in `migrate_watchlist_symbols`.
  It moves replaced keys into the invalidation set, preserves first-occurrence
  order, and deduplicates only lists with a rewritten alias. Saved-layout preset
  snapshots, cache removal, request invalidation, persistence, and refetching stay
  at their original orchestration points.
- Partial search-context responses reuse the existing map and extend it with
  fresh values before filtering once to the requested scope. Removed the
  unconditional full-cache clone and redundant filtered-map allocation. Complete
  success still replaces the cache; errors and stale requests retain it.
- Outcome labels update directly while borrowing the symbol list, avoiding an
  intermediate vector and key copies for unchanged labels. Expired names remain
  cached, and configuration is saved only if a label changes.
- Added three tests for changed/unchanged preset and live-watchlist lists, saved
  snapshots, cache invalidation and idempotence; mixed partial context results;
  and expired/unchanged/changed outcome label persistence. Updated the component
  guide with the new module boundaries and existing response policies.

Validation (using the local ALSA prefix documented above):

- Compared all 48 original method/test/helper bodies after extraction, allowing
  visibility, whitespace, and rustfmt trailing commas: all match. The extracted
  chart block also matches its original sequence.
- `cargo test --locked -j 2 --package kerosene --bin kerosene
  market_update::symbols::`: **60 passed** after extraction/new tests and again
  after consolidation and copy removal. The final removal of redundant partial
  context filtering was then exercised by the full suite.
- Final source comparison confirms all 38 original test/helper bodies and seven
  otherwise unchanged lifecycle methods match; chart reconciliation differs only
  in borrowed key lookups. The three intentionally changed methods were reviewed
  separately.
- `cargo test --locked -j 2`: **4,313 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- New tests use synthetic in-memory state and unpolled tasks; no live provider
  credentials or network requests were needed for them.

## 2026-09-27: compute symbol-search ranking once per rebuild

- Reviewed symbol-search result generation, request planning, context-key
  selection, market filtering, DEX listing/ranking, volume lookup/formatting,
  and their tests. Traced the view's use of cached result indices and favourite
  counts. Kept refresh timing, request identity, pending refreshes, query
  matching, and market eligibility unchanged.
- Replaced repeated favourite-list scans inside the sort comparator with a
  borrowed key-to-position map and a rank recorded per matching symbol. Duplicate
  favourites keep their first saved position; duplicate matching rows still count
  individually and retain input order when their favourite ranks are equal.
- Relevance scores and finite volumes are calculated once per matching
  non-favourite row when the selected sort needs them. Comparisons reuse those
  values and share one ticker/key fallback. Alphabetical sorting does not compute
  relevance or volume, and favourites only use their saved order. All ranking
  data is local to one rebuild; no persistent cache or invalidation state was added.
- Exchange rank now borrows the DEX substring instead of allocating it per
  comparison. The DEX selector also deduplicates borrowed names before owning
  its output. Exact case, lexical exchange grouping, and primary-DEX ordering
  for same-ticker fallback are preserved.
- Removed the volume helper's redundant fallback: it retried the same lookup
  only when the ticker equaled the symbol key. Exact-pair lookup, missing-data
  behavior, finite negative/zero values, and non-finite rejection are unchanged.
- Added five ranking tests across all four modes, covering favourite precedence,
  first duplicate favourite, stable duplicate rows, missing outcome metadata,
  exact/prefix/substring/category/keyword matches, Unicode and untrimmed queries,
  exchange grouping, primary-DEX ties, missing/non-finite volumes, negative
  volumes, and signed-zero ties. Updated the component guide.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene
  market_state::symbol_search::`: **26 passed** both before and after the
  production changes, including all five new tests.
- Compared query matching and relevance scoring bodies with HEAD: unchanged.
- `cargo test --locked -j 2`: **4,318 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- Validation uses synthetic symbol metadata and contexts. The performance
  improvement is removal of repeated scans, calculations, and allocations;
  no measured runtime speedup is claimed.

## Next candidates

1. Comparison crosshair and background setup still have repeated drawing code.
   With the common frame context, evaluate whether sharing those sequences
   makes the modes clearer while preserving their formatting and range guards.
2. Continue reviewing the remaining API request and symbol-lifecycle modules and
   integration stream internals, including provider-specific socket commands and
   event parsing.
3. Continue across the unreviewed areas in the coverage table. Large files often
   include inline tests, so distinguish production complexity from file length.
