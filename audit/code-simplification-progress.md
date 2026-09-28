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
| Charting and canvas | Instance construction, persisted chart settings, annotation loading, and comparison settings reviewed. Comparison-chart rendering contexts, axes, background setup, and crosshair drawing reviewed and consolidated; mode-specific formatting and series calculations retained. Metadata-driven chart identity reconciliation reviewed and separated from symbol refresh orchestration. Trading-overlay synchronization reviewed: order assembly is separated, chart symbols and pending indicators are borrowed, Chase overlays iterate without an intermediate vector, and marker parsing uses the existing numeric helper directly. Baseline-tested regressions preserve account/symbol boundaries, replacement/decorating order, distinct numeric policies, hidden-state clearing, and stable marker ordering. Clipping, trade grouping, and overlay drawing inspected; compact geometry and differing blur policies retained. Order/current-price/position drawing now shares style construction and consumes prepared order labels on the final paint pass. Thirty-six synthetic previews remain byte-identical across themes, animation, distortion, and privacy states. Order-label and right-axis badge layout reviewed with their hit-test callers: duplicate label packing is shared and one redundant badge sort removed. Fixed-position and variable-height badge policies remain separate from left-label packing, with crowded-edge/tie regressions and unchanged synthetic renders. Chart input dispatch, press/drag, drawing-tool, cursor, wheel, and HUD handlers inspected. Drag release shares its common reset/redraw path; annotation copies are deferred until coordinate/lock checks pass. Baseline regressions cover every drag kind, surface identity, per-gesture cleanup, message capture, cache invalidation, and preview snapshot preservation. Quick-order opening/replacement share action construction while callers retain priority and missing-price fallthrough. HUD size editing shares accepted-character insertion and finish-key handling; existing tests are extracted. Baseline regressions preserve click precedence, source/visual coordinates, decimal replacement, length limits, and edit flags. Visible-price/scroll/heatmap-bound helpers inspected and kept compact. Annotation hit testing and selected/live drawing now iterate anchors without a temporary vector, borrow annotation records, and fold Fibonacci bounds directly. Anchor order, invalid raw values, topmost/handle/lock rules, and 32 synthetic renders preserve baseline behavior. Primary/secondary real-time candles share tail mutation while retaining validation, append-only trimming, missing-series rejection, status ownership, and cache invalidation. Baseline regressions cover both series and websocket price flashes; redundant websocket validation is removed. Other chart rendering internals remain. |
| Market data | Mid-price update visibility filtering reviewed; unnecessary catalog copies removed. Persistent API cache reviewed and split into candle policy, queued writes, and storage, with write coalescing simplified. Public shared reads and read admission inspected and retained. API exports, order-book reads, chart asset-context reads, watchlist context requests/parsing, and exchange statistics reviewed. Spot chart context lookup indexed once per response; differing parser and partial-result policies retained. Symbol metadata orchestration, perpetual/spot parsers, DEX registry parsing, and listings parsers reviewed; unnecessary metadata copies removed. Symbol refresh, legacy spot migration, label updates, and search context results reviewed and split; shared watchlist alias rewriting and removed intermediate copies. Symbol search planning, filtering, sorting, DEX listing/ranking, and volume lookup reviewed; ranking work moved out of comparisons. Live-watchlist and ticker-tape context completion reviewed and shared, retaining their distinct status/refresh policies. Watchlist history completion inspected; row-cache refresh now shares one borrowed metadata index per batch. Candle request/response policies and watchlist/outcome-volume history inspected; candle normalization deduplicates in place and trailing-run searches stop at the final gap. Outcome parsing, contract/template resolution, question membership, and label helpers reviewed; the temporary question index borrows shared records and expiry formatting is shared. Calendar, unstaking, and ETF API entry points/conversion helpers inspected. Order-status request/parsing and outcome-volume batching/cancellation reviewed; identity checks, concurrency, and partial-failure behavior retained. Remaining API requests, books, and other widgets need review. |
| Wallets and account state | Wallet detail and cluster read-result/websocket filters reviewed. Account picker/setup routes traced; unreachable legacy credential-editing handlers removed. Active Add Account, connection, and switching safety boundaries inspected and retained. Account user-stream handling and risk scrubbing inspected for copies but unchanged. Account refresh admission, rate-limit retries, and reconciliation flow inspected; the original fills snapshot is retained across terminal mutations. Account bootstrap, wallet detail/snapshot/order-count reads, and analytics HTTP ownership reviewed; helpers now share borrowed clients without changing request policy. Account persistence snapshots and wallet tracker model, queue, selection, refresh, and compact-model paths were revisited. Tracker ranking now borrows candidates until selection, refresh-all borrows tracked state and copies only admitted addresses, and core refresh setup reuses its row lookup. Seven baseline-tested regressions preserve FIFO precedence, timestamp ties, batch bounds, loading/retry/age and snapshot rules, queue deduplication, and request context. Account bootstrap merging and fee parsing were inspected and retained. Completeness warning comparisons now borrow one owned entry, and summaries borrow unique messages; section/global deduplication and actionability policies have baseline-tested regressions. Selected-DEX scope construction borrows input before producing its normalized owned key. Freshness, balance, leverage, and scope helpers were traced without changing their policies. Hydromancer single/batch portfolio parsers now move owned fields and tuple values into a separate portfolio model module; alias precedence, error order, metadata selection, and redaction have baseline-tested regressions. Batch fetches borrow address lists and reuse scope limits while retaining shared result-map copies for repeated inputs. Portfolio getters and native bootstrap/wallet conversion now deserialize retained JSON by reference into owned models; exact errors, defaults, lookup precedence, repeated access, and redacted previews have baseline-tested regressions. Wallet snapshot conversion moves parsed positions into aggregation. Scoped DEX lists and wallet detail response matching now borrow DEX names; joined Hydromancer helpers borrow addresses, scopes, and keys from their owning tasks. Four baseline-tested regressions preserve raw/fallback list values, ordering, duplicates, and repeated portfolio results. Wallet tracker config restoration shares first-seen address normalization; local/remote label lists share one collection and sort/deduplication path, reused by subscription filtering. Count-redaction wrappers use the established standard formatting pattern. Baseline-tested regressions preserve list/muting behavior and exact redacted output. Address-book config/import/export and remote read policy were inspected and retained. Wallet display and label lookup now normalize once and share remote-first nonblank-label selection; two baseline-tested regressions preserve normalization, invalid-input fallback, visible strings, and Unicode shortening. Tracker rows borrow model data, reuse remote status, and move display strings into identity controls. Hydromancer detail conversion consumes temporary DEX states and order vectors. Wallet addition/restoration shares completion while retaining distinct row, label, and subscription policies; four baseline-tested regressions cover these paths and exact order DEX classification. Wallet detail stream branches share matching-window snapshot completion; seven baseline-tested regressions cover event fields, filtering, timestamps, pending requests, independent windows, fills, and lag recovery. Completion callbacks move captured addresses and lag batches consume task iterators directly. Cluster runtime/config models and their distinct stream freshness rules were reviewed and retained. The cluster update module now separates management, read data, order planning, execution results, and position calculations; all 30 production function bodies are preserved. Three baseline-tested regressions cover result/status classification, redaction, progress, and execution/member/cloid identity. Cluster refresh selection now shares one borrowed lookup and captures only selected profile IDs, preserving repeated entries and cached-row policies with four baseline-tested regressions. Cluster views now separate members, ticket, positions, and executions, borrow display strings, reuse the selected cluster, and build only visible price inputs. Disabled close controls avoid symbol copies; rendered/control regressions and eight identical synthetic previews verify the changed view paths. Cluster order/close plans share prepared-leg construction, and dispatch moves captured keys and completion contexts into tasks. Position aggregation borrows symbols until creating a distinct summary. Four baseline-tested regressions preserve arithmetic order, missing-value semantics, independent results, execution identity/order, counter rollover, and spot invalidation; a constructor regression verifies request/context fields and captured targets. Wallet detail summary/table preparation borrows stored positions while retaining eager owned spot synthesis; the table filters in place and caches stable symbol sort keys. Position, order, and spot tables share their identical container, and error/warning text borrows state. Two baseline-tested regressions cover projected data, visibility, selection order, and detail states; six synthetic previews match byte for byte. Account position sections and prepared display/summary rows now borrow their inputs, preserving upstream snapshot ownership. Baseline-tested regressions cover all 12 sort columns in both directions, stable ties, missing values, grouped rendering, privacy, and independently owned symbol/hide/close messages; four synthetic previews match byte for byte. Account projection now borrows native rows and eagerly owns synthesized outcome/spot rows, following the wallet-detail pattern; table filtering and PnL-card consumers retain those borrows. Spot selection uses one sorted collection of borrowed candidates instead of intermediate collections and copied keys. Baseline-tested cases preserve synthesis order, account/universe gates, all selection tiers, and tie rules; four position-section previews remain byte-identical. Other account/portfolio flows remain. |
| Journal and analytics | Fill API pagination, identity, normalization, merging, and same-timestamp chain ordering reviewed; normalization deduplicates adjacent identities and avoids copying single-fill groups. Aggregation orchestration, position reconciliation, and journal view preparation reviewed. Identical non-perp classification and fee arithmetic now live in the journal domain. Note lookup/editing and account-scoped state reviewed; note lookup borrows entries and duplicate reset paths share one implementation. Snapshot models, planning, assembly, and metrics reviewed and separated; request bounds and history admission are shared. Snapshot update callers inspected, with freshness/admission policies retained. Journal cache persistence and tests reviewed; platform-specific replacement retained. Cockpit rendering/analytics reviewed and split by panel; per-asset aggregation copies coin names only for distinct output rows. Detail/chrome/list views, summary preparation/series/drawing, and small trade-card helpers reviewed; simplified series iteration and reused detail values. Snapshot canvas interaction/rendering reviewed and separated; the canvas borrows its snapshot. Account analytics HTTP fan-out, reserve/name/history parsing, income assembly, and portfolio data selection reviewed; token validation is centralized, recent-payment formatting is bounded to 12 valid rows, portfolio bucket construction is direct, and unused theme construction is removed. Portfolio/income panes, table variants, projection generation, chart layout/hover/tooltip, and PnL area rendering reviewed; daily histories and income labels now borrow data, hidden-chip preparation is skipped, and common table cells/status wrapping are shared. PnL-card state/metrics, privacy text, preview/export rendering, contrast, and output paths reviewed; digit masking is shared and position percentages are reused. Owned export snapshots and account binding remain intact. Account metric helpers reviewed and retained; portfolio/income refresh lifecycle now has one shared implementation with independent state per feature, while caller admission and result policies remain explicit. |
| Orders, signing, Chase, TWAP | Chase/TWAP market-subscription assembly reviewed and shared with order-book panes; lifecycle eligibility filters and event mappings retained. Removed discarded theme constructions from order/Chase entry points after checking theme purity; request and lifecycle code is otherwise byte-identical. The shared execution core has been reviewed and separated into redacted models, capability policy, client-order IDs, task dispatch, and preparation. Place/cancel/modify validation bodies, nonce/hash logic, and captured task inputs are preserved. Prepared-order helpers share request field construction, covered by a 156-case baseline-tested regression; all existing core tests move beside their responsibilities. Root execution state and guards now live in focused account-context, pending-request, automation-identity, quick-order-model, and move-context modules. Their bodies are preserved except for removing a committed-key presence forwarding alias. Two baseline-tested regressions preserve signing-error priority and HUD overlap exemptions; move-context coverage also verifies the existing case-sensitive replacement rule. Ticket, quick-order, Quick Trade, HUD, close-position, and advanced-start orchestration were inspected; distinct admission, pricing, snapshot, and recovery policies stay explicit. Five submission surfaces now share prepared-indicator field construction, and quick-order percentage validation borrows provenance. Three baseline-tested regressions cover raw indicator fields, invalid values, IOC projection, independent presentation flags, and valid Quick Trade dispatch in both directions; spot recalculation also preserves its provenance. Shared result classification, pending status models, one-shot/cancel reconciliation, and NUKE aggregation were reviewed and separated into focused modules; transient order UI cleanup now has its own update module. Close-position and Quick Trade results reuse serialized ticket completion. Cancellation follow-up shares verification setup, and status callbacks move owned contexts instead of cloning them. Baseline-tested regressions preserve cleanup, account gates, surface identity, verification reasons, and optional order identity; existing classification and reconciliation bodies remain unchanged. Exchange-response deserialization and analysis were reviewed; typed decoding now borrows retained JSON, response queries share a borrowed status slice, and IOC detection searches messages without collecting owned copies. Three baseline-tested regressions preserve typed/raw ownership, object/sequence decoding, missing/malformed states, fill-size rules, and IOC raw-message redaction. Chase place/modify/cancel result admission now uses one order lookup, client-order ID verification defers its fallback account copy, and placement-status handling borrows the admitted order through each transition. Baseline-tested regressions preserve stale-result refresh differences, missing-ID fallback, stopping state, and remaining-fill accounting. Chase OID-status handling now borrows the admitted order, and request/error paths share the lifecycle verification transition. TWAP slice results consume pending placements while preserving pending cancellations; retry plans and final-use task IDs move into their next owners. Baseline-tested regressions preserve all lifecycle intents, account normalization, stop/archive and fill rules, late-result refreshes, and child/retry identity. Closed-order cancellation text matching now lives in shared execution code rather than Chase; ordinary, Chase, and TWAP recovery policies remain distinct. TWAP cancellation shares cleanup and retry accounting, prepares summaries once, and moves final-use task IDs. TWAP refresh applies policy through one lookup. Baseline-tested cancellation and refresh matrices preserve event text, saturation, child targeting, stop/archive behavior, account identity, and loading follow-ups. TWAP status reconciliation now shares matching-child updates and unknown/transport retry accounting, while preserving missing-status exhaustion, exact IDs, fill confirmation, and distinct feedback. Final-use status task IDs move into their next owners, and account-fill completion shares one eligibility condition. Baseline-tested regressions cover duplicate/mismatched child IDs, absent returned IDs, retry saturation, retained plans, and stale admission. TWAP slice execution now borrows cached order books and retry sizing through planning, transfers admitted retry plans after key validation, and passes only slice indices to skip handling. Baseline-tested regressions preserve randomized sizing, gate order, retry retention, cached depth, and skip accounting. Chase/TWAP retryable exchange phrases now share one predicate while preserving caller precedence and reusing TWAP normalization. TWAP no-fill confirmation shares the fill-update pass, retaining unknown-ID children and prioritizing late fills. Fill summaries, identifier hashing, metrics, and cancellation/refresh helpers were reviewed and otherwise retained. Signing builders, wire types, client/nonce flow, numeric formatting, key capture, and hash assembly inspected; existing shared policies retained. Temporary hash concatenation buffers are a follow-up candidate requiring baseline signature checks. Remaining signing models and automation state machines still need review. |
| Config, persistence, secrets | Chart snapshot/config boundaries reviewed, schema unchanged. Proxy URL normalization, redacted labels, deserialization/revalidation, settings commit order, and startup fallback inspected; existing security and persistence behavior retained. Remaining persistence/security code needs review. |
| Subscriptions and transport | Subscription assembly reviewed across market, user data, Hydromancer, Telegram, timer/input, and window families. Shared selected-provider book setup and reduced symbol copies; remaining eligibility/identity differences retained. Market adapters and user-data routing/dispatch inspected. Shared reconnect-before-notify behavior and snapshot timing, split Hydromancer adapters, and reduced owned payload copies. Native manager lifecycle/commands, both managers' subscription reference counts/coalescers, and Hydromancer registry/session state inspected; provider-specific lifecycle and routing retained. Proxy transport/admission reviewed and retained. Hydromancer frame/control parsing now moves resume strings out of JSON and borrows error text. Both API probes now share telemetry state/update code with independent provider state; snapshot fields and atomic ordering remain unchanged. Hydromancer connection/retry/idle-wait handling reviewed; connect errors and timeouts share one retry path. Fill tuple parsing borrows addresses and feed subscriptions move their final topic use; event formats, dedupe policy, and command cancellation are retained. Liquidation/tracked-trade subscription, receive, dedupe, recovery, and cleanup now share one handler; feature payloads/parsers and separate history limits remain explicit. Remaining integration stream internals still need review. |
| Feeds, integrations, assistant | Calendar fetch/refresh, filters, summary, and row views reviewed; date parsing is shared within each view and cached during API sorting. Farside ETF flow parsing reviewed; data and label extraction now share one chart marker lookup. SEC API requests, submissions, structured earnings, document selection, and text summaries reviewed and split by responsibility; shared HTTP request/status handling and reduced summary text copies. Shared-client request construction inspected across API, Hydromancer, HyperDash, and OpenRouter paths; unnecessary temporary client copies removed. Liquidation/tracked-trade update admission, control status, lag handling, and bucket accumulation reviewed; control transitions and liquidation accumulation are shared, while filtering/freshness, alerts, and retention remain feed-specific. Feed aggregation keys, intent, row models, alert eligibility, and row views reviewed; merge checks share one predicate, suppressed alerts defer row construction, and views reuse themes/owned strings and skip hidden-column formatting. Remaining feed controls, headers, responsive thresholds, footer charts/summaries, and connection presentation reviewed; identical controls are shared and wallet counts reused, while chart math and per-feed presentation differences remain intact. X feed state/update/rendering and REST helpers inspected; author profiles now update in place through one path, source options sort borrowed lists with cached keys, and newest-post selection borrows IDs until request creation. X credential staging/commit/clear paths now share cleanup and borrowed validation, with request-generation regressions; the unchanged HTTP client is separated from state. X refresh admission and token/auth results reviewed; the shared refresh decision covers absent/expiring access tokens, dead guards are removed, and owned credentials/tasks avoid extra copies. Telegram channel, post-merge, avatar, media, and reference-price paths reviewed: alert preparation is bounded to formatted messages, mention updates borrow history and share construction, avatar merging uses one lookup, and input/media copies are deferred. Existing auth-test registry races are isolated with the shared test lock. Telegram fast-mode update handlers are separated from post merging; auth requests share generation/status/result setup, login and private-channel scans reuse the signed-in predicate, and request admission/cleanup order has focused regressions. The feature guide now reflects optional fast/private mode and actual polling/session behavior. Telegram public HTTP/parsing is separated from feed state; avatar/media downloads share request/validation code and HTML extraction borrows source slices. Model/redaction, normalization, fetch timing, ordering, and size policies are preserved, with local HTTP and parser regressions. Telegram live views now borrow posts, profiles, candidates, and status/title text; shared avatar rendering preserves image/initials behavior and impact widgets consume prepared buffers. Styles are separated with unchanged bodies, and private scan controls no longer wrap an always-present widget in Option. Telegram sign-in controls now borrow fixed options/text and iterate code characters directly. Fast-feed lifecycle, resolution/backfill, and cursor sequencing were traced and preserved; media helpers are separated from the root, with unchanged classification, limits, and follow-up behavior. Fast-feed authentication/challenge ownership and session/client cleanup are separated into focused modules with nearby tests, preserving their function bodies and platform policies. Private-channel scans cache sort keys with stable ordering and adjacent deduplication, and session-file paths use a fixed array. Pending-auth removal and restoration now share an explicit guard, removing a verified nested-lock deadlock on wrong-stage submissions while retaining the original challenge and error text; a deadline-bounded subprocess regression covers restoration and unrelated request/session ownership. Assistant internals remain. |
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

