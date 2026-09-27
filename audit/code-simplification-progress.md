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
| Charting and canvas | Instance construction, persisted chart settings, annotation loading, and comparison settings reviewed. Comparison-chart rendering contexts, axes, background setup, and crosshair drawing reviewed and consolidated; mode-specific formatting and series calculations retained. Metadata-driven chart identity reconciliation reviewed and separated from symbol refresh orchestration. Most chart rendering and interactions remain. |
| Market data | Mid-price update visibility filtering reviewed; unnecessary catalog copies removed. Persistent API cache reviewed and split into candle policy, queued writes, and storage, with write coalescing simplified. Public shared reads and read admission inspected and retained. API exports, order-book reads, chart asset-context reads, watchlist context requests/parsing, and exchange statistics reviewed. Spot chart context lookup indexed once per response; differing parser and partial-result policies retained. Symbol metadata orchestration, perpetual/spot parsers, DEX registry parsing, and listings parsers reviewed; unnecessary metadata copies removed. Symbol refresh, legacy spot migration, label updates, and search context results reviewed and split; shared watchlist alias rewriting and removed intermediate copies. Symbol search planning, filtering, sorting, DEX listing/ranking, and volume lookup reviewed; ranking work moved out of comparisons. Live-watchlist and ticker-tape context completion reviewed and shared, retaining their distinct status/refresh policies. Watchlist history completion inspected; row-cache refresh now shares one borrowed metadata index per batch. Candle request/response policies and watchlist/outcome-volume history inspected; candle normalization deduplicates in place and trailing-run searches stop at the final gap. Outcome parsing, contract/template resolution, question membership, and label helpers reviewed; the temporary question index borrows shared records and expiry formatting is shared. Calendar, unstaking, and ETF API entry points/conversion helpers inspected. Remaining API requests, ETF flow parsing, books, and other widgets need review. |
| Wallets and account state | Wallet detail and cluster read-result/websocket filters reviewed. Account picker/setup routes traced; unreachable legacy credential-editing handlers removed. Active Add Account, connection, and switching safety boundaries inspected and retained. Account user-stream handling and risk scrubbing inspected for copies but unchanged. Broader account and portfolio flows remain. |
| Journal and analytics | Fill API pagination, identity, normalization, merging, and same-timestamp chain ordering reviewed; normalization deduplicates adjacent identities and avoids copying single-fill groups. Aggregation orchestration, position reconciliation, and journal view preparation reviewed. Identical non-perp classification and fee arithmetic now live in the journal domain. Note lookup/editing and account-scoped state reviewed; note lookup borrows entries and duplicate reset paths share one implementation. Cache/loaded-page callers, constructors, and parsing helpers inspected. Broader snapshot, cache, view rendering, and account analytics review remains. |
| Orders, signing, Chase, TWAP | Chase/TWAP market-subscription assembly reviewed and shared with order-book panes; lifecycle eligibility filters and event mappings retained. Order execution, signing, and automation state-machine review remains. |
| Config, persistence, secrets | Chart snapshot/config boundaries reviewed, schema unchanged. Remaining persistence/security code needs review. |
| Subscriptions and transport | Subscription assembly reviewed across market, user data, Hydromancer, Telegram, timer/input, and window families. Shared selected-provider book setup and reduced symbol copies; remaining eligibility/identity differences retained. Market adapters and user-data routing/dispatch inspected. Shared reconnect-before-notify behavior and snapshot timing, split Hydromancer adapters, and reduced owned payload copies. Native manager lifecycle/commands, both managers' subscription reference counts/coalescers, and Hydromancer registry/session state inspected; provider-specific lifecycle and routing retained. Remaining integration stream internals still need review. |
| Feeds, integrations, assistant | Calendar fetch/refresh, filters, summary, and row views reviewed; date parsing is shared within each view and cached during API sorting. Farside ETF flow parsing inspected. SEC API requests, submissions, structured earnings, document selection, and text summaries reviewed and split by responsibility; shared HTTP request/status handling and reduced summary text copies. Other integration and assistant internals still need substantive review. |
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

## 2026-09-27: share watchlist and ticker-tape context reconciliation

- Reviewed live-watchlist message handling, context/history completion, result
  status helpers/tests, row generation, and ticker-tape request/completion paths.
  Kept history timestamp guards, error presentation, exchange-stat snapshots,
  and forced/non-forced follow-up scheduling in their existing callers.
