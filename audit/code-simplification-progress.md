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
| Market data | Mid-price update visibility filtering reviewed; unnecessary catalog copies removed. Persistent API cache reviewed and split into candle policy, queued writes, and storage, with write coalescing simplified. Public shared reads and read admission inspected and retained. API exports, order-book reads, chart asset-context reads, watchlist context requests/parsing, and exchange statistics reviewed. Spot chart context lookup indexed once per response; differing parser and partial-result policies retained. Symbol metadata orchestration, perpetual/spot parsers, DEX registry parsing, and listings parsers reviewed; unnecessary metadata copies removed. Symbol refresh, legacy spot migration, label updates, and search context results reviewed and split; shared watchlist alias rewriting and removed intermediate copies. Symbol search planning, filtering, sorting, DEX listing/ranking, and volume lookup reviewed; ranking work moved out of comparisons. Live-watchlist and ticker-tape context completion reviewed and shared, retaining their distinct status/refresh policies. Watchlist history completion inspected; row-cache refresh now shares one borrowed metadata index per batch. Candle request/response policies and watchlist/outcome-volume history inspected; candle normalization deduplicates in place and trailing-run searches stop at the final gap. Outcome parsing, contract/template resolution, question membership, and label helpers reviewed; the temporary question index borrows shared records and expiry formatting is shared. Calendar, unstaking, and ETF API entry points/conversion helpers inspected. Order-status request/parsing and outcome-volume batching/cancellation reviewed; identity checks, concurrency, and partial-failure behavior retained. Remaining API requests, books, and other widgets need review. |
| Wallets and account state | Wallet detail and cluster read-result/websocket filters reviewed. Account picker/setup routes traced; unreachable legacy credential-editing handlers removed. Active Add Account, connection, and switching safety boundaries inspected and retained. Account user-stream handling and risk scrubbing inspected for copies but unchanged. Account refresh admission, rate-limit retries, and reconciliation flow inspected; the original fills snapshot is retained across terminal mutations. Account bootstrap, wallet detail/snapshot/order-count reads, and analytics HTTP ownership reviewed; helpers now share borrowed clients without changing request policy. Other account and portfolio flows remain. |
| Journal and analytics | Fill API pagination, identity, normalization, merging, and same-timestamp chain ordering reviewed; normalization deduplicates adjacent identities and avoids copying single-fill groups. Aggregation orchestration, position reconciliation, and journal view preparation reviewed. Identical non-perp classification and fee arithmetic now live in the journal domain. Note lookup/editing and account-scoped state reviewed; note lookup borrows entries and duplicate reset paths share one implementation. Snapshot models, planning, assembly, and metrics reviewed and separated; request bounds and history admission are shared. Snapshot update callers inspected, with freshness/admission policies retained. Journal cache persistence and tests reviewed; platform-specific replacement retained. Cockpit rendering/analytics reviewed and split by panel; per-asset aggregation copies coin names only for distinct output rows. Detail/chrome/list views, summary preparation/series/drawing, and small trade-card helpers reviewed; simplified series iteration and reused detail values. Snapshot canvas interaction/rendering reviewed and separated; the canvas borrows its snapshot. Account analytics HTTP fan-out, reserve/name/history parsing, income assembly, and portfolio data selection reviewed; token validation is centralized, recent-payment formatting is bounded to 12 valid rows, portfolio bucket construction is direct, and unused theme construction is removed. Portfolio/income panes, table variants, projection generation, chart layout/hover/tooltip, and PnL area rendering reviewed; daily histories and income labels now borrow data, hidden-chip preparation is skipped, and common table cells/status wrapping are shared. PnL-card state/metrics, privacy text, preview/export rendering, contrast, and output paths reviewed; digit masking is shared and position percentages are reused. Owned export snapshots and account binding remain intact. Account metric helpers reviewed and retained; portfolio/income refresh lifecycle now has one shared implementation with independent state per feature, while caller admission and result policies remain explicit. |
| Orders, signing, Chase, TWAP | Chase/TWAP market-subscription assembly reviewed and shared with order-book panes; lifecycle eligibility filters and event mappings retained. Removed discarded theme constructions from order/Chase entry points after checking theme purity; request and lifecycle code is otherwise byte-identical. Substantive order execution, signing, and automation state-machine review remains. |
| Config, persistence, secrets | Chart snapshot/config boundaries reviewed, schema unchanged. Proxy URL normalization, redacted labels, deserialization/revalidation, settings commit order, and startup fallback inspected; existing security and persistence behavior retained. Remaining persistence/security code needs review. |
| Subscriptions and transport | Subscription assembly reviewed across market, user data, Hydromancer, Telegram, timer/input, and window families. Shared selected-provider book setup and reduced symbol copies; remaining eligibility/identity differences retained. Market adapters and user-data routing/dispatch inspected. Shared reconnect-before-notify behavior and snapshot timing, split Hydromancer adapters, and reduced owned payload copies. Native manager lifecycle/commands, both managers' subscription reference counts/coalescers, and Hydromancer registry/session state inspected; provider-specific lifecycle and routing retained. Proxy transport/admission reviewed and retained. Hydromancer frame/control parsing now moves resume strings out of JSON and borrows error text. Both API probes now share telemetry state/update code with independent provider state; snapshot fields and atomic ordering remain unchanged. Hydromancer connection/retry/idle-wait handling reviewed; connect errors and timeouts share one retry path. Fill tuple parsing borrows addresses and feed subscriptions move their final topic use; event formats, dedupe policy, and command cancellation are retained. Liquidation/tracked-trade subscription, receive, dedupe, recovery, and cleanup now share one handler; feature payloads/parsers and separate history limits remain explicit. Remaining integration stream internals still need review. |
| Feeds, integrations, assistant | Calendar fetch/refresh, filters, summary, and row views reviewed; date parsing is shared within each view and cached during API sorting. Farside ETF flow parsing reviewed; data and label extraction now share one chart marker lookup. SEC API requests, submissions, structured earnings, document selection, and text summaries reviewed and split by responsibility; shared HTTP request/status handling and reduced summary text copies. Shared-client request construction inspected across API, Hydromancer, HyperDash, and OpenRouter paths; unnecessary temporary client copies removed. Other integration and assistant internals still need substantive review. |
| Views, settings, commands, app shell | Architecture mapped; default live-watchlist Add Widget creation reviewed and shared with restoration. Account/layout-picker selection styles reviewed and consolidated. Theme construction and discarded theme calls in root update, notifications, chart helpers, and order helpers reviewed; the unused calls are removed. Notification delivery, sound entry points, toast retention/animation, and order-status alert policy reviewed; repeated delivery now shares one helper and copies messages only for desktop notifications. Substantive review of other surfaces remains. |
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