## 2026-09-28: share feed control transitions and liquidation bucket accumulation

- Reviewed liquidation/tracked-trade update admission, control statuses,
  receipt timestamps, lag behavior, hidden-symbol filters, alerts, history,
  bucket rebuilding, and retention. Inspected aggregation and alert entry
  points; retained their different row limits and merge policies.
- Moved the eight repeated Hydromancer control transitions into
  `feed_update/hydromancer_status.rs`. Scope/generation/address checks still
  precede the helper. Labels, error redaction, lazy clock reads, and timestamp
  preservation/clearing are unchanged. Data messages remain in the caller:
  hidden liquidations refresh freshness, while filtered tracked trades do not.
  Lag still clears only liquidation-derived buckets and keeps both feeds' rows.
- Live liquidations and history rebuilding now use one accumulation helper for
  minute/second buckets and long/short notional. Live updates retain pruning
  and the 10,000-row cap; rebuilding clears and replays retained events without
  age pruning. Events move into the deque after accumulation, eliminating the
  previous per-event clone while retaining normalization, alert, and scroll
  behavior.
- Added five regressions before production changes: all control transitions
  across both feeds with independent state; live bucket sides, retention, and
  bounded history; rebuilding with old events and boundary timestamps; hidden
  liquidation freshness; and hidden tracked-trade rejection without freshness,
  dedupe, history, or alert changes.
- Source comparison verified all sixteen original control branches against the
  shared helper, excluding the caller-owned liquidation lag cleanup. The
  tracked-trade data handler is unchanged apart from indentation. Updated the
  integration guide. No schema, routes, subscription identities, dependencies,
  assets, or trading behavior changed; no measured speedup is claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene feed_update`:
  **87 passed** before and after production changes, including the five new
  regressions and existing stale-scope, lag cleanup, redaction, and alert tests.
- `cargo test --locked -j 2`: **4,391 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests), including the risk-filtering callers of bucket rebuilding.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- Control-transition/data-handler source comparison, `cargo fmt -- --check`,
  and `git diff --check`: passed.

## 2026-09-28: simplify feed row merging and rendering ownership

- Reviewed liquidation/tracked-trade grouping keys, row accumulation, intent,
  alert suppression/formatting, and row/cell rendering. Retained their distinct
  grouping identities, merge windows, scan limits, visibility filters, and
  render limits. Both aggregators now take one mutable row lookup for a merge,
  replacing the repeated immutable/mutable lookup pair.
- Extracted the existing tracked-trade order/hash/time-span predicate. Rows and
  alert eligibility share it; suppressed alerts now compare borrowed event
  metadata before allocating an owned row and its strings. Eligible alerts
  still construct the same single-fill row, and disabled aggregation still
  bypasses history scanning.
- Blank incoming fee tokens retain the current nonblank string in place,
  removing a clone-and-replace of that same value. Empty/current-token
  precedence, whitespace preservation, mixed-token labels, and fee arithmetic
  are unchanged.
- Row views move owned address/coin strings into cells. Tracked-trade cells
  reuse the parent view's theme, move the address and display label, and clone
  the raw coin only when a distinct-label tooltip needs it. PnL, fee, and intent
  formatting runs only for visible columns. Wallet actions, tooltip contents,
  icons, colors, formatting, and widget layout are unchanged.
- Added three regressions before production changes covering bidirectional
  alert time boundaries, order/hash grouping, identity mismatches, searching
  past unrelated history, ungrouped alerts, and blank/mixed fee-token handling.
- Source comparisons verified the extracted merge predicate and the exact view
  transformations; widget construction is unchanged beyond the listed ownership,
  theme reuse, and conditional-formatting changes. Updated the integration
  guide. No schemas, messages, routes, subscriptions, dependencies, or assets
  changed; no measured speedup is claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene feed_state`:
  **15 passed** before and after production changes, including the three new
  regressions and existing row/key/intent tests.
- `cargo test --locked -j 2 --package kerosene --bin kerosene feed_views`:
  **12 passed**, including formatting and row-style coverage.
- `cargo check --locked -j 2`: passed after the final cell changes.
- `cargo test --locked -j 2`: **4,394 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- Merge/view source comparisons, `cargo fmt -- --check`, and
  `git diff --check`: passed.

## 2026-09-28: consolidate feed controls and reuse wallet counts

- Reviewed both feeds' top bars, connection controls, settings dropdowns,
  headers, responsive layouts, empty states, and liquidation footer chart and
  summary calculations. Retained the explicit breakpoints, chart sampling and
  scaling, summary fold order, status logic, and differing clear/settings button
  corner policies and text sizes.
- Added `feed_views/controls.rs` for the identical header-text settings, toggle
  widget/style, and settings-dropdown container. Eighteen header cells, six
  toggles, and two dropdown wrappers now share those implementations. Caller
  labels, widths, visibility conditions, messages, row order, spacing, and
  10px/11px toggle text sizes remain unchanged.
- Tracked-trade wallet lists are built once during the existing empty-state
  checks; their counts pass through responsive rendering into the top bar.
  This avoids collecting/sorting the same labeled and unmuted lists again in
  the header. Missing keys/labels now skip unused list preparation. Both feed
  entry points construct their otherwise-unused theme only for an empty state.
- Source comparison verified both old toggle bodies and dropdown styles against
  the shared helpers, and expanded all header-helper calls back to the original
  widget bodies. Footer charts, responsive thresholds, and connection-status
  calculations are byte-for-byte unchanged. Address-count sources, saturating
  muted-count arithmetic, empty-state precedence, and displayed labels remain
  the same.
- Updated the integration guide. Reused existing view tests and compilation for
  this widget extraction; no new calculation/state behavior was introduced.
  No schemas, messages, routes, subscriptions, dependencies, or assets changed;
  no measured speedup is claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene feed_views`:
  **12 passed** before and after production changes.
- `cargo check --locked -j 2`: passed.
- `cargo test --locked -j 2`: **4,394 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- Widget/source comparisons, `cargo fmt -- --check`, and `git diff --check`:
  passed.

## 2026-09-28: simplify X profile updates and feed preparation

- Inspected X feed state, request/result handling, author images, source options,
  post selection, rendering, REST helpers, and the surrounding persistence and
  timer boundaries. Kept authentication and credential handling, rate-limit
  behavior, source identity, response parsing, and UI construction unchanged.
- Avatar scheduling now updates profiles through one map-entry path instead of
  removing/reinserting image-bearing profiles and duplicating metadata updates
  for posts without an image URL. Existing strings reuse their allocations;
  new profiles initialize their metadata once. Image URLs remain borrowed until
  a changed URL or scheduled task needs ownership.
- Preserved metadata-only updates, cached images, in-flight suppression, retry
  backoff, URL-change invalidation, global request generations, and stale-result
  handling. Added three lifecycle regressions before production changes and
  moved the update tests into `feed_update/x/tests.rs`. Tests inspect tasks and
  supply local completions; they do not call X or fetch remote images.
- Source options now sort references with cached ASCII-folded names and borrowed
  ID keys. Only surviving options copy their names/IDs; stable tie order,
  duplicate selection, private flags, and the source list itself are preserved.
- Newest-post selection returns a borrowed ID. The refresh path makes one owned
  copy after selecting across matching instances, preserving numeric ordering,
  invalid-ID filtering, and the final input on numeric ties. Added two model
  regressions for source ordering and numeric-ID edge cases before the changes.
- Updated the integration guide. No config schemas, message routes,
  subscriptions, dependencies, assets, or view behavior changed; no measured
  speedup is claimed.

Validation (using the local ALSA prefix documented above):

- Focused update tests (`cargo test --locked -j 2 --package kerosene --bin
  kerosene x::tests`): **4 passed** before and after production changes.
- Focused model tests (`cargo test --locked -j 2 --package kerosene --bin
  kerosene x_feed::tests`): **8 passed** before and after production changes.
- `cargo test --locked -j 2`: **4,399 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-28: consolidate X credential cleanup and separate the HTTP client

- Reviewed X credential staging, secret snapshots, direct/OAuth commits,
  clearing, refresh expiry, and request generations alongside the encrypted
  storage/persistence callers. Direct-token commits now delegate to the existing
  OAuth commit path, and commits/clearing share one helper that zeroizes all six
  editable/pending buffers. Update handlers still persist successfully before
  committing candidate credentials through the existing setter.
- Credential snapshots reuse the existing owned accessors. Candidate and pending
  snapshot validation borrow trimmed inputs before allocating; accepted
  candidates enter zeroizing buffers immediately. Tasks, pending credentials,
  and persistence retain independent owned copies where their lifetimes require
  them. Empty/partial inputs preserve pending state and the original errors.
- Added three regressions before production edits: candidate validation and
  snapshot ownership; direct/OAuth commits with changed/unchanged credentials;
  and clearing with/without saved credentials. Coverage includes all six input
  buffers, private/public source behavior, cached data, in-flight/rate-limit
  state, expiry, and exact generation counts. Preserved the existing extra
  invalidation when clearing changes saved credentials.
- Moved REST requests, body/image validation, response parsing, and private wire
  types into `x_feed/client.rs`; the root retains domain types and runtime
  state, with the same public crate-facing function paths through re-exports.
  Model and client tests now live beside their respective modules. Source
  comparison verified the relocated HTTP bodies/wire types and retained
  profile-key helper byte-for-byte, including error strings and request order.
- Updated the X feature and integration guides, and corrected the security
  guide's stale description of `SensitiveString`: it is a wrapper with redacted
  debug output, not a type alias. No schemas, message routes, subscriptions,
  request payloads, dependencies, assets, or UI behavior changed.

Validation (using the local ALSA prefix documented above):

- Focused model/client tests: **11 passed** before production edits with
  `cargo test --locked -j 2 --package kerosene --bin kerosene x_feed::tests`,
  and **11 passed** after final changes with the `x_feed::` filter (including
  the relocated client test).
- `cargo test --locked -j 2`: **4,402 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- Source comparisons also verified unchanged request constants, credential
  replacement, refresh-expiry checks, and request invalidation.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-28: simplify X refresh admission and credential ownership

- Reviewed all three X refresh entrypoints, token/auth result handling, source
  batching, and secret persistence admission. The state-level token-refresh
  decision now includes a missing access token, removing the same extra
  condition from auth, list, and timeline callers. Complete refresh credentials,
  unknown expiry, the 60-second window, and in-flight suppression retain their
  existing behavior.
- Removed the unreachable missing-token branch after the timeline's earlier
  token guard. Visible requests still clear stale pane errors before attempting
  authentication; background requests leave those errors alone, and an existing
  connection attempt still suppresses a duplicate task. Combined adjacent
  synchronous early-return guards without changing their order or conditions.
- Open-pane refreshes feed their task iterator directly into `Task::batch`;
  the pane-ID snapshot remains because scheduling mutates terminal state.
  Multiple panes on the same source still share one pending request.
- Token results move the existing zeroizing fallback refresh token instead of
  copying it. Connection labels borrow the incoming username before storing the
  owned user. The auth client's sequential user lookup borrows the access token
  already owned by the outer task; HTTP request construction and parsing are
  otherwise source-identical. Persistence-before-commit and stale-result guards
  remain in place.
- Added six regressions before production edits, covering admission across all
  three entrypoints, pending-refresh suppression, visibility/connect behavior,
  multi-pane source batching, rotated versus pending/saved fallback credentials,
  stale results, failed persistence, and partial-list status labels. Credential
  tests use synthetic data, encrypted payload round-trips, and the existing
  test-only config-save callback; no X requests or user-config writes run.
- Began the Telegram follow-up: inspected public/private channel management,
  feed merging, ticker reference updates, avatar scheduling, media scheduling,
  and media result guards. Identified avoidable input/candidate/mention copies
  and eager media URL preparation for the next batch; kept this commit scoped
  to the X lifecycle changes.
- Updated the integration guide. No schemas, message routes, subscriptions,
  external request payloads, dependencies, assets, or view layouts changed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene feed_update::x::tests`:
  **10 passed** before and after the refresh/result simplifications.