- Added `market_update/context_results.rs` for the identical response-scoping
  policy used by live watchlists and the ticker tape. Both callers still reject
  mismatched request IDs, requested symbol lists, and already-completed requests
  before invoking the helper.
- Current contexts outside the completed request survive. Complete responses
  clear omitted requested entries; partial responses preserve their old values.
  Incoming values must belong to both the original request and current scope.
  Still-relevant errors prune removed symbols without advancing freshness. An
  error for a wholly obsolete request still becomes successful completion and
  records its timestamp, matching the prior behavior.
- Successful reconciliation moves the retained cache into the returned response
  instead of cloning its keys and values into a new map. Error handling no longer
  builds an unused preserved-context map. The live-watchlist request-symbol set
  now consumes the message's vector rather than copying its strings.
- Kept symbol-search context handling separate: its scope and failure-cache
  policies differ. No timer, subscription, request-generation, status-prefix,
  persistence, or trading rules changed.
- Moved all 20 existing inline live-watchlist/ticker-tape tests into adjacent
  test modules. Added two shared integration tests exercising both message routes:
  22 scope/response cases and six stale/not-loading/mismatched-scope rejections.
  They check every context field, status isolation, timestamps, loading flags,
  request IDs, and pending/request-symbol cleanup. Updated the component guide.

Validation (using the local ALSA prefix documented above):

- Before consolidation, compared production bodies and moved tests with HEAD:
  unchanged except formatting in moved tests.
- `cargo test --locked -j 2 --package kerosene --bin kerosene market_update::`:
  **191 passed** before and after consolidation, including both new test matrices.
- Final source comparison verifies all 36 other production/test/helper function
  bodies match HEAD; only the two context-result handlers changed.
- `cargo test --locked -j 2`: **4,320 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- New tests use synthetic, in-memory context snapshots and unpolled tasks; no
  live provider requests or credentials were needed for them.

## 2026-09-27: share live-watchlist metadata lookup across panes

- Reviewed row-cache refresh, symbol labels, market visibility, live-mid alias
  resolution and freshness, and the callers of single/bulk row refresh.
- Both refresh entry points now use a shared helper that builds one borrowed
  metadata index for the affected panes. Previously, every pane rebuilt the
  entire index on each bulk refresh, including frequent mid-price updates.
  Empty batches and unknown single-pane IDs still return without indexing.
- The index exists only during the refresh, with last-duplicate-key precedence
  unchanged. Rows are still computed and applied one pane at a time; no catalog
  copies, persistent cache, or buffer of all replacement row caches is needed.
- Removed a context fallback that retried the same key lookup. Exact market
  scoping, current visibility, label fallbacks, mid-price resolution/freshness,
  history, funding, percentage calculations, and stable sorting are preserved.
- Updated existing row-generation tests to exercise the refresh entry point.
  Added two integration tests covering bulk/single parity, independent pane
  sorting, duplicate metadata and rows, missing prices, cached outcome labels,
  hidden/incomplete/fallback markets, all row values, metadata replacement,
  visibility changes, empty panes, and unknown IDs. Updated the component guide.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene
  market_state::live_watchlist::rows::`: **8 passed** before and after the
  production changes, including both new tests.
- Source comparison against HEAD: history/row calculations and sorting helpers
  are unchanged.
- `cargo test --locked -j 2`: **4,322 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- Validation uses synthetic metadata and in-memory state. The performance
  improvement removes repeated catalog indexing; no measured speedup is claimed.

## 2026-09-27: share comparison-chart background and crosshair drawing

- Reviewed normalized and pair-ratio drawing, the shared frame context and axes,
  crosshair call parameters, background helpers, and existing software-renderer
  coverage.
- Moved identical frame creation, transparent clearing, gradient, and dotted
  background setup into `SpaghettiCanvas::background_frame`. Each mode calls it
  after its own data/range checks and before its existing grid/series drawing.
- Replaced the two crosshair modules with `spaghetti/crosshair.rs`. It draws the
  reticle and labels once; callers supply an optional value label at the cursor's
  Y coordinate. Percentage precision and positive-range guard remain in normalized
  rendering; ratio formatting remains in pair-ratio rendering. Cursor guards,
  guide/style/scale settings, time calculation, text styling, and layer order are
  unchanged.
- Expanded the rendering matrix to cover independent dotted/gradient backgrounds.
  Added tests for cursor boundaries and all nine crosshair enum variants, with
  guides disabled/enabled and two scales across normalized, ratio-line, and
  ratio-candle modes. Synthetic preview helpers are shared between these tests.
- Updated the charting component guide. No series math, interaction, message,
  persistence, dependency, or account changes.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene spaghetti::`:
  **42 passed** before and after the production changes, including both new tests.