## 2026-09-27: separate snapshot planning and share request bounds

- Reviewed all snapshot models, request planning, assembly, marker generation,
  candle/price metrics, and tests, plus update callers' fill/live selection,
  request admission, coverage/timeframe changes, and stale-result checks.
- Split the 916-line snapshot module into a 245-line model/assembly entry point,
  `snapshot/requests.rs`, and `snapshot/metrics.rs`. Tests now live beside their
  owners, with common synthetic fixtures in `snapshot/tests.rs`. Existing public
  entry points and all model definitions remain unchanged.
- Shared complete-history admission between automatic and pinned requests.
  Shared saturating request bounds between initial and retry requests. Retries
  now name only the changed timeframe/index/range and copy their remaining
  context with struct update syntax.
- Retained separate live-position admission and lookback policies. Automatic
  rung selection still budgets both padding sides even for open positions;
  actual open requests stop at the reference time. Pinned fine-timeframe live
  requests retain their independent lookback cap and validation order.
- Added four regression tests for error precedence, explicit bounds near zero
  and `u64::MAX`, reversed trade endpoints, open-request padding/rung selection,
  retry equality across every rung/coverage with preserved provider generations,
  terminal/invalid rung handling, and capped versus automatic live lookback.
- Metric calculations, fill VWAP, candle/marker order, snapshot assembly,
  redaction, update routes, provider/account checks, and persistence are unchanged.
  Updated the journal component map and request-policy documentation.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene journal`:
  **141 passed** before and after the production change, including all four new
  tests and existing snapshot update/view tests.
- Before consolidation, source comparison verified all 19 production function
  bodies survived the split unchanged. Final comparison verifies all six models
  and 15 unchanged bodies; only the four planned request functions differ.
  All 15 snapshot tests were preserved through relocation.
- `cargo test --locked -j 2`: **4,350 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-27: separate cockpit panels and borrow asset grouping keys

- Reviewed all cockpit rendering and analytics, snapshot metric traversal, and
  journal cache persistence/tests. Compared journal replacement behavior with
  config replacement: platform gates, file eligibility, sync errors, cleanup,
  and filename conventions differ, so retained their separate implementations.
- Kept snapshot metrics' collected overlapping references and ordered passes:
  they make entry/exit error precedence and drawdown order explicit. Replacing
  them with an accumulator or repeated filtered scans offers no demonstrated
  benefit sufficient to justify the extra complexity here.
- Split the 812-line cockpit into a 204-line orchestration/chrome module and
  focused `cockpit/{bars,heatmap,tiles,win_loss}.rs` modules. Each panel keeps its
  own helpers and constants. The win/loss panel now uses a free view function
  because it never reads terminal state; its resulting widgets own their data.
- Per-asset analytics groups trades by borrowed `&str` keys and copies each
  distinct coin once into the owned output rows. It retains exact key matching,
  trade iteration/fee arithmetic, sorting, tie-breaking, and non-finite values.
- Added two regression tests for repeated/exact-case/empty coin keys, fee modes,
  rebates, tied totals, open/incomplete trades, accumulation-order sensitivity,
  empty input, infinities, and NaNs. Updated the component guide's view map.
- Window selection, all-time KPI scope, panel order, colors, canvas geometry,
  dimensions, denomination formatting, messages, cache behavior, and snapshot
  metrics are unchanged.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene journal`:
  **143 passed** before and after the production change, including both new
  analytics tests and existing cockpit/detail/editor construction tests.