- `cargo test --locked -j 2`: **4,408 passed, 0 failed, 6 ignored**, including
  the final borrowed-token client change; doc-tests passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- HTTP source comparison, `cargo fmt -- --check`, and `git diff --check`:
  passed.

## 2026-09-28: simplify Telegram update ownership and bound alert preparation

- Reviewed channel creation, post merging, ticker references, alerts, avatar
  merging, media scheduling, and their result guards. Channel normalization and
  private candidate conversion now borrow the source input/candidate. Public
  refresh tasks feed directly into `Task::batch`, retaining the owned channel
  snapshot needed while scheduling mutates terminal state.
- Alert preparation retains at most three formatted messages and an overflow
  count instead of cloning entire posts and their media/mention data. Disabled
  alerts skip preparation. Messages are still delivered after feed sorting and
  pruning, in arrival order, with the same count/preview rules; the first
  version of a new post wins if that page later edits it. Backfill, previously
  seen IDs, and initial-load suppression are unchanged.
- Post updates and full mention refreshes borrow previous mention slices.
  Mention construction now shares one path that carries forward only the
  reference price/timestamp and moves the newly resolved metadata. Captured
  prices remain immutable; missing-reference resolution, unresolved timestamps,
  first matching history entry, and matching/filter rules are preserved.
- Avatar merging uses one cached-profile lookup and prepares request-owned
  strings only for eligible fetches. It preserves incoming handles, unchanged
  URLs, pending request state, backoff, and URL-change invalidation. Media URLs
  are cloned after eligibility checks. Kept media's two-pass target snapshot and
  first-match update semantics, plus all result guards and merge precedence.
- Moved the existing inline update tests into `telegram/tests.rs`; new ownership
  regressions live in `tests/ownership.rs`. Four new tests cover alert order,
  limits/counts and pruned/duplicate posts; media eligibility/request order;
  avatar cache/pending/failure state; and mention metadata/reference history.
- The pre-refactor parallel baseline exposed an existing pending-auth registry
  race: another auth test cleared the accepted-code test's placeholder, then
  poisoned its test lock. All 70 tests passed serially. Added the existing
  pending-auth test lock to 13 other auth-mutating tests and a test-only guard
  around config-clear's shared-registry cleanup. The parallel baseline then
  passed before production changes. Normal runtime cleanup is unchanged.
- Updated the integration guide and corrected the Telegram feature guide's
  reference-price description. No schemas, messages, routes, subscriptions,
  dependencies, request payloads, assets, or view layouts changed; no measured
  speedup is claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene feed_update::telegram::tests`:
  initial parallel baseline **68 passed, 2 failed** from the shared-auth test
  race; serial baseline **70 passed**. After the test-isolation correction,
  **70 passed** in parallel both before and after production changes.
- `cargo test --locked -j 2`: **4,412 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- Source comparison verified unchanged reference-price lookup, avatar/media
  result guards, media merge precedence, and alert text formatting. Existing
  test bodies match apart from formatting and the added isolation guards.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-28: separate Telegram fast-mode updates and share auth request setup

- Reviewed Telegram auth admission, API ID validation, code/password submission,
  sign-out, stale auth results, abandoned challenges, stream events, and status
  sanitizers alongside state, subscription, config, and view entry points.
  Moved fast-mode handlers and their sanitizers into `feed_update/telegram/fast.rs`;
  the parent retains message dispatch, channel management, public refreshes,
  and post/avatar/media updates.
- Four auth requests now share generation allocation, in-flight/status updates,
  and result-message mapping. Each caller retains its validation, zeroizing
  credential preparation, and cleanup sequence. Code/password callers capture
  the challenge ID before the shared helper allocates the result ID. Sign-out
  still clears pending challenges before API ID validation and invalidates the
  private-channel scan only after validation succeeds.
- Login requests and private-channel scan admission reuse the existing
  `signed_in()` state predicate. Password-required results move their owned hint
  into state rather than cloning it; display sanitization stays unchanged.
  Result handling, signed-in/signed-out cleanup differences, request/reconnect
  guards, and subscription identity are preserved.
- Added two table-driven regressions before production edits covering all four
  auth entry points, normal and saturated request generations, retained challenge
  state, selective input clearing, private-scan invalidation, and invalid API ID
  behavior. Tests use synthetic credentials and the existing pending-auth test
  lock; real request tasks are inspected without being polled. Moved the private
  sign-out status-helper test beside its helper.
- Corrected the Telegram feature guide's obsolete public-only claims and
  documented existing onboarding, private-channel selection, stored preferences,
  runtime credentials/session storage, sign-out outcomes, and public polling
  admission. In particular, the timer remains active while a healthy fast
  connection suppresses background public fetching. Updated the integration
  guide and code map. No schemas, message routes, subscriptions, external request
  payloads, dependencies, assets, or view layouts changed.

Validation (using the local ALSA prefix documented above):

- Focused Telegram update tests: **72 passed** before production edits with
  `cargo test --locked -j 2 --package kerosene --bin kerosene feed_update::telegram::tests`,
  and **72 passed** after refactoring with the `feed_update::telegram::` filter
  (including the relocated helper test).
- `cargo test --locked -j 2`: **4,414 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- Source comparison verified 32 untouched function bodies and all 76 original
  test/helper bodies byte-for-byte. The auth-result handler differs only in
  moving its hint; the four request callers and shared signed-in admission were
  reviewed separately for validation, credential, and generation ordering.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-28: separate Telegram public client and consolidate image fetching

- Reviewed public HTTP requests, response bounds, image validation, HTML/profile
  parsing, channel/text normalization, and their model/update/view callers.
  Moved public requests and HTML parsing into `telegram_feed/client.rs`, keeping
  the three existing fetch paths available through root re-exports. Feed state,
  redacted debug output, shared text/image helpers, and normalization remain in
  the root. Model and client tests now live beside their respective modules.
- Avatar and media downloads share one request/status/body/signature path. The
  callers still normalize channels first and retain separate 512 KiB/2 MiB
  limits and exact error context. Request headers, timeout, observed transport,
  status-before-body precedence, declared/chunked body checks, and signature-based
  image acceptance are unchanged. Missing content-type errors use a borrowed
  fallback string.
- HTML attribute and inner-fragment extraction now return borrowed slices.
  Normalization still creates owned model strings, while IDs, timestamps, and
  source fragments avoid temporary copies. Kept parsing, media precedence,
  entity decoding, stable ordering, and latest-post selection unchanged. The
  existing limit operation still releases excess vector capacity.
- Added five regressions before the download/parser simplifications: three local
  HTTP tests cover both fetchers' GET/user-agent behavior, image signatures versus
  declared content type, missing headers, channel/URL/status/read error ordering,
  exact size limits, and declared/chunked overflow. Two parser tests cover equal
  sort keys, limit edges, pre-epoch timestamps, channel normalization, Unicode
  profile metadata, entities, and missing/invalid metadata fallbacks. Live
  Telegram tests remain ignored; all new HTTP traffic stays on loopback.
- Inspected candidate/chip/post rendering and noted unnecessary owned candidate,
  profile, post, and sparkline copies for the next batch. Updated the feature and
  integration guides. No schemas, message routes, subscriptions, dependencies,
  assets, session behavior, or view layouts changed; no measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene telegram_feed::`:
  **35 passed, 0 failed, 2 ignored** before and after client simplification.
- `cargo test --locked -j 2`: **4,419 passed, 0 failed, 6 ignored** on the final
  Rust changes; doc-tests passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed.
- Source comparison verified all 74 named production function bodies before
  simplification and 68 untouched bodies afterward, including public feed fetch,
  bounded reads, sorting/limiting, normalization, and raster validation. All
  redacted debug implementations and existing raw HTML fixtures match exactly;
  existing test bodies match apart from indentation.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-28: borrow Telegram view data and share avatar rendering

- Reviewed Telegram live-feed controls, private-channel candidates, profile/post
  cards, image placeholders, impact chips, canvas drawing, and style helpers.
  Views now borrow posts and profiles instead of cloning entire records with
  their mentions, request metadata, and media state. Body/title/status text and
  cached initials borrow feed state; generated labels remain owned where needed.
- Private candidate selection returns references, retaining the same scan order,
  duplicates, and selected-peer filter. Collapsed lists no longer copy candidate
  titles/image handles. Expanded rows borrow candidate titles and retain the
  same add-channel message. Channel removal and clipboard messages still own
  their payloads, and image widgets still own cloned image handles.
- Shared the loaded-image/initials avatar path across profile and private
  candidate rendering, preserving fallback characters, uppercase conversion,
  clipping, sizing, and theme styling. Media placeholders avoid preparing an
  unused clipboard URL; loaded media keeps its existing copy-link action.
- Impact chips compute their tooltip before consuming the prepared symbol,
  ticker, and sparkline values, eliminating a second set of copies. Mention
  resolution, freshness, filtering, percentage calculations, row grouping, and
  sparkline generation/drawing remain unchanged. Removed the private scan
  button's always-Some wrapper and its callers' redundant optional branches.
- Moved 29 style/padding/color helpers unchanged into
  `feed_views/telegram/styles.rs`. Added three regressions before production
  edits: candidate selection across empty/partial/full/unknown selections;
  tooltip alias/suppression behavior; and private-channel/media view construction
  across image/initials, collapsed/expanded, loaded/pending/failed, and
  captioned/media-only cases. Existing screen/impact tests remain unchanged.
- Reviewed sign-in view helpers for the next pass, including static country-code
  options, code-cell digit preparation, and fixed informational text. Updated
  feature/integration docs. No schemas, message routes, subscriptions, session
  behavior, dependencies, assets, labels, layout dimensions, or canvas geometry
  changed; no measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene telegram`:
  **138 passed, 0 failed, 2 ignored** before and after ownership changes.
- `cargo test --locked -j 2`: **4,422 passed, 0 failed, 6 ignored** on final Rust
  changes; doc-tests passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo build --locked -j 2`: passed.
- Source comparisons verified all 29 relocated helper bodies, 33 unchanged
  view/helper bodies, both canvas Program implementations, and all original
  model/view test bodies. Layout modifier sequences match across the 12 changed
  view builders; candidate filtering differs only by dropping cloning.
- Headless smoke: the debug binary ran with `--test` under Xvfb, opened its
  1600x960 Kerosene window, and exited at the expected 20-second timeout without
  panic markers. This checks startup with in-memory configuration, not a pixel
  comparison of Telegram content or hardware GPU rendering.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-28: simplify Telegram sign-in preparation and separate fast media helpers

- Finished the sign-in view allocation review. The country picker borrows its
  fixed dialing-code slice and selected string, allocating the selected value
  when constructing the existing change message. Code cells iterate characters
  directly and count through a cloned iterator instead of collecting a vector;
  the five-cell limit, character semantics, caret condition, and input events
  remain unchanged. Informational cards, notes, and status chips borrow text.
- Expanded the existing screen-construction regression before production edits
  to cover several dialing codes (including a value outside the fixed list) and
  empty, partial, complete, oversized, and Unicode code inputs. Kept the same
  labels, dimensions, styles, secure-input behavior, and message variants.
- Traced the fast-feed connection/health loop, background channel resolution,
  backfill, live post conversion, cursor generations, and client/session cleanup.
  Retained their separate delivery/recording/scheduling order and task-owned
  snapshots: live delivery records its cursor before spawning a media follow-up,
  whereas backfill schedules its media jobs before recording the cursor.
- Extracted media classification, private avatar/preview download helpers, size
  limits, and asynchronous follow-up events into `telegram_fast_feed/media.rs`.
  Removed the single-use chat-photo forwarding function by calling the same
  bounded downloader with the same avatar limit. Classification, thumbnail
  selection, byte accumulation, timeout/semaphore behavior, failure events, and
  stream/session orchestration otherwise remain unchanged. Moved inline fast-feed
  tests to `telegram_fast_feed/tests.rs` without changing their behavior.
- Updated feature/integration documentation. No schemas, message routes,
  subscriptions, credentials/session formats, dependencies, assets, layout
  behavior, or canvas geometry changed; no measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene telegram`:
  **138 passed, 0 failed, 2 ignored** before and after the simplifications.
- `cargo test --locked -j 2`: **4,422 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo build --locked -j 2`: passed.
- Source comparison verified 69 retained fast-feed function bodies exactly;
  the avatar helper only substitutes its forwarding call. Relocated fast-feed
  tests match rustfmt output exactly (including a collapsed fixture closure).
  The five changed view builders retain their layout modifier sequences, and
  all other view/helper bodies are unchanged.
- Headless smoke: the debug binary ran with `--test` under Xvfb, opened the
  1600x960 Kerosene window, and reached the expected 20-second timeout without
  panic markers. This checks startup with in-memory configuration, not a pixel
  comparison or live Telegram authentication.
- `cargo fmt -- --check` and `git diff --check`: passed.

## 2026-09-28: separate Telegram auth/session helpers and cache candidate sort keys

- Extracted authentication, bundled-credential lookup, pending challenge storage,
  and sign-out outcome handling into `telegram_fast_feed/auth.rs`. Extracted
  session paths, SQLite retries, file permissions/removal, short-lived client
  serialization, and pool shutdown into `telegram_fast_feed/session.rs`.
  Existing callers continue through the root exports; stream orchestration,
  private scans, resolution/backfill, and cursor generations remain in the root.
- Kept authentication validation, challenge/result request IDs, zeroizing inputs,
  registry locking, client-operation serialization, timeout/retry values,
  remote/local sign-out ordering, and cursor invalidation unchanged. Moved
  auth/session tests beside their implementations without changing their bodies
  or the shared test lock used by pending-auth registry fixtures.
- Added a candidate-order regression against the original comparison sort, then
  switched to cached ASCII-folded title/peer-ID keys. The regression covers
  case ties, peer-ID tie-breaking, non-ASCII titles, retention of the first
  adjacent duplicate and its avatar, and retention of nonadjacent repeated IDs.
  Private dialog admission, scan timeout, avatar downloads, and scan limit remain
  unchanged. The session file family now uses a four-element array, avoiding its
  temporary vector while preserving path values and iteration order.