- All **297** before/after synthetic preview PNGs are byte-for-byte identical,
  verified by SHA-256. They cover the specified backgrounds, viewports, modes,
  styles, and cursor positions using tiny-skia. This is a software-renderer
  comparison, not a hardware GPU comparison. Previews are outside the repository
  under `/tmp/kerosene-comparison-draw.lW0FUY/{before,after}`.
- Source comparison against HEAD confirms the canvas program, both modes'
  series/range calculations and base drawing, and the ratio data/formatting/range
  helpers are unchanged.
- `cargo test --locked -j 2`: **4,324 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check`, `git diff --check`, and `cargo build --locked -j 2`:
  passed.
- Headless startup smoke: the built binary with `--test` opened the 1600x960
  Kerosene window under Xvfb, confirmed by `xwininfo`. Expected timeout exit 124
  after 20 seconds, no panic, and only the existing EGL/DRI3 warning. In-memory
  test mode kept personal configuration out of the run.

## 2026-09-27: simplify candle normalization and trailing-run searches

- Reviewed candle request orchestration, wire models, response parsing, interval
  rules, normalization/gap detection, watchlist/screener history, outcome-volume
  batching, and candle cache consumers. Provider fallback, credential scoping,
  error redaction, cache policy, gap filling, partial-result rules, and request
  concurrency remain separate and unchanged.
- `normalize_candles` now uses in-place deduplication after the same validity
  filter and stable sort. Swapping a duplicate into the retained slot preserves
  the last valid input's complete payload. The result reuses the input allocation
  instead of allocating a second candle vector; input capacity is retained.
- Both trailing-run helpers now search adjacent pairs backward and stop at the
  final discontinuity. Exact/tolerant thresholds, unknown-interval behavior,
  saturating arithmetic, and returned suffix indices are unchanged.
- Added an exhaustive test over 3,906 short input sequences, comparing every
  retained candle field against a timestamp-map oracle. It covers duplicate
  ordering, invalid updates, multiple groups, empty inputs, negative finite
  prices, and signed-zero volume. Added 12 trailing-run boundary cases including
  duplicate/backward timestamps, threshold edges, and `u64` limits.
- Updated the charting guide. No network, cache wire format, chart geometry,
  signing, order, or persistence changes.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene api::candles::`:
  **22 passed** before and after the production changes, including both new tests.
- Source comparison against HEAD confirms candle validation, gap thresholds,
  and all gap-detection predicates are unchanged.
- `cargo test --locked -j 2`: **4,326 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- The changes remove a result-vector allocation and scans before the final gap;
  no measured runtime speedup is claimed.

## 2026-09-27: borrow outcome question records during symbol parsing

- Reviewed outcome metadata validation, question parsing/membership, contract
  resolution, template parameter validation/interpolation, encoding, and the
  resulting symbol model and eligibility gates. Their validation, fallback,
  deadline, fee, label, and serialization rules remain unchanged.
- Outcome parsing now owns one temporary record per question and indexes borrowed
  references by outcome ID. Previously, each membership cloned the entire
  question record, including its rules and membership lists. Named, settled, and
  fallback associations retain their order and first-entry lookup precedence.
- The question constructor consumes raw metadata, moving names/descriptions,
  membership lists, and parsed fields into the record. Removed its unused
  `Clone` derive. The final `OutcomeSymbolInfo` still owns all of its data;
  neither the lookup nor references escape the parse call.