- Source comparison verifies all 20 cockpit functions are retained. Nineteen
  bodies are unchanged after removing indentation; the orchestration body only
  changes the stateless win/loss call from a method to a free function. All
  constants and canvas drawing bodies are unchanged.
- `cargo test --locked -j 2`: **4,352 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check`, `git diff --check`, and `cargo build --locked -j 2`:
  passed.
- Headless startup smoke: the built binary with `--test` opened the 1600x960
  Kerosene window under Xvfb, confirmed by `xwininfo`. Expected timeout exit 124
  after 20 seconds and no panic markers. In-memory test mode kept personal
  configuration out of the run. This checks startup, while focused view tests
  construct the journal panels. Logs: `/tmp/kerosene-cockpit-smoke.hoSyfN`.
- Removed per-trade coin-name copies; no measured runtime speedup is claimed.

## 2026-09-27: simplify journal series traversal and view preparation

- Reviewed detail/chrome/list views, account-label and display-name selection,
  summary series preparation/drawing/tooltip code, account-value clipping, and
  small trade-card helpers. Left differing labels, number formatting, clipping,
  source eligibility, and geometry policies intact.
- Cumulative PnL now walks sorted trades once and uses its existing last-point
  update to coalesce equal timestamps. Removed nested timestamp/index tracking
  while retaining the exact per-trade addition order, including across groups.
- Latest-value subtraction consumes a peekable iterator through each target
  timestamp, replacing a separate index and repeated indexed reads. Stable
  duplicate precedence, carry-forward behavior, and finite-input filtering stay
  the same.
- Summary views now compute fill-based PnL only when portfolio-margin history
  is unavailable, avoiding an unused allocation, sort, and accumulation pass.
  Portfolio history eligibility and source priority are unchanged.
- Detail views reuse their display coin, snapshot lookup, and R-multiple. Loaded
  snapshot admission is applied once to the shared borrowed option, and formatted
  R text moves into the widget without a redundant clone.
- Added three regressions for stable ties and rounding-sensitive accumulation,
  epoch/baseline collisions, effective-fee filtering, open timestamps, empty and
  non-finite input, cumulative overflow, duplicate subtraction samples, and
  carry-forward boundaries. Updated the journal component guide.
- No order, persistence, account selection, request, message, style, geometry,
  leading-baseline, or portfolio-window behavior changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene journal`:
  **146 passed** before and after the production change, including all three new
  series tests and existing view/snapshot/account tests.
- Source comparison confirms chart drawing/layout, account-value preparation,
  leading-zero baselines, portfolio-source selection, and window helpers are
  unchanged.
- `cargo test --locked -j 2`: **4,355 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check`, `git diff --check`, and `cargo build --locked -j 2`:
  passed.
- Headless startup smoke: the built binary with `--test` opened the 1600x960
  Kerosene window under Xvfb, confirmed by `xwininfo`. Expected timeout exit 124
  after 20 seconds, with no panic markers. In-memory test mode kept personal
  configuration out of the run. Logs: `/tmp/kerosene-series-smoke.aoUo2k`.
- Removed unnecessary preparation and bookkeeping; no measured runtime speedup
  is claimed.

## 2026-09-27: separate snapshot canvas responsibilities and borrow loaded data

- Reviewed the complete snapshot view/canvas: loading/unavailable precedence,
  metric formatting, reset identity, zoom/pan/drag handling, viewport clamping,
  visible-data/scale selection, live-position guides, candles, and marker groups.
- Split approximately 1,006 production lines into a 204-line status/metrics view
  and focused `snapshot/{canvas,interaction,plot,drawing,markers}.rs` modules.
  Interaction state keeps its fields private and exposes only the drag-state
  query needed by the canvas cursor. Tests live beside interaction and markers.
- The loaded view and canvas now borrow `JournalTradeSnapshot` for the view's
  lifetime. This removes two full snapshot clones per loaded-view construction,
  including the candle and marker vectors. Checked the installed iced trait and
  widget lifetime bounds; its separately owned canvas state remains `'static`.
- Added three regressions before changing production code: exact reset-key
  fields/exclusions, anchored zoom/pan/reset/cursor-exit ranges and wheel capture
  outside the plot, and adjacent marker chains/strict distance boundary with
  separate buy/sell groups. Existing tests remain intact after relocation.
- Preserved all view/canvas constants, saturation/rounding order, overscroll and
  minimum-context rules, reset-key formatting, selection order, price fallback,
  candle geometry, guide labels, marker sums/radii, colors, and event capture.
  Updated the component map and canvas ownership documentation.
- No persisted schema, provider request, message route, account logic, or order
  behavior changed. Visible-data collection is unchanged.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene journal`:
  **149 passed** before and after the production change, including all three new
  interaction/grouping regressions and existing view/snapshot tests.
- Source comparison verifies 36 production bodies are unchanged, along with all
  15 constants and the fields of four state/geometry models (allowing visibility
  changes within the snapshot module). Five view/delegation
  bodies change only ownership or the drag accessor; all seven canvas tests are
  retained.
- `cargo test --locked -j 2`: **4,358 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check`, `git diff --check`, and `cargo build --locked -j 2`:
  passed.