- Updated the integration and feature guides. No schemas, message routes,
  subscriptions, credentials/session formats, dependencies, assets, or UI changed;
  no measured speedup claimed. Authentication lock-scope consolidation remains
  a follow-up rather than being folded into the module move.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene telegram`:
  **139 passed, 0 failed, 2 ignored** before and after the production refactor.
- `cargo test --locked -j 2 --package kerosene --bin kerosene config::tests::clear`:
  **12 passed, 0 failed**. These cover all four Telegram session files, unrelated
  file preservation, and redacted cleanup failures.
- `cargo test --locked -j 2`: **4,423 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Source comparison verified 75 unchanged production function bodies after the
  sort/dedup helper extraction; only cached sorting and the path array differ.
  All 18 test/fixture bodies match the baseline, including the new candidate
  regression. All platform permission blocks and their cfg attributes are
  preserved exactly. Live Telegram authentication was not exercised.

## 2026-09-28: simplify pending Telegram auth ownership and remove nested locking

- The lock-scope follow-up found that both submission handlers matched directly
  on `pending_auths().lock()?.remove(...)`. The temporary guard survives through
  the match arms, so the wrong-challenge arm attempted to acquire its own lock
  again while restoring the challenge. A standalone Rust probe confirmed the
  guard lifetime, and a regression against the public login-code submission
  reproduced the deadlock before production changes.
- Each handler now takes one explicit registry guard, moves the session path into
  its request key, removes the challenge, and restores a mismatched challenge
  through that same guard. The block ends before client/network work. This
  removes redundant locking, enum reconstruction, and path copies while making
  the existing wrong-stage error reachable. Initial login-code requests also
  move their final session-path use into the registry instead of cloning it.
- The new regression constructs a synthetic password challenge and submits a
  login code without contacting Telegram or opening a session file. It checks
  the exact error, original token identity and request/session key, and unrelated
  challenges. It runs in a subprocess with a 10-second deadline so recurrence
  fails without locking the rest of the test suite. The baseline failed at that
  deadline; the changed code completes normally. Existing auth tests are intact.
- Traced wallet tracker config normalization/local persistence, queued versus
  automatic selection, loading/backoff/age admission, compact position bias,
  and refresh task setup. Selection currently copies every eligible core address
  before taking a bounded batch and copies each successive best order candidate.
  These can borrow until final selection; single-core refresh can reuse its
  existing entry lookup. Queue ordering, stable timestamp ties, different
  core/order prerequisites, and account persistence snapshots need preservation
  in that follow-up. No wallet/account production changes in this batch.
- Updated auth documentation. Error strings, input validation, zeroizing
  buffers, request generations, asynchronous result handling, session cleanup,
  and cursor invalidation remain unchanged. No schema, routing, subscription,
  dependency, asset, or UI changes.

Validation (using the local ALSA prefix documented above):

- The new exact regression **failed before the fix** with the expected 10-second
  deadlock deadline, then passed after the lock-scope change.
- `cargo test --locked -j 2 --package kerosene --bin kerosene telegram`:
  **140 passed, 0 failed, 2 ignored** on final Rust changes.
- `cargo test --locked -j 2`: **4,424 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Source comparison confirms all 14 unaffected auth helper bodies, both network
  submission/result-handling blocks, and all existing auth test/fixture bodies
  are unchanged. Live Telegram authentication was not exercised.

## 2026-09-28: borrow wallet refresh candidates and simplify queue setup

- Automatic core selection now sorts borrowed addresses and copies only the
  selected batch. Automatic order selection borrows its current best candidate
  and copies only the final address. Stable timestamp ties, initial-load and
  staleness rules, retry deadlines, loading barriers, queued FIFO work, and the
  distinct core/order snapshot prerequisites are unchanged.
- Refresh-all rebuilds the core queue directly from borrowed tracked addresses,
  avoiding a copy of the full tracked list and redundant tracked-membership
  scans. It reuses queue capacity, skips core-loading rows, retains first
  occurrence order/deduplication, and still admits missing rows, muted addresses,
  and retry-delayed entries at the queueing stage. Single-address queue helpers
  use direct `contains` checks and one tracker binding.
- Single-core refresh setup reuses the row returned by `entry(...).or_default()`
  to set both loading fields, removing a second lookup and an impossible missing
  row branch. Task-owned addresses, provider/context capture, batch setup, order
  refresh, asynchronous work, result messages, and dispatch remain unchanged.
- Added seven tracker regressions before production changes: automatic core age
  ordering/ties/batch limits; queued core FIFO, skipped entries, and no automatic
  top-up; automatic order age/tie/snapshot rules; queued order snapshot bypass;
  independent global loading barriers before queue draining; queue admission and
  refresh-all deduplication; and new/existing row request-context setup. Fixtures
  use synthetic keys and do not execute network tasks.
- Reviewed queue callers, remote additions/removal, and existing stale-provider
  result tests. Also inspected account bootstrap append helpers, fee parsing,
  fetch-scope normalization, and completeness state/warning helpers. Kept the
  straightforward consuming merge loops and explicit wire defaults; warning
  comparison/summary code contains avoidable copies for the next pass.
- Updated the account/wallet component guide. No schema, routing, subscription,
  provider policy, network behavior, order placement, dependency, asset, or UI
  changes; no measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene wallet`:
  **220 passed, 0 failed, 1 ignored** before and after the production changes.
- `cargo test --locked -j 2`: **4,431 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Source comparison confirms selection changes only candidate/output ownership;
  all refresh task construction, batch/order setup, and dispatch are byte-identical.

## 2026-09-28: reduce account warning and fetch-scope string copies

- Completeness warning insertion now builds one owned section/message entry and
  borrows it for `contains`, removing the full string copy previously made for
  every comparison. Summary preparation collects borrowed message slices rather
  than copying each unique message. Per-section pair deduplication and global
  message deduplication retain their separate roles and original ordering.
- Added two regressions before the warning changes. They cover duplicate and
  shared messages across all sections, distinct case/whitespace/Unicode text,
  first-occurrence summary order, exact section output, empty-warning fallback
  text, and incomplete versus degraded position actionability. Warning strings,
  storage, state flags, fallback text, and returned owned UI strings are intact.
- The selected-DEX fetch-scope constructor now accepts `AsRef<str>`, matching the
  all-markets constructor, and borrows its input before normalization. Both live
  callers pass borrowed strings; the previous conversion made an unnecessary
  owned copy before the normalized key allocation. A regression passed before
  and after this change for string slices, borrowed/owned Strings, blank values,
  surrounding whitespace, mixed ASCII case, and non-ASCII text. Scope variants,
  default fallback, membership, request weight, and refresh interval are unchanged.
- Traced account freshness and its separate per-DEX/order/position/spot timestamps,
  collateral-balance fallbacks, exact-symbol leverage lookup, and fetch-scope
  callers. Kept those policy distinctions explicit. Inspected owned Hydromancer
  portfolio JSON parsing and its consumers; top-level field extraction currently
  clones values that can be moved, which is the next ownership-review target.
- Updated the account/wallet guide. No schemas, message routes, subscriptions,
  freshness thresholds, trading calculations, provider policy, network behavior,
  dependencies, assets, or UI changes; no measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene account::types::data`:
  **28 passed** before/after warning changes, then **29 passed** before/after the
  scope-constructor change; no failures or ignored tests.
- `cargo test --locked -j 2`: **4,434 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Source comparison confirms 7 other completeness helper bodies and 10 other
  fetch-scope helper bodies are unchanged. All pre-existing completeness and
  fetch-scope test bodies are preserved.

## 2026-09-28: separate portfolio parsing and move owned response data

- Extracted the Hydromancer portfolio model, redacted Debug implementation,
  single/batch parsing, native/DEX merge, scoped conversion, and batch-limit
  helper into `account/data/bootstrap/hydromancer/portfolio.rs`, with tests in
  its `portfolio/tests.rs`. Request orchestration remains in the parent module;
  existing callers retain their model and batch-limit exports.
- Single response parsing now takes fields from its owned JSON value instead of
  deep-cloning them. Batch parsing takes the selected aliased arrays, consumes
  each two-value tuple, and moves its address/payload into the result. Failed
  wallet strings also move into the output. A private field-taking helper keeps
  the existing primary-field precedence, including present null/malformed values.
- Preserved missing-versus-null behavior, abstraction defaults, tuple validation
  order, arbitrary payload retention, duplicate/order behavior, invalid optional
  failure filtering, whole-batch errors, native metadata selection, and redaction.
  Added five regressions before production changes covering those cases; existing
  scoped conversion, chunk-limit, and redaction tests moved unchanged.
- Batch request helpers borrow the public task's owned address list and serialize
  each chunk directly, avoiding both the full-list copies and intermediate chunk
  vectors. Request setup uses the existing scope-limit helper instead of repeating
  100/500 literals. Sequential chunk processing, concurrent native/DEX requests,
  payload fields, error handling, address normalization, and result assembly retain
  their previous behavior. Shared map-value copies remain because removing an
  entry could change results for repeated or case-equivalent input addresses.
- Updated the account/wallet guide. No schema, routing, subscriptions, network
  endpoint/auth, request limits, trading calculations, dependencies, assets, or UI
  changes; no measured speedup claimed. Model getters still clone raw JSON for
  deserialization, which warrants a separate review of borrowing and error parity.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene account::data::bootstrap::hydromancer`:
  **11 passed, 0 failed** before and after the refactor.
- `cargo test --locked -j 2`: **4,439 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Source comparison confirms 18 other production helper bodies are unchanged;
  batch fetch bodies differ only in shared limits and address ownership. All
  relocated tests/fixtures match the baseline after rustfmt, and redacted Debug
  is preserved exactly. Live authenticated API requests were not exercised.

## 2026-09-28: borrow retained account JSON during model conversion

- Portfolio clearinghouse and spot getters now deserialize directly from borrowed
  JSON values. Native bootstrap parsing and wallet snapshot conversion use the
  same approach, removing full JSON tree copies while retaining owned models and
  original response values needed for previews or independent balance parsing.
- Wallet snapshot conversion reads margin usage before moving parsed positions
  into aggregation, removing a second copy of every position. HTTP handling,
  raw equity/withdrawable extraction, auxiliary spot valuation, HIP-3 aggregation,
  provider fallback, warnings, and snapshot calculations retain their behavior.
- Added five regressions before production changes. They preserve exact nested
  type/range/tuple errors, missing/null defaults, native-key/direct-shape precedence,
  selected/missing/unselected DEX handling, normalized symbols, independent returned
  models, repeated getter access, and bootstrap error detail/original JSON previews.
  Existing preview-redaction and portfolio-margin wallet tests also pass.
- Checked the locked serde_json 1.0.149 owned/reference deserializers and Serde
  1.0.228 String visitor against these derived model types. Kept consuming parses
  where callers no longer need raw JSON, and retained owned model copies required
  by the separate native/map/HIP-3 return values.
- Updated the account/wallet guide. No schema, routes, subscriptions, network
  request policy, trading calculations, dependencies, assets, or UI changes; no
  measured speedup claimed. Authenticated API requests were not exercised.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene account::data::bootstrap`:
  **26 passed, 0 failed** before production changes, including all five new tests.
- `cargo test --locked -j 2 --package kerosene --bin kerosene account::`:
  **201 passed, 0 failed** after the refactor.
- `cargo test --locked -j 2`: **4,444 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Source comparison confirms wallet HTTP handling and everything from raw headline
  parsing onward are unchanged, including all other tracker helpers. Required
  response changes are limited to the deserializer/import; portfolio production
  changes are limited to borrowed inputs/deserialization and the trait import.

## 2026-09-28: borrow scoped DEX lists and joined account request inputs

- `AccountDataFetchScope::hip3_dexes` now returns borrowed string slices instead
  of copying every DEX name. A flat match preserves the caller-supplied fallback
  for an empty all-markets list and the exact stored values for other scopes.
  Constructor normalization, ordering, duplicates, request weights, refresh
  intervals, and main-order eligibility retain their previous behavior.
- Hydromancer single-portfolio and scoped-order helpers borrow addresses, scopes,
  and keys from their callers. The vector-response helper also borrows the key.
  Joined requests no longer copy inputs per DEX, and JSON payload construction
  borrows address/DEX strings directly. Enclosing tasks retain their owned inputs
  and Zeroizing keys until joined work completes; task and fallback boundaries
  continue to own the state they need.
- Wallet detail response lists borrow their DEX labels through conversion. Names
  become owned only in returned wallet rows or account maps/freshness records.
  Existing native bootstrap and tracker request builders use the borrowed list
  directly without changing their request/result code. Required native/model
  copies and repeated batch-result ownership remain intact.
- Added four regressions before production changes: stored order/duplicates/raw
  values versus constructor normalization, exact empty-list fallback (including
  caller-owned strings), selected scopes ignoring fallback, and portfolio result
  order/repeated entries with missing DEX states skipped. Updated two existing
  market-registry tests to compare borrowed names.
- Reviewed all list/helper callers and the locked serde_json 1.0.149 JSON macro
  and reqwest 0.12.28 request serializer. Payload bytes are owned by requests;
  async borrowing remains within joined tasks. Request groups, payload fields,
  endpoints, auth formatting, response/error policy, and fallback are unchanged.
- Updated the account/wallet guide. No schema, message routes, subscriptions,
  trading calculations, dependencies, assets, or UI changes; no measured speedup
  claimed. Live authenticated API requests were not exercised.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene account::`:
  **205 passed, 0 failed** before and after the refactor.
- `cargo test --locked -j 2`: **4,448 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests), including the market-registry callers.
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Source comparison confirms 20 other production helper bodies and all diagnostic
  strings in the four request modules are unchanged. Production diff review
  confirms the remaining changes concern input/result ownership and construction.

## 2026-09-28: consolidate wallet config restoration and label collection

- Wallet tracker restoration now chains current and legacy addresses through one
  normalization/deduplication helper and uses it independently for muted entries.
  It retains first-seen order, case/whitespace normalization, invalid-address
  filtering, legacy inclusion regardless of label, and untracked muted entries.
  Remaining initialization and model/persistence methods are unchanged.
- Local and combined local/remote label lists use one collection/sort/deduplication
  helper. The combined list now needs one sorting pass instead of sorting each
  source and then sorting again. Subscription selection reuses that combined list
  and filters muted addresses. Removed the obsolete per-book subscription helper;
  its existing regression now exercises the actual terminal subscription method.
- Removed three identical count-redaction wrapper types from wallet config. Debug
  implementations use the existing `format_args!` pattern already used by runtime
  address-book and cluster state. Fields, redaction text, defaults, and config wire
  definitions remain unchanged.
- Added three regressions before production changes: tracker list ordering and
  independent muted normalization, combined label/remote overlap and subscription
  filtering, and exact empty/populated count redaction including pretty Debug.
  Cases cover blank labels, metadata without labels, duplicate mutes, independent
  owned results, empty sources, and preserved initial tracker metadata/state.
- Inspected address-book config conversion, import/export, display helpers, tracker
  persistence, and remote database state/read helpers. Kept config/legacy label
  precedence, import fill-only merging, metadata normalization, remote exclusion
  from persistence, pagination validation, and whole-snapshot failure policy
  explicit. Display helpers still repeat normalization and merit further review.