- Added a grouped-versus-isolated parsing regression test comparing complete
  symbols for seven outcomes (both sides), with two active parent questions,
  an unused question, repeated memberships, settled members, a fallback, and a
  standalone market. It also checks input order, parent fields, selectability,
  and settlement gates explicitly. Updated the component guide.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene
  api::exchange_symbols::outcomes::`: **27 passed** before and after the
  production changes, including the new regression test.
- Source comparison against HEAD confirms symbol construction, metadata
  validation, fee and quote helpers are unchanged except borrowed lookup access.
  Contract/template resolution and signing/order code were not modified.
- `cargo test --locked -j 2`: **4,327 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- Tests use synthetic and existing public metadata fixtures; no live requests
  or orders were sent. Removed redundant data copies without claiming a measured
  runtime speedup.

## 2026-09-27: share outcome expiry-label formatting

- Reviewed outcome market/side/venue/source labels, bucket and fallback wording,
  target-price formatting, expiry/countdown helpers, and their public call sites.
  Also inspected calendar and unstaking readers, ETF orchestration/HTTP helpers,
  THYP/BHYP conversions, and numeric helpers; their differing response and
  fallback policies remain separate. ETF flow parsing still needs review.
- Binary, legacy recurring, bucket, and fallback labels now use one optional
  expiry-suffix helper. Each caller retains its existing condition text,
  early-return rules, own-versus-question expiry source, and short-label flag.
- Countdown formatting reuses the already parsed timestamp instead of parsing
  the same expiry again. Shared remaining-time calculation keeps checked clock
  conversion, saturating subtraction, expired wording, and duration formatting.
  The intermediate string used only to append `left` is no longer needed.
- Added two public-label regression tests: 24 combinations of binary/bucket/
  fallback sides and expiry availability, plus invalid question-expiry precedence,
  unrepresentable clocks, pre-epoch dates, and legacy recurring labels. Existing
  expiry tests now assert the complete formatted countdown. Updated the guide.
- No contract verification, metadata parsing, order, message, persistence, or
  request behavior changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene outcome`:
  **194 passed** before and after the production changes, including both new
  regression tests.
- Reviewed the production diff: condition wording, source selection, date parse
  format, clock conversion, and remaining-time arithmetic are unchanged.
- `cargo test --locked -j 2`: **4,329 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-27: reuse parsed calendar dates

- Reviewed calendar HTTP handling, refresh/retry and stale-result guards, view
  filters, summary selection, status, controls, and row rendering. Fetch/error
  handling, refresh behavior, clocks, and rendering remain unchanged.
- API sorting now computes each timestamp key at most once instead of parsing
  both dates during every comparison. It retains whole-second precision,
  raw-date tie breaking, invalid-date ordering, and stable equal-key ordering.
  Temporary keys own copies of date strings; the event model is unchanged.
- Each view builds one temporary vector of borrowed events and parsed dates,
  shared by filtering and the next-event summary. The view retains full timestamp
  precision, inclusive time boundaries, local-date filtering, unknown-date
  visibility, and first-match summary ties. No persistent cache or invalidation
  is introduced.
- Added four regression tests covering all nine filter combinations, invalid and
  empty dates, empty/single lists, time cutoffs, timezone offsets, fractional
  seconds, duplicate ordering, summary eligibility, and retained event fields.
  The tests passed against the original implementation before the refactor.
- Updated the integrations guide. Also inspected Farside flow extraction,
  derivation, and date parsing; those helpers are unchanged.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene calendar`:
  **18 passed** before and after the production changes, including all four new
  regression tests.
- Reviewed the production diff: filtering, summary selection, UI composition,
  request/error handling, and refresh/retry policies are unchanged.
- `cargo test --locked -j 2`: **4,333 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- No measured runtime speedup is claimed; this change removes repeated date
  parsing while keeping temporary storage local to sorting/rendering.

## 2026-09-27: separate SEC reader responsibilities

- Reviewed the SEC API's HTTP requests, ticker/submission parsing, structured
  earnings selection/formatting, archive URLs, document selection, HTML/text
  conversion, summary ranking, public exports, and existing tests. Also reviewed
  Farside ETF flow extraction and its tests; those parsing rules remain separate.
- Split the 1,629-line SEC module into a 124-line entry point and five focused
  children: `http`, `submissions`, `earnings`, `documents`, and `summary`. Wire
  models stay beside their readers, while public request/result types and
  filing-summary orchestration remain at the entry point. Existing API exports
  are preserved, and internal helpers only expose what their parent needs.
- Both SEC response readers now call one request/status helper, retaining the
  shared client, configured user agent, observed GET, endpoint labels, request
  errors, and status checks before body decoding. JSON/text decoding and their
  distinct errors remain in the wrappers.
- Moved all 12 existing tests beside the modules they cover. Added two loopback
  HTTP regression tests covering method/headers, JSON and Unicode text success,
  three failure statuses with incomplete bodies, malformed JSON, incomplete text,
  and request-construction failure. The tests passed before the production change.
  No live EDGAR/ETF requests or credentials are needed for these tests.
- Updated the integrations guide with the module map. No chart request/cache,
  message, persistence, dependency, or trading behavior changes.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene api::sec::`:
  **14 passed** before and after the production changes, including both new
  regression tests.