- Headless startup smoke: the built binary with `--test` opened the 1600x960
  Kerosene window under Xvfb, confirmed by `xwininfo`. Expected timeout exit 124
  after 20 seconds, with no panic markers. In-memory test mode kept personal
  configuration out of the run. This checks startup; the focused tests exercise
  snapshot interactions and marker layouts. Logs:
  `/tmp/kerosene-snapshot-canvas-smoke.5XqVO7`.
- Removed two full snapshot copies; no measured runtime speedup is claimed.

## 2026-09-27: simplify income assembly and portfolio data preparation

- Reviewed account analytics HTTP helpers, required/optional income endpoint
  handling, reserve and spot-name parsers, daily interest deduplication, snapshot
  assembly, portfolio history parsing, and portfolio data selection. Retained
  request fan-out, response-body timing, endpoint error precedence/redaction,
  parser fallback priority, and the distinct daily/hourly validation policies.
- Income token-row construction now owns numeric validation and projection
  arithmetic. Snapshot assembly counts a rejected row in one place and sums
  accepted row fields in the original input order. Missing reserves remain
  excluded from the invalid-row count; finite signed/zero values stay accepted.
- Recent payments sort borrowed hourly entries, then validate and format the
  first 12 valid rows. Stable timestamp ties, aggregate exclusion, numeric-token
  precedence, unknown labels, and invalid-rate defaults are preserved. Token
  IDs are parsed once; discarded older rows no longer allocate labels or look
  up rates.
- Portfolio buckets now take parsed history vectors directly and parse volume
  once, retaining the missing-versus-invalid distinction. Removed a pass-through
  numeric helper; history parsing still preserves input order and counts only
  invalid values with admissible timestamps.
- Removed six unused theme constructions from portfolio data helpers after
  inspecting theme creation. Bucket priority, empty-bucket handling, time-window
  selection, and historical performance calculations are unchanged.
- Added six regressions before editing production code: token parse failures and
  arithmetic overflow/counting, signed values and stable projection ties, recent
  valid-payment limits/ties, label/rate fallback precedence, volume presence, and
  history ordering/error counts. Updated the account component guide.
- No requests, schema, message routes, order behavior, rendering, or persisted
  state changed. No measured runtime speedup is claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene account_analytics`:
  **28 passed** before and after the production changes, including all six new
  regression tests and the existing endpoint/error-redaction tests.
- `cargo test --locked -j 2 --package kerosene --bin kerosene portfolio_state`:
  **36 passed**.
- Source comparison confirms portfolio data selection differs only by the
  removal of six unused theme calls.
- `cargo test --locked -j 2`: **4,364 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- No GUI smoke run for this batch: drawing, interaction, windows, and startup
  were not changed.

## 2026-09-27: simplify portfolio and income view preparation

- Reviewed portfolio/income panes, headers, daily rows, totals, tabs/status,
  projection generation, compact/wide tables, chart layout/hover/tooltip, and PnL
  area rendering. Kept the distinct raw-token versus denominated-value formats,
  daily calculation policies, chart scale/gradient rules, and pane controls.
- Portfolio daily rows borrow the selected bucket histories instead of copying
  them before calling read-only helpers. Missing history still yields no rows;
  bucket selection and the seven-day display limit are unchanged.
- Portfolio percent mode no longer builds a second performance series for a
  hidden chip. The hero takes a single optional chip value, removing a redundant
  visibility flag. Dollar mode retains its existing performance calculation.
- Income chart layout borrows labels from its projection data. Only visible axis
  labels and the hovered tooltip allocate canvas text; geometry, hover behavior,
  tooltip sizing, and canvas state are unchanged. Existing tests now keep their
  input fixtures alive for the borrowed layout.
- Income status branches share one outer container. Unavailable, initial
  loading, loaded/stale, and empty-state precedence and padding are unchanged.
- Compact/wide income tables construct shared cells once and keep each column
  order explicit. Removed the unused denomination argument from payment rows,
  whose amounts remain in raw token units. Width portions, labels, colors,
  fonts, spacing, breakpoints, and amount formatting are retained.
- Updated the account component guide. No calculations, network requests,
  persisted schema, messages, subscriptions, or trading behavior changed.
- Removed unused preparation and copies; no measured runtime speedup is claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene portfolio`:
  **94 passed** before and after the changes.
- `cargo test --locked -j 2 --package kerosene --bin kerosene income`:
  **32 passed** before and after the changes.
- Source comparison confirms income drawing, layout math, hover selection, and
  tooltip math differ only in label ownership and corresponding type signatures.
  Reviewed both table variants for matching cells/order and style arguments.