- Updated the account/wallet guide. No config schema, routes, subscription identity,
  network policy, trading calculations, dependencies, assets, or UI changes; no
  measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene wallet`:
  **223 passed, 0 failed, 1 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,451 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- `cargo build --locked -j 2`: passed. Headless `--test` startup under Xvfb
  opened a 1600x960 window, reached the expected 20-second timeout (124), and
  produced no panic markers. The smoke test used in-memory configuration.
- Source comparison confirms all 9 public wallet config struct definitions and
  field attributes are unchanged, as are tracker initialization after the lists
  and the remaining model/persistence methods.

## 2026-09-28: share wallet label lookup and simplify row display ownership

- Wallet display and label entry points now normalize their address once and
  share a lookup of the first nonblank label, checking the remote book before the
  local book. Removed the two superseded single-book helper methods after tracing
  all callers. Their existing display/redaction tests now exercise the terminal's
  actual display method. Unlabeled display construction moves its short text.
- Added two regressions before production changes covering normalized/mixed-case
  addresses, Unicode whitespace, local/remote/missing/blank labels, remote
  membership independent of label presence, exact primary/secondary strings,
  invalid book keys, raw invalid-input preservation, the ten-character shortening
  boundary, and multibyte text. Invalid addresses cannot acquire labels from
  matching malformed book keys. Normalization and shortening implementations are
  unchanged, as are display types and their redacted Debug implementations.
- Tracker row rendering now borrows loaded row state with an empty fallback and
  computes remote membership once. Its identity cell consumes the prepared
  display's label and address strings, removing a repeated label lookup, the
  redundant label parameter, and display-string copies. The existing widget
  construction, actions, tooltip text, hover keys, styles, dimensions, metrics,
  and status precedence retain their behavior.
- Reviewed the tracker row's metrics, status, identity, and action helpers and
  display consumers in wallet, feed, cluster, and portfolio views. Retained owned
  strings where widgets/tasks require them and left calculations and display
  policies intact. Updated the account/wallet guide.
- No schemas, message routes, subscription identities, network behavior, trading
  calculations, dependencies, assets, or visible UI changes; no measured speedup
  claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene wallet`:
  **225 passed, 0 failed, 1 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,453 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- `cargo build --locked -j 2`: passed. Headless `--test` startup under Xvfb
  opened a 1600x960 window, reached the expected 20-second timeout (124), and
  produced no panic markers, using in-memory configuration.
- Source comparison confirms identity widget construction, remote membership,
  symbol formatting, normalization/shortening, and redacted display types remain
  unchanged. Production diff review confirms row preparation changes only
  ownership and reuse of the same values.

## 2026-09-28: move wallet detail rows and share wallet-add completion

- Hydromancer wallet-detail conversion now consumes its temporary per-DEX map,
  moving positions into detail rows, and consumes the fetched order vector.
  This removes intermediate position/order copies. The separate native snapshot,
  source row order, DEX metadata, field values, warnings, and provider behavior
  remain intact. Exact known-DEX membership uses `contains` instead of an
  equivalent `iter().any` predicate.
- New-wallet addition and muted-wallet restoration now share input clearing,
  deferred persistence, refresh queueing, and refresh dispatch. The branches
  retain their different policies: restoration preserves an existing row,
  protects remote labels, and always refreshes the tracked-trade subscription;
  a new wallet resets its row and refreshes that subscription only for a nonblank
  submitted label. Invalid/already-visible inputs retain their early returns.
- Added four regressions before production changes. Three cover new/restored
  rows with blank/nonblank labels, metadata preservation, unmuting, blocked and
  immediate refresh, request context, save scheduling, input clearing, rejected
  input preservation, and exact toast text. Existing remote-label protection tests
  also pass. The fourth covers exact known DEX prefixes, native/spot/outcome names,
  unknown or differently cased prefixes, and empty/multiple-colon suffixes.
- Reviewed wallet detail task/window setup, result identity/context checks,
  multi-window websocket handling, and tracker entry editing/removal. Retained
  snapshot copies needed by separate returned fields or multiple windows, the
  existing spot cost-basis policy, and all non-add entry handlers.
- Updated the account/wallet guide. No schemas, routes, subscription identities,
  network request policy, trading calculations, dependencies, assets, or UI layout
  changes; no measured speedup claimed. Live authenticated provider requests were
  not exercised.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene wallet`:
  **229 passed, 0 failed, 1 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,457 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- A local Rust 1.96.1 probe compared borrowed/cloned and consuming conversion over
  600 maps with varied sizes, capacities, empty entries, repeated values, and
  deletions; every resulting row sequence and value matched.
- Source comparison confirms detail production changes are limited to ownership
  and the equivalent membership check. HTTP, fallback, parsing, fill-policy code,
  and all tracker handlers following the add branch are unchanged.

## 2026-09-28: consolidate wallet detail stream completion

- Position, order, balance, and fill events now share matching-window selection,
  snapshot timestamps, window refresh timestamps, and error clearing through
  `update_wallet_detail_snapshots`. Event-specific filtering and mutations stay
  in their branches, and each loaded window retains independent owned data.
  Windows still awaiting an initial snapshot receive timestamp/error updates
  without completing or invalidating pending REST requests.
- Detail-fetch completion moves its captured address into the result message.
  Lag recovery passes its task iterator directly to `Task::batch`, avoiding a
  temporary task vector. Checked the locked iced runtime 0.14.0 implementation:
  `Task::perform` accepts `FnOnce`, and `Task::batch` consumes its iterator
  synchronously. Request arguments, scheduling, scope/context capture, and lag
  admission remain intact.
- Moved the existing fill tests into `wallet_state/details/tests.rs` and added
  seven regressions before production changes. They cover matching and unrelated
  windows, source normalization, missing initial snapshots, shared timestamps,
  pending requests, independent snapshots, position/margin fields, source row
  order, DEX order replacement/normalization, hidden-symbol filtering, spot
  margin metadata, fill deduplication/first occurrence, all-mids routing, and
  lag recovery across idle and loading windows.
- Reviewed detail window setup/refresh, result identity/context guards, user-data
  subscription identity, and detail-window close handling. Also reviewed cluster
  runtime/config models, member limits/weights, execution retention, and stream
  updates. Cluster position freshness and lag invalidation remain separate
  because they gate trading; order/balance frames cannot refresh position age.
- Updated the account/wallet guide. No schema, message route, subscription
  identity, request policy, trading calculation, dependency, asset, or UI layout
  changes. No measured speedup claimed; no authenticated live provider requests.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene wallet`:
  **236 passed, 0 failed, 1 ignored** before and after stream consolidation.
- `cargo test --locked -j 2` on the final production change:
  **4,464 passed, 0 failed, 6 ignored**; doc-tests passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- `cargo build --locked -j 2`: passed. Headless `--test` startup under Xvfb
  opened a 1600x960 window, reached the expected 20-second timeout (124), and
  produced no panic markers, using in-memory configuration.
- Source comparison confirms the four event-specific mutation bodies and fill
  merge helper are unchanged. Window/task setup differs only in moving the
  captured completion address and the resulting rustfmt layout; final diff
  review confirms lag admission, state updates, and task inputs are unchanged.

## 2026-09-28: separate wallet cluster update responsibilities

- Reduced `wallet_cluster_update.rs` from roughly 1,900 lines including tests to
  a 154-line message router and shared preparation types. Five child modules
  own cluster/member editing (`management.rs`), snapshot requests/results and
  streams (`data.rs`), order/close planning and member eligibility (`orders.rs`),
  dispatch/result reconciliation (`execution.rs`), and position aggregation and
  close sizing (`positions.rs`). Implementation modules range from 173 to 391
  lines; tests live beside their subject.
- Moved 25 methods and three free helpers. All 30 production function bodies,
  including the two retained router/status methods, match the original source
  exactly. Signatures retain their types and parameters; moved private entry
  points are exposed only within this feature. Shared preparation types and
  constants are unchanged. Existing tests/fixtures preserve their bodies after
  formatting at their new indentation.
- Added three result regressions before moving production code. They cover
  resting responses for limit/market/IOC orders, fills, rejection, ambiguous
  acknowledgements, transport failure, open/filled/rejected/canceled/unknown
  status replies, error redaction, progress reporting, and independent legs.
  Both result routes require execution/member/cloid identity to mutate a leg.
- Reviewed management persistence and membership guards, read context/address
  validation, position freshness, weighted allocation and reduce-only/close
  checks, captured profile signing context, dispatch, and reconciliation. Kept
  these policies and their evaluation order intact. Position aggregation and
  the optional-value accumulation policy are also unchanged.
- Updated the account/wallet component map and cluster guide. No config schema,
  message route, subscription identity, request policy, trading calculation,
  dependency, asset, or UI layout changes; no live signed orders or authenticated
  provider requests were sent.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene wallet_cluster`:
  **18 passed, 0 failed, 0 ignored** before and after the module split.
- `cargo test --locked -j 2`: **4,467 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- `cargo build --locked -j 2`: passed. Headless `--test` startup under Xvfb
  opened a 1600x960 window, reached the expected 20-second timeout (124), and
  produced no panic markers, using in-memory configuration.
- Source comparison verifies all production bodies, function signatures/types,
  constants, shared structs, and the 12 existing test/fixture bodies, allowing
  only module visibility and formatting where needed.

## 2026-09-28: share cluster refresh selection and copy only request IDs

- Full-cluster and single-member refreshes now share selected-cluster lookup and
  member selection. The common path borrows cluster/member metadata, then copies
  only the selected cluster ID and matching profile IDs before mutating state.
  This removes whole-cluster clones, unused name/input-draft copies, and the
  second member scan used by single-member refreshes.
- The selected IDs retain source order and repeated entries. Refreshing an empty
  selected cluster still reaches the existing empty batch path; a missing
  selection or unmatched single member still returns immediately. Read refreshes
  continue to include zero-weight and already-loading members. Invalid-address
  rows consume their selected ID rather than cloning it again.
- Added four regressions before production changes, covering full and targeted
  refreshes, repeated valid/missing IDs, task counts, normalized addresses,
  zero-weight members, pending requests, current request context, cached
  snapshots/warnings/position timestamps, invalid-address resets, unrelated
  rows, missing selections, empty clusters, unchanged membership/drafts, and
  exact singular/plural status messages.
- Reviewed callers in cluster management, boot restoration, freshness recovery,
  stream lag recovery, and execution reconciliation. Their admission and ownership
  boundaries remain intact. Task construction still owns identity, scope, provider
  credentials, and context for asynchronous completion.
- Updated the account/wallet guide. No schema, message route, subscription
  identity, request policy, trading calculation, signing, dependency, asset, or
  UI layout changes. No measured speedup claimed; no live provider requests or
  signed orders were sent.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene wallet_cluster`:
  **22 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,471 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- `cargo build --locked -j 2`: passed. Headless `--test` startup under Xvfb
  opened a 1600x960 window, reached the expected 20-second timeout (124), and
  produced no panic markers, using in-memory configuration.
- Source comparison confirms unchanged fetch-task construction, read-result and
  stream handlers, and freshness checks. The dispatch loop differs only in
  equivalent member-ID references, ownership, and rustfmt layout; missing-profile
  counting, row updates, task arguments, and status text remain unchanged.

## 2026-09-28: separate cluster views and avoid discarded UI copies

- Split the 739-line cluster view into a 128-line window shell and four focused
  sections: `members.rs`, `ticket.rs`, `positions.rs`, and `executions.rs`.
  Production child modules range from 97 to 226 lines. Existing header/layout
  helpers remain with their section, and feature-private visibility is retained.
- The shell passes its already-selected cluster to the members section, removing
  a copied ID and a repeated search. Status, profile-name, snapshot-state, and
  execution label/message text now borrow application state; callback messages
  still own their IDs. Execution kind labels use a local enum match in place of
  a one-method private trait.
- Close-button construction borrows the symbol and copies it only for an enabled
  message. The ticket constructs its price input only outside market mode. Button
  order, enablement, fractions, order kinds, labels, formatting, layout, colors,
  and all message values remain intact.
- Added two baseline-tested headless view regressions. One renders and clicks all
  six close controls for long-only, short-only, hedged, empty, and threshold-size
  positions, verifying disabled controls and exact message fields. The other
  renders empty and populated windows, market/limit tickets, member errors,
  order/close execution labels, and all leg-status colors.
- Compared eight synthetic-data previews before and after the production change;
  every PNG is byte-identical. Preview generation is opt-in through the test-only
  `KEROSENE_CLUSTER_PREVIEW_DIR`; artifacts remain outside the repository.
- Updated the account/wallet component guide. No state/config schema, route,
  subscription, network request, trading calculation, signing, dependency, or
  bundled asset changes. No measured speedup claimed; no live provider requests
  or signed orders were sent.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene wallet_cluster`:
  **24 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,473 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo check --locked -j 2`, `cargo fmt -- --check`, and
  `git diff --check`: passed.
- `cargo build --locked -j 2`: passed. Headless `--test` startup under Xvfb
  opened a 1600x960 window, reached the expected 20-second timeout (124), and
  produced no panic markers, using in-memory configuration.
- Source comparison verifies seven view bodies are unchanged. The remaining six
  methods contain only the reviewed borrowing, direct cluster selection, local
  kind-label match, and visible-widget construction changes. The close-button
  helper retains identical message fields while owning symbols only when used.
  Production message constructors and nonempty string literals are preserved;
  only the empty placeholder for the now-unnecessary cluster lookup is removed.

## 2026-09-28: share cluster leg construction and move owned execution inputs

- Order and close planning now share `PreparedClusterLeg::new`, retaining the
  original request/context construction and every field mapping. The caller's
  recorded direction remains a separate input from the prepared request's
  direction. Removed unused `Clone` derives from the two private planning types.
- Execution dispatch moves each captured signing key and completion context into
  its task. Only the client-order ID is copied for the independent execution
  history row. Task arguments, callback fields, result handling, member order,
  account targeting, and spot-balance invalidation remain unchanged.
- Position aggregation borrows source symbol names and copies only when creating
  a distinct summary. Grouping, arithmetic order, stable sorting, member labels,
  mid-price fallback, and optional-total semantics are unchanged. Documented the
  existing rule that a missing value clears the total and a later known value
  starts a new total.
- Added four regressions before production changes: two cover aggregation,
  ordering, skipped sizes, zero-weight members, missing values, label fallback,
  independent results, and empty selection; two cover execution identity,
  leg order, counter rollover, exact status messages, empty preparation, and
  connected-account spot invalidation. Added a constructor regression after the
  change for request/context fields, captured subaccount targets, order kinds,
  market types, and the separate recorded/request directions.