- Source comparison confirms 49 production function bodies are byte-identical;
  the remaining two wrappers share a request/status block that is byte-identical
  to both originals. All 9 constants and 17 model definitions/derives are unchanged
  apart from internal visibility. All 15 existing test/helper bodies match modulo
  formatting.
- `cargo test --locked -j 2`: **4,335 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-27: reduce repeated SEC summary text processing

- Reviewed headline/highlight extraction, candidate boundaries, scoring,
  boilerplate rejection, and duplicate detection. Also inspected user-fill
  requests/pagination, fill normalization/identity, and order-status parsing,
  matching, redaction, and terminal-status predicates. Those account/trading
  readers and their differing validation policies are unchanged.
- Summary searches now share one ASCII-lowercased copy of the full filing,
  previously copied once for the headline and once per highlight category.
  ASCII case folding retains the byte offsets used to select original text.
- Candidate windows borrow their text until normalization rather than allocating
  an intermediate string. Boundary scans, decimal handling, UTF-8 adjustment,
  normalization, output limits, and selected original casing are preserved.
- Duplicate detection now compares the normalized character iterators directly.
  The first 80 ASCII alphanumerics, case folding, empty matches, candidate order,
  and fallback exclusion rules remain the same. This also removes the fallback's
  temporary copy of the excluded headline.
- Added three regression tests: six complete summary outputs, decimal/Unicode
  window cases, and eight duplicate-prefix cases. They cover empty input,
  category order and the five-highlight limit, equal-score first matches,
  boilerplate exclusion, cross-category deduplication, fallback exclusion,
  punctuation/case handling, non-ASCII text, and the 80-character boundary.
- Updated the integrations guide. No request, event, persistence, chart-cache,
  metric calculation, or trading behavior changes.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene
  api::sec::summary::`: **6 passed** against the original implementation,
  including all three new regression tests.
- `cargo test --locked -j 2 --package kerosene --bin kerosene api::sec::`:
  **17 passed** after the refactor, including all six summary tests.
- Source comparison confirms 12 existing helper bodies are byte-identical,
  including HTML conversion, normalization, boundaries, trimming, relevance,
  scoring, and boilerplate detection. The production diff retains the same
  keywords, selection order, score comparisons, and character transforms.
- `cargo test --locked -j 2`: **4,338 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- Removed repeated allocations/copies without claiming a measured runtime speedup.

## 2026-09-27: simplify fill normalization and group processing

- Reviewed journal fill identity, comparison, normalization, merge counts,
  position-chain grouping/selection, callers in loading/cache/aggregation, and
  existing aggregation regressions. Also inspected trade constructors and
  parsing/ID helpers; their differing initialization and attribution rules remain
  explicit and unchanged.
- Normalization now deduplicates adjacent fills after the same stable identity
  sort. The comparison covers all eight identity fields, preserving the first
  complete payload while removing a temporary hash set and owned identity copies.
  Tie-breaking comparisons run only when preceding fields are equal.
- Replaced the manual group scan and whole-vector copy with mutable time/coin
  chunks. Single-fill groups stay in place; multi-fill groups use the existing
  chain selector and move its results back into that group. The input vector and
  its capacity are retained. Multi-fill chain selection still owns a temporary
  result, keeping its established traversal and fallback behavior unchanged.
- Added three regression tests covering every identity field, differences in
  all non-identity payload fields, string-based price/size distinctions, the
  first surviving duplicate, complete payload preservation, time/coin boundaries,
  disconnected chains, cycles, invalid position values, settlements, empty and
  single inputs, idempotence, and existing/incoming merge-count precedence.
  All use synthetic fills and compare complete serialized payloads.
- Updated the journal component guide. No API request, persistence format,
  account scope, trade calculation, signing, or order-placement changes.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene journal::`:
  **73 passed** before and after the production change, including all three new
  regression tests and existing trade reconstruction/cache tests.