- `cargo test --locked -j 2`: **4,364 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo check --locked -j 2`, `cargo build --locked -j 2`,
  `cargo fmt -- --check`, and `git diff --check`: passed.
- Headless startup smoke: the built binary with `--test` opened the 1600x960
  Kerosene window under Xvfb, confirmed by `xwininfo`. Expected timeout exit 124
  after 20 seconds, with no panic markers. In-memory test mode kept personal
  configuration out of the run. This validates startup; the existing focused
  tests cover chart geometry, hover behavior, and history calculations. Logs:
  `/tmp/kerosene-portfolio-view-smoke.vGSjE3`.

## 2026-09-28: share PnL-card masking and remove discarded theme construction

- Reviewed PnL-card state, metrics and fallbacks, render-text/privacy rules,
  preview and shared canvas rendering, contrast, export requests/output, and
  window controls. Kept owned async export snapshots, account-binding checks,
  unknown-total refusal, privacy defaults, font timing, and rendering intact.
- Whole-price and fractional-price masking now share one ASCII-digit walk.
  Whole-price rules reuse the already computed digit count. Preserved sign and
  whitespace handling, ASCII-only masking, separators/non-digit characters,
  significant-digit selection, and existing byte-length behavior for fractions.
- Reused the position's asset-move percentage for leveraged return and removed
  a pass-through number parser. Pricing fallback and all arithmetic are unchanged.
- Added a regression before production changes covering empty/whitespace values,
  signed prices, separators, non-ASCII characters, short fractions, mixed digit
  and non-digit fractions, and privacy-disabled output.
- Audited `theme()`/`get_theme_by_name`, palette helpers, transparency adjustment,
  and iced's custom-theme constructor. They construct immutable palette data;
  discarded calls do not synchronize state or perform I/O.
- Removed 24 unused `let _theme = self.theme()` calls across 18 files: root
  message dispatch, notifications, chart allocation/overlays/heatmap, and order,
  Chase, fee, and symbol helpers. This includes a palette construction on every
  message and repeated nested constructions during chart/order processing.
- Source comparison confirms every other byte in those 18 files is unchanged.
  This is a scoped removal of unused work, not a completed review of order
  execution or chart lifecycle behavior. No request, signing, account, freshness,
  retry, notification, or routing policy changed.
- Updated the PnL-card and theme component guides. No schema, dependencies,
  assets, message variants, or subscriptions changed. No measured runtime
  speedup is claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene pnl_card`:
  **32 passed** before and after production changes, including the new privacy
  regression, account binding, contrast, render text, and offscreen PNG tests.
- Focused targets using the same test command: `order_execution` **401 passed**,
  `app_update::routing` **15 passed**, `chart_state` **73 passed**, and
  `notification_state` **5 passed**.
- Source comparison confirms exactly 24 call deletions across 18 files, with
  every other byte unchanged; no discarded theme bindings remain.
- `cargo test --locked -j 2`: **4,365 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check`, `git diff --check`, and `cargo build --locked -j 2`:
  passed.
- Headless startup smoke: the built binary with `--test` opened the 1600x960
  Kerosene window under Xvfb, confirmed by `xwininfo`. Expected timeout exit 124
  after 20 seconds, with no panic markers. In-memory test mode kept personal
  configuration out of the run. Logs: `/tmp/kerosene-pnl-theme-smoke.Sr431z`.

## 2026-09-28: consolidate portfolio and income refresh lifecycle

- Reviewed account metric helpers and tests; retained their small implementations,
  liquidation-field precedence, finite/positive parsing, and funding sign rule.
  Inspected notification delivery, toast animation/retention, and portfolio/income
  completion handling. Notification consolidation remains a next candidate.
- Portfolio and income had identical implementations of request generation,
  completion admission, queued follow-up consumption, and invalidation. Moved
  these five methods and their three fields into `AnalyticsRefreshState` in
  `portfolio_state/refresh.rs`; each feature owns an independent instance.
- Updated refresh dispatch/completion, account/provider/key invalidation, loading
  indicators, agent snapshot reads, and their tests to use the embedded state.
  No forwarding methods or generic state wrappers were added.
- Preserved default state, saturating counter increments, generation-only
  completion admission, stale-result rejection, retained follow-up flags across
  begin/finish, and explicit clearing on invalidation. Account/PM eligibility,
  response error handling/redaction, income alerts, and follow-up dispatch remain
  in their existing callers.
- Added two lifecycle regressions before production changes, exercising both old
  implementations and both new feature defaults: stale completion, duplicate
  completion, repeated queue requests, one-time follow-up consumption,
  invalidation, pending follow-up on begin, and saturation at `u64::MAX`.
- Source comparison confirms the five shared method bodies match both originals
  after member renaming, and 12 caller files differ only by member paths and
  formatting. Updated the account component guide.
- No persistence, message, subscription, assistant snapshot JSON shape, order,
  calculation, toast, network, or rendering behavior changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene portfolio`:
  **96 passed** before and after production changes, including both new lifecycle
  regressions and existing request-coalescing, stale-account/provider/key, pane,
  history, and portfolio tests.
- `cargo test --locked -j 2 --package kerosene --bin kerosene agent_snapshot`:
  **9 passed**.
- Source comparison verifies all five lifecycle method bodies match both old
  implementations, and the 12 caller files change only paths and formatting.