- Updated the account/wallet guide. No schema, route, subscription, request
  policy, signing implementation, dependency, asset, or UI changes. Dispatch
  tests inspect unpolled tasks; no exchange requests were sent by these tests.
  No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene wallet_cluster`:
  **28 passed, 0 failed, 0 ignored** before production changes; **29 passed,
  0 failed, 0 ignored** afterward, including the new constructor regression.
- `cargo test --locked -j 2`: **4,478 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Source comparison verifies exact shared-constructor field mapping and confirms
  every other planning guard, calculation, callback, route, and result handler
  is unchanged apart from the reviewed ownership changes and explanatory comment.

## 2026-09-28: borrow wallet detail positions and share table containers

- Wallet detail summary and table preparation now use borrowed stored position
  rows with owned synthesized spot rows. The helper remains eager, preserving
  spot synthesis before caller iteration, source row order, and spot append order.
  Its visibility is narrowed to the wallet-view feature.
- The positions table retains visible rows in its existing buffer and caches the
  DEX-qualified symbol sort keys. Filtering predicates, stable equal-key order,
  invalid-size presentation, symbol labels, row construction, and selection
  messages are preserved. Summary arithmetic and spot pricing/cost-basis code
  are unchanged.
- Positions, orders, and spot balances now share their identical table container
  through `wallet_views/style.rs`; padding, fill width, background, and border
  values are unchanged. Error and warning text borrow the window state/snapshot.
- Added two regressions before production changes. One checks stored row fields
  and order, synthesized spot values, unavailable cost basis/prices, and skipped
  balances. The other renders the positions table and loading, waiting, error,
  populated, and empty detail windows, and clicks every visible position symbol
  to verify exact selection messages and ordering. Fixtures include zero/tiny
  sizes, invalid sizes, duplicate symbols, muted symbols, DEX-qualified symbols,
  spot balances, orders, and warnings.
- All six synthetic previews are byte-identical before and after the change.
  The populated preview was also inspected visually. Preview generation uses the
  opt-in test-only `KEROSENE_WALLET_DETAIL_PREVIEW_DIR`; artifacts remain outside
  the repository. Updated the account/wallet guide.
- No config/schema, route, subscription, request, signing, dependency, or bundled
  asset changes. No measured speedup claimed. Account position projection and
  sort-row ownership were inspected as follow-up candidates, without changing
  their live-account completeness, fee, or action policies.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene wallet_views`:
  **32 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,480 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo check --locked -j 2`, `cargo fmt -- --check`, and
  `git diff --check`: passed.
- `cargo build --locked -j 2`: passed. Headless `--test` startup under Xvfb
  opened a 1600x960 window, reached the expected 20-second timeout (124), and
  produced no panic markers, using in-memory configuration.
- Source comparison confirms the shared container matches all three original
  wrappers, and preserves order/spot selection, position rendering after row
  preparation, summary calculations, and spot synthesis. Root error and warning
  views differ only in their reviewed text borrows.

## 2026-09-28: borrow account position sections and prepared row inputs

- Perpetual, spot, and outcome section lists now borrow positions from the
  existing projection. `PositionRowData` borrows its asset position and symbol
  instead of cloning wire fields during both display and summary preparation.
  Prepared numeric metrics remain values, with unchanged calculation order.
- Sorting accepts an iterator of position references. Its comparison rules,
  direction handling, symbol tie-breaks, and missing-fee placement are unchanged.
  Replaced the verbose table-scoped visibility spelling with equivalent
  `pub(super)` within the same module.
- Rendering consumes borrowed row data synchronously and keeps independent owned
  text/messages. Upstream account projection, chart overlay conversion, PnL-card
  consumers, close/hide actions, fee completeness gates, summary accumulation,
  and privacy formatting retain their existing policies.
- Added two regressions before production changes. The sort matrix covers all
  12 columns in both directions, unavailable values, stable equal-symbol ties,
  and unchanged source order. The headless regression renders all three sections
  at narrow/wide widths with privacy and close enablement varied, then clicks
  symbol, hide, and eligible close controls to verify exact messages. Its input
  position vector is dropped before rendering and interaction, proving widgets
  do not retain references to temporary preparation data.
- Four synthetic before/after previews are byte-identical; the wide populated
  preview was also inspected visually. Preview output is opt-in through the
  test-only `KEROSENE_POSITION_PREVIEW_DIR` and remains outside the repository.
- Updated the account/wallet guide. No schema, route, subscription, request,
  signing, dependency, or asset changes; no measured speedup claimed. Tests
  inspect emitted UI messages without submitting orders.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene account_views::positions`:
  **43 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,482 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo check --locked -j 2`, `cargo fmt -- --check`, and
  `git diff --check`: passed.
- `cargo build --locked -j 2`: passed. Headless `--test` startup under Xvfb
  opened a 1600x960 window, reached the expected 20-second timeout (124), and
  produced no panic markers, using in-memory configuration.
- Source comparison verifies complete changed files against the reviewed
  lifetime/reference/visibility substitutions, allowing rustfmt changes. Account
  projection, chart/export consumers, summary rendering, and action construction
  are unchanged.

## 2026-09-28: separate the shared order-execution core by responsibility

- Replaced the 2,372-line shared execution core with a 15-line module/export
  boundary and five focused implementations: redacted intent/prepared models,
  capability policy and labels, client-order IDs, exchange tasks, and order
  preparation. Production modules range from 68 to 394 lines.
- Existing public crate-facing types and task functions remain available through
  `core.rs`. Three helpers gain only the parent-module visibility needed across
  the new children. Request ownership, captured signing targets, task arguments,
  redacted formatting, atomic nonce ordering, hash inputs, and capability labels
  are preserved.
- `place_request_with_context` now uses `place_request_with_existing_cloid` for
  the duplicate request-field mapping. Context creation and ID allocation remain
  in their original order; every request/context value remains independently
  owned. No placement, cancellation, or modification validation is reordered.
- Moved 43 core tests beside their responsibilities, with seven preparation
  fixtures shared by place/cancel/modify test modules. Added one of those tests
  before production changes: 156 combinations of all surfaces, exchange order
  kinds, directions, and reduce-only flags verify both builders' exact fields,
  unique generated IDs, unchanged account text, and ownership after dropping the
  prepared input. The new test only constructs requests.
- Located ticket, quick-order, Quick Trade, HUD, close, NUKE, cluster, Chase, and
  TWAP request-builder call sites; read root account/pending-request gates.
  Their snapshot and result-handling policies are retained for further review.
- Updated the trading/execution guide. No config/schema, message route,
  subscription, signing payload, dependency, asset, or UI changes. No measured
  speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene order_execution`:
  **402 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,483 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Source comparison verifies all 33 other production function bodies and all
  50 test/fixture bodies are preserved apart from whitespace. Complete new-file
  comparison checks the extracted model definitions, helper visibility changes,
  task wrappers, and the single reviewed request-construction delegation.
- GUI smoke was not repeated: this pass changes no startup, window, rendering,
  canvas, or platform behavior.

## 2026-09-28: separate shared execution account, pending, and identity state

- Replaced the 1,160-line execution root with a 47-line module/export boundary.
  Account snapshots and signing capture, pending-request state and guards,
  automation identities, quick-order form models, and captured move contexts
  now live in five focused modules of 90 to 211 lines.
- Existing callers keep their root import paths. Move-context error re-exports
  are test-only because production callers use the inferred error type. Moved
  quick-order forms and move context beside their existing submission code;
  account and pending tests now sit beside their implementations.
- Removed the redundant private committed-key presence alias. Signing capture
  calls the existing public predicate directly; missing-key, watch-only,
  disconnected-account, account-mismatch, and profile-validation priority stays
  unchanged. Committed key ownership and subaccount binding are preserved.
- Kept standard and HUD pending predicates separate and unchanged. HUD overlap
  exemptions, request short-circuit order, optimistic-indicator classification,
  concurrency limits, NUKE accounting, and exact status text are retained.
- Preserved the distinct account matching rules: shared account lookups trim and
  ignore case, while move replacement requires the trimmed current account to
  exactly match its captured account. Spot balance invalidation and Chase/TWAP
  metadata identity checks retain their existing policy and mutation order.
- Added two regressions before production changes: signing guards reject draft
  keys and preserve error priority across missing/blank keys, absent profiles,
  ghost accounts, disconnection, and mismatch; HUD overlap tests independently
  cover tracking, indicators, status checks, other-surface blocking, and status
  preservation. Extended the existing move-context regression with case-only
  account changes. Moved all 16 root tests and their two fixtures.
- Updated the trading/execution module map. Redacted formatting, config/schema,
  message routing, subscriptions, signing payloads, dependencies, assets, and UI
  behavior are unchanged. Tests inspect local state without submitting orders.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene order_execution`:
  **404 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,485 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Source comparison verifies all 47 other production function bodies unchanged;
  the two affected bodies match the reviewed forwarding removal. Complete-file
  comparison verifies the extraction, imports, model definitions, re-exports,
  and all 18 test/fixture bodies after rustfmt.
- GUI smoke was not repeated: no startup, window, rendering, canvas, or platform
  behavior changed.

## 2026-09-28: share prepared placement indicators and borrow quick-order provenance

- Ticket, quick-order, close-position, HUD, and Quick Trade submission now use
  one prepared-order adapter in `order_pending_indicators.rs`. It copies the
  account, symbol, side, size, and price into independently owned indicator
  fields, then uses the existing insertion and chart-sync path. The two
  primitive placement constructors are now test-only fixture helpers.
- Each caller retains its original projection decision: ticket and quick-order
  use the prepared exchange kind (including IOC as a taker); close-position and
  HUD use their existing submission flags; Quick Trade always uses the market
  indicator. Validation, exact status text, pending-action assignment, HUD
  animation/feed/sound and tracking, CLOID construction, spot invalidation, and
  task/result contexts remain in their original order.
- Quick-order percentage validation now borrows its provenance instead of
  cloning its account/symbol strings and market-universe config. Comparisons,
  snapshot freshness gates, side-specific spot recalculation, review feedback,
  and form recovery are unchanged.
- Reviewed ticket/quick-order snapshot matching, Quick Trade request binding,
  HUD admission, close-position preflight, and advanced startup. Their distinct
  requirements remain explicit rather than adding a generic submission gate.
  Shared indicator validation, ownership, ID allocation, optimistic projections,
  and chart-sync calls were also inspected; their policies are retained.
- Added three regressions before production changes. An 18-case ticket matrix
  covers market/limit/IOC, both sides, exact raw wire fields, invalid size/price,
  status, pending actions, and task creation. A 12-case quick-order matrix varies
  the presentation flag independently from exchange kind. A two-side Quick
  Trade regression verifies prepared market size/price and captured account.
  The existing spot recalculation test additionally checks unchanged provenance
  while quantity is restored for review. Task futures are never polled by the
  new tests, so they cannot submit orders.
- Updated the trading/execution guide. No schema, route, subscription, signing
  payload, dependency, asset, or rendering changes. No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene order_`:
  **1,062 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,488 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Complete-source comparison verifies all six production files against the
  reviewed field-construction substitutions, test-only primitive helpers, and
  single provenance borrow, allowing rustfmt changes. Existing indicator
  insertion, projections, request construction, and result callbacks are
  unchanged.
- GUI smoke was not repeated: this pass leaves rendering, startup, window,
  canvas, and platform implementations unchanged; existing indicator tests
  continue to cover chart line/pulse synchronization.

## 2026-09-28: separate result reconciliation and share completion paths

- Replaced the 894-line result module with a 16-line export boundary and five
  focused implementations: classification, pending status models/cleanup,
  one-shot placement reconciliation, cancellation verification, and NUKE
  aggregation. These production modules range from 78 to 256 lines.
- Moved close-menu toggling and workspace transient cleanup into
  `order_update/transient_ui.rs`, retaining both function bodies and the
  originating-workspace/detached-window tests. Message dispatch is unchanged.
- Close-position and Quick Trade result handlers now delegate to the existing
  ticket completion method for global pending-action cleanup, indicator removal,
  classification, and outcome application. HUD concurrency and quick-order form
  recovery retain their distinct completion steps.
- Unknown and possibly completed cancellations share status-verification setup.
  Their original predicates, priority, optional order-ID label, exact messages,
  refresh order, and local-order retention are preserved. Follow-up task setup
  moves the final-use cancelled-order identity and account instead of copying
  them. One-shot and NUKE status callbacks similarly own the original placement
  context while retaining the copies needed by the independent request future.
- Classification, redaction, request matching, stale-account filters, refresh
  cleanup, status-result branches, and NUKE accounting are unchanged. Helpers
  used across new children retain their former parent scope with `pub(super)`;
  existing crate-facing import paths stay available.
- Added two regressions before production changes: 24 serialized-completion
  cases cover three handlers, four outcome categories, current/stale accounts,
  selective indicator removal, captured surface, pending requests, and refresh
  tasks. Six cancellation cases cover transport uncertainty, terminal-looking
  rejection, ordinary rejection, and presence/absence of captured order identity.
  These tests only construct tasks; they never poll exchange or status requests.
- Organized all 48 tests and 19 fixture helpers into six result-test groups,
  shared fixtures, and transient-UI tests. Updated the trading/execution guide.
  No config/schema, route, subscription, signing payload, dependency, asset,
  rendering, or platform changes. No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene order_`:
  **1,064 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,490 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Complete-source comparison verifies all 17 changed Rust files against the
  reviewed extraction and substitutions after rustfmt. All 39 other result/UI
  function bodies and all 67 test/fixture bodies are preserved.
- `cargo build --locked -j 2`: passed. Headless `--test` startup under Xvfb
  opened a 1600x960 window, reached the expected 20-second timeout (124), and
  produced no panic markers, using in-memory configuration.

## 2026-09-28: borrow exchange-response decoding and search errors directly

- Exchange-response decoding now deserializes the retained JSON body by
  reference into owned typed fields. It avoids cloning the entire raw tree
  before parsing, while preserving the original value on malformed-body fallback.
- Added one private status-slice accessor used by order-ID lookup, fill queries,
  error and ambiguity predicates, and debug status counts. Empty/missing data,
  first-status OID precedence, fill summation order, and confirmation policies
  are unchanged. Error detection uses `any` directly; an empty list remains
  non-erroneous when the envelope status is `ok`.
- IOC no-match detection now searches envelope, raw fallback, and typed error
  messages directly in their existing order. Removed the private owned message
  list and its string copies. Raw fallback text still passes through the existing
  redactor before matching; structured/non-string typed errors remain excluded.
  Raw string summaries also pass their borrowed text directly to redaction.
- Read Chase placement-result and TWAP slice-result consumers, including their
  differing fill, retry, unexpected-resting, and status-check policies. Those
  handlers and classification order remain unchanged for further lifecycle review.
- Added three regressions before production changes. Thirty-two decoding cases
  cover object and sequence bodies, arbitrary status values, unknown fields,
  absent/null data, malformed fallback retention, and both success/error envelopes.
  Eight IOC cases cover each message source, ASCII case folding, later errors,
  ignored non-string statuses, and redacted raw text. Ten fill cases preserve
  reported-versus-validated size behavior, conflicting status fields, and empties.
- Updated the trading/execution guide. No schema, request construction, signing
  payload, account guard, automation transition, dependency, asset, or UI changes.
  These regressions only parse local fixtures. No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene signing`:
  **94 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,493 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Complete-source comparison verifies both production files against the reviewed
  borrowing and status-access substitutions after rustfmt. Summary formatting,
  per-status classification helpers, fill-size parsing, and consumer modules
  remain unchanged.
- GUI smoke was not repeated: no startup, window, rendering, canvas, or platform
  behavior changed.

## 2026-09-28: reuse admitted Chase orders during result handling

- Place, modify, and cancel result admission now reads account/lifecycle state
  from one order lookup, removing redundant existence checks and repeat lookups.
  Stale modify results return before copying the unused account address. Startup
  pending ownership and result-refresh classification retain their existing order.
- Client-order ID verification reuses its initial lookup and copies the fallback
  account only when the identifier is unavailable. The successful path no longer
  copies the account twice; the fallback still captures it before failure handling
  can remove the order, preserving its account-specific refresh.
- Placement-status handling borrows the admitted order through each transition,
  removing repeated optional lookups whose absence was impossible without an
  intervening map mutation. Terminal-wide stop, removal, and refresh operations
  run after the borrow ends. Two final-use status strings move instead of cloning.
- Lifecycle eligibility, response classification order, status text, retry counts,
  timestamps, stopping reasons, fill accounting, and captured signing tasks remain
  unchanged. Read OID-status, modify, and cancel reconciliation paths; OID-status
  and TWAP slice-result handlers remain candidates for later review.
- Added three regressions before production edits: 36 stale/absent-result cases
  cover three handlers, account ownership, three outcomes, pending-action retention,
  and handler-specific refresh policy; ten client-order ID fallback cases cover
  missing orders/identifiers, account switches, redaction, and stopping state;
  eight fill-status cases cover partial progress, optional OIDs, stopping orders,
  and account ownership. Tasks are constructed but never polled.