- Source comparison confirms the identity types/conversion, merge implementation,
  and full position-chain selection implementation are byte-identical. Only
  comparison evaluation, deduplication, and applying groups changed.
- `cargo test --locked -j 2`: **4,341 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- Removed redundant allocations and copies; no measured runtime speedup is claimed.

## 2026-09-27: centralize journal classification and fee arithmetic

- Reviewed aggregation orchestration, current-position reconciliation, journal
  filters, analytics, list/detail preparation, chart series/outcome admission,
  and Assistant journal row/ranking call sites. Kept differing settlement, flip,
  partial-history, reconciliation, scoring, and display rules explicit.
- Moved the identical non-perp predicate from aggregation, snapshot admission,
  and analytics to `journal::is_non_perp_coin`. The perp filter uses its negation.
  Prefix checks and named-pair detection retain their exact matching behavior;
  spot/outcome filters retain their individual rules and possible overlap.
- `AggregatedTrade::effective_pnl` now owns optional fee subtraction, replacing
  copies in analytics, trade rows/details/sorting, cumulative chart series,
  outcome tiles, and Assistant journal exports/ranking. Callers retain their
  own fee flag, eligibility, non-finite handling, accumulation and ordering.
- Preserved an existing eligibility difference: KPI scoring excludes named
  spot pairs, whereas the outcome strip excludes only `@`/`#` prefixes. Its
  selector remains separate and a regression test records that behavior.
- Expanded classification/filter cases and added three regression tests for
  complete KPI totals versus scored trades, raw fee arithmetic (rebates, signed
  zero, infinities, NaNs), and named-pair outcome-strip eligibility. Direct rule
  tests now live beside their journal owners. Updated the component guide.
- No metric formula, trade reconstruction, snapshot geometry, persistence,
  Assistant redaction, API request, or order behavior changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene journal`:
  **135 passed** before and after the production change, including all three new
  regression tests, snapshot admission, view construction, sorting, and exports.
- Source comparison confirms aggregation/reconciliation bodies and outcome
  eligibility/rendering are unchanged. The shared fee formula matches the
  original arithmetic; Assistant row/selection bodies differ only by replacing
  the duplicate formula/helper calls.
- `cargo test --locked -j 2`: **4,344 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-27: borrow journal notes and share account reset

- Traced note lookup, the reflection editor, edit/save/cancel handling, Assistant
  read-only callers, account switching, and journal reset callers. Left note
  migration, persistence, draft mutation, and account-switch ownership intact.
- Replaced the owned-key-only helper with `note_entry_for_trade`, which borrows
  the matching map key and note together. `note_for_trade` no longer allocates a
  key or hashes the selected key again. The display resolves its note once and
  copies the selected key only when constructing the edit message. The editor
  also borrows raw tag input instead of copying it before widget construction.
- Consolidated identical account-data reset bodies: the address-specific variant
  runs the common reset and then sets its address. Existing snapshot invalidation
  policies, saved notes, preferences, and request generations are unchanged.
- Added two note-lookup regressions covering empty current-ID precedence, ordered
  aliases, exact case, missing/duplicate aliases, and borrowed note identity.
  Expanded the reset test to cover both reset variants, loaded history, drafts,
  selection, status, and retained notes/preferences. Updated the component guide.
- No message, persistence format, trade calculation, API request, or order changes.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene journal`:
  **137 passed** before and after the production change, including both new
  note-lookup cases, the expanded reset case, and existing view/account tests.
- Source comparison confirms account switching, persistence snapshots, the
  no-address reset, and all snapshot-clearing policies are unchanged.
- `cargo test --locked -j 2`: **4,346 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- Removed redundant copies and lookups; no measured runtime speedup is claimed.

## Next candidates

1. Continue through journal snapshot request planning, metrics, cache, and view
   rendering, including the large cockpit renderer. Initial inspection found
   repeated historical-trade admission checks and request-padding calculations
   in snapshot planning; inspect tests and error precedence before consolidating.
   Account state and persistence
   snapshots retain intentional copies; avoid a broad ownership rewrite without
   a demonstrated benefit. Farside's repeated chart-marker lookup remains a
   smaller candidate.
2. Continue reviewing the remaining API request and symbol-lifecycle modules and
   integration stream internals, including provider-specific socket commands and
   event parsing.
3. Continue across the unreviewed areas in the coverage table. Large files often
   include inline tests, so distinguish production complexity from file length.