- `cargo test --locked -j 2`: **4,367 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check`, `git diff --check`, and `cargo build --locked -j 2`:
  passed.
- Headless startup smoke: the built binary with `--test` opened the 1600x960
  Kerosene window under Xvfb, confirmed by `xwininfo`. Expected timeout exit 124
  after 20 seconds, with no panic markers. In-memory test mode kept personal
  configuration out of the run. Logs: `/tmp/kerosene-refresh-smoke.16bN3a`.

## 2026-09-28: consolidate alert delivery

- Reviewed notification delivery, desktop transport, sound entry points, toast
  retention/animation, and interest/tracked-trade/Telegram alert callers. Kept
  caller eligibility, order-status policy, and platform transport in place.
- Shared the toast, sound, and desktop sequence through a private `push_alert`
  helper. Error, trade, interest, tracked-trade, and Telegram alerts retain their
  existing severity, desktop title, and sound kind. Sound and desktop delivery
  still use independent global toggles and follow toast insertion/pruning.
- Clone the message only when desktop notifications are enabled. The toast takes
  ownership of the original message. External delivery still occurs if the toast
  queue immediately prunes a new informational alert to retain existing errors.
- Replaced three one-line sound forwarding functions with calls to the existing
  `sound::play(SoundKind)` entry point. Audio synthesis, queueing, volume, and
  fallback behavior remain unchanged.
- Added two regressions before production changes: all alert entry points retain
  their toast content, severity, IDs, and existing order status with both external
  channels disabled; feed alerts preserve a full error queue while consuming IDs,
  and the next error evicts the oldest error.
- Source comparison confirms desktop transport, toast animation/retention, and
  order-status methods are unchanged; `sound.rs` only loses the three forwarding
  wrappers. Updated the integration component guide. No config, message,
  subscription, dependency, asset, or trading behavior changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene notification_state`:
  **7 passed** before and after production changes, including both new regressions.
- `cargo test --locked -j 2`: **4,369 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- Physical audio and OS notification delivery were not exercised; their transport
  implementations are unchanged.
- `cargo build --locked -j 2`: passed. Headless startup smoke opened the 1600x960
  Kerosene window under Xvfb, confirmed by `xwininfo`; expected timeout exit 124
  after 20 seconds with no panic markers. In-memory `--test` mode kept personal
  configuration out of the run. Logs: `/tmp/kerosene-notification-smoke.ARxS8q`.

## 2026-09-28: simplify analytics completion and follow-up handling

- Reviewed portfolio/income completion handlers, refresh model, pane/timer
  eligibility, request callers, connected-account snapshot lookup, and account
  refresh/reconciliation ownership. Kept account snapshot copies that span
  terminal-wide updates and reconciliation; no account/trading code changed.
- Each analytics handler now finishes a valid request, applies its result only
  for the connected account, then uses one follow-up dispatch path. Removed
  repeated follow-up branches for old-account responses. Income still checks
  the current account's Portfolio Margin eligibility before starting a follow-up.
- Moved income snapshot application into `apply_income_snapshot` in the same
  module, keeping completion routing short. Alert admission, timestamp tracking,
  summation order, formatting, data replacement, and error clearing are unchanged.
  No generic dispatch layer or forwarding methods were added.
- Added three regressions before production changes: old-account portfolio
  responses with a new connection or disconnection; income follow-ups with
  eligible, non-PM, mismatched, missing, or disconnected account snapshots; and
  stale generations preserving the current request's queued follow-up. Both
  success and error payloads are exercised for old-account replies, including
  preservation of current data/errors and suppression of stale income alerts.
- Source comparison confirms request construction is unchanged and result
  application bodies match after whitespace normalization. Updated the account
  component guide. No config, message routes, subscriptions, dependencies,
  network requests, trading policies, or UI behavior changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene portfolio_update`:
  **16 passed** before and after production changes, including the three new
  regressions and one existing combined-portfolio test matched by the filter.
- `cargo test --locked -j 2`: **4,372 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-28: simplify request preparation and Farside chart extraction

- Reviewed outcome-volume batching/cancellation, order-status request/parsing,
  unstaking response mapping, ETF fetch orchestration, fund conversions, and
  Farside extraction. Retained bounded concurrency, partial-success policy,
  response identity checks, financial calculations, and source-specific errors.
- Removed 14 temporary HTTP client clones across 11 files: calendar, ETF HTTP,
  unstaking, order status, Hydromancer funding/bootstrap/latency, HyperDash, and
  OpenRouter. Each clone was immediately followed by `get` or `post`. The local
  reqwest 0.12.28 implementation takes `&self` and gives the request builder its
  own client handle, so the temporary copy was unnecessary reference counting.
- Source comparison confirms those 11 files differ only by the 14 deleted
  `.clone()` calls. Clients, pools, headers, authentication, payloads, timeouts,
  proxy/read-budget/telemetry hooks, and response/error handling are unchanged.
  Ownership copies passed into asynchronous work were left in place.
- Combined Farside's one-caller data/label extractors into `extract_chart_data`.
  It finds the first BHYP marker once, validates the cumulative values first,
  then validates the preceding labels and matching lengths. Removed duplicate
  marker scanning without adding a parser abstraction or changing array scanning,
  date parsing, flow derivation, or error ordering.
- Adjusted extraction tests to exercise the shared entry point, strengthened the
  missing-label fixture, and added two regressions before production changes:
  first-chart selection when two BHYP charts exist, and data-error precedence
  when the first chart is invalid and a later chart is valid.
- Updated integration and market-data guides. No schemas, message routes,
  subscriptions, dependencies, assets, request policies, or trading behavior
  changed. No measured runtime speedup is claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene hype_etfs`:
  **38 passed** before and after production changes, including the two new
  parser regressions and existing ETF conversion, refresh, and rendering tests.