- Updated the trading/execution guide. No request payload, schema, route,
  subscription, dependency, asset, UI, or platform changes. No measured speedup
  claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene chase`:
  **203 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,496 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Complete-source comparison verifies all four production files against the
  reviewed lookup and ownership substitutions after rustfmt.
- GUI smoke was not repeated: no startup, window, rendering, canvas, or platform
  behavior changed.

## 2026-09-28: share Chase verification and consume TWAP result inputs

- Chase OID-status handling now retains the admitted mutable order through each
  branch, removing eight optional re-lookups and repeated reads of stop state.
  Account matching uses the existing shared predicate on the disjoint connected
  address field. Terminal-wide operations run after the borrow ends. Final-use
  status/archive strings move instead of cloning.
- Added `ChaseLifecycle::verifying_order_status`, shared by status-request setup
  and failed OID-status responses. It preserves stop intent, converts both
  missing-order verification states to unresolved missing-order verification,
  and sends other states to modify verification. Request setup also captures the
  account and updates the order through one lookup. Open/filled/terminal/missing
  responses retain their separate policies, including disconnected-stop archival.
- TWAP slice results take only a pending `Place` operation, preserving pending
  cancellations and the existing late-result refresh policy. The handler borrows
  the admitted TWAP directly instead of copying the slice and looking it up again.
  Retry plans move their original fields through struct update, and final-use
  client-order IDs move into follow-up task inputs. Independent child history,
  reconciliation state, and cancellation state retain the copies they need.
- Classification order, status text, retry limits, slice timing, key capture,
  stop/archive rules, and fill accounting are preserved. Read TWAP pause, refresh,
  completion, and status-task helpers to verify the ownership boundary; those
  helpers remain unchanged in this batch.
- Added four regressions before their respective production changes: 136 Chase
  request/error cases cover every lifecycle variant, account normalization,
  timestamps, redaction, and retained progress; 32 OID outcome cases cover stop,
  archive, account, and fill behavior. Sixty late TWAP result cases preserve
  absent/cancel pending operations, retry state, and refresh policies across
  active/terminal and matching/switched accounts. Six placement outcomes preserve
  child identifiers, complete retry plans, fill quantities, and follow-up state.
  These tests construct tasks but never poll exchange/status requests.
- Updated the trading/execution guide and coverage table. No schema, route,
  subscription, request payload, dependency, asset, UI, or platform changes.
  No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene chase`:
  **205 passed, 0 failed, 0 ignored** before and after the Chase changes.
- `cargo test --locked -j 2 --package kerosene --bin kerosene twap`:
  **139 passed, 0 failed, 0 ignored** before and after the TWAP changes.
- `cargo test --locked -j 2`: **4,500 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Complete-source comparison verifies all four production files against the
  reviewed transition extraction, lookup simplification, and ownership moves
  after rustfmt. The TWAP response body otherwise changes only formatting.
- GUI smoke was not repeated: no startup, window, rendering, canvas, or platform
  behavior changed.

## 2026-09-28: consolidate cancellation recovery and TWAP task ownership

- Moved the identical Chase/TWAP closed-order cancellation text predicate into
  `order_execution/exchange_errors.rs`. Ordinary cancellation no longer imports
  a Chase-specific helper. Its ASCII normalization, nine substring checks, and
  caller-specific recovery policies are unchanged. Moved the three predicate
  tests beside the shared implementation.
- TWAP unexpected-cancel results now reject absent/mismatched operations at the
  entry guard, prepare the summary once, and share confirmed/closed completion
  cleanup. Rejected/ambiguous and transport-unknown results share saturating retry
  accounting, exhaustion, delay, and pause setup while retaining their exact
  messages and error flags. Transport text suggesting a closed order still takes
  the retry path. Matching-child updates, stop completion, archival, and refresh
  ordering are preserved; the final retry client-order ID moves into its task.
- TWAP account refresh now applies policy and captures the original account
  through one lookup. Four status/cancellation callbacks move final-use IDs
  instead of copying them; the request futures retain their required independent
  copies. Immediate status checks still avoid the delayed path's timer.
- Added three regressions before production changes: 34 cancellation-text cases
  preserve ASCII case/substrings, historical spelling, and nonmatching whitespace
  or Unicode; 72 cancellation-result cases preserve exact events, saturation,
  stop/archive behavior, current/switched accounts, progress, and unrelated child
  state; 36 refresh cases preserve absent/active/terminal order behavior, account
  ownership, and loading follow-ups. Network and signing tasks are never polled.
- Full validation exposed existing Telegram test interference: a private-channel
  cursor seeded to 99 was cleared before its test action. Channel mutations and
  several runtime-config-clear tests bypassed the existing cursor test mutex.
  Added that mutex to 23 synchronous tests and documented its scope. Existing
  asynchronous guards, assertions, and Telegram/config runtime code are unchanged.
- Updated the trading/execution guide and coverage table. No schema, route,
  subscription, signing payload, dependency, asset, UI, or platform changes.
  Retryable error classification remains a later consolidation candidate; its
  normalization and TWAP-specific priority rules were retained. No measured
  speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene order_`:
  **1,074 passed, 0 failed, 0 ignored** before and after production changes.
- Initial `cargo test --locked -j 2`: **4,502 passed, 1 failed, 6 ignored**;
  the failure was the cursor-fixture interference described above.
- `cargo test --locked -j 2 --package kerosene --bin kerosene telegram -- --test-threads=16`:
  **140 passed, 0 failed, 2 ignored** after adding the missing test guards.
- Final `cargo test --locked -j 2`: **4,503 passed, 0 failed, 6 ignored**;
  doc-tests passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Complete-source comparison verifies all 13 extracted/edited trading files
  against reviewed substitutions and the consolidated result handler after
  rustfmt; the removed cancellation predicate bodies are identical. A separate
  comparison verifies the cursor changes add only test locks, retaining existing
  assertions and production bodies.
- GUI smoke was not repeated: no startup, window, rendering, canvas, or platform
  behavior changed.

## 2026-09-28: share TWAP status reconciliation updates

- Seven status-result branches now use one private child-update helper for exact
  client-order ID matching, returned-OID adoption, child status, and exchange
  summary. Missing returned OIDs retain each matching child's existing OID;
  duplicate matching children are all updated and unrelated children are untouched.
- Unrecognized exchange statuses and transport errors share retry planning,
  counter updates, exhaustion, pause, and follow-up setup. Their existing message
  text and `StatusUnknown` versus `NetworkError` pause reasons stay separate.
  Missing-status exhaustion retains its own stop/no-fill/fail-closed policy.
- Status admission now returns directly for absent orders, alongside existing
  stale-client-ID and terminal-state guards. The former absent-order archive call
  had no state effect. Final-use client-order IDs move into retry/cancel task
  inputs; retained pending-cancel state keeps its independent copy.
- Account-fill reconciliation combines identical requested-stop and confirmed
  no-fill completion checks. Eligibility, finish ordering, archival, timeout
  behavior, exact account matching, and fill calculations remain unchanged.
- Added three regressions before production edits: 24 child-update cases cover
  all seven update branches, duplicate/exact/different-case/absent client IDs,
  optional returned OIDs, stopping state, and retained fill/price/fee fields.
  Twelve retry cases preserve saturation, retry-plan retention, pause timing,
  exact feedback, and previous-account task creation. Four admission cases preserve
  absent/stale/terminal no-ops. Exchange and status futures are never polled.
- Read the slice execution/planning and fill-reconciliation paths for the next
  ownership review; scheduling and its order-book/retry-plan copies remain intact.
  Updated the trading/execution guide and coverage table. No schema, route,
  subscription, request payload, dependency, asset, UI, or platform changes.
  No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene twap`:
  **144 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,506 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Complete-source comparison verifies both production files against the reviewed
  child-update extraction, retry consolidation, ownership moves, and completion
  condition after rustfmt. Other fill-reconciliation bodies are unchanged.
- GUI smoke was not repeated: no startup, window, rendering, canvas, or platform
  behavior changed.

## 2026-09-28: borrow TWAP planning inputs

- Slice execution no longer clones both sides of the cached order book before
  metadata, staleness, and exchange-admission checks. Planning borrows the retained
  book after sizing, and keeps one mutable order lookup through validation and
  dispatch. Reviewed intervening pause/sizing methods: they do not change the book,
  side, price bounds, or precision.
- Retry sizing borrows the existing plan. The plan moves out of state only after
  price validation and signing-key availability checks; early returns retain it.
  Skip handling accepts only the optional slice index it needs, allowing the order
  borrow to end before terminal-level skip updates. Child, pending-placement, and
  request copies needed by their independent owners remain.
- Added three regressions before production changes: eight dispatch cases preserve
  buy/sell depth selection, fixed/random sizing, retry identity, counters, child
  fill details, and cached book contents. Twelve admission cases preserve retry
  retention and random-seed timing across missing/stale books, loading,
  reconciliation, throttling, and missing keys, including staleness before loading
  feedback. Four skip cases preserve range/minimum-notional messages, retry child
  updates, attempt counters, and scheduling. Signing/network futures are not polled.
- Updated the trading/execution guide and coverage table. No scheduling formulas,
  guard order, wire payloads, schema, route, subscription, dependency, asset, UI,
  or platform changes. No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene twap`:
  **147 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,509 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Complete-source comparison verifies both production files against the reviewed
  borrowing, lookup reuse, retry transfer, and skip-argument substitutions after
  rustfmt. Slice validation, scheduling, and request construction are unchanged.
- GUI smoke was not repeated: no startup, window, rendering, canvas, or platform
  behavior changed.

## 2026-09-28: share retry recognition and TWAP fill confirmation

- Moved Chase's retryable exchange-error recognition into shared execution code.
  TWAP uses the same eight substring checks through an internal helper that accepts
  its already ASCII-lowercased summary, avoiding another normalization allocation.
  The original predicate bodies were verified identical. Chase still checks closed
  orders first; TWAP still checks retryable errors before terminal errors.
- TWAP fill reconciliation now confirms absent fills at the end of each child
  update instead of traversing all children again. The existing order-ID guard
  supplies the confirmation requirement, and valid late fills update child status
  before confirmation. Fill summation order, numeric checks, monotonic totals,
  rejected-child handling, pause resolution, and terminal transitions are unchanged.
- Added three regressions before production changes: 32 retry-text cases preserve
  ASCII case, substring matching, and nonmatching Unicode/whitespace; 57 TWAP
  classification cases preserve retry priority and terminal/consume distinctions;
  two mixed-child reconciliation cases preserve late fills, rejected children,
  missing IDs, invalid/mismatched fills, retained metadata, fees, and pause state.
  Moved the retry predicate test beside the shared implementation after extraction.
- Reviewed TWAP fill-summary deduplication, exact coin/side matching, response
  summaries, ID hashing, weighted-average metrics, and cancellation/refresh helpers.
  Their policies remain explicit; no aggregation cache or calculation changes were
  introduced. Updated the trading/execution guide and coverage table. No schema,
  route, subscription, wire payload, dependency, asset, UI, or platform changes.
  No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- Initial baseline caught an incorrect assertion in the new reconciliation test:
  absent fee-token metadata retains the 0.01 fee, rather than multiplying it by
  price. Corrected that assertion before production changes; accounting stayed intact.
- Baseline `cargo test --locked -j 2 --package kerosene --bin kerosene twap`:
  **149 passed, 0 failed, 0 ignored** after correcting the fixture assertion.
- Baseline `cargo test --locked -j 2 --package kerosene --bin kerosene chase`:
  **204 passed, 0 failed, 0 ignored**.
- Final `cargo test --locked -j 2 --package kerosene --bin kerosene order_`:
  **1,082 passed, 0 failed, 0 ignored**. Final `twap_state::` target:
  **30 passed, 0 failed, 0 ignored**.
- `cargo test --locked -j 2`: **4,512 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`:
  passed. `cargo fmt -- --check` and `git diff --check`: passed.
- Complete-source comparison verifies the five production files and two test-move
  files against the reviewed extraction, substitutions, and loop consolidation
  after rustfmt. Unrelated reconciliation and Chase result-handler bodies remain
  unchanged.
- GUI smoke was not repeated: no startup, window, rendering, canvas, or platform
  behavior changed.

## 2026-09-28: simplify chart overlay synchronization

- Split order-line assembly into `chart_state/overlays/orders.rs` and moved inline
  tests into `overlays/tests.rs`; the root overlay module goes from 559 to 142
  lines. Position, trade-marker, and reference-price synchronization remain in the
  root, with fill mapping in the existing `trades.rs` module.
- Position, order, and trade synchronization borrow each chart's symbol until
  applying the prepared result. Chase overlays remain an iterator until merging
  into confirmed orders, removing an intermediate vector without changing source
  order, replacement precedence, matching, or numeric filtering.
- Pending-indicator selection now returns borrowed entries through an iterator.
  The former eager `then_some` copied every indicator's owned strings, including
  entries rejected by account/symbol filtering. The selector also borrows the
  connected account; its existing matching helper retains whitespace, ASCII-case,
  and blank-address handling. Only standalone output lines copy the symbol they own.
- Removed a trade-parser wrapper that only forwarded to the existing positive,
  finite-number helper. Marker filtering and stable timestamp sorting are unchanged.
- Added three regressions before production changes: combined order assembly
  covers duplicate Chase OIDs, exact Chase account matching, invalid Chase values,
  confirmed-order parsing differences, pending modification/cancellation, market
  loaders, and hidden-symbol clearing. Multiple chart instances preserve positions,
  account switches, normalized snapshot ownership, exact symbols, and tied
  marker ordering. Ten selector cases preserve ordered IDs and account/symbol
  matching. Existing iterator-consumer assertions keep their original meaning.
- Read the account position projection used by charts and retained its synthesis
  and pair-selection policies for a separate ownership review. Updated the chart
  guide and coverage table. No rendering formulas, caches, order submission,
  schema, message routes, subscriptions, dependencies, or assets changed.
  No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene chart_state::overlays`:
  **12 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2 --package kerosene --bin kerosene order_pending_indicators::`:
  **30 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,515 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`,
  `cargo build --locked -j 2`, `cargo fmt -- --check`, and `git diff --check`: passed.
- Complete-source comparison verifies all five edited/extracted production and
  test files against the reviewed borrowing, iteration, extraction, and test move
  after rustfmt. The remaining overlay and indicator bodies are unchanged.
- Headless GUI smoke: the newly built binary opened its 1600x960 test-mode window
  under Xvfb; expected 20-second timeout (exit 124), no panic markers. This validates
  startup; overlay behavior is covered by the state regressions above. No drawing
  code changed, and no pixel-equivalence claim is made.

## 2026-09-28: borrow account position projection and spot candidates

- Account projection now returns borrowed native rows plus owned outcome/spot rows
  through `Cow`, matching the existing wallet-detail projection pattern. Synthesis
  remains eager and retains native, outcome, then optional spot ordering. Connected
  account ownership, portfolio-margin/universe gates, native wire fields, and all
  outcome/spot calculations are unchanged.
- Position-table filters and section/summary inputs retain row references; PnL-card
  lookup and iteration retain borrowed native rows. Chart overlays and tab counts
  consume the same projection without cloning native positions. Widgets, action
  messages, and final PnL metrics still own their outputs. Removed an unused position
  slice parameter from the table header.
- Spot-pair selection now filters and stably sorts one vector of symbol references,
  replacing four candidate vectors and copies of every key. Selection returns the
  borrowed pair; only synthesized positions copy its key. Fill reconciliation,
  last-fill timestamp ties, first live mark, first-candidate fallback, USD quote
  filtering, ticker case rules, and asset-index ordering retain their precedence.
- Moved existing projection tests beside the module; `account_positions.rs` goes
  from 714 to 257 lines. Added two regressions before production edits: four
  mixed native/outcome/spot cases preserve source order, malformed native values,
  account normalization/switching, and universe/portfolio-margin policy. Six pair
  cases preserve all selection tiers, stable equal-index ordering, timestamp ties,
  and exclusion of otherwise attractive non-USD pairs.
- Updated the account/wallet guide and coverage table. No layout, numeric formulas,
  signing/order payloads, config schema, message routes, subscriptions, dependencies,
  or assets changed. Outcome and spot missing-value policies remain distinct.
  No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene account_positions::`:
  **16 passed, 0 failed, 0 ignored** before and after production changes.