- Source checks confirmed exactly 14 clone deletions with every other byte in
  those files unchanged, and unchanged Farside array/date/flow helpers.
- `cargo test --locked -j 2`: **4,374 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-28: borrow clients across account and symbol read helpers

- Traced client ownership through account bootstrap/retries, wallet details and
  HIP-3 requests, wallet snapshots/order counts, all-mids, fills pagination,
  portfolio/income HTTP readers, symbol discovery, and listing snapshots.
- Six private HTTP helpers now take `&reqwest::Client`. Their callers await the
  helper within the same operation, and request builders already retain the
  client handle they need. Public fetch functions continue owning account,
  scope, and request inputs, preserving their use in independent iced tasks.
- Removed 23 client clones. Concurrent symbol families share one borrowed client
  instead of creating separate handles before each nested request group.
  Account/wallet readers also borrow the shared static client, including the
  wallet tracker's spot-price fallback.
- Kept request construction and ordering, concurrency groups, retry delays,
  rate-limit admission, proxy/telemetry routing, status handling, redaction,
  parsing, caches, and partial-result policy unchanged. Account payload and
  snapshot copies are outside this change.
- Updated existing loopback HTTP tests to pass client references. Source
  comparison applies the exact ownership/call-site replacements to the original
  files, runs rustfmt, and confirms byte-for-byte equality with all 15 edited
  Rust files; no request or result-handling logic changed.
- Updated the integration guide. Reused existing HTTP, account, wallet, symbol,
  and application test coverage for the ownership-only change. No schema,
  message, subscription, dependency, or asset changes, and
  no measured runtime speedup is claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene account_analytics`:
  **28 passed**, including local HTTP response/status, redaction, partial-data,
  and parsing tests.
- `cargo test --locked -j 2`: **4,374 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- Exact source transformation check for all 15 Rust files: passed.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-28: simplify Hydromancer frame ownership and review proxy boundaries

- Reviewed proxy URL parsing, validation on pool construction, route selection,
  leases, cooldowns, bounded retries, request admission, cancellation, routing
  isolation, settings commit order, and startup failure handling. Retained this
  code: secret deserialization bypasses `ProxyUrl::parse`, so stored URLs must
  be validated again when enabling a pool. Documented that requirement.
- Inspected native/Hydromancer subscription reference counts, outbound command
  handling, inbound frames, session updates, and control messages. Kept the
  providers' different unsubscribe payloads, readiness/reconnect rules, and
  redacted debug output in their existing modules.
- Hydromancer frame parsing now removes `cursor` and `sessionId` from the owned
  JSON object and moves string values directly into `Zeroizing<String>`, through
  one small helper. This avoids copying each resume string before dropping its
  original. Non-string values are still removed and ignored; other payload
  fields and frame classification remain unchanged.
- Control-message parsing borrows error text while applying the existing status
  formatting/redaction, removing two temporary string copies. Error/message
  precedence, fallback text, authentication labels, and retry delay are unchanged.
- Added two regressions before production changes for malformed resume-field
  types, retention of unrelated/nested data, and exact preservation of empty,
  whitespace-containing, and Unicode resume strings. Existing tests cover
  missing fields, session/cursor updates, redaction, and socket dispatch.
- Source checks confirm control parsing differs only by the two eliminated
  copies; proxy, read-budget, session, socket, and subscription policy code is
  unchanged. Updated integration and proxy guides. No schema, dependencies,
  messages, subscriptions, or trading behavior changed; no measured speedup is
  claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene ws::hydromancer`:
  **90 passed** before and after production changes, including the two new
  parser regressions, session transitions, socket/control handling, reconnect
  requests, and redacted diagnostics.
- `cargo test --locked -j 2`: **4,376 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests). This includes existing proxy transport/settings/storage and
  read-budget checks.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- Source comparison, `cargo fmt -- --check`, and `git diff --check`: passed.

## 2026-09-28: share API-probe telemetry updates

- Reviewed transport counters, latency snapshots, API-probe callers, status-bar
  health/presentation, and Hydromancer connection attempts, retry sleeps, idle
  waits, and shutdown handling.
- Grouped each provider's five API-probe atomics into `ApiProbeTelemetry`. Both
  providers now use one implementation for attempt, success, and failure
  updates while retaining independent state and the existing public functions.
- Kept the public snapshot fields, defaults, timestamp acquisition, atomic
  ordering, and store/load sequences unchanged. Starting another attempt or
  recording failure still retains the previous successful measurement; the
  status bar continues using separate flags to identify pending/failed probes.
- Added two deterministic tests using local state for retry/failure history,
  subsequent successful recovery, and provider isolation. All six existing
  telemetry tests are byte-for-byte unchanged.
- Source comparison verified that each of the six original update functions
  performs the same stores as its shared method, and that the public snapshot
  performs the same loads after the internal field-path substitutions.
- Updated the subscription/transport guide. No schema, messages, subscriptions,
  dependencies, UI layout, connection policies, or trading behavior changed.
  Hydromancer connection-failure duplication remains a separate candidate;
  retry and idle-wait loops retain their distinct termination policies.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene ws::telemetry`:
  **6 passed** before the change and **8 passed** afterward.
- `cargo test --locked -j 2 --package kerosene --bin kerosene
  status_bar::connectivity`: **7 passed**, including failed-probe and stale
  measurement presentation.
- `cargo test --locked -j 2`: **4,378 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- Source comparisons, `cargo fmt -- --check`, and `git diff --check`: passed.

## 2026-09-28: consolidate Hydromancer connection failures and borrow fill addresses

- Hydromancer connect errors and timeouts now normalize to a result before one
  failure-handling path. Kept redacted error text, timeout labels, failure
  telemetry, reconnect broadcasts, interruptible sleeps, exponential backoff,
  and successful-connection reset behavior. The connecting command select and
  connected read/write loop are unchanged.
- Added a loopback handshake-rejection test before production changes. It
  verifies HTTP failure reporting, one- then two-second advertised retry delays,
  an actual second connection attempt, and prompt shutdown during the retry
  wait. Existing timeout and cancellation tests cover the other branch.
- Reviewed live/replay fill selection, liquidation/tracked-trade conversion,
  dedupe keys/capacity, and subscription guards. The tuple helper now borrows its
  address instead of allocating a string discarded by liquidation parsing;
  tracked-trade output still owns the exact address. Added a regression for
  required string validation, including null/numeric rejection and acceptance
  of an empty string. Numeric parsing and event/dedupe formats are unchanged.
- Removed two final-use topic clones in feed subscription commands. Retained
  the independent copies needed by unsubscribe guards.
- Inspected idle/retry waits, session readiness/resume handling, connected
  commands, and key-rotation persistence ordering; retained their policies.
  The two fill-stream receive loops are still a consolidation candidate.
- Source checks verified both original retry sequences match the retained one,
  all other manager code is unchanged, and ownership edits are exact. Updated
  the integration guide. No schemas, messages, subscriptions, dependencies,
  assets, or trading behavior changed; no measured speedup is claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene ws::hydromancer`:
  **92 passed** before and after production changes, including the two new
  regressions, timeout/cancellation, socket routing, session, and redaction
  checks. The final loopback fixture sends a plain HTTP 403 response.
- `cargo test --locked -j 2`: **4,380 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- Source comparison, existing-test preservation checks, `cargo fmt -- --check`,
  and `git diff --check`: passed.

## 2026-09-28: share Hydromancer fill-stream lifecycle

- Consolidated the liquidation and tracked-trade subscription/receive loops in
  `ws/hydromancer/fill_stream.rs`. Each adapter still creates its original
  subscription payload, keeps its stream identity and 10,000-message output
  buffer, and acquires the manager at the same point. Empty tracked-address
  lists still return before manager acquisition.
- A private two-variant feed type pairs each channel with its existing parser,
  dedupe key, output variant, and history limit (20,000/50,000). The shared
  handler forwards controls before fills, preserves live/replay filtering and
  duplicate history across reconnect messages, requests recovery before lag
  delivery, and retains the two-second pause after successful recovery/delivery.
- Subscription guards now have one creation/cleanup path. Failed subscribe,
  failed downstream delivery, closed broadcasts, and future cancellation keep
  their existing termination behavior. The common handler accepts a sink so
  channel-driven tests can exercise it without network access or manager-registry
  test hooks.
- Added six tests, each exercising both feeds: mixed control/live/replay data
  and malformed/nonmatching input; failed control/fill delivery; reconnect
  before failed lag notification; pause after successful lag notification;
  cancellation while waiting; and failed subscription. They assert command
  ordering and exact unsubscribe payloads where applicable.
- Source comparison matches both original stream bodies to the shared one after
  the explicit feed-configuration/event-wrapping substitutions and rustfmt
  differences. Subscription builders, channel sizes, parsing helpers, history
  implementation, reconnect gate, and manager code retain their behavior.
- Updated the subscription architecture guide. No config, public messages,
  subscription identities, dependencies, assets, or trading behavior changed.

Validation (using the local ALSA prefix documented above):

- Baseline `cargo test --locked -j 2 --package kerosene --bin kerosene
  ws::hydromancer`: **92 passed**.
- New `ws::hydromancer::fill_stream` tests: **6 passed**; the complete
  Hydromancer target after extraction: **98 passed**.
- `cargo test --locked -j 2`: **4,386 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- Original-loop source comparison, `cargo fmt -- --check`, and
  `git diff --check`: passed.

## Next candidates

1. Continue through liquidation/tracked-trade state, aggregation, status, and
   update paths, preserving feed-specific filtering, history, and alert policy.
2. Continue through account data, wallet model, and update ownership. Account
   state/persistence copies remain intentional where they protect a snapshot
   across terminal mutations.
3. Continue across the unreviewed areas in the coverage table. Large files often
   include inline tests, so distinguish production complexity from file length.