- Baseline position-section rendering target: **1 passed**; saved four synthetic
  previews across narrow/wide, privacy, and close-control states.
- Final `account_views::positions::` target: **43 passed**; final `pnl_card::`
  target: **30 passed**. The four position-section PNGs are byte-identical to the
  baseline; rendering also verifies independently owned action messages.
- `cargo test --locked -j 2`: **4,517 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`,
  `cargo build --locked -j 2`, `cargo fmt -- --check`, and `git diff --check`: passed.
- Complete-source comparison verifies all seven production/test-move files against
  the reviewed ownership, collection, consumer-type, and extraction substitutions
  after rustfmt. Synthesis arithmetic and drawing bodies remain unchanged.
- Headless GUI smoke: the newly built binary opened the 1600x960 test-mode window
  under Xvfb, reached the expected 20-second timeout (exit 124), and emitted no panic
  markers. This validates startup alongside the focused rendering regressions.

## 2026-09-28: simplify trading overlay drawing

- Reviewed chart clipping geometry, projected drawing helpers, order overlays,
  current-price and position overlays, and trade-marker grouping/layout. Kept the
  compact geometry and bounded grouping calculations, including the distinct
  edge-blur policies of general and order-specific drawing.
- Order styling now selects the buy/sell color independently from animation
  alpha/width and builds one segmented-line style from the selected dash/offset.
  Visible-order preparation reuses its existing animation predicate. Position
  entry color is selected once; current-price and liquidation lines reuse their
  style for badge connectors.
- The final order-label pass consumes the prepared rows and moves each side-label
  string into canvas text. Lines/badges and connectors still draw first, retaining
  their original paint order. Label geometry, stacking, hover/spinner behavior,
  finite-value guards, colors, alpha, widths, and dash parameters are unchanged.
- Added a synthetic software-renderer regression before production edits. Twelve
  theme/distortion/animation cases each render visible, obscured-position, and
  hidden-account states. They include both order sides, static/moving/dragged and
  placing/cancelling/modifying orders, overlapping badges, cancel hover, long/short
  positions, candle/line series, invalid/offscreen orders, and nonfinite animation
  phase. Privacy assertions compare suppressed overlays with absent source rows.
- Updated the charting guide and coverage table. No order requests, state machines,
  config schema, message routes, subscriptions, dependencies, or assets changed.
  Removed label copies; no measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene chart::overlays::`:
  **16 passed, 0 failed, 0 ignored** before and after production changes.
- All **36 synthetic overlay PNGs are byte-identical** before and after.
- `cargo test --locked -j 2`: **4,518 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`,
  `cargo build --locked -j 2`, `cargo fmt -- --check`, and `git diff --check`: passed.
- Complete-source comparison verifies all five production files against the
  reviewed style and ownership substitutions after rustfmt. An initial patch
  matched the connector loop instead of the final label loop; compilation and
  source comparison caught it, and it was corrected before final validation.
- Headless GUI smoke: the newly built binary opened its 1600x960 test-mode window
  under Xvfb; expected 20-second timeout (exit 124), no panic markers. This validates
  startup alongside the focused synthetic canvas comparisons.

## 2026-09-28: share order-label packing and remove redundant badge sorting

- Reviewed left order-label layout, reserved-region helpers, right-axis badge
  anchors/packing/connectors, and order hit testing. Traced chart input dispatch,
  cursor selection, wheel handling, and the initial press/drag paths to confirm
  how shared label geometry reaches clicks; substantive interaction-handler
  cleanup remains for later review.
- General order-label packing and the bands above/below a position label now use
  one packing routine with an explicit reserved-range slice. The single-position
  branch passes empty slices because its bands already exclude that position.
  Sorting, side preference at equal prices, clamping, reserved-range adjustment,
  reverse overflow packing, and final shifting retain their original order and
  arithmetic. Drawing and hit testing continue to consume the same positions.
- Removed the first of two identical stable sorts on the non-fixed right-axis
  badge path. Each band retains its own sort by source coordinate and rank.
  Variable badge heights, fixed position placement, side preferences, invalid
  input filtering, and crowded-edge policy remain separate from left labels.
- Added three baseline regressions: eleven left-label cases cover no/one/multiple
  reserved regions, reversed reserved input, equal-price index ties, narrow and
  invalid heights, overflow shifting, and empty input. Two duplicate-key cases
  preserve band output. Badge cases cover stable rank ties, signed-zero source
  order, variable heights, dense packing, invalid anchors, and invalid heights.
- Updated the charting guide and coverage table. No interaction dispatch, order
  requests, config schema, message routes, subscriptions, dependencies, or assets
  changed. No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene chart::`:
  **288 passed, 0 failed, 0 ignored** before and after production changes,
  including label/badge geometry, order hit testing, input, and canvas rendering.
- All **36 synthetic overlay PNGs are byte-identical** before and after across
  themes, animation/pending states, fisheye effects, and privacy settings.
- `cargo test --locked -j 2`: **4,521 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`,
  `cargo build --locked -j 2`, `cargo fmt -- --check`, and `git diff --check`: passed.
- Complete-source comparison verifies both production files against the reviewed
  packing consolidation and sort removal after rustfmt. Reserved-range helper
  bodies and the badge-packing implementation remain unchanged.
- Headless GUI smoke: the newly built binary opened its 1600x960 test-mode window
  under Xvfb; expected 20-second timeout (exit 124), no panic markers.

## 2026-09-28: share drag-release cleanup and defer annotation copies

- Reviewed chart input dispatch, press/drag, drawing-tool, cursor, wheel, and HUD
  handlers. Kept action priority, per-surface routing, source/visual coordinates,
  panel/viewport math, and order construction explicit in their existing paths.
- Drag release now takes the active gesture once, clears its start point once,
  and shares the fallback redraw. Order, annotation, and viewport/panel branches
  retain their own payload cleanup and messages. An absent gesture still returns
  without clearing other state. Panning alone still invalidates cached geometry
  so a full-detail heatmap is rebuilt after the reduced-detail drag frames.
- Annotation motion borrows the original snapshot until coordinate validation
  succeeds, then copies it for the live preview. Selection checks the first
  matching annotation's lock before copying it. Successful drags still retain an
  independent base and live copy; missing candle data retains the existing preview.
  Moved the touched selection helper's local import to the file's import block.
- Added two baseline regressions. Fifty-four release cases cover all eight drag
  kinds plus no gesture, docked/detached surfaces, complete/missing/nonfinite
  payloads, annotation validation, panel rounding/fallback, capture status,
  unrelated-state retention, and actual cache rebuilds. Annotation body/anchor
  motion checks two cursor updates against the original snapshot and verifies
  preview retention when candle inputs disappear.
- Updated the charting guide and coverage table. No message routes, subscriptions,
  config schema, signing, dependencies, assets, or drawing code changed. No measured
  speedup claimed.

Validation (using the local ALSA prefix documented above):

- Baseline `cargo test --locked -j 2 --package kerosene --bin kerosene chart::tests::input::`:
  **32 passed, 0 failed, 0 ignored** before production changes.
- Final `cargo test --locked -j 2 --package kerosene --bin kerosene chart::`:
  **290 passed, 0 failed, 0 ignored**, including the new regressions and existing
  drawing/order input priority, HUD safety, hit-testing, and rendering tests.
- `cargo test --locked -j 2`: **4,523 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`,
  `cargo build --locked -j 2`, `cargo fmt -- --check`, and `git diff --check`: passed.
- Complete-source comparison verifies both production files against the reviewed
  ownership and release-control-flow substitutions after rustfmt. The underlying
  gesture calculations, message payloads, and invalid-value policies are unchanged.
- Headless GUI smoke: the newly built binary opened its 1600x960 test-mode window
  under Xvfb; expected 20-second timeout (exit 124), no panic markers.

## 2026-09-28: consolidate quick-order actions and HUD size editing

- Initial and replacement quick-order cards now share coordinate conversion and
  `OpenQuickOrder` construction. Right-click handlers retain their visual bounds
  checks, precedence over range/tool/order actions, and fallthrough when price
  inputs are missing. Payloads still carry source coordinates and the original
  chart/surface IDs, and still clamp only the price-conversion coordinate.
- HUD size text now shares replacement, optional decimal zero-prefixing, insertion,
  and change tracking after the existing digit/decimal acceptance check. Decimal
  eligibility is still tested against the old value before replacement. Length
  normalization still runs only after an accepted character and after the whole
  text chunk; the single-use normalizer is replaced by `String::truncate` in that
  branch. Enter and Escape share their identical finish handling.
- Extracted existing HUD tests without changing their bodies; `hud.rs` goes from
  457 to 272 lines. Added four baseline regressions: forty right-click cases cover
  docked/detached surfaces, loaded/missing prices, initial/replacement cards, range
  clearing, drawing tools, and live/pending orders; four additional cases preserve
  source coordinates and price clamping. Sixteen text and twenty-four named-key
  cases cover decimal replacement, ASCII filtering, ignored input, leading zeros,
  length limits, delete/backspace, whitespace completion, capture, and edit flags.
- Updated the charting guide and coverage table. No order placement, signing,
  message routes, subscriptions, config schema, dependencies, assets, or drawing
  code changed. No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene chart::`:
  **294 passed, 0 failed, 0 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,527 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`,
  `cargo build --locked -j 2`, `cargo fmt -- --check`, and `git diff --check`: passed.
- Complete-source comparison verifies both production files and the extracted HUD
  tests against the reviewed substitutions/extraction after rustfmt. Other input
  handlers, scroll-size formulas, and HUD control messages remain unchanged.
- Headless GUI smoke: the newly built binary opened its 1600x960 test-mode window
  under Xvfb; expected 20-second timeout (exit 124), no panic markers.

## 2026-09-28: borrow annotation rendering and iterate anchor handles

- Reviewed visible candle/price ranges, scroll bounds, follow-price statistics,
  heatmap time bounds, annotation hit testing, and candle data lifecycle. Kept the
  compact viewport calculations and distinct numeric fallbacks unchanged; primary
  and secondary real-time candle updates remain a follow-up consolidation candidate.
- `AnnotationKind::anchor_points` now returns an iterator over a fixed pair or
  the borrowed Fibonacci slice instead of allocating a vector. Both binary-local
  consumers iterate directly. Placement order, empty single-axis handles, and raw
  unvalidated Fibonacci values/counts remain unchanged.
- Selected/live annotation drawing now borrows the source record instead of
  cloning its geometry and label strings. Fibonacci drawing folds leftmost X
  while converting anchors, removing another temporary vector while preserving
  conversion failure handling, fold order, and the final clamp. Draft construction
  still owns the geometry it builds for its cursor preview.
- Trend-line and measurement body hit tests share their identical branch. Fixed
  the hit-test doc comment to reflect existing behavior: later annotations win,
  handles precede the body within each annotation, hidden items are skipped, and
  locked items remain selectable while editing handlers enforce the lock.
- Added four baseline regressions: twelve anchor cases preserve every shape's
  order, empty/malformed Fibonacci lists, signed zero, and NaN bits. Hit tests
  cover normal/inverted axes, topmost bodies versus older handles, locked/hidden
  items, and line/rectangle/Fibonacci bodies. Synthetic rendering covers all shape
  families with selected and live line/rectangle/Fibonacci/hidden targets across
  two themes and normal/distorted projection, including locked selection and
  handles outside Select mode; clearing the preview restores the original image.
- Updated the charting guide and coverage table. No config wire types, order
  requests, signing, message routes, subscriptions, dependencies, or assets changed.
  No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene annotation`:
  **33 passed, 0 failed, 0 ignored** before and after production changes.
- All **32 synthetic annotation PNGs are byte-identical** before and after.
- `cargo test --locked -j 2`: **4,531 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`,
  `cargo build --locked -j 2`, `cargo fmt -- --check`, and `git diff --check`: passed.
- Complete-source comparison verifies all three production files against the
  reviewed iteration, borrowing, equivalent fold/branch, and documentation edits
  after rustfmt. Remaining annotation drawing calculations are unchanged.
- Headless GUI smoke: the newly built binary opened its 1600x960 test-mode window
  under Xvfb; expected 20-second timeout (exit 124), no panic markers.

## 2026-09-28: consolidate real-time candle tail updates

- Primary and secondary series now share one validated tail-update helper:
  same-time updates replace the tail, older updates are rejected, and newer
  candles append. Only append trims history. Both public callers keep validation,
  missing-secondary handling, result flags, status ownership, and cache clearing
  in their original order. Set/merge history policies remain explicit.
- Removed the second validity check and always-true price-flash guard in the
  websocket handler; the early rejection still gates all update processing.
  Gap repair, sparse-market backoff, provider guards, funding refresh, and
  rollover persistence remain unchanged.
- Added three baseline regressions. Sixteen primary/secondary admission cases
  cover empty history, invalid and out-of-order data, exact tail replacement,
  negative prices, signed zero, unchanged replacements, retained identity/status,
  unrelated-series isolation, and actual render-cache clearing. Two additional
  cases reject missing secondary series. Four history cases at/above the cap
  preserve rejected/replaced history and trim only after append. Five websocket
  cases retain existing flashes for rejected/unchanged data and update direction
  and timestamps only after applied price changes.
- Updated the charting guide and coverage table. Also inspected signing action
  builders, wire types, nonce/client flow, numeric formatting, key capture, and
  hash construction. Kept the existing shared wire/signing flow and numeric/key
  policies; temporary hash concatenation buffers warrant a separate review with
  baseline signature vectors. No signing code changed in this batch.
- No config wire types, order requests, message routes, subscriptions,
  dependencies, or assets changed. No measured speedup claimed.

Validation (using the local ALSA prefix documented above):

- `cargo test --locked -j 2 --package kerosene --bin kerosene candle`:
  **201 passed, 0 failed, 1 ignored** before and after production changes.
- `cargo test --locked -j 2`: **4,534 passed, 0 failed, 6 ignored**; doc-tests
  passed (0 tests).
- `cargo clippy --locked -j 2 --all-targets --all-features -- -D warnings`,
  `cargo build --locked -j 2`, `cargo fmt -- --check`, and `git diff --check`: passed.
- Complete-source comparison verifies both production files against the reviewed
  extraction and removal of redundant validation after rustfmt. Remaining
  websocket source/admission, repair, and caching logic is unchanged.
- Headless GUI smoke: the newly built binary opened its 1600x960 test-mode window
  under Xvfb; expected 20-second timeout (exit 124), no panic markers.

## Next candidates

1. Review temporary concatenation buffers in `signing/crypto.rs` for direct
   incremental hashing. Preserve exact byte order, markers, validation errors,
   and signature bytes; establish baseline hash/signature coverage first. Keep
   wire field order, nonce allocation, numeric formatting, and key handling intact.
2. Continue across the unreviewed areas in the coverage table, including remaining
   automation internals and app surfaces. Large files often include inline tests,
   so distinguish production complexity from file length.
