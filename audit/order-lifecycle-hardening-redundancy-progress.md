# Order Lifecycle Hardening and Redundancy Progress

## Baseline

- Starting HEAD: `cbd48106a75fcbeb982f5e6ed2d53772bbf7123b`
- Branch: `main` (`origin/main` plus the committed goal prompt)
- Pre-existing dirty paths: none
- Architecture snapshot date: 2026-07-10
- Baseline validation:
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed.
  - `cargo test --package kerosene --bin kerosene signing::tests` did not
    compile Kerosene because `alsa-sys` could not find the system `alsa.pc`
    package through `pkg-config`.
  - `cargo check` stopped at the same missing system ALSA dependency before
    checking Kerosene.
- Validation constraint: do not describe Rust tests or type-checking as passed
  until the system ALSA development package is available. Continue using
  source inspection, formatting, diff checks, and focused test additions, and
  rerun the blocked commands when the environment permits.

## Verified Architecture Snapshot

- All signed L1 mutations converge in `src/signing/client.rs` through
  `sign_and_post`. The current action set is place, cancel by OID, cancel by
  CLOID, modify, and leverage update (`src/signing/actions.rs:28-34`,
  `src/signing/client.rs:234-293`).
- Place and modify preparation preserves asset, side, price and size strings,
  TIF/order kind, reduce-only, OID/CLOID, and action field order without a
  conversion after `PreparedExchangeOrder`/`PreparedModifyOrder`. The final
  signed-payload boundary now independently rejects non-positive/non-finite
  numeric strings and a missing or malformed placement CLOID before hashing or
  posting (`src/signing/actions.rs:36-158`, `src/signing/client.rs:106-135`).
- User and automation placement paths use `place_order_task`; cancel/modify
  paths use the corresponding task wrappers in `src/order_execution/core.rs`.
  A repository-wide call-site search found no feature-owned signing pipeline.
- General placement preparation is centralized by `PlaceIntent` and
  `PreparedExchangeOrder` in `src/order_execution/core.rs:51-61` and
  `src/order_execution/core.rs:326-380`. Chase, TWAP, and NUKE construct
  prepared values after their own state-machine/planner validation, then reuse
  the same task/signing boundary.
- One-shot placements receive a 128-bit hashed CLOID and immutable
  `OneShotPlacementContext`; uncertain results use `orderStatus` by CLOID plus
  an account refresh (`src/order_execution/core.rs:354-380`,
  `src/order_update/results.rs:616-663`).
- One-shot result classification distinguishes accepted-resting, filled,
  cancelled, rejected, ambiguous, and transport-unknown outcomes
  (`src/order_update/results.rs:15-30`, `src/order_update/results.rs:173-215`).
- Signed mutation transport uses a dedicated client with no redirects or HTTP
  retries, a 5-second connect timeout, a 15-second total timeout at both client
  and request scope, and a 60-second idle-pool timeout. Every local-build,
  connect/send, body-read, and JSON-parse failure remains conservatively
  transport-unknown and is order-aware redacted before task mapping into
  `Message`; no placement or modify is automatically retried. Parsed responses
  also preserve uncertainty whenever an explicit error conflicts with a
  structured possible effect (`src/signing/client.rs:74-232`,
  `src/signing/model/exchange_response/analysis.rs:83-144`).
- Parsed exchange responses retain exact nested status values for lifecycle
  classification, but every public response-model `Debug` layer exposes only a
  recognized response-type label, status counts, and redaction markers. The
  private wire helper has no `Debug` surface, and the type-only summary replaces
  an unrecognized external type without changing ordinary protocol copy
  (`src/signing/model/exchange_response.rs:19-67`,
  `src/signing/model/exchange_response/analysis.rs:24-31`,
  `src/signing/model/exchange_response/analysis.rs:227-247`).
- Every shared one-shot/cancel/move result is normalized into an
  `ExecutionOutcome` whose exact status remains available to unchanged visible
  status, cancellation heuristics, and reconciliation consumers. Its independent
  `Debug` boundary exposes only the classifier kind, error/refresh flags, and a
  redaction marker, not the normalized OID, fill size/price, or external status
  copy (`src/order_update/results.rs:17-44`,
  `src/order_update/results.rs:304-347`).
- The leverage-mutation diagnostic chain preserves exact input, symbol,
  account-correlation, asset, dex, margin-mode, and leverage values for the
  unchanged validation, equality, signing, result handling, and UI paths. The
  transient input uses `RedactedOrderInput`; submission and pending-result
  contexts expose only redaction markers plus margin mode through `Debug`; and
  the serializable signed action redacts its asset and leverage when formatted.
  Serde fields and signed bytes are unchanged (`src/message.rs:228-250`,
  `src/message.rs:727-733`, `src/order_execution.rs:537-581`,
  `src/signing/actions/wire.rs:166-188`).
- Order-status REST parsing validates the returned OID/CLOID for concrete order
  bodies before handing a result to lifecycle code. Both public task exits now
  sanitize every error before message mapping; HTTP previews redact before
  truncation; correlation failures are value-neutral; and successful result
  diagnostics retain only redacted/boolean identifier metadata
  (`src/api/order_status.rs:16-82`, `src/api/order_status/parsing.rs:10-62`,
  `src/api/order_status/model.rs:8-24`).
- Transient order-message OID/CLOID fields use exact-value wrappers whose
  diagnostic representations are redacted. Producers wrap only when publishing
  a message, and the order/chart update boundary restores the original
  primitive before invoking unchanged handlers. This covers cancel, move,
  Chase, TWAP, and chart cancel-hover messages without changing runtime state or
  exchange types (`src/message.rs:254-308`, `src/message.rs:1141-1429`,
  `src/order_update.rs:80-104`, `src/order_update.rs:205-431`,
  `src/chart_update.rs:218-240`).
- Cancel and move direct/status callbacks carry a collision-aware runtime
  request sequence in addition to account, symbol, and OID. Cancel keeps one
  immutable request owner across its awaiting-result and checking-status
  phases; move keeps the sequence and exact prepared target price in its
  captured-key context and status request. Successful refresh cleanup requires
  the origin symbol's open-order lane; move cleanup additionally distinguishes
  target absence, the expected price, and a different valid live price. These
  values are local correlation/reconciliation only and do not change the signed
  action or add mutation retries (`src/order_execution.rs:992-1053`,
  `src/order_update/results.rs:90-290`,
  `src/order_update/results.rs:761-788`).
- The independently formattable cancel/move correlation layers now keep account,
  symbol, OID, and expected price behind redaction markers. `MoveOrderKey`
  preserves exact `Eq`/`Hash` map and drag identity; pending status records
  preserve exact matching while retaining only request sequence and phase as
  useful diagnostic metadata. The captured-key `PendingMoveOrderContext`
  remains deliberately non-formattable, and no production diagnostic sink was
  found (`src/order_execution.rs:1009-1062`,
  `src/order_update/results.rs:102-290`).
- Connected-account task results carry read-provider, request-generation, and
  dispatch-time account-revision context. Address-scoped user-data changes
  supersede an earlier request before it can replace state or drive lifecycle
  reconciliation; an initial-load frame with no mergeable base queues one
  post-frame refresh. Websocket lag retains its existing reconciliation path
  (`src/read_data_provider.rs:23-157`,
  `src/account_update/connection/refresh.rs:94-213`,
  `src/account_update/stream.rs:45-230`).
- User-data application messages carry their complete stream parameters,
  including a runtime-only consumer generation whose diagnostics redact the
  address. Account, wallet-detail, and wallet-cluster routes require an exact
  current recipe plus its normalized emitted source before applying state.
  Generation rotation covers same-address account sessions, detail-window
  disappearance/recreation (including connected-address exclusion), selected
  cluster topology/profile bindings, and visible-dex changes
  (`src/ws/user_streams.rs:20-94`,
  `src/subscription_state/user_data.rs:11-213`,
  `src/account_update.rs:97-103`,
  `src/wallet_cluster_update.rs:92-98`).
- Chase and TWAP retain captured account/key identity and explicit lifecycle or
  pending-operation state. Their active state and in-flight requests are
  runtime-only. Each TWAP CLOID status task now has one separately armed retry
  attempt because the CLOID remains live during later account-fill repair.
  Unexpected-child cancellation likewise separates its scheduled retry count
  from the exact attempt that owns an in-flight target-specific cancel task.
  Authoritative TWAP fill size, price, fee, remaining target, and completion
  derive from per-child fill metrics independently of that child's display
  status. Terminal TWAPs cannot schedule slices, status checks, cancel retries,
  or timer ticks. Result/status/fill/deadline terminal paths archive and scrub;
  final pre-dispatch initial/retry skips preserve their established no-history
  behavior but now scrub the captured key directly. A retryable cancel result
  that was already in flight still requests immediate account reconciliation
  but no longer creates a delayed retry trigger after authoritative fills
  complete the TWAP.
- TWAP planning/activity text remains exact as it moves from planned-slice
  validation or sanitized result/status handling into order status,
  `TwapEvent`, the live activity view, and terminal advanced-order history.
  Independent `Debug` formatting of the pre-recording skip and live event
  exposes only kind, timing where applicable, error state, and a redaction
  marker. The root `TwapOrder` formatter still reports only the event count;
  event/history fields and serialization are unchanged
  (`src/order_execution/twap/execution/planning.rs:15-86`,
  `src/order_execution/twap/execution/skip.rs:11-25`,
  `src/twap_state/model.rs:43-76`, `src/twap_state/order.rs:150-158`,
  `src/advanced_order_history/snapshots.rs:61-118`).
- The transient TWAP helper chain retains exact editable form inputs, parsed
  cadence, cached book/freshness state, direct-response OID/fill metrics, and
  authoritative account-fill metrics for unchanged validation, planning, child
  settlement, and reconciliation. Each helper now has an explicit diagnostic
  boundary: exact values are replaced by markers, response optionality remains
  visible, and the form retains only its randomization boolean. The start
  snapshot and root order keep their existing independent redaction, while the
  captured-key initializer remains non-formattable
  (`src/twap_state/model.rs:18-48`, `src/twap_state/model.rs:133-146`,
  `src/twap_state/fills.rs:11-130`,
  `src/order_execution/twap/start/validation.rs:12-47`).
- Chase/TWAP market/adoption messages now wrap their exact symbol only while it
  crosses the Elm boundary, and all nine initial-book, mutation, cancel, and
  order-status result variants wrap the original `Result<T, String>`. Derived
  `Message` diagnostics expose symbol redaction plus only `Ok`/`Err` shape;
  producers preserve the same IDs/attempts/provider context/payloads, and
  `update_order` consumes each wrapper immediately before invoking the unchanged
  handler (`src/message.rs:310-368`, `src/message.rs:1282-1371`,
  `src/order_update.rs:177-354`,
  `src/subscription_state/market/chase.rs:82-118`,
  `src/subscription_state/market/twap.rs:68-104`).
- Every remaining signed-mutation and `orderStatus` result carried by `Message`
  now uses that same exact boxed-result wrapper: leverage; wallet-cluster
  direct/status; ticket/preset/Alfred one-shot; cancel direct/status; close;
  NUKE direct/status; one-shot status; quick; HUD; and move direct/status.
  Publishers move the same result into the wrapper, and the order or wallet-
  cluster update route immediately restores it before unchanged handlers run.
  A repository-wide field search finds no raw exchange-response or order-status
  result box left in the Elm enum (`src/message.rs:790-793`,
  `src/message.rs:988-999`, `src/message.rs:1203-1263`,
  `src/message.rs:1465-1503`, `src/order_update.rs:57-165`,
  `src/order_update.rs:411-483`, `src/wallet_cluster_update.rs:143-164`).
- The 11 direct order/position symbol fields across outcome-sell prefill,
  cluster close, cancel/status, position hide/close, and move drag/intent/
  result/status messages now use `RedactedOrderSymbol`. Advanced-history
  navigation uses `RedactedAdvancedOrderHistoryId` because the exact persisted
  identity embeds its account address. Producers wrap once at message
  construction and the order, account, cluster, or history update boundary
  immediately restores the original string; price, fraction, route, state, and
  handler inputs remain exact (`src/message.rs:310-370`,
  `src/message.rs:805-815`, `src/message.rs:1010-1018`,
  `src/message.rs:1239-1273`, `src/message.rs:1346-1354`,
  `src/message.rs:1509-1534`, `src/order_update.rs:42-126`,
  `src/order_update.rs:281-283`, `src/order_update.rs:441-485`,
  `src/account_update.rs:15-19`, `src/wallet_cluster_update.rs:134-147`).
- Every remaining direct financial `Message` field is now value-neutral in
  derived diagnostics: order-book price, main/quick percentages, preset edit
  input and exact preset payload, market-slippage input, connected/cluster close
  fractions, quick-open price, and move price. Strings reuse
  `RedactedOrderInput`; `RedactedOrderValue<T>` preserves exact numeric bits and
  the nested `OrderPreset`. Immediate update arms restore the original values
  before unchanged parsing, clamping, validation, preparation, or state
  mutation. Quick-order canvas coordinates remain ordinary UI geometry
  (`src/message.rs:227-278`, `src/message.rs:831-862`,
  `src/message.rs:998-1001`, `src/message.rs:1040-1048`,
  `src/message.rs:1299-1303`, `src/message.rs:1514-1525`,
  `src/message.rs:1549-1557`, `src/order_update.rs:34-73`,
  `src/order_update.rs:123-130`, `src/order_update.rs:380-403`,
  `src/order_update.rs:451-460`, `src/preferences_update.rs:406-407`,
  `src/wallet_cluster_update.rs:137-146`).
- Terminal advanced-order history retains the exact Chase/TWAP account,
  symbol, financial, timing, identifier, status, activity, and child fields
  required by its persisted schema and existing views. Each independently
  formattable entry/log/child layer now exposes only allowlisted kind/source/
  index, boolean/presence/count metadata, and redaction markers; the temporary
  Chase fill-metrics helper likewise redacts every financial value. Serde
  fields/defaults and direct view/snapshot access remain unchanged
  (`src/advanced_order_history/model.rs:8-217`,
  `src/advanced_order_history/snapshots.rs:19-243`,
  `src/config/schema.rs:427-432`,
  `src/order_views/advanced_history_details/sections.rs:17-215`).
- Config snapshots contain terminal advanced-order history but no live
  Chase/TWAP maps, pending order contexts, or captured signing keys; boot
  reconstructs those runtime owners empty. When main-window closure leaves the
  daemon alive for final persistence, the exit flag now stays armed through
  save or clear and the exit task. A root guard rejects fresh order, leverage,
  move, close/NUKE, cluster, preset/Alfred, and automation-start/adoption
  intents plus a new config-clear request before feature routing; Chase/TWAP
  progress gates independently keep autonomous work queued. Result/status
  reconciliation and explicit exposure-reducing cancellation remain available.
  A config clear started before close completes its established cleanup/error
  handling and then exits when the main-window owner is set. A failed final save
  still clears the fence under the separately deferred F-24 policy.
- Saved-profile deletion stages encrypted cleanup in existing zeroizing payload
  scopes, then moves the removed `AccountProfile` into one rollback owner. A
  pre-install durable-save failure moves that same key allocation back; a
  successful save scrubs its agent and legacy per-profile integration keys
  before ID-only keychain cleanup. Plain config snapshots still contain empty
  credential fields. Post-install config-save failure policy remains separately
  deferred as F-31 (`src/account_state/switching/saved_delete.rs:18-95`,
  `src/account_state/switching/saved_delete.rs:235-340`).
- Wallet-address editing and connect-time address rebinding share one move-only
  rollback owner for the active profile key and key-input buffer. Persistence
  receives one short-lived saved-profile snapshot after the active key is
  absent; ordinary failure restores the exact allocations, while success
  scrubs old keys plus transient profile identity/address copies. Existing
  storage ordering and failure feedback remain unchanged
  (`src/account_update/profile_rebinding.rs:9-69`,
  `src/account_update/profile.rs:150-234`,
  `src/account_update/connection.rs:129-216`).
- Explicit agent-key saving constructs one caller-owned, ghost-filtered profile
  snapshot with the draft key substituted directly, leaving the prior canonical
  signing key untouched while either storage backend runs. Failure drops the
  staged key; success moves that exact persisted allocation into the same
  originating profile after an identity check, then drops all other snapshot
  keys before scheduling config persistence. Backend payload/serialization
  copies remain in their existing synchronous zeroizing scopes
  (`src/account_state/persistence.rs:38-75`,
  `src/account_update/profile.rs:335-389`).
- Account switching evaluates same-profile, pending-request, Chase, and
  uncertain-TWAP gates before constructing a minimal target. A saved target
  clones only its canonical agent key and moves that exact allocation into the
  intentional key-input owner; rejected and ghost switches copy no key. All
  switch callers converge on this boundary
  (`src/account_state/switching.rs:15-35`,
  `src/account_state/switching.rs:324-399`).
- Keyed add-account submission validates against the window draft, moves one
  key-bearing profile into the credential-storage snapshot, and exposes only a
  keyless canonical metadata shell during synchronous persistence so encrypted
  config can still commit profile metadata atomically. Failure removes that
  shell and drops staging while preserving the exact draft; success moves the
  staged profile into canonical state. First-account synchronization moves the
  verified normalized draft allocation into the key input, while ordinary
  switch-on-add still uses the post-gate switch boundary
  (`src/account_update/add_window.rs:24-30`,
  `src/account_update/add_window.rs:88-289`).
- Deferred legacy account-key loading is an OS-keychain-only step after an
  authorized switch finds no bundled key. Its lookup shell contains only the
  profile secret ID and the pre-existing per-profile Hydromancer fallback that
  affects legacy precedence. A loaded agent allocation moves into the canonical
  profile and is copied once for the active input; a newly migrated normalized
  Hydromancer allocation moves into global runtime state and is copied once for
  its input.
  Conflict, trimming, generation/cache invalidation, bundle persistence, and
  cleanup ordering retain their established owners
  (`src/account_state/switching.rs:98-210`).
- Startup partial-bundle hydration treats the active legacy profile as one
  agent/Hydromancer transaction. It reads through an identity-only shell,
  and, when the bundle has no global Hydromancer key, places both values in the
  attempted bundle before profile-wide cleanup. It move-retains those loaded
  buffers in plaintext config until storage succeeds, so store failure
  preserves retry authority. An already-populated bundle retains its
  established Hydromancer precedence pending the separately deferred
  conflict-policy decision
  (`src/config/files/storage.rs:178-228`,
  `src/config/files/storage.rs:396-479`).
- OS-keychain-to-encrypted selection constructs the candidate payload directly
  from borrowed persisted-profile references. Legacy readers receive exact IDs
  and non-secret presence guards rather than canonical metadata or credential
  contents; newly loaded missing values, or required normalized buffers, move
  into the payload. Keychain cleanup snapshots contain profile identities only,
  including when moved into the asynchronous clear-config task
  (`src/config/secrets/model.rs:237-305`,
  `src/secret_storage/encrypted.rs:13-35`,
  `src/secret_storage/selection.rs:36-58`,
  `src/secret_storage/selection.rs:355-532`,
  `src/config_persistence/clear.rs:80-84`).
- Chase reconciliation for fills, refreshes, stops, archives, and final
  replacements now derives open-order authority independently for each active
  Chase's origin symbol. Account-wide fill completeness remains a separate
  requirement, and websocket disappearance remains dex-scoped; an unrelated
  scoped REST lane cannot authorize absence or a new placement
  (`src/account_update/stream/chase/fill_reconciliation.rs:17-146`,
  `src/account_update/stream/chase/refresh.rs:37-292`,
  `src/order_execution/chase/lifecycle/place.rs:247-290`).
- Wallet-cluster fan-out has a separate update route, but each member leg uses
  shared preparation/signing, its own CLOID, and a tuple of execution ID,
  profile secret ID, and CLOID for local correlation.
- This structure remains appropriate: the campaign should harden handoffs and
  idempotence, not replace it with a monolithic order manager.

## Lifecycle Assurance Matrix

| Surface/operation | Immutable origin identity | Correlation key | Idempotency key | Immediate-result classifier | Authoritative reconciliation | Stale-result guard | Terminal cleanup | Existing tests | Gaps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Ticket place | Submission snapshot; captured account/key; `OneShotPlacementContext` with account, surface, symbol, kind | Indicator ID + placement context; request ID after uncertain result | Unique one-shot CLOID | `classify_execution_result` | `orderStatus` by CLOID + connected-account refresh; fills consume market projections | Serialized pending gate; snapshot equality; current-account and pending-request match | Clears global action/indicator; exact status or a complete open-order/fill snapshot covering the origin symbol removes the status request | `order_execution/submit/tests.rs`, `order_execution/core.rs` tests, `order_update/results/tests.rs` | F-02 addressed in Turn 3; executable regression validation remains environment-blocked by missing ALSA metadata |
| Preset place | Preset preflight then the shared ticket fields/context with `OrderSurface::Preset` | Same as ticket | Unique one-shot CLOID | Shared classifier | Shared one-shot reconciliation | Pending/reconciliation gates run before preflight and again on submit | Shared ticket/result cleanup | `order_update/presets.rs` tests | No confirmed gap; retain double preflight as deliberate queued-event defense |
| Alfred place | Parsed draft preflight; then current form and captured signing context | Same ticket result message/context | Unique one-shot CLOID | Shared classifier | Shared one-shot reconciliation | Alfred preflight plus shared submit gates | Shared ticket/result cleanup | `alfred_update/submit.rs` tests and ticket tests | Verify command-to-form handoff in the later origin-identity track |
| Quick place | Chart ID/surface snapshot and recovery data; captured account; one-shot context | Indicator ID + CLOID context; optional form recovery | Unique one-shot CLOID | Shared classifier | Shared CLOID status + account refresh | Chart/surface/symbol and percentage provenance checks; current-account match | Clears global action/indicator; rejection may restore matching form | `order_execution/quick_order/submit/tests.rs`, `order_update/quick_order/form/tests.rs` | No confirmed gap |
| HUD place | `HudOrderRequest` captures chart/surface/symbol/side; account context; one-shot context | Market: global action + CLOID. Limit: HUD in-flight ID + indicator + CLOID | Unique one-shot CLOID | Shared classifier | Shared CLOID status + account refresh | Chart/surface/symbol/arm checks; per-account limit tracker; current-account match | Market clears global action; limit finishes its tracker entry; both clear indicator | `order_update/hud.rs` tests, `order_execution/hud.rs` tests | Per-tracker ID wraps without collision handling; practically remote, audit with other allocators |
| Close-position place (UI or Alfred) | Fresh connected-account position snapshot; account/key; coin/fraction; one-shot context | Global close action + indicator + CLOID context | Unique one-shot CLOID | Shared classifier | Shared CLOID status + account refresh | Pending/reconciliation/freshness/completeness gates; current-account match | Clears close action/indicator; shared one-shot terminal handling | `order_execution/position_actions/close/tests/**`, result tests | No confirmed gap |
| NUKE child place (UI or Alfred) | Parent execution ID; connected account; planner output; per-child one-shot context | Execution ID + child CLOID in the result context and aggregate settlement set | Unique one-shot CLOID per child; the first terminal transition claims it | Shared classifier | Uncertain child gets CLOID status; parent refreshes after aggregate completion | Current-account and execution-ID checks; duplicate settlement is a no-op | CLOID-keyed confirmed/failed/uncertain totals; parent removed after the unique settled-child count reaches total | `order_execution/position_actions/nuke/tests/**`, direct/status duplicate regressions in `order_update/results/tests.rs` | F-01 addressed in Turn 2; executable regression validation remains environment-blocked by missing ALSA metadata |
| Cancel by OID | Connected account + symbol + OID + runtime request sequence; one request owns awaiting-result/checking-status phases | Request sequence + account + OID + symbol; indicator is presentation only | Target OID; runtime sequence is correlation only | Shared classifier plus confirmed-cancel predicate | `orderStatus` by OID + origin-symbol open-order refresh | Exact request/phase/account match for direct result; exact request/account/OID/symbol match for status; refresh must cover origin lane | Confirmed/terminal result removes matching local order; only status-check phase and a covering snapshot can release refresh uncertainty | `order_execution/position_actions/cancel.rs` tests, cancel result/status duplicate, stale-attempt, and scoped-refresh tests | F-14/F-15 addressed in Turns 13-14; executable validation remains environment-blocked |
| Move/modify | Connected account + symbol + OID + runtime request sequence; original key and exact prepared target captured in `PendingMoveOrderContext` | Request sequence + account + symbol + OID; indicator ID is presentation/local projection only | Target OID; runtime sequence is correlation only | Shared classifier plus confirmed-modify predicate | `orderStatus` by OID + origin-symbol refresh that classifies target absent/expected-price/different-valid-price | Exact request/account/move-key context for direct result; exact request/account/OID/symbol for status; refresh rejects uncovered or malformed target evidence | Removes only the matching move context/indicator; terminal status or sufficient target-lane evidence clears status uncertainty | `order_execution/quick_order/move_order/tests/**`, `order_update/move_order.rs` duplicate/stale-attempt/scoped-refresh tests | F-14/F-15 addressed in Turns 13-14; executable validation remains environment-blocked |
| Chase place/replacement | `ChaseOrder` captures ID, account, agent key, symbol, side, sizes, start time, lifecycle | Chase ID + lifecycle + dispatch-time place attempt; current CLOID is checked by status path | CLOID hashes account + chase ID + start + attempt | Chase-specific strict response analysis | CLOID status + account-wide fills + origin-symbol open-order refresh/stream reconciliation | Exact place-attempt equality + `expects_place_result`; current account, symbol identity, prior-exposure, per-symbol open-order authority, reconciliation, and final-exit progress gates | Moves to verification/resting/stop/archive; late stopped placement is cancelled; final replacement gate repeats origin-lane proof; exit-pending place remains queued | Chase lifecycle/place/result/status tests, duplicate/late direct-result regressions, account stream tests, wrong-scope replacement tests, and exit-fence/resumption tests | F-05/F-16/F-23 addressed in Turns 7/15/22; executable regression validation remains environment-blocked by missing ALSA metadata |
| Chase modify | Chase ID + captured account/key + current OID + lifecycle + desired price | Chase ID + OID + dispatch-time reprice count + `expects_modify_result` | Target OID; no separate exchange idempotency key (runtime sequence is correlation only) | `is_confirmed_modify_result` and Chase-specific error handling | OID status + account-wide fills + origin-symbol open-order refresh/stream reconciliation | Exact reprice-count/lifecycle/OID match; account, symbol, origin-lane, reconciliation, and final-exit progress checks before correction/replacement | Verification/resting/stop flow; terminal Chase archived only from covering evidence; exit-pending reprice/size correction remains queued | `order_update/chase/modify/tests/**`, including duplicate/late results, wrong-scope refresh, and exit-fence/resumption tests | F-05/F-16/F-23 addressed in Turns 7/15/22; executable regression validation remains environment-blocked by missing ALSA metadata |
| Chase cancel | Chase ID + captured account/key + OID + stopping phase | Chase ID + OID + `expects_cancel_result` | Target OID; bounded retry treats terminal-not-open responses specially | Confirmed-cancel predicate plus Chase cancel classification | OID status + account-wide fills + origin-symbol account refresh/open-order disappearance | Exact stopping phase/OID; REST archive requires the origin open-order lane; websocket disappearance is dex-scoped | Verifying-cancel then covering-snapshot archive; bounded manual-check terminal | `order_update/chase/cancel/tests.rs`, Chase stop/status tests, and wrong-scope archive regression | F-08/F-16 addressed in Turns 9/15; retry idempotence depends on target-specific cancel semantics; executable validation remains environment-blocked |
| TWAP child place | `TwapOrder` captures ID/account/key/symbol/plan; `TwapPendingSlice` captures index/size/price/CLOID/retry | TWAP ID + dispatch-time slice index/retry count + current `pending_op`; status path adds exact CLOID and armed retry attempt | Deterministic child CLOID (runtime index/retry tuple is correlation only) | TWAP-specific IOC/fill/resting/transport classification | CLOID status + scoped account-fill refresh + reconciliation deadline | Exact pending slice/retry for placement; status result requires current CLOID/attempt; current-account, terminal, and final-exit progress guards remain | Finishes attempt once; child status/fills updated; status ownership cleared on result or account-fill resolution; result/status/fill/deadline terminals archive and scrub; final pre-dispatch skip terminalization directly scrubs; exit-pending due slice stays unsent | `order_execution/twap/tests/**`, including duplicate/late slice/status results, terminal initial/retry skip key lifetime, exit-fence/resumption, and activity diagnostic redaction, plus `twap_state/tests/**` | F-05/F-19/F-23/F-28/F-44 addressed in Turns 7/18/22/25/37; F-29 final-skip history visibility is deferred; executable regression validation remains environment-blocked by missing ALSA metadata |
| TWAP unexpected-child cancel | TWAP ID + captured key + OID/CLOID target + exact armed retry attempt | Pending target plus current retry count and one in-flight attempt; retry and result messages both carry the attempt | Target-specific cancel by CLOID preferred, else OID | Confirmed-cancel/terminal-not-open/error handling | Immediate origin-account refresh; child status and later fills | Dispatch atomically requires non-terminal exact target/retry with no existing owner; result requires exact target/retry/owner | Consumes one owner, then clears pending cancel and finishes once or schedules the next bounded attempt; a result arriving after fill terminalization retains refresh but cannot schedule retry work | `order_execution/twap/tests/cancel.rs`, placement/status entry-path tests, fill/cancel and terminal-result characterizations | F-08/F-20/F-22 addressed in Turns 9/19/21; F-21's delivery-order-dependent child label is deferred for an explicit UX/history semantics decision; financial accounting is order-independent |
| Wallet-cluster order child | Execution ID + profile secret ID + member address/key + one-shot context | Execution/profile/CLOID plus account, symbol, surface, and order kind | Unique one-shot CLOID per member leg; direct result may leave `Pending` once | Shared classifier | CLOID status + member refresh + member user stream | Full origin match; direct requires `Pending`, status requires `Checking`; pending executions are not evicted | First terminal leg outcome is immutable; execution complete when every leg terminal | Cluster planning/member tests plus adversarial result/status tests in `wallet_cluster_update.rs` | F-04 addressed in Turn 5; executable regression validation remains environment-blocked by missing ALSA metadata |
| Wallet-cluster close child | Same as cluster order, plus fresh per-member position snapshot and reduce-only plan | Same full correlation tuple with `ClusterClose` surface | Unique one-shot CLOID per member leg; direct result may leave `Pending` once | Shared classifier | CLOID status + member refresh + member stream | Freshness/side/position preflight plus the shared exact transition guard | Same first-terminal-wins handling as cluster orders | Close sizing/freshness tests and shared adversarial result tests | F-04 addressed by the shared Turn 5 transition guard |
| Leverage update | `PendingLeverageUpdateContext` captures account, symbol, asset, dex, margin mode, leverage | Full pending-context equality | None; mutation is not blindly retried | Confirmed-default predicate; other non-error bodies are uncertain | Scoped account refresh for outcomes that may have committed | Pending-context equality + current-account match | Pending context cleared once; matching form updated only on confirmed default | `order_update/leverage.rs` tests, signing action/response tests, diagnostic-redaction tests | F-42 diagnostic exposure addressed in Turn 35; executable validation remains environment-blocked. No exact mutation status endpoint; verify refresh completeness is sufficient in the remaining external-status audit |

Turn 8 verified that every placement and modify row above reaches the shared
signed-action builder. Valid field mapping is direct and covered at both the
prepared-request and signed JSON/msgpack boundaries. The cross-cutting F-07
guard applies to every such row without duplicating market, sizing, precision,
side, reduce-only, or asset/symbol policy in signing.

Turn 9 verified the parsed-response classification for every shared and
advanced mutation handler. The cross-cutting F-08 guard applies to every caller
of the shared classifier, including one-shot, quick/HUD, cancel, move, NUKE, and
wallet-cluster results, and explicitly to Chase place/modify/cancel plus TWAP
child-place/unexpected-cancel handling. Pure errors, unambiguous successes,
transport failures, and unstructured malformed responses retain their prior
paths; only simultaneous error/effect claims reconcile.

Turn 36 hardens the diagnostic handoff immediately after that shared
classification. Normalized status remains byte-for-byte available to all
one-shot, quick/HUD, cancel, move, NUKE, and wallet-cluster consumers, while the
local `ExecutionOutcome` formatter now reports only kind and control flags. It
does not alter classification, visible copy, or reconciliation behavior.

Turn 37 closes the equivalent live TWAP activity gap. Exact planned-skip and
event messages still drive the same order status, activity rows, and persisted
terminal history, while their pre-recording/runtime formatters expose only
structural metadata. Existing error sanitizers remain the external-text gate;
no scheduling, result, retry, archive, serialization, or visible copy changes.

Turn 38 closes the persisted successor graph for both Chase and TWAP. Entry,
log, child, and pre-snapshot fill-metrics diagnostics no longer traverse exact
account/order/history values; their serde representation and all direct history
view/snapshot calculations remain exact. Config compatibility and terminal
archive/upsert/pruning behavior are unchanged.

Turn 16 adds a causal precondition to every matrix row that uses the connected
account snapshot as authoritative fallback: a successful REST result may reach
one-shot, cancel/move, Chase, connected-account TWAP, leverage, or overlay
reconciliation only when `account_data_revision` is unchanged since dispatch.
If a user-data delta advanced it, the current merged state and uncertainty are
preserved while one sequential post-delta refresh starts. Off-account TWAP fill
reconciliation remains separately address/generation-scoped and does not
replace connected account state.

Turn 17 adds a preceding provenance requirement to every row consuming account
or selected-cluster user data: the frame's address, purpose, dex scope, and
runtime recipe generation must still equal the subscription requested by
current application state. An old same-address recipe therefore cannot drive
order, fill, Chase/TWAP, or cluster-close freshness transitions after account,
window, cluster, profile, or market-scope replacement.

Turn 18 adds exact ownership to TWAP child-status repair. At most one lookup is
armed for the current CLOID/retry attempt, and only that attempt can consume a
result. The CLOID can remain set independently while a definitive exchange
status waits for account-fill confirmation without allowing a duplicate or
older status result to rewrite that later reconciliation phase.

Turn 19 extends exact ownership to TWAP unexpected-child cancellation. A
target-specific retry trigger must atomically arm the current attempt before it
can dispatch, and only a result carrying that target and attempt can consume
retry budget or finish the pending cancellation. Scheduled backoff remains a
separate phase with no in-flight owner.

Turn 20 verifies both fill-versus-cancel delivery orders. Per-child and
aggregate fill quantities, average price, fee, remaining target, completion,
history financial metrics, and repeated-fill idempotence are equivalent. The
child label is deliberately not normalized: current live/history rendering
exposes whether `Filled` or `UnexpectedRestingCancelled` wrote last, and
choosing a dominant or combined label requires the deferred F-21 product
decision.

Turn 21 closes the remaining TWAP terminalization plumbing audit. Terminal
statuses reject slice scheduling, timer subscriptions, status dispatch, cancel
retry dispatch, and delayed status mutation. Terminal archive calls upsert one
history identity and scrub the captured key. If full-fill reconciliation wins
while an unexpected-child cancel is in flight, a retryable late result now
keeps the immediate origin-account refresh without creating a delayed trigger
that terminal state could only reject.

Turn 22 begins Track 9 by proving the restart persistence boundary and fencing
the live final-save interval. Live automation/request owners remain absent from
config and boot empty. After the main window closes, the final-exit flag remains
set until process exit; Chase place/reprice/size correction and due TWAP slices
stay queued, while exact status repair and exposure-reducing cancel paths retain
their existing authority. A failed final save clears the flag and the same
queued values resume through their normal gates.

Turn 23 extends that shutdown authority across every fresh signed-mutation
intent and both persistence modes. The root update boundary fences all direct
order surfaces plus leverage, presets, Alfred trading, wallet-cluster fan-out,
and Chase/TWAP start/adoption messages after `WindowClosed`, while cancel/stop
and result/status messages remain routable. It also rejects a new clear request
after close. A clear already requested before close no longer clears the same
owner during deferred-save handoff, start, or partial-result handling; success
or redacted failure returns the existing clear work plus `iced::exit()`. A
clear-start race that discovers already-dispatched trading work honors the close
with an exit rather than leaving an unfenced headless daemon. Normal in-app
clear and all exchange/result semantics are unchanged.

Turn 25 completes the captured-key terminal-assignment map. Chase terminality
is represented by removal, so dropping the removed `CapturedAgentKey` scrubs it.
TWAP result, status, fill, stop, deadline, and reconciliation-timeout paths all
reach `archive_twap_if_terminal`, which upserts history and clears the key. The
two exceptions are final pre-dispatch initial/retry skips: their shared
`schedule_after_attempt` can terminalize without reaching archive. Those paths
now clear the key at terminality while preserving the established absence of a
new visible/persisted history row; nonterminal skips retain the key for later
slices. Whether final skips should enter history is separately deferred as
F-29.

Turn 26 closes the saved-profile deletion raw-key owner map. Encrypted deletion
prepares its replacement blob from `Zeroizing` profile/payload/plaintext values;
OS keychain deletion first persists its established cleanup intent. Both modes
then move the removed profile into the same rollback owner. An ordinary durable
save failure reinserts the original allocation, while success explicitly
scrubs that owner before keychain cleanup receives only the profile secret ID.
No raw profile is cloned for cleanup and no unrelated account key is copied.
The distinct post-install-save marker can still make runtime and disk disagree;
F-31 preserves and characterizes that exceptional behavior pending an explicit
durability/feedback decision.

Turn 27 extends the same one-owner rule to both active-profile address-rebind
entry points. The old paths each held cloned rollback keys, a complete cloned
account vector, and another persisted clone. They now move the two canonical
usable key buffers into a shared rollback owner, mutate only the active binding,
and build one short-lived persistence snapshot whose active key is already
empty. Ordinary encrypted/keychain failures move the exact allocations back;
success scrubs the rollback immediately. F-31 now records the common installed-
snapshot policy gap across saved deletion and OS-keychain rebinding without
changing any exceptional feedback.

Turn 28 closes the explicit-save caller's agent-key commit clone graph. Storage
still receives a complete saved-profile credential bundle, but the caller now
builds one snapshot directly with the draft key: the old committed active key is
never copied into caller staging, ghost profiles remain absent, and every
unrelated saved key is copied once there rather than twice before backend-owned
serialization. The canonical signing key stays unchanged during the synchronous
storage callback. A rejected save drops the staged draft; an accepted save moves
the exact persisted allocation into the original profile and destroys the
remaining snapshot before the unchanged debounced config save.

Turn 29 begins the repository-wide Track 9 diagnostic audit at the signed
response boundary. F-10 made the top-level response safe, but its public nested
inner/data models still derived raw `Debug`, and a type-only summary trusted the
externally supplied response type. All three diagnostic layers now independently
retain only allowlisted type/count metadata while exact status values remain
available to the unchanged classifiers. Safe protocol summaries remain exact.

Turn 30 closes the ordinary account-switch credential clone graph. The old
entry point copied the complete target profile before any no-op or financial
gate and copied its key again after a successful switch. Target capture now
occurs only after every blocker plus synchronous old-account cleanup. Saved
profiles create one moveable key-input allocation, rejected switches create
none, and ghost switching never copies a stray canonical key before scrubbing
it. Existing switch, stop, reset, persistence, and connect/disconnect behavior
is unchanged.

### Mutation Transport Phase Audit

| Boundary | Can the exchange have observed the mutation? | Downstream treatment | Replay/retry behavior |
| --- | --- | --- | --- |
| Wire validation, msgpack, key decode/signing, JSON serialization, or dedicated-client initialization | No | Preserved `Err` shape; existing handlers conservatively classify or handle it as status unknown | No HTTP dispatch and no application retry |
| Request construction or connect failure | No for builder/connect failures, but reqwest's public error string is deliberately not used as a financial proof | `TransportUnknown`; exact CLOID/OID status and/or scoped account refresh | Dedicated client has retries and redirects disabled |
| Send error or 15-second timeout | Possibly; bytes may have left the process | `TransportUnknown`; exact reconciliation | No transport replay and no placement/modify retry |
| HTTP response body read failure | Yes | `TransportUnknown`; exact reconciliation | No retry |
| Non-JSON or structurally unparseable HTTP body, regardless of HTTP status | Yes | `TransportUnknown`; redacted bounded body snippet, then exact reconciliation | No retry |
| Non-success HTTP envelope with apparently successful/incomplete parsed body | Yes | F-11 `TransportUnknown`; exact reconciliation | No retry |
| Parsed exchange rejection under any HTTP status | Yes, with explicit no-effect error | `Rejected`; no ambiguity cleanup is released by inference | No retry |
| Parsed success or conflicting effect/error statuses | Yes | Strict success classifier, or F-08 `Ambiguous` reconciliation on conflict | No retry |

The result type intentionally remains `Result<ExchangeResponse, String>` in
this turn. Distinguishing provably local errors in downstream UX would change
form recovery, refresh, and automation failure paths without improving safety;
the existing over-approximation is fail-closed. The transport boundary instead
prevents hidden redirects/replays, bounds every request, and sanitizes both
`Ok` and `Err` diagnostic formatting before derived `Message::Debug` can observe
the result. Existing bounded Chase/TWAP cancel retries are lifecycle-level,
target-specific cancellation policy, not HTTP replay.

## Ranked Findings and Audit Candidates

### F-01 — NUKE child aggregation is not idempotent

- Status: addressed in Turn 2; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium, with a High consequence if duplicate delivery occurs during
  a multi-child emergency close
- Scope: NUKE result and status-result plumbing
- Preconditions/event ordering:
  1. A NUKE parent has at least two child placements.
  2. The same child's `NukeResult` or `NukePlacementStatusLoaded` is handled
     twice before a different child settles.
  3. Both messages carry the current execution ID and account context.
- Evidence:
  - `PendingNukeExecution` stores totals only and every `record_*` call
    increments `completed` (`src/order_execution.rs:85-129`).
  - `handle_nuke_result` and `handle_nuke_placement_status_result` receive a
    child CLOID in context but pass only `execution_id` into the record helpers
    (`src/order_update/results.rs:466-510`, `src/order_update/results.rs:785-845`).
- Violated invariant: one logical child contributes exactly once to its parent
  aggregate.
- Risk: the parent can reach `completed >= total`, clear its pending blocker,
  and report completion while another child is still in flight.
- Implemented fix: NUKE parents retain a runtime-only set of settled child
  CLOIDs. The first confirmed, failed, or uncertain terminal transition claims
  its CLOID; subsequent transitions for that CLOID cannot increment any count
  or finish the parent. Completion is derived from the number of unique settled
  children (`src/order_execution.rs:84-186`,
  `src/order_update/results.rs:470-522`,
  `src/order_update/results.rs:813-886`).
- Regression coverage: direct exchange results and `orderStatus` results each
  deliver the same child twice in a two-child execution, assert the unchanged
  `1/2` progress text and pending parent, then settle a distinct CLOID
  (`src/order_update/results/tests.rs:995-1072`). The direct regression also
  proves `PendingNukeExecution` debug output does not expose its retained CLOID.
- Protected behavior: unique child outcomes retain the existing confirmed,
  failed, uncertain, skipped, refresh, error-state, and status-text behavior.
  The change does not affect request construction, signing, dispatch, order
  semantics, views, persistence, or user interaction timing.

### F-02 — Successful refresh may clear unresolved one-shot status state too broadly

- Status: addressed in Turn 3; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: shared one-shot placement reconciliation for ticket, preset, Alfred,
  quick-order, HUD, and close-position surfaces
- Preconditions/event ordering:
  1. A one-shot placement has an unresolved CLOID status request.
  2. The exact `orderStatus` response is missing, non-definitive, or fails.
  3. A connected-account refresh returns `Ok(AccountData)` while open orders or
     fills are incomplete, or while its open-order scope excludes the origin
     symbol's market.
  4. The account-wide cleanup treats that snapshot as resolution.
- Evidence:
  - Open-order and fill requests are best-effort. Failures mark their sections
    incomplete but the bootstrap still returns `Ok(AccountData)`
    (`src/account/data/bootstrap.rs:160-228`,
    `src/account/data/bootstrap/responses/best_effort.rs:26-38`).
  - `AccountDataFetchScope` can contain only one HIP-3 dex, and its completeness
    applies to that fetched scope (`src/account/types/data/fetch_scope.rs:40-73`).
  - The prior pending record discarded `symbol_key`, and successful refresh
    cleanup removed every request for the account without inspecting either
    completeness lane or scope.
- Violated invariant: fallback reconciliation may release an uncertain
  placement only when the snapshot contains both independent outcome lanes —
  open orders for the placement's origin market and account-wide fills.
- Risk: a partial or unrelated-market snapshot can release the pending-trading
  blocker while an open or filled order remains absent from local account state,
  allowing the next mutation to be prepared from incomplete exposure.
- Implemented fix: pending one-shot status records retain the runtime-only
  origin symbol. Refresh cleanup now removes each account-matching request only
  when fills are complete and the snapshot successfully fetched that symbol's
  open-order lane. The account-data helper uses per-market fetch timestamps so
  an unrelated dex failure does not invalidate a healthy origin lane
  (`src/account/types/data.rs:198-213`,
  `src/order_update/results.rs:33-86`,
  `src/order_update/results.rs:583-606`).
- Regression coverage: incomplete open-order and incomplete fill snapshots both
  retain the request and trading blocker; a complete snapshot for another
  HIP-3 dex also retains them; a later complete snapshot covering the origin
  dex performs the existing cleanup (`src/order_update/results/tests.rs:660-729`).
- Protected behavior: exact CLOID status handling is unchanged. Existing tests
  continue to characterize that a complete, covering fallback refresh clears
  missing, cancelled, or errored status requests with the same status text and
  blocker behavior (`src/order_update/results/tests.rs:578-658`). No request,
  signing, order semantics, view, persistence, or normal-path timing changed.

### F-03 — Pending one-shot debug output exposes the CLOID

- Status: addressed in Turn 4; focused test added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium privacy hardening
- Scope: `PendingOneShotStatusRequest` diagnostic formatting only
- Evidence: the formatter redacted the account address but previously emitted
  the full CLOID, while `OneShotPlacementContext`, signed wire types, and order
  status result models already redact the same identifier.
- Violated invariant: a sensitive order correlation identifier retained for
  lifecycle matching must not be exposed through general diagnostic formatting.
- Risk: formatting parent application state or a pending request during
  diagnostics could disclose a stable identifier derived from account and order
  inputs even though adjacent boundaries are redacted.
- Implemented fix: the existing `cloid` debug field now emits `<redacted>` while
  retaining the same struct/field shape (`src/order_update/results.rs:42-50`).
- Regression coverage: the focused formatter test requires explicit redaction
  markers for both the account address and CLOID and rejects both synthetic raw
  values (`src/order_update/results/tests.rs:175-185`).
- Protected behavior: storage, equality, request correlation, status handling,
  UI strings, order semantics, and persistence are unchanged. Only `Debug`
  output differs.

### F-04 — Wallet-cluster result transitions lack focused correlation/idempotence coverage

- Status: addressed in Turn 5; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium
- Scope: wallet-cluster order and close direct-result/status-result plumbing
- Preconditions/event ordering:
  1. A cluster execution retains a pending, checking, or terminal member leg.
  2. A stale or mismatched result reaches the current execution ID, or a direct
     or status result is delivered more than once/out of its expected phase.
  3. The prior helper finds no leg or finds an already-terminal leg but does not
     report whether the update was valid.
  4. The handler still rewrites state or launches repair/refresh and aggregate
     status work.
- Evidence:
  - The direct and status handlers previously assigned any target status to a
    matching execution/profile/CLOID leg regardless of its current phase.
  - The void update helper could not tell callers that an execution, profile, or
    CLOID did not match, so they still refreshed a member, launched CLOID status
    repair, and/or recomputed aggregate UI state.
  - Handler-level adversarial cases prove a confirmed leg could be rewritten as
    `Checking` by a late ambiguous direct result or as `Failed` by a conflicting
    duplicate; a status result could settle a leg that was never `Checking`.
- Violated invariant: each cluster child accepts exactly one direct result from
  `Pending`; only an ambiguity-created `Checking` child accepts one status
  result; terminal outcomes are immutable.
- Risk: duplicate or misrouted lifecycle messages can regress a completed
  execution to pending, change confirmed member outcomes into failures, produce
  false aggregate problem counts, and launch irrelevant repair tasks.
- Implemented fix: one transition helper now verifies execution ID, profile
  secret ID, CLOID, account address, symbol, cluster/close surface, exchange
  order kind, and expected source phase before mutating a leg. Direct handlers
  require `Pending`; status handlers require `Checking`; failed transitions
  return before any repair, refresh, aggregate status, or message mutation
  (`src/wallet_cluster_update.rs:1096-1272`).
- Regression coverage: exact-origin mismatch cases cover execution, profile,
  CLOID, address, symbol, surface, and order kind; both order and close legs
  reject terminal-to-checking and terminal-to-failed duplicates; status results
  are ignored before `Checking`, settle it once, and cannot rewrite the terminal
  outcome (`src/wallet_cluster_update.rs:1652-1834`).
- Protected behavior: valid direct transitions remain
  `Pending -> Confirmed|Failed|Uncertain|Checking`, and valid reconciliation
  remains `Checking -> Confirmed|Failed|Uncertain`. Existing messages, aggregate
  status strings, refresh behavior, fan-out sizing, order preparation, signing,
  UI, and persistence are unchanged for unique correctly correlated results.

### F-05 — Advanced place/modify result messages rely on lifecycle state rather than per-attempt tokens

- Status: addressed in Turn 7; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium invariant hardening
- Scope: Chase place, Chase modify, and TWAP child-place direct-result message
  correlation
- Preconditions/event ordering:
  1. A direct result for attempt N is handled and advances the strategy.
  2. The same strategy later dispatches attempt N+1 and returns to `Placing`,
     `Modifying` with the same OID, or a TWAP `Place` pending operation.
  3. A duplicate or delayed message for attempt N arrives while attempt N+1 is
     in that recurring coarse phase.
  4. The old handler sees a valid current phase (and, for modify, the same OID)
     and applies the earlier exchange outcome to the current attempt.
- Evidence:
  - Chase replacement placement already increments `place_attempt_count` and
    derives a distinct CLOID, but the result closure previously discarded the
    attempt (`src/order_execution/chase/lifecycle/place.rs:358-394`).
  - Chase increments `reprice_count` before each modify, while Hyperliquid may
    retain the OID, but the result message previously retained only the OID
    (`src/order_execution/chase/lifecycle/reprice.rs:333-348`).
  - TWAP pending state already owns a slice index and retry count. A retry
    reuses its logical child and CLOID, but the direct result previously carried
    only the TWAP ID (`src/order_execution/twap/execution.rs:308-318`).
  - The immediate-duplicate phase guards were sound, but they did not identify
    which dispatch owned a result once the same phase recurred.
- Violated invariant: an asynchronous mutation result may settle only the exact
  in-flight attempt whose dispatch created it.
- Risk: a stale rejection, fill, ambiguity, or acceptance can fail, credit,
  pause, verify, cancel, or otherwise settle a later Chase/TWAP attempt despite
  referring to earlier exchange work.
- Implemented fix: the three internal result messages now capture the safe
  runtime sequence already assigned before dispatch: Chase place attempt,
  Chase reprice count, or TWAP slice index plus retry count. Result handlers
  require exact sequence equality in addition to their existing lifecycle,
  OID, and pending-operation checks (`src/message.rs:1170-1175`,
  `src/message.rs:1214-1223`, `src/order_update/chase/result.rs:126-157`,
  `src/order_update/chase/modify.rs:35-59`,
  `src/order_execution/twap/slice_result.rs:38-61`). No CLOID, account value,
  or new persisted/runtime state was added. A mismatched sequence returns
  before account-refresh or status-repair follow-up; a same-attempt result that
  arrives after another lifecycle transition retains the former conservative
  refresh behavior.
- Regression coverage: each surface receives a conflicting duplicate after the
  first direct result has advanced its phase. Separate late-result cases put a
  newer Chase place attempt, same-OID Chase reprice, or TWAP slice/retry in
  flight and prove an earlier result cannot mutate it
  (`src/order_update/chase/result/tests.rs:154-209`,
  `src/order_update/chase/modify/tests/success.rs:58-115`,
  `src/order_execution/twap/tests/place_result.rs:103-188`). The TWAP case
  independently checks both slice-index and retry-count mismatch, and the
  Chase-place/TWAP cases prove stale outcomes cannot launch account refresh.
- Protected behavior: an exactly matching direct result follows the same
  classifier, reconciliation, fill, retry, stop, archive, refresh, and visible
  status paths as before. Chase/TWAP scheduling, pricing, sizing, repricing,
  CLOIDs, signed requests, UI, persistence, and normal timing are unchanged.

### F-06 — Wallet-cluster leg debug output exposes its lifecycle message

- Status: addressed in Turn 6; focused test added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium privacy hardening
- Evidence: `WalletClusterExecutionLeg::Debug` redacts the explicit CLOID,
  address, symbol, size, and price fields but formats `message` verbatim
  (`src/wallet_cluster_state.rs:220-236`). Unexpected-resting result messages
  embed the CLOID and exchange summary (`src/wallet_cluster_update.rs:1141-1149`,
  `src/wallet_cluster_update.rs:1189-1195`).
- Risk: diagnostic formatting can bypass the explicit field redaction and reveal
  the same order identifier or order-result detail through the derived lifecycle
  message.
- Implemented fix: the custom formatter now emits `<redacted>` for `message`
  while retaining the field name and all safe status metadata
  (`src/wallet_cluster_state.rs:220-236`).
- Regression coverage: a synthetic leg embeds its CLOID in the stored lifecycle
  message, proves the formatted output omits it, and independently proves the
  stored message remains unchanged for the existing view
  (`src/wallet_cluster_state.rs:608-630`).
- Protected behavior: the view continues reading `leg.message` directly; result
  handling, state, UI copy, order semantics, and persistence are unchanged.

### F-07 — Signed order actions lack independent structural validation

- Status: addressed in Turn 8; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium invariant hardening
- Scope: every placement and modify action at the shared signing-payload
  boundary
- Preconditions/event ordering:
  1. An upstream regression, malformed retained automation value, or a new
     caller constructs a place/modify request with a non-numeric, non-finite,
     zero, negative-zero, or negative price/size string, or a placement without
     the repository's 128-bit hexadecimal CLOID.
  2. The request reaches `HyperliquidL1Action` through the existing builder.
  3. The prior payload builder serializes, hashes, signs, and posts the malformed
     action without an independent structural check.
- Evidence:
  - `prepare_place_order` and `prepare_modify_order` correctly validate and
    quantize ordinary intents, and the audited NUKE, Chase, and TWAP direct
    constructors perform their own positive-finite planning checks.
  - Nevertheless, `PlaceOrderRequest` and the action constructors retain price,
    size, and CLOID as unconstrained `String`/`Option<String>` values, and the
    sole `build_signed_exchange_payload_with_nonce` path previously serialized
    them without validation (`src/signing/client.rs:17-25`,
    `src/signing/actions/builders.rs:10-72`, `src/signing/client.rs:83-104`).
  - Repository-wide searches found no feature-owned `/exchange`, signing, or
    place/modify path that bypasses this shared payload builder.
- Violated invariant: Kerosene must never sign or post an order action whose
  numeric wire values are not positive and finite, and every placement must
  retain the exact CLOID required for uncertain-outcome correlation.
- Risk: an upstream plumbing regression could send a malformed financial
  mutation instead of failing closed. A placement without a valid CLOID would
  also remove the exact `orderStatus` correlation key used after an ambiguous
  transport outcome.
- Implemented fix: `HyperliquidL1Action::validate_wire_structure` checks every
  order and modify child immediately before msgpack serialization. It parses
  price/size only to validate positive finiteness, never rewrites the original
  strings, and requires placement CLOIDs to be `0x` plus exactly 32 hexadecimal
  digits. Static errors contain no rejected value (`src/signing/actions.rs:36-158`,
  `src/signing/client.rs:83-104`).
- Regression coverage: a valid signed IOC placement and GTC modify prove asset,
  OID, side, exact price/size strings, reduce-only, TIF, and CLOID are unchanged.
  Adversarial cases reject parse failures, NaN, infinities, zero, negative zero,
  negative values, invalid modify size, absent CLOID, wrong length, wrong
  prefix, and non-hex content before signing; synthetic sensitive values are
  absent from errors (`src/signing/client/tests.rs:86-260`). The prepared
  request test now explicitly protects side, order kind, and reduce-only mapping
  in addition to its existing asset/price/size/CLOID assertions
  (`src/order_execution/core.rs:2183-2210`).
- Protected behavior: current prepared values already satisfy the guard. The
  validator borrows wire strings and does not normalize, round, reserialize, or
  alter valid action bytes. Market capability, symbol/asset selection, sizing,
  precision, price/slippage, side, reduce-only, TIF, CLOID generation, signing,
  result handling, UI, timing, and persistence remain unchanged for valid work.

### F-08 — Contradictory exchange acknowledgements can be consumed as definitive outcomes

- Status: addressed in Turn 9; focused tests added, but executable validation is
  blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: shared response analysis; every shared-classifier caller; and Chase
  and TWAP parsed mutation-result handling
- Preconditions/event ordering:
  1. A mutation receives a syntactically parseable `ExchangeResponse` rather
     than a transport `Err`.
  2. The top-level envelope or one structured status explicitly reports an
     error, while a structured status also reports a possible resting, filled,
     successful-cancel, or otherwise non-error exchange effect.
  3. A handler evaluates the error, IOC-no-match, or terminal-cancel signal
     before recognizing the conflicting effect.
  4. The handler removes/fails automation, accounts for no fill, or declares a
     child cancel complete without first resolving the exact CLOID/OID.
- Evidence:
  - `ExchangeResponse::is_error` recognizes both top-level and status-level
    errors. The prior `has_potential_order_effect` returned early for every
    top-level error even when the parsed body retained a concrete order effect,
    while the shared one-shot classifier relied on that predicate to separate
    rejection from ambiguity (`src/signing/model/exchange_response/analysis.rs:83-144`,
    `src/order_update/results.rs:180-215`).
  - Shared one-shot handling already treated an inner resting/fill plus error as
    ambiguous, but Chase place/modify/cancel had earlier `is_error` branches;
    TWAP child placement evaluated IOC/error/fill paths before its ambiguity
    branch; and TWAP unexpected-cancel accepted a terminal-error substring even
    alongside a successful-cancel status.
  - The fixed guards and ordering are visible at
    `src/order_update/chase/result.rs:162-175`,
    `src/order_update/chase/modify.rs:61-85`,
    `src/order_update/chase/cancel.rs:59-84`,
    `src/order_execution/twap/slice_result.rs:69-103`,
    `src/order_execution/twap/slice_result.rs:211-320`, and
    `src/order_execution/twap/cancel.rs:114-139`.
- Violated invariant: when explicit error and possible exchange-effect signals
  disagree, local state must remain uncertain until exact target status or an
  authoritative account snapshot resolves the mutation.
- Risk: Kerosene could discard a Chase whose order actually rests, stop or
  requeue automation despite a modify effect, omit a TWAP fill, or clear an
  unexpected child while the response itself is internally contradictory. That
  can lose live child-operation tracking and allow later work to proceed from an
  incorrect exposure model.
- Why existing checks were insufficient: strict confirmed-result predicates
  correctly rejected mixed responses, but the affected handlers consumed an
  error-specific branch before reaching their existing uncertain-result path.
  The top-level-envelope early return also hid structured effects from the
  otherwise conservative shared classifier.
- Implemented fix: response analysis now detects the conjunction of `is_error`
  and `has_potential_order_effect` as one explicit conflict category, including a
  top-level error with a structured effect. Shared classification consequently
  produces `Ambiguous`; Chase place/modify/cancel route conflicts through their
  existing CLOID/OID verification; TWAP child placement prevents a conflict
  from entering its earlier IOC/error/fill/resting branches while preserving
  the established ordering for every non-conflicting response. It also does not
  seed child OID/fill/average-price bookkeeping from the contradictory payload,
  so a later empty refresh cannot credit an unconfirmed fill. Contradictory
  unexpected-cancel results retain the existing bounded, target-specific
  reconciliation/retry path. No placement/modify retry, new retry mechanism,
  identifier, state field, or policy was added.
- Regression coverage: response-model and shared-classifier tests cover both
  mixed statuses and a top-level error with a structured resting effect, while
  an unstructured top-level error remains a definitive rejection
  (`src/signing/tests/responses/status.rs:116-151`,
  `src/signing/tests/responses/strings.rs:89-107`,
  `src/order_update/results/tests.rs:443-509`). Handler tests prove conflicting
  Chase place, modify, and cancel results retain verification state, and that a
  conflicting TWAP slice neither retains provisional effect fields nor credits
  or settles a fill before CLOID status, while a conflicting unexpected-child
  cancel remains pending
  (`src/order_update/chase/result/tests.rs:132-166`,
  `src/order_update/chase/modify/tests/reconciliation.rs:38-77`,
  `src/order_update/chase/cancel/tests.rs:160-194`,
  `src/order_execution/twap/tests/place_result.rs:79-150`,
  `src/order_execution/twap/tests/cancel.rs:97-126`).
- Protected behavior: existing pure-rejection, valid resting/fill/cancel,
  unstructured-malformed, transport-unknown, refresh, bounded cancel retry,
  visible normal-path status, wire, signing, pricing, sizing, timing,
  persistence, and UI behavior is unchanged. Only a parseable internally
  contradictory acknowledgement now takes an already-established uncertain
  reconciliation path.
- Residual uncertainty: Turn 10 completed the phase audit and retained the
  conservative `String` error shape deliberately; transport replay, timeout,
  HTTP-envelope conflict, and pre-message redaction are now independently
  guarded under F-09 through F-11.

### F-09 — Generic HTTP policy can replay or indefinitely strand a signed mutation

- Status: addressed in Turn 10; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High, with a Critical duplicate-mutation consequence if an
  anomalous redirect is emitted after an exchange-side effect
- Scope: the one `/exchange` HTTP client and request builder used by every
  place, cancel-by-OID, cancel-by-CLOID, modify, and leverage action
- Preconditions/event ordering:
  1. A valid action has already been serialized and signed.
  2. The generic shared reqwest client receives a 307/308 redirect or a
     retryable protocol NACK, or its configured construction fails and the
     fallback client is used.
  3. Redirect/retry middleware can clone and resend the JSON POST below the
     application lifecycle, or the fallback has no total request timeout.
  4. The state machine observes only the eventual response/error and cannot
     correlate how many wire sends occurred.
- Evidence:
  - The prior `post_exchange` cloned the shared `api::CLIENT`; that client used
    reqwest defaults for redirect/retry policy and fell back to `Client::new()`
    if the configured builder failed (`src/api.rs:47-58`).
  - All five signed action wrappers converge on `sign_and_post`, and the JSON
    request body is replayable (`src/signing/client.rs:74-135`,
    `src/signing/client.rs:234-293`).
  - The audited reqwest 0.12 implementation follows up to ten redirects by
    default, preserves POST/body for 307/308, permits safe-protocol retries by
    default, and gives a default client no total timeout. Kerosene must not rely
    on generic HTTP safety assumptions as its mutation idempotency policy.
- Violated invariant: exactly one transport send may be initiated for one
  signed lifecycle attempt; any uncertainty must return to Kerosene for exact
  reconciliation, never trigger hidden middleware replay.
- Risk: an infrastructure redirect or lower-level replay can submit the same
  signed mutation more than once without a second state-machine attempt. CLOID
  and nonce behavior provide useful exchange defenses, but cancel, modify, and
  leverage actions do not all have a placement CLOID, and client code must not
  assume undocumented duplicate suppression. An unbounded fallback request can
  also strand global or automation pending state indefinitely.
- Implemented fix: `/exchange` now has a dedicated lazily built client with
  redirects disabled, retries disabled, the existing 5-second connect and
  15-second total limits, and the existing 60-second idle-pool limit. Client
  construction failure is returned through the normal conservative error path
  instead of falling back to an unbounded generic client. The request builder
  independently reapplies the 15-second limit as prudent redundancy
  (`src/signing/client.rs:16-23`, `src/signing/client.rs:137-176`).
- Regression coverage: a request-construction test proves the mutation-local
  timeout exists even on a default client, and a loopback 307 server proves the
  dedicated client returns the redirect without issuing a second POST
  (`src/signing/client/tests.rs:60-137`).
- Protected behavior: endpoint, method, JSON body, headers/user agent, signing,
  nonce/expiry, response parsing, all application result classifications, and
  the ordinary 5/15/60-second timing policy remain unchanged. No dependency,
  exchange retry, application retry, UI, persistence, or trading-policy change
  was introduced.

### F-10 — Signed mutation results can expose details before update-time redaction

- Status: addressed in Turn 10; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium privacy and diagnostic-boundary hardening
- Scope: signed response/error text between `sign_and_post`, task mapping,
  derived `Message::Debug`, `ExchangeResponse::Debug`, and update handlers
- Preconditions/event ordering:
  1. A local, reqwest, response-body, or parse error includes a credential,
     bearer value, long hexadecimal token, or 128-bit CLOID; or a successful
     response includes size, price, and OID details.
  2. A task closure maps that `Result<ExchangeResponse, String>` directly into
     a `Message` variant.
  3. Derived `Message::Debug` formats the nested result before an update handler
     applies its redundant sanitizer.
- Evidence:
  - `Message` derives `Debug`, and every signed-result variant stores the raw
    result shape (`src/message.rs:571-572`, `src/message.rs:1079-1137`).
  - The prior `sign_and_post` returned build/post errors directly, while
    individual handlers performed redaction only after message delivery.
  - `ExchangeResponse::Debug` previously embedded `summary()`, which exposes
    deliberate user-facing resting/fill size, price, and OID details even when
    the raw response model itself is not formatted.
- Violated invariant: every mutation result must already be diagnostically safe
  at the boundary where it can enter a derived-debug message; handler-time
  redaction is useful redundancy, not the first privacy boundary.
- Risk: diagnostic logging, panic context, or state formatting between task
  completion and update handling can disclose secrets, order correlation IDs,
  or order details that adjacent models deliberately redact.
- Implemented fix: the single signing exit now applies order-aware redaction to
  every `Err` before task mapping. The malformed-body path uses the same
  bounded sanitizer, and response error/unknown summaries redact 128-bit hex
  identifiers without changing the generic application redactor. Successful
  `ExchangeResponse::Debug` keeps safe status/type/count metadata but replaces
  its human summary with `<redacted>` (`src/signing/client.rs:74-95`,
  `src/signing/client.rs:211-218`,
  `src/helpers/formatting/text.rs:65-79`,
  `src/signing/model/exchange_response/analysis.rs:227-246`).
- Regression coverage: tests prove a bearer token, API key, private key, long
  hex token, and CLOID are absent from result/debug output; safe error copy and
  successful response semantics remain exact; the order-only helper preserves
  a short OID; and successful response debug omits fill size, price, and OID
  (`src/signing/client/tests.rs:466-529`,
  `src/helpers/formatting/text.rs:461-469`,
  `src/signing/tests/responses/status.rs:57-76`,
  `src/signing/tests/responses/strings.rs:158-184`).
- Protected behavior: `ExchangeResponse::summary()` remains unchanged for valid
  user-facing results; safe transport error strings remain byte-for-byte
  unchanged; `Ok`/`Err` shape, outcome classification, reconciliation, task
  count/timing, controls, persistence, and wire behavior are untouched. Only
  diagnostic/error content that matches the existing sensitive-order policy is
  redacted earlier.

### F-11 — A non-success HTTP envelope can masquerade as mutation success

- Status: addressed in Turn 10; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High outcome-classification hardening
- Scope: HTTP status/body handoff immediately after `/exchange` returns
- Preconditions/event ordering:
  1. The signed request reaches an exchange server or intermediary.
  2. It returns a non-2xx HTTP status but a syntactically valid body whose
     exchange-level fields appear successful or merely incomplete rather than
     explicitly erroneous.
  3. The prior transport reads only the body and discards the HTTP status.
  4. Normal parsed-success handling can accept, fill, rest, or advance
     automation from a response whose two protocol layers disagree.
- Evidence: the prior `post_exchange` chained `.send().text()` and passed only
  raw body text to `parse_exchange_response`. The current handoff captures both
  values at `src/signing/client.rs:137-152`, while parsed-response conflict
  analysis has no access to HTTP status once it is discarded.
- Violated invariant: a successful local lifecycle transition requires success
  at both the HTTP envelope and exchange response layer; disagreement is
  uncertain exposure, not confirmation.
- Risk: an upstream error page or intermediary response shaped like a valid
  exchange result could be consumed as acceptance/fill and allow automation to
  continue from unverified state.
- Implemented fix: HTTP status now accompanies the raw body into one parser.
  A non-success envelope plus an apparently non-error exchange body becomes a
  value-neutral transport error and exact reconciliation. A structured pure
  rejection remains rejected, and an error/effect conflict remains an F-08
  ambiguity, preserving the stronger information already in the body
  (`src/signing/client.rs:220-232`).
- Regression coverage: a 500 envelope with a resting-success body cannot expose
  its OID or confirm success; a 400 structured rejection remains classifiable;
  and a 500 mixed resting/error body still reaches conflict reconciliation
  (`src/signing/client/tests.rs:429-463`).
- Protected behavior: every 2xx response, structured rejection, error/effect
  conflict, syntactically invalid body, redaction rule, timeout, normal status
  string, and reconciliation path retains its prior behavior. Only a previously
  unrepresented HTTP/exchange success disagreement becomes fail-closed.

### F-12 — `orderStatus` diagnostics can expose correlation values

- Status: addressed in Turn 11; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium privacy hardening
- Scope: OID/CLOID status-query HTTP/parser errors, successful result debug,
  and the result value before task-message mapping
- Preconditions/event ordering: an HTTP error body echoes a 128-bit CLOID, or a
  parsed concrete order returns a mismatching OID/CLOID, or an external status
  string embeds sensitive order text; the result is mapped into a derived
  `Message::Debug` variant before handler-time redaction.
- Evidence: before Turn 11, the parser included expected and returned
  correlation values in mismatch errors, the HTTP preview used the generic
  40-hex rather than order-aware 32-hex redactor, and `OrderStatusResult::Debug`
  emitted `status` verbatim. All status-query callers converge on the two public
  functions now guarded at `src/api/order_status.rs:16-35`; result variants are
  stored in derived messages at `src/message.rs:874-879`,
  `src/message.rs:1094-1099`, `src/message.rs:1128-1137`, and the advanced-order
  variants at `src/message.rs:1188-1239`.
- Violated invariant: exact correlation values must be retained for matching
  but omitted from diagnostic errors and result formatting before message
  mapping.
- Risk: an anomalous or hostile reconciliation response can bypass the custom
  result redaction through either the error lane or the externally supplied
  status field.
- Implemented fix: both public status functions apply one order-aware sanitizer
  to `Err` before returning to `Task::perform`; HTTP bodies are redacted before
  their 160-character preview is taken; parser mismatch/missing diagnostics no
  longer embed expected or returned identifiers; and result debug sanitizes the
  status field while retaining exact stored data (`src/api/order_status.rs:16-35`,
  `src/api/order_status.rs:77-82`, `src/api/order_status/parsing.rs:15-62`,
  `src/api/order_status/model.rs:16-24`).
- Regression coverage: adversarial tests cover a CLOID crossing the prior
  preview truncation boundary, secret/CLOID errors before message mapping,
  unchanged safe errors and successful results, value-neutral OID/CLOID
  mismatch and missing-field errors, external parser errors, and status-field
  debug redaction (`src/api/order_status/tests/validation.rs:3-145`,
  `src/api/order_status/tests/parsing.rs:24-89`).
- Protected behavior: exact request body/correlation comparisons, parsed
  success values, stored status/summary data, status classification,
  retries/timeouts, state transitions, and normal user-visible status paths are
  unchanged. Only sensitive or anomalous diagnostic values are removed.

### F-13 — Derived order diagnostics expose raw OID/CLOID context fields

- Status: addressed in Turn 12; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium privacy hardening
- Scope: order-related `Message` variants with direct numeric OID or owned
  CLOID fields, independent of their already-redacted result payloads, plus the
  adjacent stopped-Chase cancel request formatter
- Preconditions/event ordering: any diagnostic formats the derived `Debug` for
  a cancel, move, Chase, TWAP, or chart-hover message carrying a raw correlation
  identifier, or formats a stopped-Chase cancel request during diagnostics.
- Evidence: before Turn 12, `Message` derived `Debug` over raw fields on
  cancel/status, TWAP cancel/status, Chase modify/cancel/status, move
  intent/result/status, and chart cancel-hover variants. Sanitizing F-12's
  `Result` lane did not affect those sibling context fields. A repository-wide
  custom-formatter scan also found that `StoppedChaseCancelRequest` redacted its
  agent key but emitted its OID (`src/order_update/chase/result/stop_cancel.rs:18-26`).
- Violated invariant: order identifiers needed for routing and equality must
  not be emitted by general diagnostic formatting.
- Risk: a normal or failing lifecycle message can disclose the identifier even
  though response models, placement contexts, account keys, and addresses use
  redacted diagnostic representations.
- Implemented fix: added copy/owned `RedactedOrderId` and
  `RedactedClientOrderId` message-boundary wrappers, converted every direct
  exchange identifier producer at publication, and restored the exact values
  once at `update_order` or the chart update consumer. The stopped-Chase request
  formatter now redacts its OID alongside its key (`src/message.rs:254-308`,
  `src/message.rs:1141-1429`, `src/order_update.rs:80-104`,
  `src/order_update.rs:205-431`, `src/chart/interaction/hud.rs:103-122`,
  `src/chart_update.rs:218-240`,
  `src/order_update/chase/result/stop_cancel.rs:18-26`).
- Regression coverage: one whole-message test formats all fifteen affected
  variants, including optional TWAP identifiers and chart hover, and rejects
  the raw OID/CLOID; a round-trip test proves wrapper values are byte/numerically
  exact; the stopped-Chase formatter test rejects both its key and OID
  (`src/message.rs:1621-1729`,
  `src/order_update/chase/result/tests/stop_cancel.rs:38-52`).
- Protected behavior: message variant names/routing, exact OID/CLOID values,
  handler arguments, account/symbol/attempt context, cancellation, movement,
  Chase/TWAP state transitions, chart hover state, task timing, UI, signing,
  persistence, and trading semantics are unchanged. Only diagnostic formatting
  differs.

### F-14 — Cancel and move results can settle a later same-OID attempt

- Status: addressed in Turn 13; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: cancel and drag-to-move dispatch, direct-result ownership, status-result
  ownership, refresh ordering, and runtime request-ID allocation
- Preconditions/event ordering:
  1. Cancel attempt A is dispatched and its presentation indicator expires, or
     a complete refresh clears its account-scoped pending slot before the direct
     result arrives. A later attempt B is then dispatched for the same account
     and OID.
  2. Alternatively, move attempt A leaves direct/status work in flight, its
     OID-keyed context is cleared by an earlier outcome or account-state reset,
     and attempt B reuses the same account, symbol, and OID.
  3. A delayed or duplicate direct/status message from A arrives while B owns
     the structurally identical account/OID tuple.
- Evidence: before Turn 13, cancel direct results recovered their target from
  the indicator and then whichever pending cancel matched only the account;
  cancel status results matched account/OID/symbol but not an attempt or phase.
  Move direct results matched the current OID-keyed context only by account,
  and move status results likewise matched only account/OID/symbol. The refresh
  cleanup also treated a cancel waiting for its first direct result as though it
  were already in status reconciliation. The corrected ownership checks are at
  `src/order_update/results.rs:88-183`,
  `src/order_update/results.rs:359-459`, and
  `src/order_update/move_order.rs:30-160`.
- Violated invariant: a callback may transition, clear, or reconcile only the
  exact logical mutation attempt that dispatched it; OID reuse and a
  presentation indicator are not sufficient ownership.
- Risk: an old cancel result can remove the wrong local order or release the
  newer cancel blocker; an old move result can consume the newer captured key
  context, clear its indicator, patch the snapshot with the old target price,
  or cause the newer true result to be ignored. An old status result can settle
  either newer attempt before its own outcome is known.
- Implemented fix: one collision-aware, wrap-safe runtime request sequence is
  shared by one-shot status, cancel, and move state and skips every live ID.
  Cancel creates its immutable account/OID/symbol owner before dispatch and
  explicitly transitions it from `AwaitingResult` to `CheckingStatus`; only the
  latter phase can be cleared by refresh. Direct and status messages carry the
  same sequence and must match the current phase. Move captures the sequence in
  `PendingMoveOrderContext`, propagates it to uncertain status work, and
  requires it at both handlers (`src/order_execution/position_actions/cancel.rs:81-104`,
  `src/order_execution/quick_order/move_order.rs:162-200`,
  `src/order_update/results.rs:637-670`).
- Regression coverage: stale same-OID direct/status results preserve the newer
  cancel and move attempt; a duplicate cancel direct result cannot override an
  active status check; a complete account refresh cannot erase a cancel still
  awaiting its direct result; indicator expiry does not remove authoritative
  cancel ownership; and allocator wrap skips live one-shot, cancel, move-status,
  and move-context IDs (`src/order_update/results/tests.rs:794-826`,
  `src/order_update/results/tests.rs:1427-1508`,
  `src/order_update/results/tests.rs:1587-1607`,
  `src/order_update/move_order.rs:683-702`,
  `src/order_update/move_order.rs:767-849`).
- Protected behavior: signed cancel/modify actions, account/key capture, OID,
  symbol, price and size preparation, response classification, normal status
  text, refresh/status task timing, local confirmed-price projection, UI,
  persistence, and retry policy are unchanged. The sequence is runtime-only
  correlation and is never sent to the exchange.

### F-15 — Cancel/move refresh cleanup is not scoped to sufficient evidence

- Status: addressed in Turn 14; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: authoritative fallback cleanup for uncertain cancel and move attempts
- Preconditions/event ordering: an uncertain cancel/move originates on one
  main or HIP-3 symbol lane; the visible market universe changes before its
  refresh is dispatched or completed, or the move target remains open at an
  unverified price; a successful snapshot then reports its own scoped open-order
  section as complete.
- Evidence: refresh scope follows the currently selected market universe
  (`src/account_update/connection/refresh.rs:81-92`), and `AccountData` already
  exposes the correct per-symbol lane predicate
  (`src/account/types/data.rs:198-213`). Before Turn 14, shared cancel/move
  cleanup checked only snapshot-wide `open_orders_complete` plus account before
  dropping either request, and the move's authoritative prepared target was
  discarded with its direct-result context.
- Violated invariant: uncertainty may be released only by authoritative data
  that covers the operation's origin lane and distinguishes the relevant
  terminal/open state; an open move additionally needs evidence of whether its
  expected price committed.
- Risk: a complete but unrelated HIP-3 lane can unblock a cancel or move whose
  exposure remains unknown. A same-lane move refresh can also erase the status
  request without comparing the live order price to the dispatched target,
  allowing later actions to proceed without establishing which modification
  won.
- Implemented fix: cancel refresh cleanup now requires its immutable origin
  symbol's open-order lane and remains phase-gated. Move captures the exact
  already-prepared target price with its request/account/key context before
  dispatch, propagates it into ambiguous status state after the presentation
  indicator is consumed, and releases refresh uncertainty only after a covering
  snapshot establishes one of three explicit states: target absent, target open
  at the expected price, or target open at a different valid price. A malformed
  live or retained price is not reconciliation evidence for an open target
  (`src/order_execution/quick_order/move_order.rs:162-204`,
  `src/order_execution.rs:992-1053`,
  `src/order_update/move_order.rs:30-143`,
  `src/order_update/results.rs:187-290`,
  `src/order_update/results.rs:761-788`).
- Why a different valid price still resolves: the successful post-result
  snapshot is the existing authoritative current-state fallback. Distinguishing
  that state proves the requested price is not the live winner while preserving
  the prior cleanup contract; retaining uncertainty solely because the valid
  live price differs would change enabled/disabled behavior and could strand a
  provably current open order. Wrong-scope, incomplete, absent-account, or
  malformed evidence remains blocked.
- Regression coverage: wrong HIP-3 scope and incomplete origin-lane snapshots
  retain cancel/move requests; a covering cancel snapshot preserves cleanup
  when the target remains open; a move result carries its expected price after
  its indicator disappears; malformed live price retains uncertainty; formatted
  expected, different valid, and absent target states are distinguished; and
  existing valid-price/terminal cleanup remains unchanged
  (`src/order_update/results/tests.rs:1527-1565`,
  `src/order_update/move_order.rs:651-714`,
  `src/order_update/results/tests.rs:230-257`).
- Protected behavior: account refresh scope/cadence/generations, exact status
  tasks, all user-visible strings, normal cleanup timing, pending indicators,
  prepared and signed modify values, local confirmed-price projection, retries,
  UI, and persistence are unchanged. The new target price is runtime-only and
  redacted in the pending request's custom `Debug` output.

### F-16 — Unrelated scoped refresh can replace or archive a Chase with unknown exposure

- Status: addressed in Turn 15; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Critical
- Scope: Chase account-refresh reconciliation, fill-completion cleanup,
  stop/cancel verification, and the final prior-exposure gate before a
  replacement placement
- Preconditions/event ordering:
  1. A Chase owns a known order on one HIP-3 lane, such as `flx:BTC`, and is
     awaiting account verification after a terminal no-fill status, target
     fill, or confirmed cancel/stop transition.
  2. The selected market universe changes before the required account refresh,
     so the refresh scope is another HIP-3 dex, such as `xyz`.
  3. That request returns complete account-wide fills and a complete open-order
     result for `xyz`, but necessarily carries no authoritative `flx` orders.
  4. Before Turn 15, Chase reconciliation treated snapshot-wide
     `open_orders_complete` as authority for every Chase. It could therefore
     queue a replacement, archive a fully filled Chase without cancelling a
     still-open known order, or archive `Stopping::VerifyingCancel` without
     proving the origin lane clear. The final known-OID placement guard repeated
     the same global check.
- Affected order surfaces: adopted and newly placed Chase orders across
  replacement, reprice verification, fill completion, stop, and archive paths.
- Evidence: refresh scope is derived from the current market universe
  (`src/account_update/connection/refresh.rs:81-92`), while the account model's
  authority predicate is explicitly per symbol
  (`src/account/types/data.rs:198-213`). The affected reconciliation decisions
  converge in `reconcile_chase_after_account_refresh`,
  `reconcile_chase_fills_from_snapshot`, and `chase_place_at_best`; their fixed
  boundaries are now at `src/account_update/stream/chase/refresh.rs:37-292`,
  `src/account_update/stream/chase/fill_reconciliation.rs:17-146`, and
  `src/order_execution/chase/lifecycle/place.rs:247-290`.
- Violated invariant: only an open-order snapshot that successfully fetched a
  Chase's immutable origin-symbol lane may prove its known exposure absent,
  authorize terminal archive, or permit a replacement placement. Fill evidence
  remains independently account-wide.
- Risk: a wrong-scope snapshot can falsely declare unknown exchange exposure
  safe. At the strongest boundary it can dispatch a replacement while another
  known Chase order remains live; after a target fill it can instead leave a
  residual live order unmanaged by archiving the Chase that should cancel it.
- Why existing checks did not cover it: account address, provider generation,
  lifecycle, OID/CLOID, attempt, fill completeness, and pending-operation gates
  can all be valid in this ordering. Snapshot-wide completeness describes only
  the request's own scope. The already-correct websocket disappearance gate is
  keyed by dex (`src/account_update/stream/chase/disappearance.rs:13-51`) and
  does not constrain the separate REST-refresh path.
- Implemented fix: derive the set of active Chase symbols whose open-order lane
  the snapshot actually covers, using the established
  `has_complete_open_orders_for_symbol` predicate. Pass that immutable evidence
  into fill reconciliation; gate each refresh, stop, removal, correction, and
  replacement decision by its Chase's symbol; and repeat the same per-symbol
  requirement at the final known-exposure placement guard. Missing authority
  retains verification and requests refresh rather than mutating exchange
  state. No polling, retry, or request was added.
- Regression coverage: an unrelated HIP-3 refresh cannot place a resolved
  no-fill replacement, archive a stopping Chase, or archive a fully filled
  Chase whose open orders are unknown; a direct final-dispatch test proves the
  redundant placement guard also blocks the replacement
  (`src/account_update/stream/tests/chase_reprice/account_refresh.rs:152-173`,
  `src/account_update/stream/tests/chase_stop.rs:64-91`,
  `src/account_update/stream/tests/chase_fills/completion.rs:23-47`,
  `src/order_execution/chase/lifecycle/tests/place.rs:101-125`). Existing
  covering-snapshot replacement, stop cleanup, fill archive, and CLOID-attempt
  tests characterize unchanged normal behavior.
- Smallest behavior-preserving fix: reuse the existing account-lane predicate
  at the three authoritative boundaries; do not change Chase state types,
  exchange payloads, task cadence, pricing/sizing, or status copy.
- Protected behavior: repricing, limits, timing, status strings, retry policy,
  fill totals, order identity, normal covering-snapshot transitions, signing,
  UI, persistence, and secret handling are unchanged. Only an unrelated or
  incomplete origin lane remains in the existing verification state.
- Residual uncertainty: formatting, call-site inventory, and state-transition
  review pass, but Rust type-check/tests/clippy must execute on a host with ALSA
  development metadata. Reversed REST-versus-websocket delivery remains a
  separate Track 8 candidate and is not claimed resolved by this fix.

### F-17 — In-flight REST snapshot can erase a newer user-data event

- Status: addressed in Turn 16; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Critical
- Scope: connected-account REST request ownership, ordinary user-data position,
  open-order, fill, and spot-balance frames, initial account loading, and every
  order lifecycle that treats a successful connected snapshot as authoritative
  fallback evidence
- Preconditions/event ordering:
  1. A connected-account REST refresh starts at local account revision `R`.
  2. The server can capture or serve that request's snapshot before a later
     user-data frame, while delivery of the REST result remains in flight.
  3. A valid same-account websocket frame then changes positions, open orders,
     fills, or balances and advances local state to `R+1`. No second refresh or
     lag notification is otherwise required.
  4. The first REST result arrives last with the same address, read provider,
     key generation, and request generation. Before Turn 16, it replaced the
     websocket-merged `AccountData`, bumped the revision again, and immediately
     ran one-shot, cancel/move, Chase, and TWAP reconciliation from the
     potentially pre-event body.
  5. During initial loading the related gap was earlier: without a base
     `AccountData`, an ordinary account frame was not merged and did not queue a
     follow-up, so the pre-frame result could reconcile as final.
- Affected order surfaces: ticket, preset, Alfred, quick, HUD, close, NUKE,
  cancel, move, Chase, connected-account TWAP, and leverage flows that depend on
  the shared connected snapshot cleanup boundary. Wallet-detail/cluster state
  and off-account TWAP reconciliation use separate state/generations.
- Evidence: all mergeable account websocket variants set
  `account_data_changed` and advance `account_data_revision`
  (`src/account_update/stream.rs:101-230`). Before Turn 16,
  `AccountDataRequestContext::ConnectedSnapshot` carried provider and request
  generation but not its starting revision, while `apply_account_data_loaded`
  unconditionally installed a same-generation success and invoked shared
  cleanup (`src/order_update/results.rs:736-788`) plus Chase/TWAP reconciliation.
  The fixed ownership and application boundaries are now
  `src/read_data_provider.rs:23-157` and
  `src/account_update/connection/refresh.rs:94-213`; initial-load coalescing is
  at `src/account_update/stream.rs:193-230`.
- Violated invariant: data that may predate a newer authoritative user-data
  event must not erase that event or prove a financial mutation absent. A
  connected snapshot may settle lifecycle uncertainty only if its causal
  request context still owns the current account revision.
- Risk: a live order or fill can disappear locally, a pending one-shot/cancel/
  move check can be cleared by pre-operation absence, and Chase can archive or
  progress toward replacement after newer exposure was already observed. This
  falsely declares unknown exchange exposure safe and can enable a duplicate
  or unmanaged mutation.
- Why existing checks did not cover it: the address, provider/key generation,
  and request generation all remain valid because there is only one REST
  request. `account_refresh_followup_pending` was set by an explicit refresh or
  lag event, not by ordinary mergeable websocket frames. The revision existed
  for sizing provenance but was not captured by the REST task.
- Implemented fix: capture `account_data_revision_at_dispatch` in every
  connected-snapshot request context. After the existing provider, generation,
  and address checks, reject a successful body whose revision no longer owns
  current account state before changing loading data, revisions, pending
  uncertainty, or automation. Preserve the websocket-merged snapshot, consume
  any queued reason into one sequential post-frame refresh, and reuse the
  existing backoff/generation machinery. If initial loading has no base to
  merge into, mark the in-flight result for the existing follow-up branch; it
  may populate display state but cannot reconcile orders before the post-frame
  request completes.
- Regression coverage: an adversarial request-before/open-order-frame/result-
  after test proves the live order and account revision remain, the pending
  one-shot status is not cleared, and exactly one newer request generation is
  active. A separate initial-load test proves an otherwise unmergeable account
  frame queues reconciliation without starting a competing request
  (`src/account_update/connection/refresh.rs:805-869`,
  `src/account_update/stream/tests.rs:200-225`). Existing same-generation load,
  queued-follow-up, stale-generation/provider, scoped-open-order, and
  off-account TWAP tests characterize protected behavior.
- Smallest behavior-preserving fix: add one runtime-only revision value to the
  existing context and one pre-application ownership check, plus reuse the
  existing initial-load follow-up bit. Do not merge lanes speculatively, add a
  timer, accept causally unproven absence, or alter response/status semantics.
- Protected behavior: REST fetch scope, cadence, providers, errors, 429
  backoff, explicit/lag follow-ups, successful no-conflict application,
  display layout/copy, order preparation/signing, exact status handlers,
  persistence, and secrets are unchanged. Conflict handling starts at most one
  sequential request at a time; continued live deltas remain fail-closed under
  the existing rate-limit policy rather than accepting an unowned snapshot.
- Residual uncertainty: Rust type-check/tests/clippy remain blocked by ALSA
  metadata. Same-address user-stream subscription identity across reconnects
  and channel closure is a separate Track 8 candidate; this REST ownership
  guard does not claim to establish stream-generation provenance.

### F-18 — Replaced same-address user stream can apply one late frame

- Status: addressed in Turn 17; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Critical
- Scope: iced user-data recipe identity/cancellation, connected-account stream
  application, wallet-detail streams, selected wallet-cluster streams, account
  session and consumer-topology changes, dex-scope changes, manager reconnect,
  lag, and broadcast closure
- Preconditions/event ordering:
  1. An account, wallet-detail, or cluster recipe has pulled a valid frame from
     its shared-manager receiver.
  2. Before iced can send that frame to the application queue, state replaces
     or removes the recipe. Real paths include a same-address wallet reconnect,
     account switch away/back, detail exclusion or close/reopen, selected
     cluster/member/profile changes, and visible-dex scope changes.
  3. The old and new recipe use the same normalized wallet address. Before
     Turn 17, the resulting `Message` carried only that address and data.
  4. iced cancellation races only with `stream.next()`. Once a message wins,
     the tracker awaits the application-queue send without selecting
     cancellation again; the canceled old recipe can therefore enqueue that
     one frame after state already requests the new recipe.
  5. The address-only handler accepts the frame as belonging to the new
     session. Independent old/new recipe tasks can also reverse delivery.
- Affected order surfaces: every connected-account lifecycle consumer of open
  orders, fills, positions, balances, or lag—including one-shot, cancel/move,
  Chase, TWAP, leverage, and overlays—and wallet-cluster close/order planning
  that relies on member position freshness. Wallet details have the same stale
  display risk but do not dispatch trades.
- Evidence: `WsUserDataStreamParams` was hashed into recipe identity but was
  discarded by each `.map`, leaving only `Option<RedactedAddress>` in the three
  messages. iced 0.14's tracker selects cancellation against `stream.next()` at
  lines 96-105 of its `subscription/tracker.rs`, then awaits `receiver.send`
  without another cancellation check. The shared manager's ordinary socket
  reconnect remains ordered inside one receiver; its broadcast sender closes
  only if the singleton manager task/command channel tears down, not on normal
  reconnect. Thus recipe replacement—not the manager reconnect loop—is the
  concrete overlap boundary.
- Violated invariant: a user-data frame may change trading or freshness state
  only when both its source address and the exact runtime stream incarnation
  still belong to the current consumer.
- Risk: an old open-order/fill frame can regress or falsely advance automation
  reconciliation after a same-address reconnect. A late old cluster position
  frame can clear `stale`, refresh `positions_refreshed_ms`, and authorize a
  reduce-only close size from superseded member exposure. That can falsely
  declare unknown exchange state safe and enable a wrong or duplicate
  mutation.
- Why existing checks did not cover it: address normalization proves wallet
  ownership, and purpose/dex fields distinguish simultaneously live recipes,
  but none identifies a replacement incarnation with equal parameters.
  Manager refcounts prevent duplicate wire subscriptions from colliding; they
  cannot retract an item the old iced task already pulled. REST request
  generations and Turn 16's revision guard do not identify websocket recipe
  ownership.
- Implemented fix: add a runtime-only generation to
  `WsUserDataStreamParams`, retain the entire params value in each subscription
  output with `Subscription::with`, and add one shared guard that requires the
  exact current params plus exact normalized source before any route applies
  data. Rotate the account generation at successful connect/reconnect,
  disconnect, switch, invalidation, and clear. Allocate unique per-address
  detail generations across open/close/reopen and connected-address exclusion,
  while leaving unrelated detail streams unchanged and clearing retained
  address keys on config clear. Rotate the selected-cluster generation for
  topology/profile changes. Rotate all affected consumers only when visible
  dex inputs change. Generations participate in iced identity but are ignored
  by exchange subscription construction and are not persisted.
- Regression coverage: a canceled same-address account recipe queues a lag
  frame and cannot mark the new account session loading/reconciling
  (`src/account_update/stream/tests.rs:85-105`). A canceled cluster recipe
  queues a full position snapshot and cannot clear `stale` or refresh its
  close-sizing timestamp (`src/wallet_cluster_update.rs:2142-2194`). Parameter
  tests prove generation and source are both required, a reopened detail gets
  a unique generation without rotating another address, config clear removes
  detail provenance addresses, and diagnostics redact the wallet address
  (`src/subscription_state/user_data.rs:226-348`,
  `src/ws/user_streams.rs:309-327`, `src/message.rs:1969-2189`). The existing
  same-account reconnect test now proves the production connect path rotates
  its generation.
- Smallest behavior-preserving fix: enrich the existing recipe/message context
  and reject non-current queued events at update entry. Do not change parsing,
  wire topics, reconnect policy, manager buffering/refcounts, REST cadence,
  lifecycle classification, order preparation/signing, or views.
- Protected behavior: current recipe frames take the identical update paths;
  normal manager reconnects stay within the same sequential receiver; lag
  still emits before requesting the existing shared reconnect and REST repair;
  address/dex subscription payloads, stream cadence outside an actual recipe
  replacement, order semantics, status copy, UI, persistence, and secrets are
  unchanged.
- Residual uncertainty: formatting, exhaustive recipe-input mutation search,
  manager reconnect/lag/closure tracing, source normalization, redaction, and
  adversarial ordering review pass. Rust type-check/tests/clippy must execute on
  a host with ALSA development metadata. A terminal panic/abort of the
  singleton manager task would close the broadcast channel and is not a normal
  recoverable reconnect state; startup/shutdown fault handling remains for the
  later lifecycle-close track.

### F-19 — TWAP status results do not own an exact retry attempt

- Status: addressed in Turn 18; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: TWAP child-status task dispatch, result correlation, bounded status
  retries, account-fill confirmation, and reconciliation cleanup
- Preconditions/event ordering:
  1. A TWAP child has an ambiguous placement outcome and retains its CLOID for
     status repair.
  2. A status result is duplicated, or results from separately queued calls for
     the same CLOID are delivered out of order.
  3. One result has already consumed or advanced the mutable retry counter, or
     moved the child into filled/no-fill account reconciliation.
  4. The delayed result still finds the same `status_check_cloid`; that value
     intentionally remains present while account fills confirm the outcome.
- Evidence: both immediate and delayed status tasks previously emitted only
  the TWAP ID and CLOID. Dispatch had no in-flight owner, while the handler
  accepted every non-terminal result matching that CLOID and derived its next
  action from shared `status_check_retries`. Filled and canceled results retain
  the CLOID during their separate account-fill reconciliation phase
  (`src/order_execution/twap/status/tasks.rs:13-100`,
  `src/order_execution/twap/status.rs:34-225`).
- Violated invariant: exactly one status task may own a TWAP child retry
  attempt, and exactly one matching result may consume that attempt or advance
  its reconciliation phase.
- Risk: a duplicate missing/unclear/error result can consume bounded retry
  budget more than once. More importantly, a stale missing result can rewrite
  `AwaitingNoFillConfirmation` back to `StatusUnknown` and launch another
  lookup after a later canceled result already entered account-fill proof. A
  reversed open result can also enter target-specific cancellation after a
  newer status moved to fill reconciliation. These transitions cannot place a
  new child because the CLOID gate remains closed, but they can corrupt the
  fail-closed lifecycle, issue an obsolete cancel, or terminally fail a TWAP
  using evidence that no longer owns the phase.
- Why existing checks did not cover it: Turn 7 correlates the mutation result
  to an exact slice index/retry, but status repair is a recurring read phase
  after that pending mutation has cleared. CLOID equality proves the logical
  child, not which status attempt owns current mutable state; the terminal
  guard applies only after the TWAP has already terminated.
- Implemented fix: add one runtime-only optional status attempt to `TwapOrder`.
  A shared arming helper verifies the TWAP is non-terminal, the CLOID is still
  current, and no task already owns it, then captures the current bounded retry
  count in `TwapOrderStatusLoaded`. The handler requires exact CLOID/attempt
  ownership and clears it before applying the result, so every subsequent copy
  is stale. A retry arms the next attempt synchronously. Account-fill success
  and timeout cleanup clear any outstanding ownership independently of the
  CLOID (`src/twap_state/model.rs:184-189`,
  `src/order_execution/twap/status/tasks.rs:13-100`,
  `src/order_execution/twap/status.rs:34-64`,
  `src/order_execution/twap/fills.rs:176-182`).
- Regression coverage: one test calls status dispatch twice and proves only one
  task is created; one duplicates a missing result and proves retry count,
  events, and the next armed attempt advance only once; one delivers a stale
  missing result after canceled/no-fill status and proves the deadline and
  `AwaitingNoFillConfirmation` phase are unchanged
  (`src/order_execution/twap/tests/account/status_check.rs:137-215`). Existing
  filled, canceled, rejected, retry-exhaustion, stop, and terminal-result tests
  now supply and/or assert the attempt owner. Custom `TwapOrder::Debug` exposes
  only whether an attempt exists.
- Smallest behavior-preserving fix: correlate the existing read task with the
  existing retry counter rather than adding a mutation retry, timer, exchange
  identifier, or new lifecycle policy. No attempt value is signed, sent to the
  exchange, rendered, or persisted.
- Protected behavior: deterministic child CLOIDs, IOC request construction,
  slice size/price/cadence/randomization, retry limits and delays, status REST
  endpoint, fill attribution, no-fill confirmation, stop/timeout/archive paths,
  status strings, tasks for unique results, UI, persistence, and secrets are
  unchanged.
- Residual uncertainty: constructor/producer/consumer inventory, Elm message
  ordering, retry and reconciliation cleanup paths, redacted diagnostics, and
  diff review pass. Rust type-check/tests/clippy must execute on a host with
  ALSA development metadata. TWAP unexpected-resting cancel results remain a
  separate exact-attempt correlation candidate for continued Track 6 audit.

### F-20 — TWAP unexpected-child cancel attempts lack single result ownership

- Status: addressed in Turn 19; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: TWAP unexpected-resting child cancellation dispatch, delayed retry
  triggers, result messages, bounded retry accounting, stop/terminal guards,
  and diagnostic state
- Preconditions/event ordering:
  1. An IOC child unexpectedly rests and the TWAP begins target-specific
     cancellation by CLOID (or OID when no CLOID exists).
  2. Attempt N returns an ambiguous, contradictory, or transport-unknown
     result. The handler increments shared `cancel_retries` and schedules the
     delayed trigger for attempt N+1.
  3. A duplicate result for N arrives, or the N result is delayed until N+1 is
     in flight. The pending operation still has the same target throughout the
     bounded retry sequence.
  4. Independently, a duplicate retry-due message for N+1 can arrive while the
     same counter and target still match.
- Evidence: `TwapUnexpectedCancelRetryDue` carried an attempt and compared it
  with `cancel_retries`, but dispatch did not consume or otherwise mark that
  attempt in flight. `twap_cancel_child_task` then emitted
  `TwapUnexpectedCancelResult` with only TWAP ID and target. The result handler
  accepted any message matching the long-lived pending target and incremented
  the current shared counter (`src/order_execution/twap/cancel.rs:22-110`,
  `src/order_execution/twap/helpers/cancellation.rs:43-88`).
- Violated invariant: one retry attempt may create at most one in-flight cancel
  task, and exactly one result from that attempt may consume retry budget or
  settle the pending child.
- Risk: repeated ambiguous/error results for one dispatch can consume the
  five-attempt budget without five owned exchange attempts, transition the
  TWAP to terminal error, archive its history, and clear its captured agent key
  while the unexpected resting child may remain open. A stale earlier success
  can also finish the pending operation while a newer task still owns it.
  Duplicate retry triggers can send redundant cancellation mutations. The
  mutation is deliberately target-specific and idempotent, but duplicate
  dispatch and false retry exhaustion are unnecessary lifecycle risk.
- Why existing checks did not cover it: target matching identifies the same
  logical child, not which recurring dispatch owns current mutable state. The
  retry-trigger attempt check rejects a trigger only after the counter changes;
  it does not distinguish two copies while one task is in flight. Turn 9's
  contradictory-response classifier prevents false success but does not make
  retry accounting idempotent.
- Implemented fix: add one runtime-only optional unexpected-cancel owner to
  `TwapOrder`. Both initial entry paths capture the current retry attempt when
  they create the pending cancel. A shared arming helper requires non-terminal
  state, the exact current retry count, the exact OID/CLOID target, and no
  existing owner before any cancel task dispatch. The result message carries
  that attempt; the handler requires the exact target, retry count, and owner,
  then clears ownership before applying the result. Ambiguous/error handling
  then schedules the unchanged next retry with no owner until its delayed
  trigger claims it
  (`src/twap_state/model.rs:197-201`,
  `src/order_execution/twap/cancel.rs:22-110`,
  `src/order_execution/twap/slice_result.rs:270-412`,
  `src/order_execution/twap/status.rs:198-319`).
- Regression coverage: duplicate retry-due delivery creates one task; duplicate
  ambiguous results consume one retry/event/backoff transition; a near-limit
  duplicate error cannot falsely exhaust the budget; a stale success from
  attempt zero cannot settle attempt one; and the current attempt-one success
  retains the existing terminal transition
  (`src/order_execution/twap/tests/cancel.rs:195-460`). Placement-result and
  status-result tests prove both initial cancellation paths arm attempt zero
  (`src/order_execution/twap/tests/place_result.rs:152-186`,
  `src/order_execution/twap/tests/account/status_check.rs:300-329`). Existing
  target mismatch, terminal, redaction, contradictory response, and retry
  exhaustion coverage now models exact ownership. Custom `TwapOrder::Debug`
  exposes only whether an owner exists.
- Smallest behavior-preserving fix: correlate and atomically claim the existing
  retry count rather than changing the target-specific mutation, adding a new
  retry, or introducing an exchange status policy. The attempt is not signed,
  transmitted to Hyperliquid, rendered, or persisted.
- Protected behavior: CLOID-before-OID cancel selection, signed cancellation
  payload, classification, retry maximum and delays, refresh/invalidation,
  stop and archive behavior for unique results, status/event strings, UI,
  scheduling, persistence, and secrets are unchanged.
- Residual uncertainty: field/message/call-site inventories, initial and retry
  dispatch paths, target matching, stop/terminal guards, redacted diagnostics,
  and diff review pass. Rust type-check/tests/clippy must execute on a host with
  ALSA development metadata. Fill reconciliation can mark an unexpected child
  filled before a later cancel result rewrites its child status; that reversed
  ordering is the next distinct Track 6 candidate.

### F-21 — Filled unexpected-child label depends on cancel result ordering

- Status: deferred in Turn 20; financial invariants characterized; resolving
  the label requires explicit user-visible/history semantics approval
- Severity: Medium
- Scope: TWAP unexpected-resting child status, authoritative fill
  reconciliation, live details rows, terminal advanced-order history, and
  event-order permutations
- Preconditions/event ordering:
  1. An IOC child unexpectedly rests and a target-specific cancellation is in
     flight.
  2. Account or user-stream reconciliation proves a partial or full fill for
     the exact OID, coin, and side.
  3. The fill and confirmed/terminal-not-open cancel result arrive in opposite
     orders while both refer to that same child.
  4. Each handler writes its own single-valued `TwapChildStatus`; the later
     writer therefore determines the label.
- Evidence: fill reconciliation preserves the maximum child fill, average
  price, and fee, then assigns `Filled` for any non-rejected positive fill
  (`src/twap_state/order/reconciliation.rs:54-65`). Both definitive cancel
  branches assign `UnexpectedRestingCancelled` without consulting retained
  fill metrics (`src/order_execution/twap/cancel.rs:121-153`). Live details
  render `child.status.label()`, terminal snapshots persist that label as a
  string, and history details render it directly
  (`src/order_views/twap_details/sections/activity.rs:66-93`,
  `src/advanced_order_history/snapshots.rs:88-107`,
  `src/order_views/advanced_history_details/sections.rs:174-186`).
- Violated invariant: identical exchange facts should not produce a different
  terminal child outcome label solely because two valid messages were queued
  in a different order.
- Risk: the live row and persisted audit history can say `Canceled` for a child
  with a confirmed fill in one ordering and `Filled` in the reverse ordering.
  This is presentation/history ambiguity, not lost exposure: child and
  aggregate fill quantities, average price, fee, remaining target, scheduling,
  completion, and terminal numeric history remain correct and idempotent.
- Why existing checks do not cover it: `filled_size` intentionally carries
  fill truth independently of the status enum, so all financial calculations
  are safe while last-writer label behavior remains observable. Existing fill
  and cancel tests exercised each lane separately, not both permutations.
- Characterization coverage: partial-fill permutations now prove identical
  child/aggregate fill metrics, remaining target, pending cleanup, retry state,
  next lifecycle status, and duplicate-fill idempotence while explicitly
  recording the two current labels. Full-fill permutations prove identical
  completed status, scrubbed agent key, and archived target/fill/remaining/
  average/fee metrics while recording `Canceled` versus `Filled` history rows
  (`src/order_execution/twap/tests/cancel.rs:462-570`).
- Why deferred: making fill dominate would change an existing visible
  `Canceled` row to `Filled`; making cancel dominate would change the reverse
  case; adding `Partially filled / canceled` or an equivalent combined state
  adds new UI/history copy and semantics. All three violate this campaign's
  no-UX-change authority. The persisted history string also makes the chosen
  meaning part of the user-visible audit record. No trading-safety benefit
  justifies silently selecting one.
- Approval options: explicitly choose fill-dominant, cancel-dominant, or a new
  combined outcome label, then update live/history rendering and both
  permutation expectations together. No persisted schema migration is required
  for future entries, but existing stored strings should remain untouched
  unless separately approved.
- Protected behavior: no production source changed in Turn 20. Fill matching,
  deduplication, quantities, fees, target-specific cancellation, retry and
  refresh behavior, scheduling, stop/completion/archive behavior, current
  labels/copy, persistence format, and secrets remain unchanged.
- Residual uncertainty: the characterization tests cannot execute on this host
  until ALSA development metadata is present. Event log summary ordering also
  follows message order, but those entries truthfully record both events and do
  not alter financial state; no copy normalization is proposed.

### F-22 — Terminal TWAP cancel outcomes schedule a dead retry trigger

- Status: addressed in Turn 21; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium defense-in-depth hardening; no current duplicate exchange
  mutation or financial-state corruption was confirmed
- Scope: full-fill TWAP terminalization while an exact unexpected-child cancel
  attempt is in flight, retryable cancel-result handling, delayed retry
  dispatch, account refresh, terminal history, and captured-key lifetime
- Preconditions/event ordering:
  1. An IOC child unexpectedly rests and attempt zero owns its target-specific
     cancel task.
  2. A complete account/user-fill snapshot arrives first, attributes the full
     target to that child, transitions the TWAP to `Completed`, archives its
     financial history, and clears the captured agent key.
  3. The already-dispatched cancel then returns either an ambiguous successful
     envelope or a transport-unknown error, both of which ordinarily require a
     bounded retry for an active TWAP.
- Evidence: the result handler correctly claimed the exact cancel attempt and
  incremented its retry counter, then unconditionally built a batch containing
  both the immediate account refresh and a delayed retry-due task whenever
  `retry_cancel` was set. The later retry-due handler independently rejected
  terminal state before cloning the key or dispatching cancellation
  (`src/order_execution/twap/cancel.rs:22-75,87-262`). The pre-fix adversarial
  path therefore produced two task units even though only the refresh could do
  useful work.
- Violated invariant: once authoritative reconciliation makes automation
  terminal, an in-flight result may still request authoritative read repair but
  must not originate future lifecycle/mutation retry work.
- Risk: current code did not send a duplicate cancel because the delayed
  consumer revalidated terminal state and terminal archive scrubbed the key.
  It nevertheless scheduled a guaranteed no-op timer/message and made terminal
  safety depend unnecessarily on that second guard. A future retry refactor
  could turn this producer/consumer mismatch into post-terminal exchange work.
- Why existing checks did not cover it: Turn 19 tested that a retry-due message
  cannot dispatch for an already-terminal TWAP, but it did not reverse full
  fill and retryable cancel-result delivery to prove the producer itself stops
  creating retry work.
- Implemented fix: after preserving the existing result classification,
  attempt claim, retry count, child summary, and immediate account refresh, the
  cancel-result handler checks terminal state before batching the delayed
  retry. Terminal state returns only the unchanged immediate origin-account
  refresh; active state retains the same bounded retry task and delay
  (`src/order_execution/twap/cancel.rs:247-262`).
- Regression coverage: both ambiguous-response and transport-error permutations
  full-fill the target while cancel attempt zero is owned, assert completion,
  key scrubbing, exact financial/history stability, one refresh task, and no
  armed retry; even a synthetic retry-due delivery remains a no-op
  (`src/order_execution/twap/tests/cancel.rs:572-627`). Nearby assertions prove
  all four terminal statuses reject scheduling/timer ticks, delayed terminal
  status results cannot mutate history, and repeated archive calls upsert one
  entry while keeping the key empty (`src/twap_state/tests/timing.rs:74-91`,
  `src/order_execution/twap/tests/account/status_check.rs:332-366`,
  `src/advanced_order_history/tests/twap_snapshot.rs:38-66`).
- Smallest behavior-preserving fix: suppress one delayed message/task only
  after terminal state is already authoritative. No exchange payload, target,
  CLOID/OID preference, retry limit or delay for active orders, immediate
  refresh, local status/event copy, child label, financial value, history
  schema, persistence format, or secret handling changes.
- Residual uncertainty: source/call-site tracing, redacted state inspection,
  formatting, and diff review pass. The regression tests and Rust type-check
  still require a host with ALSA development metadata. F-21 remains separately
  deferred because its visible child-label semantics are outside this finding.

### F-23 — Final-save shutdown can start a new automation mutation

- Status: addressed in Turn 22; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: main-window closure, debounced/in-flight final config persistence,
  Chase place/reprice/size-correction dispatch, TWAP slice dispatch, failed-save
  recovery, and exposure-reducing automation cleanup
- Preconditions/event ordering:
  1. A Chase has queued placement, reprice, or size correction work, or a TWAP
     slice is due.
  2. A config save is already in flight or a debounced save is due.
  3. The main window closes. `WindowClosed` requests a final save and the iced
     daemon stays alive until the blocking file write produces `ConfigSaved`.
  4. A queued timer, book update, initial-book result, or account-reconciliation
     result reaches the automation handler before the exit task terminates the
     runtime.
- Evidence: `update_window` closes the main window before calling
  `flush_pending_config_save_and_exit`; that method deliberately returns
  `Task::none()` while a save is already in flight, or starts an asynchronous
  save while setting `config_save_exit_requested`. Before Turn 22, the Chase
  place/reprice/modify and TWAP slice gates checked account loading,
  reconciliation, rate/cooldown, and lifecycle state but not that exit flag.
  Successful/no-save branches also cleared the flag before returning
  `iced::exit()` (`src/window_update.rs:29-35`,
  `src/config_persistence/save.rs:106-172`,
  `src/order_execution/chase/lifecycle.rs:23-37`,
  `src/order_execution/chase/lifecycle/place.rs:309-317`,
  `src/order_execution/chase/lifecycle/reprice.rs:151-205`,
  `src/order_execution/twap/execution/lifecycle.rs:107-121`).
- Violated invariant: after the user closes the main window and final exit owns
  the daemon, no new exposure-progressing automation mutation may begin.
  Results for mutations already sent may still reconcile, and exact
  exposure-reducing cancellation must remain available.
- Risk: a slow or stalled config write could let an unobserved Chase placement,
  modify, size correction, or TWAP IOC slice reach the exchange after the
  trading window has disappeared. The process could then exit immediately,
  leaving the user unable to monitor the new mutation and with no persisted
  live automation to resume. This is realistic continuation of automation
  across a shutdown boundary, even though the request itself retains the right
  account, key, price, size, and identifier.
- Why existing checks did not cover it: account/reconciliation/cooldown guards
  prove exchange readiness, not application-lifecycle authority. Runtime-only
  persistence prevents restart resurrection but does not stop the still-live
  daemon. Config-save tests covered durability and failure feedback, while
  Chase/TWAP tests covered busy exchange gates without a closed-window exit
  interval.
- Implemented fix: retain `config_save_exit_requested` through successful and
  immediate `iced::exit()` tasks, clearing it only when a failed final save
  explicitly returns control to the app. Add a Chase progress gate that combines
  the unchanged exchange-readiness gate with the exit fence for place, book
  reprice, reconciled modify, and size correction. Add the same exit fence to
  the TWAP slice gate. Status checks and target-specific Chase/TWAP cancellation
  continue using their existing paths and can still reduce or reconcile known
  exposure (`src/config_persistence/save.rs:106-172`,
  `src/order_execution/chase/lifecycle.rs:23-37`,
  `src/order_execution/chase/lifecycle/place.rs:309-317`,
  `src/order_execution/chase/lifecycle/reprice.rs:151-205`,
  `src/order_execution/twap/execution/lifecycle.rs:107-121`).
- Regression coverage: config-save tests prove the fence is armed while an
  existing write finishes and remains armed through both successful-save and
  no-save exit tasks; the established failed-save test proves it clears on
  recovery. Chase tests prove placement, reprice, and queued size correction
  remain unsent with their exact queued values and resume normally when the
  fence clears. TWAP proves a due slice keeps zero attempts/sends/pending op and
  then dispatches once after recovery. Separate Chase and TWAP tests prove
  cancel retries remain authorized during exit
  (`src/config_persistence/save/tests.rs:92-142`,
  `src/order_execution/chase/lifecycle/tests/place.rs:83-114`,
  `src/order_execution/chase/lifecycle/tests/reprice/direct.rs:118-138`,
  `src/order_execution/chase/lifecycle/tests/reprice/tick.rs:23-53,113-135`,
  `src/order_execution/twap/tests/gating.rs:40-78`,
  `src/order_execution/twap/tests/cancel.rs:195-214`).
- Smallest behavior-preserving fix: reuse the existing runtime-only exit flag
  at the final mutation owners. No automation is stopped or removed; no task or
  key is persisted; no active-order cancel is suppressed; and no valid order
  payload, identifier, price, size, timing rule, retry rule, history, schema,
  status text, view, or normal in-app interaction changes.
- Residual uncertainty: this batch covers autonomous Chase/TWAP progress after
  main-window closure while final exit remains owned. F-24 separately records
  the existing failed-save path that relinquishes exit after the window is
  gone. A queued one-shot/cluster/leverage intent and close-versus-config-clear
  orchestration still require Track 9 audit. Rust type-check/tests/clippy remain
  environment-blocked before Kerosene compilation.

### F-24 — Failed final save resumes automation without a main window

- Status: deferred in Turn 22; every safe resolution changes visible window,
  exit, or unsaved-config behavior and requires explicit approval
- Severity: High
- Scope: iced main-window close semantics, final config-save failure, toast/
  retry visibility, daemon lifetime, and all runtime trading automation
- Preconditions/event ordering:
  1. The main window uses iced's default `exit_on_close_request: true`.
  2. The OS/custom close path removes that window and emits `WindowClosed`.
  3. A pending final config save fails after the window is gone.
  4. `BlockExitOnError` clears `config_save_exit_requested`, queues an immediate
     future save, pushes a retry toast, and returns `Task::none()`.
- Evidence: iced 0.14 converts a close request with the default window setting
  into `window::Action::Close`, removes the window, then emits `Event::Closed`;
  Kerosene subscribes to closed—not close-requested—events. The main-window
  handler therefore starts persistence after removal. The failure branch is
  intentionally documented to stay open and tell the user to close again, but
  there is no main window to render that feedback or receive another close.
  Because the daemon does not exit when all windows are closed and the exit flag
  is cleared, subscriptions and queued automation resume
  (`src/window_chrome.rs:6-14`, `src/app_boot/windows.rs:10-27`,
  `src/subscription_state.rs:24-26`, `src/window_update.rs:29-35`,
  `src/config_persistence/save.rs:134-172`; iced 0.14 local sources
  `iced_core/src/window/settings.rs`, `iced_winit/src/lib.rs`, and
  `iced/src/daemon.rs`, inspected 2026-07-11).
- Violated invariant: aborting exit because configuration could not be saved
  must return the user to an observable, controllable trading application; it
  must not resume client-side automation in a headless daemon.
- Risk: after a disk/permission/transient write failure, Chase or TWAP can
  continue to mutate exchange state without its main monitoring/control window.
  The promised toast and “close again” recovery are not actionable. A later
  ordinary retry can save successfully but cannot exit because the exit owner
  was cleared, leaving the process alive until externally terminated.
- Why current checks do not cover it: config-save unit tests exercise the pure
  exit decision and state flags without a real window manager. The headless
  daemon behavior spans iced's automatic close action, Kerosene's closed-event
  subscription, and the later asynchronous save result.
- Why deferred: intercepting `CloseRequested` and keeping the window open until
  save completion changes close timing/appearance; reopening the main window on
  failure visibly reverses a close and must restore window state; exiting on
  failure discards the current durability guarantee; retaining a permanent
  headless fence strands the process. Selecting among those behaviors exceeds
  this campaign's no-UX/no-workflow authority.
- Approval options: (1) keep the main window open during final save and close
  only after success; (2) close immediately but reopen on save failure; or (3)
  honor close even on save failure and accept unsaved preference/layout loss.
  Whichever policy is chosen should add a real close-request/save-result window
  lifecycle test and define behavior when auxiliary windows remain.
- Protected behavior: Turn 22 does not alter the established save-error branch,
  toast copy, automatic retry due time, or window creation/closure behavior.
  F-23 only fences the interval where final exit still owns the daemon.
- Residual uncertainty: the executable GUI path cannot be smoked on this host
  because compilation stops at missing ALSA metadata. Source semantics are
  explicit, but platform-specific window-manager timing should be verified once
  a product policy is approved.

### F-25 — Final exit does not fence fresh non-automation mutation intents

- Status: addressed in Turn 23; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: root update routing; ticket/preset/Alfred/quick/HUD/close/NUKE/move,
  leverage, wallet-cluster order/close, Chase start/adoption, and TWAP start;
  result/status reconciliation and explicit cancellation cleanup
- Preconditions/event ordering:
  1. A click, chart gesture, Alfred submission, cluster action, or other
     mutation-intent message is already queued but has not reached root update.
  2. `WindowClosed` for the main window runs first and final save or clear keeps
     the iced daemon alive.
  3. The queued intent reaches root update before the process-exit task runs.
- Evidence: every mutation surface reaches a finite message set in
  `update_order`, `update_alfred`, or `update_wallet_cluster`; Alfred's trading,
  close, and NUKE branches call shared execution methods directly after their
  command message is routed. Before Turn 23, `TradingTerminal::update` selected
  a feature route without checking exit ownership. Turn 22 guarded autonomous
  Chase/TWAP progress but did not cover fresh one-shot, modify, leverage, fan-out,
  or automation-start messages (`src/app_update.rs:18-45`,
  `src/order_update.rs:54-175,365-450`,
  `src/alfred_update/submit.rs:21-391`,
  `src/wallet_cluster_update.rs:134-143`).
- Violated invariant: once main-window closure transfers ownership to final
  persistence and exit, no fresh signed exchange mutation may begin. Already-
  dispatched work must still reconcile, and exact exposure-reducing cleanup
  must remain authorized.
- Risk: a queued order, close, modify, leverage change, cluster fan-out, or
  automation start could reach the exchange after the principal trading window
  disappeared. The process could then exit without monitoring the new action;
  active automation is intentionally not persisted. Correct account/payload
  construction does not make this shutdown ordering safe.
- Why existing checks did not cover it: per-surface pending, freshness,
  reconciliation, account, and snapshot guards prove that a request is valid
  to trade; none grants application-lifecycle authority. The prior exit tests
  exercised final-save and autonomous progress gates, not queued root intents
  or direct Alfred/cluster fan-out routes.
- Implemented fix: classify the complete fresh mutation-intent set at the root
  update boundary and return `Task::none()` while final exit owns the daemon.
  The classifier deliberately excludes `CancelOrder`, Chase/TWAP stop messages,
  all direct results, status lookups, refreshes, and retry/cancel cleanup. It
  therefore cannot clear uncertainty or suppress target-specific exposure
  reduction (`src/app_update.rs:18-45`).
- Regression coverage: one table constructs every fenced message class,
  including direct Alfred and cluster routes; a complementary table proves
  representative cancel/stop, direct result, move result, and TWAP result
  messages remain unfenced. A root integration regression retains the move-drag
  owner, proving the rejected intent never reaches its feature route
  (`src/app_update/tests.rs:88-213`).
- Smallest behavior-preserving fix: one runtime-only root predicate, with no
  state clearing, replay, persistence, view, status string, payload, price,
  size, TIF, reduce-only, account/key, timing, or normal interaction change.
  The only discarded work is a fresh intent delivered after the main window is
  already closed and final exit still owns the daemon.
- Residual uncertainty: source and task-call inventory covers every current
  signing entry. A future signed surface must be added to the classifier; the
  focused test's explicit table makes that policy visible but cannot enforce it
  automatically. Rust compilation and test execution remain blocked by ALSA.

### F-26 — Config clear drops final-exit ownership and can strand the daemon

- Status: addressed in Turn 23; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: High
- Scope: config-clear request/start/result, save-to-clear handoff, main-window
  close, daemon exit, fresh mutation fencing, runtime/key cleanup, and redacted
  failure handling
- Preconditions/event ordering:
  1. Config clear is requested with no pending trading work or active
     automation and starts immediately or waits behind an in-flight config save.
  2. The main window closes before clear completion.
  3. The old close path sets `config_save_exit_requested`, then the clear branch
     immediately clears it; start and save-to-clear handoff also clear it.
  4. A clear result arrives. On ordinary success runtime state is reset but no
     process exit is returned. If a fresh mutation raced in, the result defers
     runtime reset to preserve pending uncertainty and also returns no exit.
- Evidence: before Turn 23, `flush_pending_config_save_and_exit`,
  `handle_config_save_result`, `request_config_clear`, and
  `start_config_clear_task` each relinquished the final-exit flag on a clear
  path. `handle_config_clear_result` had no record that the main window was gone
  and every success/failure/deferred branch returned only its normal in-app
  task. The iced daemon is intentionally not configured to exit merely because
  all windows disappear (`src/config_persistence/save.rs:106-140`,
  `src/config_persistence/clear.rs:15-111`, `src/window_update.rs:29-35`).
- Violated invariant: the final persistence operation selected at close must
  not relinquish daemon ownership. It must keep fresh mutations fenced, apply
  the same clear result semantics, and terminate after completion.
- Risk: the common success case could leave a subscription daemon alive with no
  main window. More seriously, a queued order could dispatch after the flag was
  cleared; clear completion would then deliberately retain runtime/key and
  pending uncertainty while persistence was paused, producing an unmonitored
  headless trading process. A clear error likewise left its redacted feedback
  in a process with no principal window.
- Why existing checks did not cover it: clear tests correctly protected runtime
  secrets and uncertainty when trading appeared during deletion, while save
  tests correctly modeled final-save exit. No test composed clear ownership
  with `WindowClosed`, deferred-save handoff, or result completion.
- Implemented fix: config-clear request/start and save handoff preserve an
  already-set final-exit owner, while root routing discards a new clear request
  delivered only after close. A start-time trading/automation race keeps the
  owner and returns `iced::exit()` after the existing refusal. Clear results run
  their unchanged success, partial-warning, deferred-runtime, or redacted-error
  branch; when exit-owned, the returned task batches that work with `iced::exit()`
  and keeps the fence armed until execution
  (`src/config_persistence/save.rs:106-140`,
  `src/config_persistence/clear.rs:15-111`).
- Regression coverage: root tests prove a newly delivered clear message is
  rejected before routing. Save tests cover immediate clear and save-to-clear
  handoff ownership. Clear tests prove the request helper cannot clear an
  existing owner and cover a start-time pending-trading race, successful runtime
  reset plus exit, and redacted failure plus exit
  (`src/app_update/tests.rs:216-227`,
  `src/config_persistence/save/tests.rs:105-134`,
  `src/config_persistence/clear.rs:731-743,833-847,1295-1334`).
- Smallest behavior-preserving fix: reuse the existing final-exit flag; add no
  schema or persisted state. In-app clear begins with the flag false and keeps
  all established UI/status/runtime behavior. Exit-owned clear differs only
  after the main window is already closed: it remains fenced and terminates as
  the user requested. Unlike F-24's failed save, exiting after a clear failure
  does not discard an unsaved config snapshot; the old config remains or
  persistence is already paused according to the existing clear branch.
- Residual uncertainty: real iced task ordering and auxiliary-window teardown
  cannot be smoke-tested on this host. F-24 remains distinct and deferred:
  final-save failure still relinquishes ownership because resolving its
  durability/window policy requires approval.

### F-27 — Ghost-profile cluster invalidation is owned by the wrong lifecycle

- Status: addressed in Turn 24; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium; the current source cannot type-check, while the intended
  stale-stream safeguard has no executable binary until corrected
- Scope: ghost-account creation/removal, selected wallet-cluster profile
  bindings, user-data stream generation rotation, and queued cluster frames
- Preconditions/event ordering:
  1. The Turn 17 stream-generation change is compiled after system dependencies
     are available.
  2. `ghost_wallet_task` creates a new profile or
     `forget_ghost_account_task` removes an existing ghost profile.
  3. The selected cluster may reference the removed profile and must replace
     its subscription recipe before queued frames can be trusted.
- Evidence: the prior change computed `selected_cluster_profile_removed` in
  `ghost_wallet_task` using a brand-new secret ID before that profile could
  belong to any cluster, then referenced the function-local value from
  `forget_ghost_account_task`. Rust item formatting accepts the syntax, but the
  removal function has no binding in scope and the creation binding is unused.
  This is explicit in prior commit `2cbd350d38f08989f21e3305e1f2b0355c2afba0`.
  The intended saved-profile deletion path computes the same predicate in the
  correct function before removal (`ghost_wallet_task`,
  `forget_ghost_account_task`,
  `src/account_state/switching/saved_delete.rs:206-315`).
- Violated invariant: the operation that changes selected-cluster profile
  topology must own its pre-removal membership check and generation rotation;
  creation must not claim removal state for an unrelated new identity.
- Risk: the checkout fails compilation once dependency discovery reaches
  Kerosene, preventing validation and release. Merely deleting the undefined
  branch would instead omit the intended recipe-generation invalidation when a
  selected cluster loses a ghost profile, weakening stale queued-frame defense.
  Because the broken source cannot produce a running binary, no live financial
  mutation from this exact revision is claimed.
- Why existing checks did not cover it: all Rust test/check/clippy attempts stop
  in `alsa-sys` before the crate is parsed or type-checked. `cargo fmt` validates
  syntax/format only. Stream-generation regressions covered cluster edits and
  saved-profile deletion, but not ghost-profile removal.
- Implemented fix: remove the impossible creation-time membership calculation
  and compute it inside `forget_ghost_account_task` after all blocking gates but
  immediately before profile removal. Keep the existing conditional rotation
  exactly where removal commits (`src/account_state/switching/ghost.rs:82-98`).
- Regression coverage: a selected cluster references an inactive ghost
  profile, the profile is forgotten, and the test proves account removal plus
  one wrapping generation increment. The inactive profile avoids unrelated
  account-switch/disconnect tasks (`src/account_state/switching/ghost/tests.rs:202-232`).
- Smallest behavior-preserving fix: two lines move between adjacent lifecycle
  functions and one focused test. No account/profile data, cluster membership,
  subscriptions, status copy, view, order preparation, trading policy,
  persistence, or secret handling changes outside the already-intended
  removal-time invalidation.
- Residual uncertainty: source inspection, rustfmt, and diff checks pass, but
  the compiler and test remain blocked by ALSA metadata. The broader Track 9
  disconnect/profile/key lifetime audit remains incomplete.

### F-28 — Final pre-dispatch TWAP skips retain a terminal signing key

- Status: addressed in Turn 25; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium; no post-terminal dispatch is currently reachable, but a
  usable secret survives beyond its lifecycle owner
- Scope: TWAP initial and retry slice planning, range/minimum/precision skips,
  `schedule_after_attempt`, terminal status, captured agent-key lifetime, and
  existing advanced-history visibility
- Preconditions/event ordering:
  1. A TWAP reaches its final planned slice, or a retry of that final slice.
  2. Planning rejects it before mutation dispatch because size precision,
     allowed price range, rounded IOC marketability, or child minimum notional
     cannot be satisfied.
  3. `record_twap_skip` or `record_twap_retry_skip` calls
     `schedule_after_attempt`; no remaining slots changes the TWAP to `Stopped`
     or `CompletedPartial`.
  4. The helper returns directly to `execute_due_twap_slice` without calling
     `archive_twap_if_terminal`.
- Evidence: `schedule_after_attempt` has exactly three production callers.
  `finish_twap_attempt` always follows it with archive; the initial-skip and
  retry-skip helpers did not. Every other production terminal assignment in
  cancel, slice-result, status, fills/reconciliation, stop, and deadline code
  reaches the archive boundary. Terminal predicates prevent later scheduling,
  but the `CapturedAgentKey` remained nonempty and cloneable in the retained
  runtime TWAP (`src/twap_state/order/schedule.rs:71-103`,
  `src/order_execution/twap/execution/lifecycle.rs:13-77`).
- Violated invariant: a captured signing key may remain usable only while a
  nonterminal lifecycle or exact in-flight reconciliation/cancel owner can need
  it; a terminal pre-dispatch skip has neither.
- Risk: the key remains resident until config clear or process drop despite the
  TWAP being unable to perform more work. Current terminal gates prevent a new
  mutation, so no duplicate or wrong-account dispatch is claimed. The excess
  lifetime still enlarges secret exposure and makes a future terminal-gate
  regression more consequential.
- Why existing checks did not cover it: terminal archive tests begin with a
  terminal TWAP and explicitly call the archive method. Scheduler tests prove
  terminal status but did not inspect the key. Skip-path tests did not exercise
  the last initial/retry slot through `execute_due_twap_slice`.
- Implemented fix: make the canonical scheduler clear the captured key in its
  two existing terminal branches. All three callers therefore share one owner;
  nonterminal scheduling performs no clear. The change invokes no archive,
  persistence, state removal, event, status, or task work beyond the existing
  terminal branches (`src/twap_state/order/schedule.rs:71-105`).
- Regression coverage: a nonterminal initial skip proves the key remains for
  the next slice. Final initial and partially filled retry skips prove
  `Stopped`/`CompletedPartial`, existing attempt/child cleanup, an empty key,
  zero exchange tasks, and unchanged empty history
  (`src/order_execution/twap/tests/terminalization.rs:32-108`).
- Smallest behavior-preserving fix: a runtime-only `Zeroize` at the scheduler's
  two terminal assignments. No slice planning, count, cadence, randomization,
  range, minimum, retry, payload, result, event, status string, view, history,
  config, or normal nonterminal key lifetime changes.
- Residual uncertainty: F-29 records the deliberate no-history behavior rather
  than changing it silently. Rust execution remains blocked by ALSA; source,
  call-site, formatting, and diff inspection pass.

### F-29 — Final pre-dispatch TWAP skips do not enter advanced history

- Status: deferred in Turn 25; adding a history entry changes visible and
  persisted behavior and requires explicit approval
- Severity: Medium for audit continuity; no exchange mutation is unknown on
  these paths because planning fails before dispatch
- Scope: final initial/retry range, minimum-notional, rounded-price, and
  precision skips; terminal events; advanced-order history; config persistence
- Preconditions/event ordering: the F-28 ordering terminalizes the final
  pre-dispatch skip. Unlike result/status/fill/stop/deadline terminal paths, the
  skip helper returns without `archive_twap_if_terminal`.
- Evidence: both skip helpers call the pure scheduler and return; the caller
  then returns `Task::none()`. Existing live state contains the terminal status
  and event, while advanced history remains unchanged. The new characterization
  tests assert that established behavior explicitly
  (`src/order_execution/twap/execution.rs:153-205`,
  `src/order_execution/twap/execution/lifecycle.rs:13-48`,
  `src/order_execution/twap/tests/terminalization.rs:51-108`).
- Violated invariant: the architecture describes terminal advanced orders as
  inspectable history, but this terminal class remains only in the runtime TWAP
  map and disappears on restart.
- Risk: a user cannot inspect the completed skip sequence in advanced history
  after restart, weakening local audit continuity. There is no exchange-side
  exposure to reconcile because no request was prepared or sent for the final
  skip.
- Why deferred: invoking `archive_twap_if_terminal` would add a visible history
  row and schedule config persistence where the current path does neither. The
  campaign explicitly cannot change advanced-history visibility or persisted
  behavior without approval. F-28 independently removes the signing secret and
  terminal gates already forbid further work.
- Approval options: (1) archive final pre-dispatch skip exhaustion like every
  other terminal TWAP, accepting the new visible/persisted entry; or (2) retain
  the current live-only terminal event. If option 1 is selected, update the
  characterization tests and document how historical summary/status should
  represent zero-fill versus partial-fill exhaustion.
- Protected behavior: Turn 25 does not add/remove history, call persistence,
  alter the terminal event/status, or change any visible copy.
- Residual uncertainty: the history/UI meaning requires a product decision;
  the key-lifetime safety issue is separately closed by F-28.

### F-30 — Saved-account deletion clones and over-retains raw profile keys

- Status: addressed in Turn 26; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium; no current leak or wrong-account dispatch was found, but
  every profile key was copied into a rollback object and the removed key had a
  second cleanup copy beyond its only legitimate owner
- Scope: saved-profile deletion in OS keychain and encrypted-config modes;
  deletion preparation, rollback, durable save success/failure, profile cleanup,
  `AccountProfile` zeroizing fields, and redacted errors
- Preconditions/event ordering:
  1. A saved-profile deletion passes pending-trading, automation, ghost, lock,
     and encrypted-password gates.
  2. The prior implementation cloned the target profile for keychain cleanup
     and cloned the complete accounts vector into `SavedAccountDeleteRollback`.
  3. The canonical profile was removed and the post-delete config snapshot was
     saved or rejected.
  4. On failure, the cloned vector replaced canonical account state; on
     success, all rollback clones and the cleanup clone survived until the end
     of the function even though rollback was no longer legal after the first
     durable save.
- Evidence: parent commit
  `a2f8abf4ac71502e4a619495b6f53eb060752861` shows
  `SavedAccountDeleteRollback::capture` cloning `terminal.accounts` and
  `delete_saved_account_task_with_hooks` separately cloning
  `profile_snapshot`. Config snapshots already synthesize profiles with empty
  credential fields, encrypted serialization wraps plaintext in `Zeroizing`,
  and keychain deletion needs only the secret ID
  (`src/config_persistence/save/snapshot.rs:6-18`,
  `src/config/secrets/crypto.rs:23-47`,
  `src/config/secrets/keychain.rs:344-405`).
- Violated invariant: a raw profile key should have one deliberate owner. A
  deletion may retain the removed profile only while an ordinary durable-save
  failure can restore it; cleanup metadata and unrelated profiles do not need
  raw credential copies.
- Risk: an inactive-profile delete transiently multiplied every saved signing
  and legacy per-profile integration key and retained the removed copies across
  keychain I/O. `Zeroizing` eventually scrubbed them and diagnostics were
  redacted, so no disclosure is claimed. The unnecessary lifetime enlarges the
  impact of a future diagnostic, callback, or cleanup regression.
- Why existing checks did not cover it: deletion tests verified account order,
  encrypted payload contents, pending cleanup intent, save/cleanup ordering,
  rollback values, and redacted toasts, but did not distinguish moving the
  original zeroizing owner from cloning it or constrain the cleanup hook to
  identity-only input.
- Implemented fix: capture only non-credential rollback metadata before
  mutation, move the removed `AccountProfile` into that owner, reinsert it at
  the original index on ordinary save failure, and explicitly zeroize its two
  secret fields immediately after a successful first save. Replace the cleanup
  hook's `&AccountProfile` with `&str` and call the existing ID-only keychain
  helper. Encrypted staging remains before mutation and retains its existing
  zeroizing payload/plaintext boundaries
  (`src/account_state/switching/saved_delete.rs:18-95`,
  `src/account_state/switching/saved_delete.rs:150-340`).
- Regression coverage: ordinary encrypted-config and OS-keychain save-failure
  tests record the original agent-key allocation and require rollback to
  restore that exact allocation, which the old clone-based implementation
  cannot do while the original is alive. Existing success/cleanup tests now
  prove the cleanup boundary receives only the secret ID and retain the exact
  two-save ordering
  (`src/account_state/switching/saved_delete/tests.rs:186-301`,
  `src/account_state/switching/saved_delete/tests.rs:502-582`,
  `src/account_state/switching/saved_delete/tests.rs:639-721`).
- Smallest behavior-preserving fix: no storage mode, account index, encrypted
  blob, keychain intent, cleanup retry, task, toast/status string, trading gate,
  persistence schema, or successful/failing branch outcome changes. Only raw
  credential ownership and its drop boundary change.
- Residual uncertainty: source inspection proves the ownership graph, but Rust
  type-checking and tests remain blocked by ALSA metadata. F-31 separately owns
  the already-installed-snapshot branch, where the product must choose which
  outcome is authoritative.

### F-31 — Installed profile/credential snapshot can disagree with restored runtime state

- Status: deferred in Turn 26 and expanded in Turn 27; current exceptional
  behavior is characterized because either safe correction changes failure
  feedback or adds a new durable rollback policy
- Severity: Medium; this does not dispatch an exchange mutation or expose a
  key, but saved-profile identity/binding and credential-cleanup outcome can
  depend on a later save versus restart
- Scope: the first durable save in saved-account deletion and OS-keychain
  wallet-address rebinding from both typed edit and connect routes; post-install
  filesystem sync/rollback-sidecar/permission errors; runtime rollback; pending
  keychain cleanup; failure feedback
- Preconditions/event ordering:
  1. Saved deletion stages a snapshot without the profile (plus OS cleanup
     intent), or address rebinding stages the new wallet metadata with the old
     agent-key binding removed.
  2. Config replacement installs that snapshot, then parent-directory sync,
     rollback-sidecar cleanup, or post-install permission hardening fails.
  3. `save_config` returns its explicit `config snapshot was installed` marker.
  4. These lifecycle routes currently treat every first-save `Err` as
     pre-install failure, restore the old runtime profile/binding and key owner,
     skip keychain cleanup, and report failure even though disk may already
     contain the deletion/rebind and cleanup state.
- Evidence: config replacement distinguishes before-install from after-install
  failure and exposes the marker (`src/config/files/persistence.rs:16-23`,
  `src/config/files/persistence.rs:252-310`). Secret-storage migration paths
  already treat that marker as committed, but saved deletion and both rebind
  first-save branches do not (`src/secret_storage/encrypted.rs:139-151`,
  `src/account_state/switching/saved_delete.rs:290-301`,
  `src/account_update/profile.rs:169-188`,
  `src/account_update/connection.rs:151-170`).
- Violated invariant: after a destructive credential-metadata operation has
  installed its snapshot, runtime identity and cleanup ownership must not claim
  the opposite outcome without a verified durable rollback.
- Risk: a user can see the old profile or wallet binding restored with failure
  feedback, then restart into the installed deletion/new binding and its pending
  cleanup. A later successful config save can instead re-persist the restored
  state. No active automation crosses these gates, but profile availability,
  wallet-to-key binding, and credential deletion become timing-dependent.
- Why existing checks did not cover it: save-failure tests returned ordinary
  errors only. Generic config persistence tests prove the marker, while secret-
  storage migration tests cover it in different state machines.
- Characterization coverage: marked post-install errors prove the original
  profile/binding and exact key allocations are restored in memory, keychain
  cleanup is not called, established failure feedback remains, and injected
  secret text is redacted. Saved deletion additionally restores its prior
  pending-intent state
  (`src/account_state/switching/saved_delete/tests.rs:583-638`,
  `src/account_update/profile.rs:578-619`,
  `src/account_update/connection.rs:1065-1110`).
- Why deferred: treating the marker as committed would retain the installed
  deletion/new binding, continue cleanup, and replace current failure behavior
  with committed-but-durability-warning semantics. Preserving the current
  failure result instead requires a second verified config replacement plus a
  policy for rollback failure. Both change observable exceptional behavior or
  durability timing and require explicit approval.
- Approval options: (1) make the installed snapshot authoritative and reuse the
  existing committed-config warning policy; or (2) attempt a durable rollback
  so the current failure result remains authoritative, with an explicit outcome
  for rollback failure.
- Protected behavior: Turns 26-27 do not reinterpret the marker, continue
  keychain cleanup, change profile/binding visibility, or change any
  toast/status copy on these branches.
- Residual uncertainty: the snapshot bytes are known to have been installed,
  but the marker can also report a failed durability sync; the approved policy
  must state whether runtime follows installed bytes immediately or tries to
  restore the prior snapshot.

### F-32 — Wallet-address rebinding multiplies and over-retains active keys

- Status: addressed in Turn 27; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium; no current diagnostic leak or wrong-account dispatch was
  found, but usable profile/draft keys were copied beyond the rollback and
  persistence owners that could legitimately need them
- Scope: typed wallet-address editing and connect-time active-profile rebinding;
  OS-keychain and encrypted-config persistence; ordinary failure rollback;
  profile/key-input ownership; saved-account snapshots; redacted diagnostics
- Preconditions/event ordering:
  1. A non-ghost active profile receives a normalized wallet address different
     from its stored binding after pending-trading and automation gates pass.
  2. Each prior route cloned `wallet_key_input`, cloned the complete active
     profile for rollback, cloned the entire account vector to stage the new
     binding, and cloned that vector again through `persisted_accounts_from`.
  3. The canonical profile/key input were cleared and credential persistence
     either succeeded or failed.
  4. Failure replaced canonical keys with clones; success retained old usable
     rollback clones until the whole update function returned.
- Evidence: parent commit
  `696f467b4487488e691afca8e224ed1f21fa4963` shows the duplicate clone graphs in
  `update_wallet_address_input_with_hooks` and `connect_wallet_with_hooks`.
  Both paths need one account snapshot because the persistence hook mutably
  borrows the terminal, but that snapshot can be built after moving the active
  key away and can end immediately after the hook returns.
- Violated invariant: an address rebind may retain the old profile and draft
  keys only in the exact rollback owner until credential persistence settles;
  unrelated account keys and duplicate active-key allocations are not rollback
  state.
- Risk: every rebind transiently multiplied unrelated saved keys and kept the
  old active signing key usable after successful removal from credential
  storage. `Zeroizing` eventually scrubbed the copies and all observed errors
  were redacted, so no disclosure or extra exchange mutation is claimed. The
  excess lifetime enlarges the consequence of a future callback/diagnostic bug.
- Why existing checks did not cover it: tests asserted resulting key values,
  encrypted/keychain snapshots, save ordering, rollback metadata, trading
  gates, and redacted status. They did not distinguish restoring original
  zeroizing allocations from replacing them with clones.
- Implemented fix: add one shared `ActiveProfileAddressRebindRollback` that
  moves the active profile key and `wallet_key_input`, changes only the active
  address, and verifies the exact profile identity on restore. Each route now
  builds one scoped persisted-account snapshot after the active key is empty.
  Ordinary failure moves the original buffers back; success explicitly scrubs
  the old keys, profile ID, and old address. Transient removal-ID/address-input
  copies are also scrubbed when no longer needed
  (`src/account_update/profile_rebinding.rs:9-69`,
  `src/account_update/profile.rs:150-234`,
  `src/account_update/connection.rs:129-216`).
- Regression coverage: encrypted-lock and OS-keychain failure paths for both UI
  entry points capture profile-key and key-input allocation addresses and
  require those exact owners after rollback. The old clone-based implementation
  cannot satisfy them while the originals are live. Existing success tests
  continue to prove the persisted binding is removed and canonical keys are
  empty (`src/account_update/profile.rs:500-531`,
  `src/account_update/profile.rs:620-675`,
  `src/account_update/connection.rs:959-1008`,
  `src/account_update/connection.rs:1111-1174`).
- Smallest behavior-preserving fix: storage-mode selection, normalization,
  encrypted/keychain payloads, first/rollback save ordering, key-removal policy,
  stream generation, pending/automation gates, account fetch tasks, statuses,
  toasts, persistence schema, and every success/failure outcome remain
  unchanged. Only transient ownership and zeroization timing change.
- Residual uncertainty: Rust type-checking and tests remain blocked by ALSA
  metadata. Normal explicit agent-key saving still constructs a broader staged
  account clone and is the next secret-copy owner to audit; F-31 separately owns
  installed-snapshot authority.

### F-33 — Explicit agent-key save builds cascading credential clones

- Status: addressed in Turn 28; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium; canonical signing authority and failure handling were safe,
  but every saved profile key and the draft key had more caller-owned live
  allocations than the synchronous storage entry boundary requires
- Scope: `save_active_account_credentials`; saved/ghost profile filtering;
  OS-keychain and encrypted-config persistence; draft versus committed signing
  authority; post-success config scheduling; secret buffer lifetime
- Preconditions/event ordering:
  1. A non-ghost active profile saves its current key input after the existing
     pending-request/automation gate permits the operation.
  2. The prior implementation cloned the complete runtime account vector,
     replaced that clone's active key with a draft clone, then cloned every
     persisted profile again through `persisted_accounts_from`.
  3. Storage synchronously accepted or rejected the second snapshot.
  4. Rejection dropped both staging vectors while canonical state stayed old;
     success cloned the draft a third time into canonical state before both
     vectors finally dropped.
- Evidence: parent commit
  `fe49d17a57683d8d83bf56479fbe42cb2a661ec5` shows the two full profile clones
  and post-persistence draft clone in `save_active_account_credentials`.
  Storage requires a full credential bundle, but it borrows it synchronously
  and returns a boolean commit result; the successful staged key can therefore
  become canonical by move.
- Violated invariant: the explicit-save caller may own one draft allocation and
  one storage-staging allocation. The prior committed key must remain the only
  signing authority until success; unrelated saved keys should enter at most
  one caller-owned storage snapshot. Backend-required zeroizing payload and
  serialization buffers are separate synchronous scopes.
- Risk: saving one key transiently multiplied every saved agent/integration key
  before backend persistence and kept redundant draft copies alive through it.
  All key copies were `Zeroizing`, no raw value entered diagnostics, and
  canonical failure authority was correct, so no disclosure or wrong-key
  signing is claimed. The excess lifetime magnified any future callback or
  diagnostic defect.
- Why existing checks did not cover it: tests proved draft edits do not commit,
  encrypted success updates the payload, locked failure keeps the old signing
  key, and active automation blocks changed drafts. They compared values, not
  allocation ownership or the key visible in canonical state during the
  persistence callback.
- Implemented fix: construct one caller-owned persisted-account snapshot
  directly with the draft key at the active runtime index while filtering ghosts
  and never cloning the old active agent key. Keep canonical state unchanged
  during the storage hook. On success, verify the originating profile identity
  and move the exact staged key into canonical state; on failure, leave both
  canonical/draft owners untouched. Explicitly drop the remaining snapshot
  before the existing config save request; backend payload construction is
  unchanged (`src/account_state/persistence.rs:38-75`,
  `src/account_update/profile.rs:335-389`).
- Regression coverage: the success test places a ghost before the active
  profile and another saved profile after it, proves the hook sees only the two
  persisted profiles and the old canonical key, then requires canonical state
  to own the exact staged allocation. The failure control proves the original
  committed and draft allocations both survive unchanged. Existing encrypted
  success/locked failure and automation-gate tests retain backend and trading
  behavior coverage (`src/account_update/profile.rs:768-894`).
- Smallest behavior-preserving fix: key equality/change detection, ghost
  handling, pending/Chase/TWAP gates, backend selection, profile order and
  payload values, storage status/warnings, canonical commit timing, captured
  signing context, debounced config persistence, schema, tasks, and every
  visible string remain unchanged. Only snapshot construction, successful
  buffer transfer, and destruction timing change.
- Residual uncertainty: Rust type-checking and tests remain blocked by ALSA
  metadata. Account switching/add-account flows deliberately create separate
  profile and key-input owners and need inclusion in the forthcoming diagnostic
  and remaining-copy inventory; no claim beyond this explicit-save transaction
  is made.

### F-34 — Nested exchange-response diagnostics bypass top-level redaction

- Status: addressed in Turn 29; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium privacy and diagnostic-boundary hardening
- Scope: `ExchangeResponse`, its public `ExchangeResponseInner` and
  `ExchangeResponseData` layers, the private deserialization wire helper, and
  the type-only successful summary path
- Preconditions/event ordering:
  1. A structured exchange response contains an OID, size, price, arbitrary
     error text, or an anomalous `response.type` carrying sensitive content.
  2. Parsing retains the exact values needed by effect/error classification.
  3. A diagnostic formats `response.response` or its `data` directly rather
     than the already-redacted top-level `ExchangeResponse`.
  4. Separately, a response with a type but no data reaches `summary()` and the
     prior code interpolates that untrusted type verbatim.
- Evidence: parent commit
  `a1a2f0ce202aadd7b33cd43165476f826f1a21fa` has derived `Debug` on both public
  nested models, so `Vec<Value>` statuses are emitted recursively; the private
  wire model also derives the same raw formatter. The top-level custom formatter
  prints `response_type` without the order-aware sanitizer, and the no-data
  summary does likewise. Current callers inspect the exact nested fields for
  classification and TWAP fill extraction, not their formatting.
- Violated invariant: every externally populated response-model layer must be
  independently safe to format. A top-level redactor is not sufficient when
  public nested values retain raw derived formatters; externally supplied type
  text must cross a value-neutral output gate before diagnostics or status
  state.
- Risk: diagnostic, panic, or future message formatting can reveal exact order
  correlation/details or hostile error text by selecting a nested value. A
  malformed but structured type-only response can also echo secret-shaped text
  into normal order status. No existing production log call or actual secret
  disclosure is claimed; this closes a concrete bypass before one is added.
- Why existing checks did not cover it: F-10 tests formatted only the top-level
  response. That formatter deliberately avoided traversing nested fields, so
  its passing redaction assertions could not exercise either nested derived
  implementation or the no-data summary branch.
- Implemented fix: replaced nested derived formatters with explicit
  implementations that expose an allowlisted response type, a data-presence
  marker, and status counts only; removed `Debug` from the private raw wire
  helper; applied the same allowlist in the top-level formatter; and emitted a
  value-neutral marker for an unrecognized type-only summary
  (`src/signing/model/exchange_response.rs:19-67`,
  `src/signing/model/exchange_response/analysis.rs:24-31`,
  `src/signing/model/exchange_response/analysis.rs:227-247`).
- Regression coverage: one adversarial test formats the top-level, inner, and
  data models containing a `u64` OID, fill size, price, key-shaped error text,
  bearer value, and hostile response type, then rejects every raw value while
  requiring useful type/count/redaction metadata. It separately proves the
  type-only summary removes the unrecognized value
  (`src/signing/tests/responses/status.rs:79-151`). Existing response tests
  preserve exact summaries, classifiers, OID/fill extraction, and safe
  top-level debug metadata.
- Smallest behavior-preserving fix: serde fields, exact stored values,
  `summary()` for ordinary protocol types, all error/effect classifiers,
  reconciliation decisions, tasks, wire requests, persistence, and UI flow are
  unchanged. Only diagnostic formatting and unrecognized anomalous type text
  differ; recognized protocol text is byte-for-byte unchanged.
- Residual uncertainty: Rust type-checking and tests remain blocked by ALSA
  metadata. The ongoing inventory found additional local planner/state derived
  formatters and direct order-intent message fields that require case-by-case
  redaction review. It also confirmed that `switch_account_task` clones a full
  credential-bearing profile before no-op/pending/automation gates and that
  add-account submission has a broader staged-key clone graph; neither owner
  path is claimed fixed by F-34.

### F-35 — Rejected account switches clone target credentials before gates

- Status: addressed in Turn 30; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium secret-lifetime hardening; switch authority and resulting
  account selection were correct, but raw keys acquired unnecessary owners
- Scope: every `switch_account_task` caller; invalid/same-profile requests;
  pending mutation, active Chase, and uncertain-TWAP gates; normal active-TWAP
  stopping; saved and ghost target profiles; empty-key handoff into deferred
  legacy loading; key-input synchronization; config/connect task scheduling
- Preconditions/event ordering:
  1. A caller supplies an existing target profile index.
  2. The prior entry point immediately clones the complete `AccountProfile`,
     including agent and legacy per-profile integration keys.
  3. A same-profile or financial gate may then reject the switch, leaving the
     needless zeroizing clone alive through feedback construction and return.
  4. If switching succeeds, the prior path clones the snapshot's agent key
     again into `wallet_key_input`; a ghost target can likewise be cloned before
     its canonical key is scrubbed.
- Evidence: parent commit
  `051b06c43524b73f3a051db1d1142d9056da3187` shows
  `self.accounts.get(index).cloned()` before all four return gates and a second
  `profile.agent_key.clone()` on success in `switch_account_task`. Picker,
  hotkey, add-account, ghost, saved-delete fallback, and profile-selection
  callers all converge on that method.
- Violated invariant: target credentials must not be captured until index and
  financial gates authorize a switch. A saved switch may own the canonical key
  plus exactly one intentional key-input copy; a rejected or ghost switch needs
  no target-key copy.
- Risk: repeatedly selecting the current profile or attempting a blocked switch
  transiently multiplied a signing key and unrelated per-profile secret. A
  successful switch kept an intermediate key allocation in addition to the
  canonical and key-input owners. All copies were zeroizing and no wrong-key
  dispatch or disclosure is claimed, but the extra lifetime increases the
  impact of a future callback, panic, or diagnostic defect.
- Why existing checks did not cover it: switch tests asserted active index,
  stopping, state clearing, toasts, and final key values. They did not observe
  when the target snapshot was created or whether the successful key-input
  value reused the one authorized captured allocation.
- Implemented fix: added a narrow, non-`Clone`, non-`Debug` switch target that
  contains only identity/address values plus one zeroizing key. Bounds and all
  blockers run before target construction; normal active-TWAP stopping and
  connected-state cleanup still run in their established order. Saved targets
  clone the canonical key once and move it into `wallet_key_input`; ghost
  targets construct an empty key and then retain the established canonical
  scrub (`src/account_state/switching.rs:15-35`,
  `src/account_state/switching.rs:324-399`).
- Regression coverage: an injected capture boundary must remain untouched for
  same-profile, pending-NUKE, active-Chase, and uncertain-TWAP rejection while
  both canonical target and active-input allocations remain identical. The
  successful control requires the canonical allocation to remain unchanged and
  the input to own the exact one captured allocation. A ghost control starts
  with a stray key, requires an empty target, and proves both canonical/input
  owners finish scrubbed (`src/account_state/switching/tests.rs:147-265`).
  Existing switch tests retain state-reset, stop, same-wallet, terminal-TWAP,
  legacy migration, and connect/disconnect coverage.
- Smallest behavior-preserving fix: caller routing, gate order and text, active
  TWAP stop semantics, account clearing, active index, journal identity,
  address/key input values, ghost status, deferred legacy migration, stream
  resets, config scheduling, connect/disconnect tasks, toasts, and all visible
  state remain unchanged. Only capture timing, snapshot breadth, allocation
  provenance, and zeroization timing differ.
- Residual uncertainty: Rust type-checking and tests remain blocked by ALSA
  metadata. Add-account ownership is separately addressed by F-36. Deferred
  legacy profile loading still clones a full profile and key and remains in the
  owner inventory; F-35 makes no claim about that path.

### F-36 — Add-account submission multiplies keys before credential commit

- Status: addressed in Turn 31; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium secret-lifetime and provisional-authority hardening; the
  synchronous update route prevented current concurrent signing, but the new
  profile became key-bearing before storage accepted it
- Scope: add-window validation and draft lifetime; keyed/watch-only profile
  construction; OS-keychain and encrypted-config persistence; immediate
  encrypted metadata snapshot; rollback; first-account input synchronization;
  ordinary and blocked switch-on-add; config scheduling and feedback
- Preconditions/event ordering:
  1. A valid keyed add-account submission retains the key in its window draft.
  2. The prior route copied a trimmed local key, cloned it into a new canonical
     profile, and pushed that signing-capable profile before calling storage.
  3. `persisted_accounts_snapshot` then cloned the profile and every other saved
     credential into a second caller-owned snapshot; encrypted persistence also
     synchronously saved canonical metadata from `self.accounts`.
  4. Failure popped the provisional profile but retained draft/local/snapshot
     owners through the callback and error path. Success cloned the canonical
     key again for first-account input synchronization while the local copy
     remained alive to function return.
- Evidence: parent commit
  `cd89a56c4ce57370f00c6be585830d55eb78bc55` shows the draft-to-local copy,
  local-to-canonical clone, canonical-to-persistence snapshot clone, and
  canonical-to-first-input clone in `submit_add_account`. The immediate
  encrypted save builds its config snapshot from canonical `self.accounts`, so
  omitting the provisional profile entirely would weaken the existing atomic
  metadata/secret commit (`src/secret_storage/encrypted.rs:97-157`,
  `src/config_persistence/save.rs:59-84`).
- Violated invariant: until credential storage accepts a keyed new account, the
  window draft must remain rollback authority, one caller-owned storage
  allocation may carry the proposed key, and canonical account state must not
  be able to sign. Once accepted, that staged allocation should become the
  canonical key by move; the first-account key input may reuse the independently
  verified draft allocation rather than create another copy.
- Risk: the prior synchronous storage callback could observe a usable canonical
  key before commit, and routine submission transiently multiplied signing
  material beyond the draft, storage, and final two-owner requirements. No
  current callback dispatches an order and all buffers were zeroizing, so no
  wrong mutation or disclosure is claimed. The broader owner graph increased
  the consequence of a future callback, panic, or diagnostic defect.
- Why existing checks did not cover it: tests asserted validation errors,
  encrypted payload contents, failure rollback, active selection, input values,
  switching, and toasts. They did not inspect canonical key authority during
  storage or require successful/failing allocations to retain their intended
  provenance.
- Implemented fix: validate address/key directly against the draft before
  allocating a staged key; count saved profiles without cloning them; move the
  one new key-bearing profile into the ghost-filtered storage snapshot; and
  place a matching keyless metadata shell in canonical state for the immediate
  encrypted save. Failure removes the shell, drops staging before feedback, and
  leaves the exact draft allocation. Success identity-checks and replaces the
  shell with the exact staged profile. First-account synchronization takes,
  normalizes, verifies, and moves the draft allocation; later switch-on-add
  drops the draft first and retains the Turn 30 post-gate capture
  (`src/account_update/add_window.rs:24-30`,
  `src/account_update/add_window.rs:88-289`).
- Regression coverage: watch-only submission proves storage is not called;
  actual encrypted success and locked failure observe a keyless canonical shell
  plus a keyed storage snapshot; injected keychain failure requires rollback to
  preserve the exact draft allocation. First-account success requires the
  storage-staged allocation to become canonical and the normalized draft
  allocation to become the input. An adversarial storage callback that changes
  provisional metadata and the draft after accepting the snapshot must retain
  the persisted origin profile and fall back to its canonical staged key, never
  retarget the active input. Successful and Chase-blocked switch-on-add controls
  preserve canonical allocation, active-key targeting, selection, and
  established feedback; a whitespace control preserves trimming
  (`src/account_update/add_window.rs:439-767`).
- Smallest behavior-preserving fix: validation order/copy, exact errors,
  normalization, default naming, profile ordering, watch-only behavior, storage
  backend and payload values, immediate encrypted metadata persistence,
  migration-block restoration, config debounce, first-account connect task,
  switch gates, active values, toasts, window closure, persistence schema, and
  every visible interaction remain unchanged. Only raw credential ownership,
  provisional signing authority, and zeroization timing change.
- Residual uncertainty: Rust type-checking and tests remain blocked by ALSA
  metadata. Backend-required payload/encryption/keychain buffers remain their
  existing synchronous zeroizing copies. Deferred runtime legacy-profile key
  loading is separately addressed by F-37; startup legacy migration and local
  planner/message plus other diagnostic paths remain to audit.

### F-37 — Deferred legacy loading retains source keys beyond migration

- Status: addressed in Turn 32; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium secret-lifetime hardening; the active profile and input were
  correlated correctly, but loaded signing/integration buffers gained
  unnecessary owners through migration and persistence
- Scope: deferred non-active legacy account migration after account switch;
  OS-keychain/ghost/existing-key gates; legacy profile field lookup; optional
  per-profile Hydromancer fallback; agent and Hydromancer runtime installation;
  conflict, read failure, bundle persistence/cleanup, and status behavior
- Preconditions/event ordering:
  1. A saved-profile switch has passed all financial gates, made the target
     active, and found its bundled agent key empty in OS-keychain mode.
  2. The prior loader cloned the complete canonical `AccountProfile`, including
     name, address, and any legacy per-profile integration key, solely to read
     two legacy keychain fields by secret ID.
  3. Once the reader populated the cloned agent/Hydromancer fields, the path
     cloned the loaded agent into a local owner, cloned it again into canonical
     state, and moved the local clone into the active input. Hydromancer
     migration likewise allocated global and input copies while the loaded
     source remained alive.
  4. Bundle persistence and legacy cleanup then ran synchronously while those
     redundant loaded owners remained in scope; a persistence failure retained
     the two intended runtime owners under the established behavior.
- Evidence: parent commit
  `47061c87f5b0b06acba2a614f6141e39e93c6407` shows
  `profile.clone()`, `legacy_profile.agent_key.clone()`, a second canonical
  clone, and Hydromancer `trim().to_string()` before persistence. The production
  keychain reader consults only `secret_id` and pre-seeded secret fields; its
  name/address fields are unused (`src/config/secrets/keychain.rs:69-80`,
  `src/config/secrets/keychain.rs:282-312`).
- Violated invariant: legacy lookup may own one loaded agent buffer and one
  loaded Hydromancer buffer; one pre-seeded fallback copy is additionally
  required when the canonical profile already carries a legacy Hydromancer
  value. The agent must become exactly the canonical profile owner plus one
  active-input copy; a newly accepted normalized Hydromancer key must become
  exactly the global runtime owner plus one input copy. Unrelated profile
  metadata and canonical keys must not be cloned into the lookup boundary.
- Risk: switching to each deferred legacy profile transiently multiplied a
  signing key and integration key across keychain bundle construction and
  cleanup. Every secret allocation was zeroizing and no wrong-account dispatch,
  persistence loss, or disclosure was found. The extra owners increased the
  impact of a future callback, panic, or diagnostic defect.
- Why existing checks did not cover it: migration tests compared values,
  payload contents, conflict state, generation, and redacted failures. They did
  not constrain what the legacy reader could observe, identify which loaded
  allocation became canonical, or characterize bundle-write failure after
  runtime installation.
- Implemented fix: build a lookup `AccountProfile` shell with the secret ID,
  empty name/address/agent fields, and only the existing per-profile
  Hydromancer fallback needed to preserve legacy precedence. Make both
  synchronous callbacks single-use. Move the loaded agent allocation into the
  canonical profile and create one active-input copy. Consume the loaded
  Hydromancer value, retain exact trim/equality/conflict rules, move the
  normalized allocation into global state, and create one input copy
  (`src/account_state/switching.rs:98-210`).
- Regression coverage: the primary migration test requires the loader shell to
  expose only identity/empty metadata and requires the loaded agent and
  Hydromancer allocations to become the exact canonical/global owners before
  persistence. Additional controls preserve an existing per-profile
  Hydromancer fallback and its canonical allocation, whitespace trimming,
  conflict-time global/input allocations, runtime keys after bundle-write
  failure, payload contents, generation/cache behavior, success status, and
  redacted read failure (`src/account_state/switching/tests.rs:635-954`).
- Smallest behavior-preserving fix: account switch order, lookup secret ID,
  keychain prompt/read behavior, OS-keychain/ghost/existing-key gates,
  agent-required migration, canonical/input values, Hydromancer precedence,
  trimming, conflict/error/success strings, generation/cache clearing, bundle
  payload/cleanup timing, persistence-failure runtime authority, connect task,
  and every visible behavior remain unchanged. Only lookup breadth, allocation
  provenance, callback capability, and zeroization timing change.
- Residual uncertainty: Rust type-checking and tests remain blocked by ALSA
  metadata. Startup active-legacy partial-bundle hydration is separately
  addressed by F-38, while storage-selection snapshots retain separate
  ownership needs; F-37 makes no claim about those paths. The remaining
  diagnostic inventory is also incomplete.

### F-38 — Partial-bundle cleanup can delete an unmerged legacy integration key

- Status: addressed in Turn 33; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium credential-durability and secret-lifetime hardening; no
  exchange mutation is affected, but a valid saved integration credential can
  be removed from its only durable location during startup migration
- Scope: OS-keychain startup with a valid partial bundle; plaintext
  normalization and unbound-wallet binding; active legacy profile fallback;
  agent/Hydromancer merge; bundle store success/failure; original-payload
  repair; cleanup scope; warning and save-block behavior
- Preconditions/event ordering:
  1. The current keychain bundle is valid but lacks the active profile's agent
     key and any global Hydromancer key, so startup invokes the combined legacy
     profile reader with no competing integration value.
  2. That reader can populate both the legacy agent and per-profile Hydromancer
     fields. The prior merge cloned and inserted only the agent key, ignoring
     the loaded Hydromancer field.
  3. A successful bundle write made the active profile eligible for cleanup.
     Profile cleanup deletes both legacy agent and Hydromancer entries, even
     though the latter was absent from the stored bundle/global runtime state.
  4. Separately, the prior helper cloned the complete profile and loaded agent;
     on bundle-write failure, a cloned agent remained as plaintext retry
     authority while the loaded source survived unnecessarily to scope end.
- Evidence: parent commit
  `fc17cf69f613c0f4038d8066bc4cf4a71c2f9104` shows
  `load_profile_secrets` populating both fields, the active merge reading only
  `legacy_profile.agent_key`, and cleanup clearing both profile fields for each
  payload profile (`src/config/secrets/keychain.rs:282-312`,
  `src/config/secrets/keychain.rs:556-571`). The original-payload failure repair
  intentionally preserves config plaintext values missing from the stored
  bundle (`src/config/files/storage/payload.rs:159-228`).
- Violated invariant: when the candidate bundle has no global Hydromancer key,
  profile-wide legacy cleanup may run only after the unambiguous loaded value is
  represented in the successfully stored bundle. Until storage succeeds, the
  exact loaded buffers must remain plaintext retry authority. The distinct
  pre-existing disagreement case remains current-behavior authority under F-39.
- Risk: a partial-bundle migration could silently delete the active profile's
  only legacy Hydromancer key and restart without that integration credential.
  This can disable authenticated data services and forces manual re-entry. The
  agent key remained correctly correlated and no order, signature, or private
  value was exposed.
- Why existing checks did not cover it: the partial-bundle success test made
  the loader return only an agent key. Other tests covered plaintext
  Hydromancer conflicts and full storage-selection migration, but none combined
  a partial bundle, the active profile's two-field legacy reader, profile-wide
  cleanup, and store failure.
- Implemented fix: rename the helper for its two-secret responsibility; retain
  the exact lookup ID in a metadata/keyless shell while keeping the trimmed ID
  for payload binding; move the loaded agent into plaintext config and clone it
  only into the attempted payload; normalize and merge the loaded Hydromancer
  key when the bundle global is empty, move it into plaintext config, and clone
  it into the payload. Both unambiguous loaded values therefore survive
  original-payload repair on store failure. An existing bundle global retains
  its established precedence (`src/config/files/storage.rs:178-228`,
  `src/config/files/storage.rs:396-479`).
- Regression coverage: a direct helper test requires an exact untrimmed lookup
  ID, empty metadata/secret shell, trimmed payload identity, both payload
  values, and exact loaded allocations in plaintext config. Integration tests
  require both values in stored and cleanup payloads on success; exact loaded
  plaintext owners plus no cleanup on store failure; established bundle-global
  precedence and cleanup when a different legacy value exists; and preserved
  whitespace normalization
  (`src/config/files/storage/tests.rs:859-1103`). Existing binding-mismatch,
  read-failure redaction, plaintext merge, and cleanup-warning tests remain.
- Smallest behavior-preserving fix: active-index selection, exact lookup ID,
  trimmed payload ID, wallet binding/mismatch gates, bundle and plaintext
  precedence, keychain read count, success/store-failure warning text,
  `secret_migration_save_blocked`, original-payload repair, cleanup scope and
  timing, stored schema, bundle-global disagreement precedence, warnings, and
  normal hydrated values remain unchanged. No visible string or policy was
  introduced.
- Residual uncertainty: Rust type-checking and tests remain blocked by ALSA
  metadata. Keychain payload serialization and final config hydration still
  require their necessary copies. F-39 owns the bundle-global disagreement;
  the storage-selection snapshot graph is separately addressed by F-40, while
  remaining local planner/state/message diagnostics are not yet complete.

### F-40 — Storage migration and cleanup over-own profile secrets

- Status: addressed in Turn 34; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium secret-lifetime and boundary-capability hardening; no known
  log or credential exposure occurred, but normal storage migration and config
  clearing retained signing/integration keys in owners that did not need them
- Scope: OS-keychain-to-encrypted payload construction; bundled and legacy
  fallback hydration; field-level legacy-read decisions; ghost filtering;
  wallet binding; conflict authority; encryption handoff; keychain cleanup,
  unlock retry, and asynchronous clear-config profile snapshots
- Preconditions/event ordering:
  1. Runtime account profiles contain loaded agent keys and possibly legacy
     per-profile Hydromancer fallbacks.
  2. Switching to encrypted storage previously created one full persisted-
     profile snapshot for legacy hydration, while `current_secret_payload`
     independently created another before copying the required secrets into
     the candidate payload.
  3. Each full profile clone, including name, wallet address, and canonical
     secret contents, crossed the legacy profile-reader callback. Newly loaded
     fallback allocations were then copied into the payload and retained in
     the snapshot until the whole migration returned.
  4. Separately, keychain cleanup cloned full persisted profiles even though
     all consumers use only `secret_id`; config clear moved that clone into an
     asynchronous task.
- Evidence: parent commit
  `e7335ef5dac019f52286c734967928bc37e26bfe` shows the duplicate snapshots,
  full-profile legacy reader, borrowed payload setters, and full cleanup
  snapshot (`src/secret_storage/encrypted.rs:13-34`,
  `src/secret_storage/selection.rs:31-54`,
  `src/secret_storage/selection.rs:348-414`). The production legacy reader
  decides field reads solely from whether a target is empty, while full
  keychain cleanup and clear-config counting consume only profile IDs
  (`src/config/secrets/keychain.rs:69-85`,
  `src/config/secrets/keychain.rs:577-603`,
  `src/config/clear.rs:272-310`, `src/config_persistence/clear.rs:80-84`).
- Violated invariant: a secret-migration boundary should hold only the secret
  owners needed to construct the durable candidate. Legacy readers need exact
  lookup identity and the same empty/non-empty field decisions, not canonical
  values or account metadata; cleanup tasks need profile IDs only. A newly
  loaded zeroizing fallback, or its required normalized buffer, should become
  the payload owner when no competing durable value exists.
- Risk: unnecessary signing-key, integration-key, wallet-address, and account-
  metadata copies lived across synchronous callbacks and, for config clear, an
  asynchronous task. A future debug/error/callback regression would therefore
  have more secret-bearing state available than its responsibility requires.
  This finding does not claim an existing disclosure or any exchange mutation.
- Why existing checks did not cover it: storage-selection tests asserted final
  decrypted values, binding/conflict results, and cleanup ordering, but did not
  constrain callback inputs, allocation provenance, or ordinary cleanup
  profile contents. The cleanup snapshot test checked only the synthetic
  deleted-profile shell.
- Implemented fix: construct current payload profiles from filtered references
  rather than a cloned vector; iterate the canonical profiles read-only for
  binding and legacy lookup; give legacy readers identity-only shells whose
  non-secret guards preserve the exact production field-read decisions; adopt
  loaded agent and loaded/normalized Hydromancer/HyperDash buffers through
  owned payload mutators; narrow one-use bundle/global readers to `FnOnce`; and
  make every keychain cleanup snapshot an identity-only profile list
  (`src/config/secrets/model.rs:237-305`,
  `src/config/secrets/model.rs:416-622`,
  `src/secret_storage/encrypted.rs:13-35`,
  `src/secret_storage/selection.rs:36-58`,
  `src/secret_storage/selection.rs:247-532`).
- Regression coverage: focused tests require canonical values to remain absent
  from global/profile reader inputs while non-secret guards preserve read
  suppression; exact already-normalized loaded allocations to become payload
  owners; legacy-profile whitespace normalization and ghost exclusion to remain
  unchanged; bundle agent precedence and the legacy read to remain unchanged;
  and every ordinary/pending cleanup profile to contain only its ID
  (`src/secret_storage/selection.rs:699-869`). Existing integration tests
  continue to cover save-before-cleanup, cleanup retry intent, deferred profile
  hydration, wallet mismatch blocking, legacy integration migration, conflict
  blocking, save rollback, and encrypted payload contents
  (`src/secret_storage/selection.rs:1174-1710`).
- Smallest behavior-preserving fix: payload schema, profile ordering and IDs,
  wallet normalization/binding, ghost filtering, all credential values,
  legacy keychain entry/read decisions and ordering, bundle precedence,
  Hydromancer trimming/conflict text, mismatch/read/save failure text, mode and
  save-block rollback, cleanup scope/count/timing/retry intent, and every user-
  visible or trading behavior remain unchanged. Only temporary ownership,
  callback data authority, and callback capability narrow.
- Residual uncertainty: Rust type-checking and tests remain blocked by ALSA
  metadata. Payload serialization/encryption and retained keychain-bundle
  comparison require short-lived copies. F-41 records the separately deferred
  bundle-versus-legacy authority question; local order planner/state/message
  diagnostics and other external-status paths remain to audit.

### F-42 — Leverage mutation parameters escape the redacted diagnostic graph

- Status: addressed in Turn 35; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium diagnostic-confidentiality hardening; no production log sink
  or known disclosure was found, but an account-linked financial mutation was
  representable through raw derived diagnostics
- Scope: leverage text input at the Elm message boundary; immutable submission
  snapshot; pending result-correlation context; `Message::Debug`; signed
  `UpdateLeverageAction`; action-enum diagnostics; unchanged serde/signing and
  update consumers
- Preconditions/event ordering:
  1. Editing the leverage control published its raw `String` through a `Message`
     variant whose enum derives `Debug`.
  2. Applying the change captured raw symbol and leverage text in a submission
     snapshot whose derived `Debug` flowed through the same message enum.
  3. Dispatch captured address, symbol/display, asset, dex, margin mode, and
     leverage in the result context. Its custom `Debug` hid only the address.
  4. The signed wire action independently derived raw `Debug` for asset, margin
     mode, and leverage; the action enum delegated to that representation.
- Evidence: parent commit
  `fed00b0ca77642dd54acdf1076e61c69b8ec12cc` shows the raw input/snapshot and
  partially redacted context (`src/message.rs:727-733`,
  `src/order_execution.rs:537-568`). `UpdateLeverageAction` was the one signed
  mutation left with derived field-level `Debug`; the prior signing-redaction
  commit `585b7467934c06f6ced1d9fa0cb74d6052f7aca0` deliberately tested place,
  cancel, cancel-by-CLOID, and modify but omitted leverage
  (`src/signing/actions/wire.rs:166-175`,
  `src/signing/tests/actions/constructors.rs:87-122`). A repository-wide
  production formatting/logging search found no current sink, so the confirmed
  issue is diagnostic capability rather than evidence of an emitted log.
- Violated invariant: runtime validation, stale-result correlation, visible
  confirmation, and signed serialization may require exact mutation values;
  generic message/action diagnostics do not. Every independently formattable
  layer in that path should redact account-linked identity and numeric mutation
  parameters rather than rely on the absence of a current logger.
- Risk: a panic/assertion diagnostic or future message/action instrumentation
  could expose the account address, traded symbol/dex/asset, leverage input, and
  chosen leverage. No private key, signature, or raw exchange response was
  present in these types, and no live exchange operation was run.
- Why existing checks did not cover it: the general order-input message test
  omitted leverage input, the address-redaction test did not constrain the
  other pending-context fields, and the signed-action redaction test's variant
  set omitted `UpdateLeverage` even though serialization tests covered it.
- Implemented fix: publish leverage input through the existing exact-value
  `RedactedOrderInput` and restore it only at the update handler; replace
  snapshot/context diagnostics with custom redacted representations; preserve
  optional-dex shape and margin mode while hiding every identity and numeric
  mutation parameter; and replace signed leverage-action `Debug` with an
  action-shape representation that redacts asset and leverage
  (`src/message.rs:228-250`, `src/message.rs:727-733`,
  `src/order_views/header.rs:140-169`, `src/order_update.rs:45-58`,
  `src/order_execution.rs:537-581`,
  `src/signing/actions/wire.rs:166-188`).
- Regression coverage: message tests require the input, submission, and result
  diagnostics to omit sentinel address/symbol/display/dex/asset/leverage values
  while the wrapper restores the exact input. A signing test requires the enum
  representation to omit sentinel asset/leverage values. The pre-existing
  JSON/msgpack constructor-equivalence test continues to characterize the exact
  leverage wire representation (`src/message.rs:1610-1693`,
  `src/signing/tests/actions/constructors.rs:73-87`,
  `src/signing/tests/actions/constructors.rs:124-137`).
- Smallest behavior-preserving fix: the view still emits the same text on each
  enabled input event, the update handler passes the exact original allocation
  into the unchanged sanitizer, and submission equality, parsing, constraints,
  account/key selection, task timing, result correlation, refresh policy,
  status copy, serde field names/order/values, and signed bytes are unchanged.
  No prompt, persistence, schema, trading rule, error string, or visible state
  changed.
- Residual uncertainty: Kerosene has not type-checked on this host. Source and
  call-site inspection plus existing wire-equivalence coverage establish the
  intended compatibility, but the new tests cannot execute until ALSA
  development metadata is available. Remaining local planner/state diagnostics
  and other external-status paths still require Track 9 audit.

### F-43 — Normalized execution-outcome diagnostics reveal exact order status

- Status: addressed in Turn 36; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium order-privacy and diagnostic-boundary hardening; no current
  production formatter/log sink or known disclosure was found
- Scope: shared `ExchangeResponse`/transport-error normalization into
  `ExecutionOutcome`; one-shot ticket/preset/quick/HUD/close, cancel, move,
  NUKE, and wallet-cluster result consumers; direct diagnostics; unchanged
  status copy and reconciliation decisions
- Preconditions/event ordering:
  1. A shared mutation result reaches `classify_execution_result` after the
     message/result boundary has retained its exact response.
  2. The normalized status deliberately contains an exact resting OID or filled
     size/average price/OID, or sanitized external text required by existing
     user feedback and cancellation/reconciliation decisions.
  3. The local `ExecutionOutcome` derived `Debug`, so formatting it directly in
     a panic/assertion or future instrumentation recursively emitted that status
     even though the source response-model diagnostics are independently safe.
- Evidence: parent commit
  `d3b24fd77aedec777172a3c9348f403a0952274c` shows the derived formatter on
  `ExecutionOutcome` while the classifier stores `response.summary()` or a
  redacted transport error (`src/order_update/results.rs:17-36`,
  `src/order_update/results.rs:293-346`). Response summary construction retains
  fill size, average price, and OID by design
  (`src/signing/model/exchange_response/analysis.rs:317-340`). Exhaustive
  call-site inspection covers the shared results module, quick/HUD/move, and
  wallet-cluster handlers; the type is not placed in `Message`, persisted, or
  currently formatted by production code.
- Violated invariant: exact normalized copy may remain available to explicit
  lifecycle consumers, but a generic diagnostic representation should expose
  only the classification/control metadata needed to diagnose state-machine
  routing. Safe response-model `Debug` is not sufficient once exact values have
  been re-materialized in a downstream local type.
- Risk: future logging or a failure diagnostic could expose the user's exact
  order OID, fill size, and fill price, or more external status detail than the
  diagnostic caller needs. This finding does not involve private keys,
  signatures, payload serialization, or a current emitted log.
- Why existing checks did not cover it: F-34 constrained every exchange-
  response model layer, while the earlier order-result error hardening verified
  that secret-shaped external text was sanitized before visible state. Existing
  classifier tests intentionally assert exact normalized status but never
  format the containing outcome, leaving the downstream formatter outside both
  protections.
- Implemented fix: replace derived `ExecutionOutcome::Debug` with an explicit
  representation containing the outcome kind, `is_error`, and
  `refresh_account`, while rendering `status` only as `<redacted>`. Retain the
  original `String` field, derives for clone/equality, constructors, and every
  consumer unchanged (`src/order_update/results.rs:17-44`).
- Regression coverage: construct a classified filled response with synthetic
  sentinel size, price, and OID; require the stored normalized status to retain
  all three exact values; require classification and flags to remain exact; and
  require the diagnostic representation to retain useful kind/control metadata
  while omitting every sentinel (`src/order_update/results/tests.rs:578-609`).
- Smallest behavior-preserving fix: response parsing/summary, error sanitation,
  classification order, outcome equality, cancellation text inspection, move
  status prefixing, form recovery, visible order/cluster status, unexpected-
  resting handling, refresh choice, status-task dispatch, task ordering, and
  every user-facing string remain unchanged. No message, config, persistence,
  signed request, timing, trading policy, or view code changed.
- Residual uncertainty: Kerosene has not type-checked on this host. Rustfmt and
  source/call-site inspection establish the intended narrow boundary, but the
  exact regression and existing classifier/result modules cannot execute until
  ALSA development metadata is available. TWAP event/planning diagnostics,
  `MoveOrderKey`, and other local/external-status types remain to audit.

### F-44 — TWAP planning and activity diagnostics expose exact event text

- Status: addressed in Turn 37; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium order-privacy and diagnostic-boundary hardening; no current
  production formatter/log sink or known disclosure was found
- Scope: planned-slice validation failures; initial/retry skip recording; every
  TWAP event producer; transport/account error sanitizers; live activity and
  order-status consumers; terminal advanced-history snapshot copy; direct
  `TwapPlannedSliceSkip`/`TwapEvent` diagnostics
- Preconditions/event ordering:
  1. Planning can produce an exact child size, allowed price range, or child
     notional message before a slice is sent; runtime result/status/fill/cancel
     paths can produce exact size, price, OID/CLOID, or sanitized external text.
  2. The planning failure moves its message into skip/retry recording; events
     retain their message for the live activity view and terminal history, and
     order status receives the same intentional visible copy.
  3. Both the pre-recording skip and runtime event derived `Debug`, so direct
     formatting independently emitted the exact message. The root `TwapOrder`
     formatter avoided this only by reporting an event count rather than the
     event vector.
- Evidence: parent commit
  `7e923e1332f330cea4b0b72f6f30d9d500d91dcc` shows both raw derives and exact
  planner messages (`src/order_execution/twap/execution/planning.rs:14-72`,
  `src/twap_state/model.rs:43-65`). Planning skip recording moves the same
  string into event and order status; `push_event` stores it; the live view and
  history snapshot read or clone it directly
  (`src/order_execution/twap/execution/skip.rs:11-25`,
  `src/order_execution/twap/execution/lifecycle.rs:13-46`,
  `src/twap_state/order.rs:150-158`,
  `src/order_views/twap_details/sections/activity.rs:72-119`,
  `src/advanced_order_history/snapshots.rs:61-118`). Result transport and
  account-refresh errors cross `redact_sensitive_response_text` before event
  construction; structured exchange/order-status summaries have their own
  redacted model boundaries. Repository search found no production direct
  formatter for either affected type.
- Violated invariant: exact activity copy is appropriate only at explicit
  state/UI/history consumers. Generic diagnostics should retain event kind,
  timing, and error state without exposing account-linked order quantities,
  prices, identifiers, or external status detail.
- Risk: a test/panic diagnostic or future instrumentation could expose exact
  TWAP strategy bounds, child size/notional, fill price, or child identifiers.
  No raw agent key/signature is present in either type, external secret-shaped
  errors are already sanitized, and no current emitted log is claimed.
- Why existing checks did not cover it: the earlier TWAP runtime-debug hardening
  (`e2a29e93660556427fbd395cd892cc389f763ef0`) covered the order root, pending
  slices/operations, and child rows but omitted `TwapEvent`; its root test never
  added or directly formatted an event. Planner tests asserted exact skip copy
  and relied on derived `Debug` for successful-result `unwrap`, but never
  constrained that formatter.
- Implemented fix: replace both derives with explicit `Debug` implementations.
  Planned skips expose kind and `is_error`; events expose timestamp, kind, and
  `is_error`; both render `message` only as `<redacted>`. Keep every field,
  constructor, clone, and direct consumer unchanged
  (`src/order_execution/twap/execution/planning.rs:15-29`,
  `src/twap_state/model.rs:43-76`).
- Regression coverage: the existing planned-range test still requires the
  exact established skip string, then requires its formatter to retain type/
  kind metadata and omit that string. A runtime-event test likewise requires
  exact stored synthetic fill activity, useful timing/kind/error metadata, and
  no activity text in `Debug` (`src/order_execution/twap/execution/planning/tests.rs:26-44`,
  `src/twap_state/tests.rs:170-189`). Existing history-snapshot coverage still
  requires exact recent activity and summary copy.
- Smallest behavior-preserving fix: planning values and decisions, skip kind/
  error flags, slice-attempt accounting, retry scheduling, event timestamp and
  retention, status/toast copy, live activity rows, child summaries, terminal
  snapshot/log/summary values, serde schema/bytes, archive timing, task flow,
  exchange mutations, and every visible string remain unchanged. No view,
  persistence, message, or trading-policy code changed.
- Residual uncertainty: Kerosene has not type-checked on this host. Rustfmt and
  source/call-site tracing establish the intended boundary, but exact and nearby
  TWAP/history tests cannot execute until ALSA development metadata is
  available. Persisted `AdvancedOrderHistoryLog`, child, and entry types still
  derive raw nested `Debug`; that broader Chase/TWAP history diagnostic graph
  requires a cohesive follow-up rather than partial log-only redaction.

### F-45 — Persisted advanced-history diagnostics expose complete order records

- Status: addressed in Turn 38; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium account/order privacy and diagnostic-boundary hardening; no
  current production formatter/log sink or known disclosure was found
- Scope: terminal Chase/TWAP snapshot construction; `AdvancedOrderHistoryEntry`,
  nested child/log records, and `ChaseHistoryFillMetrics`; config serialization,
  defaults, load/save snapshots, archive upsert/pruning, list/details views, and
  direct diagnostics
- Preconditions/event ordering:
  1. A Chase or TWAP terminalizes and creates a bounded history snapshot, or a
     prior snapshot is deserialized from config.
  2. The entry intentionally retains an ID embedding account/time/source,
     account and symbol labels, side/strategy flags, exact target/fill/remaining
     size, prices, notional, fees/PnL, timing, status/summary, logs, and child
     OID/CLOID/status/exchange detail for existing history UI and persistence.
  3. Entry, child, and log all derived `Debug`, so formatting any layer emitted
     its complete fields; entry formatting recursively traversed both vectors.
     The temporary Chase fill-metrics helper independently derived its exact
     filled size, notional, fee, and closed PnL before snapshot creation.
- Evidence: parent commit
  `1354c94b2a2fe6a77a3afd0ce7b1969128ae5042` shows raw derives on all four
  types (`src/advanced_order_history/model.rs:27-137`). Chase/TWAP constructors
  populate every value directly; config snapshots clone entries into the same
  serde schema; history list/details views read fields directly
  (`src/advanced_order_history/snapshots.rs:19-243`,
  `src/config_persistence/save/snapshot.rs:176`,
  `src/config/schema.rs:427-432`,
  `src/order_views/advanced/rows.rs:132-171`,
  `src/order_views/advanced_history_details/sections.rs:17-215`). Repository
  search found no current production direct formatter, and `KeroseneConfig`
  itself does not implement `Debug`, so the confirmed issue is independently
  reachable model capability rather than an emitted config log.
- Violated invariant: exact persisted history may be read only by explicit
  serde, view, and lifecycle consumers. Generic diagnostics should retain enough
  structural metadata to identify the history kind/source and nested shape,
  without disclosing account-linked historical financial records or trusting
  deserialized free-form strings as safe diagnostic labels.
- Risk: a panic/assertion or future history/config instrumentation could expose
  account identity, traded symbol, exact historical sizes/prices/fees/PnL,
  child OIDs/CLOIDs, strategy timing/counts, and activity/exchange copy. No
  private key or signature is stored in history, and no current emitted log is
  claimed.
- Why existing checks did not cover it: snapshot tests verify calculations and
  exact history copy; config tests verify round-trip/default compatibility; view
  tests verify formatting. None directly formatted the models. F-44 stopped at
  the live `TwapEvent` boundary and deliberately preserved the exact terminal
  history copy, revealing this complete downstream graph rather than masking it
  with log-only redaction.
- Implemented fix: replace entry/log/child/metrics derives with explicit
  formatters. Entry retains typed kind, local source ID, side/strategy booleans,
  optional-value presence, and nested counts; child retains index and optional-
  identifier/price presence; log retains only error state; every exact string,
  identifier, financial value, timing and strategy-count value, status,
  summary, and metric is represented by a redaction marker.
  `AdvancedOrderHistoryKind` remains a safe typed formatter
  (`src/advanced_order_history/model.rs:8-217`).
- Regression coverage: construct a fully populated synthetic entry with one log
  and child plus independent Chase metrics; require exact nested JSON values;
  format all four layers; require allowlisted structural metadata and reject
  every sentinel string, identifier, financial value, strategy count, and
  timestamp; then require value-equivalent JSON after formatting
  (`src/advanced_order_history/tests/diagnostics.rs:1-150`). Existing snapshot,
  config round-trip/default, pruning, and history-view tests remain controls.
- Smallest behavior-preserving fix: all struct fields/order/visibility, serde
  derives/defaults/JSON representation, constructors and calculations, IDs,
  archive/upsert/pruning limits, config save/load, clone/copy/equality behavior,
  list/detail rendering, window routing, visible strings, and persisted values
  remain unchanged. No schema, migration, task, order, timing, or trading-policy
  code changed.
- Residual uncertainty: Kerosene has not type-checked on this host. Rustfmt and
  complete producer/consumer/serde/view tracing establish the narrow boundary,
  but the new regression and existing history/config/view tests cannot execute
  until ALSA development metadata is available. `MoveOrderKey`, TWAP form/book
  helpers, and remaining local/external-status diagnostics still require Track
  9 audit.

### F-46 — Cancel and move correlation diagnostics expose order identity

- Status: addressed in Turn 39; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium account/order privacy and diagnostic-boundary hardening; no
  current production formatter/log sink or known disclosure was found
- Scope: `MoveOrderKey`; pending cancel and move status requests; all map, drag,
  chart-overlay, result/status matching, refresh-reconciliation, disconnect,
  config-clear, and test consumers; deliberately non-formattable captured-key
  move context
- Preconditions/event ordering:
  1. A chart move captures an exact symbol/OID key for active drag and pending-
     context map ownership, or cancel/move dispatch creates a pending status
     request with request sequence, account, OID, symbol, and move target price.
  2. Exact key equality/hash lookups control overlay state and claim only the
     matching move context. Exact pending-request predicates reject stale
     request, account, OID, or symbol combinations before applying results.
  3. `MoveOrderKey` derived `Debug` over its symbol/OID, while the two explicit
     pending-request formatters redacted account/OID (and move price) but still
     emitted symbol. Formatting these independently reachable runtime records
     therefore exposed the remaining order identity.
- Evidence: parent commit
  `4b6fbf5c42fda63648e2ed50b40b93913367fea4` shows the raw key derive and
  symbol-bearing status formatters. Repository-wide use tracing found the key
  only in runtime map/drag/overlay identity and cleanup paths, and the status
  owners only in cancel/move result, status, and account-snapshot
  reconciliation. Repository-wide formatter/log searches found test formatters
  but no current production sink. `PendingMoveOrderContext`, which owns the
  captured signing key, intentionally has no `Debug` implementation
  (`src/order_execution.rs:1009-1062`,
  `src/order_execution/quick_order/move_order.rs:1-89`,
  `src/order_update/move_order.rs:1-142`,
  `src/chart_state/overlays.rs:70-112`,
  `src/order_update/results.rs:102-290`).
- Violated invariant: exact account-linked order identity belongs only in
  deliberate correlation, reconciliation, and view behavior. A generic
  diagnostic should identify the correlation record and safe control phase
  without disclosing symbol, OID, account, or target price.
- Risk: a future assertion, panic, map diagnostic, or lifecycle instrumentation
  could disclose which instrument and exchange order a local account was
  cancelling or modifying. No private key/signature exposure and no current
  emitted log are claimed.
- Why existing checks did not cover it: the pending-request diagnostic tests
  required account/OID and price redaction but explicitly treated the symbol as
  safe output. No test formatted `MoveOrderKey`; correlation tests exercised
  map and matcher behavior without specifying a diagnostic policy.
- Implemented fix: replace only `MoveOrderKey`'s derived formatter with an
  explicit type-shaped formatter that redacts both fields, and replace only the
  symbol values in the two pending-request formatters with redaction markers.
  Request sequence and cancel phase remain visible. The key retains its exact
  `Clone`, `PartialEq`, `Eq`, and `Hash` derives; the status fields,
  constructors, accessors, and match predicates are unchanged.
- Regression coverage: exact symbol/OID key values remain directly accessible
  and equal keys still resolve through a `HashSet`, while alternate symbol or
  OID values remain distinct; formatting rejects both sentinels. Cancel and
  move requests must still accept every exact correlation value and reject
  stale request, account, OID, and symbol variants before their diagnostics
  retain request/phase metadata and reject account, symbol, OID, and price
  sentinels (`src/order_execution.rs:725-754`,
  `src/order_update/results/tests.rs:195-263`).
- Smallest behavior-preserving fix: no field, constructor, accessor, match
  predicate, equality/hash behavior, map operation, drag/overlay behavior,
  state transition, cleanup, result ordering, task, message, view, persistence,
  signed value, status copy, or trading policy changed. Only direct `Debug`
  output and its tests differ.
- Residual uncertainty: Kerosene has not type-checked on this host. Rustfmt,
  source/call-site tracing, and the narrow diff establish the intended
  boundary, but the exact correlation regressions and nearby cancel/move suites
  cannot execute until ALSA development metadata is available. TWAP form/book
  helpers and remaining external-status diagnostics still require Track 9
  audit.

### F-47 — Transient TWAP helper diagnostics expose strategy and fill state

- Status: addressed in Turn 40; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium account/order privacy and diagnostic-boundary hardening; no
  current production formatter/log sink or known disclosure was found
- Scope: editable `TwapOrderForm`; parsed `TwapStartSchedule`; TWAP-owned
  `TwapBookSnapshot`; direct-response `ResponseFillSummary`; authoritative-
  account `FillSummary`; their start, planning, result, reconciliation, parent-
  formatter, view, persistence, and test consumers
- Preconditions/event ordering:
  1. The order ticket retains free-form duration, slice-count, and price-range
     inputs; startup clones them into an already-redacted immutable snapshot and
     parses exact cadence values for schedule validation and order creation.
  2. A keyed market subscription installs an exact book plus freshness time;
     slice planning clones and reads both. A direct exchange result extracts
     OID/filled size/average price, while account reconciliation independently
     deduplicates matching fills and derives size/average price/fee.
  3. All five helpers derived `Debug`. The form, schedule, and two summaries
     emitted their exact fields. The book snapshot emitted its exact freshness
     timestamp and nested book shape; its price/size levels were protected only
     because `OrderBook` currently has a separate redacted formatter.
- Evidence: parent commit
  `05402699c29f45ac8fe69bfcf57f97ca8f43377d` shows the five raw derives
  (`src/twap_state/model.rs:18-25`, `src/twap_state/model.rs:121-125`,
  `src/twap_state/fills.rs:11-16`, `src/twap_state/fills.rs:55-60`,
  `src/order_execution/twap/start/validation.rs:12-16`). Form values flow only
  through update/start/view/reset owners; schedule values through start
  validation; the snapshot through keyed book gates and slice planning;
  response metrics through slice-result settlement; and authoritative metrics
  through OID/coin/side/deduplicated fill reconciliation
  (`src/order_execution/advanced.rs:63-78`,
  `src/order_execution/twap/start.rs:105-281`,
  `src/order_execution/twap.rs:88-131`,
  `src/order_execution/twap/execution.rs:53-190`,
  `src/order_execution/twap/slice_result.rs:67-163`,
  `src/twap_state/order/reconciliation.rs:44-83`). Repository-wide formatter
  searches found tests but no production sink. `TwapOrderStartSnapshot` already
  redacts the entire form, `TwapOrder` reports only book presence and redacts
  financial fields, and `TwapOrderInit` has no formatter because it owns a
  captured key.
- Violated invariant: exact strategy inputs/cadence, TWAP-owned market timing,
  exchange order identity, fill price/size, and fee values belong only in
  deliberate validation, execution, reconciliation, history, and view paths.
  Independently reachable transient helpers must not create a raw diagnostic
  bypass around their already-redacted parents.
- Risk: a future assertion, panic, or instrumentation at a parser, planner,
  response, or reconciliation boundary could reveal a trader's configured TWAP
  strategy and exact child-order/fill details. The book levels were already
  redacted and no signing key, signature, or current emitted log is implicated.
- Why existing checks did not cover it: start-snapshot and root-order tests stop
  at their safe parent formatters; form tests cover stale equality/defaults;
  schedule tests cover parsing/capacity; book tests cover source/freshness
  gates; fill tests cover parsing, deduplication, identity, fee conversion, and
  totals. None directly formatted the five helpers.
- Implemented fix: replace only the five derived formatters with explicit non-
  recursive formatters. The form redacts four free-form inputs and retains the
  established randomization boolean; the book and cadence redact every value;
  the two fill summaries redact financial values and OID while preserving
  optional `Some`/`None` shape. Existing `Clone`, `Copy`, `Default`,
  `PartialEq`, and `Eq` derives remain where they existed.
- Regression coverage: exact form equality and every stored form/book field are
  required before and after formatting; form and snapshot diagnostics require
  type/field markers and reject all sentinels. Both fill-summary layers retain
  exact OID/size/price/fee values, preserve response optionality, and reject all
  sentinels. The parsed schedule retains its exact duration/slice count while
  formatting replaces both (`src/twap_state/tests.rs:193-359`,
  `src/order_execution/twap/start/tests.rs:209-233`). Existing form equality,
  schedule, book gating/planning, response parsing, account reconciliation, and
  TWAP start/result suites remain behavioral controls.
- Smallest behavior-preserving fix: no field, visibility, constructor, default,
  equality, parser, calculation, book value/timestamp, fill identity/dedup key,
  fee conversion, status, schedule, task, message, view, persistence, history,
  signed request, visible string, or trading policy changed. Only direct
  `Debug` output and new tests differ.
- Residual uncertainty: Kerosene has not type-checked on this host. Rustfmt,
  complete producer/consumer tracing, and the narrow diff establish the
  intended boundary, but the exact regressions and nearby TWAP suites cannot
  execute until ALSA development metadata is available. Remaining external-
  status/message diagnostics and other Track 9 boundaries still require audit.

### F-48 — Advanced-order messages expose symbols and error results

- Status: addressed in Turn 41; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium account/order privacy and diagnostic-boundary hardening; the
  debug-only unrouted-order-message invariant panic is a concrete formatter,
  but no known production misroute or emitted disclosure was found
- Scope: complete Chase/TWAP market/result subset of the Elm message graph and
  its nested diagnostic policies; five symbol-bearing market/adoption variants;
  nine initial-book, place/modify/cancel, and CLOID/OID status-result variants;
  every view/subscription/task producer; root routing; immediate order-update
  consumption; final-exit tests
- Preconditions/event ordering:
  1. Keyed Chase/TWAP book streams publish exact automation ID, symbol,
     sigfigs, provider generation, book, or lag count. Resting-order adoption
     publishes its exact symbol/OID.
  2. Initial-book fetch, TWAP placement/cancel/status, and Chase place/modify/
     cancel/status tasks publish the original `Result<T, String>` with immutable
     automation, attempt, and order identifiers.
  3. `Message` derives `Debug`. Raw symbol fields were emitted directly. An
     `Ok` result delegated to already-redacted `OrderBook`, `ExchangeResponse`,
     or `OrderStatusResult`, but `Err(String)` was emitted verbatim. Any future
     nested formatter regression also bypassed the parent message boundary.
- Evidence: parent commit
  `cb361da193d340bd2443e448a491fc0e1c60ea33` shows raw symbols and boxed raw
  results on the advanced-order variants (`src/message.rs:1222-1311`). All
  producers and consumers were traced through keyed market subscription maps,
  initial book fetch, canonical place/modify/cancel tasks, status tasks, result-
  triggered follow-up cancellation, routing, and `update_order`
  (`src/subscription_state/market/chase.rs:82-118`,
  `src/subscription_state/market/twap.rs:68-104`,
  `src/order_execution/chase.rs:296-308`,
  `src/order_execution/chase/lifecycle/place.rs:388-398`,
  `src/order_execution/chase/lifecycle/reprice.rs:339-350`,
  `src/order_execution/chase/lifecycle/stop.rs:77-90`,
  `src/order_execution/twap/execution.rs:307-318`,
  `src/order_execution/twap/helpers/cancellation.rs:45-88`,
  `src/order_execution/twap/status/tasks.rs:59-100`,
  `src/order_update.rs:159-354`). The order update catch-all formats an
  unexpectedly routed message in debug/test builds
  (`src/order_update.rs:470-477`). Existing nested response/status/book,
  OID/CLOID, input, and start-snapshot formatters are independently redacted.
- Violated invariant: exact automation symbol and result/error payloads belong
  only in keyed stream validation and lifecycle result handlers. The generic
  Elm-message diagnostic must retain useful route/correlation control metadata
  without exposing them or trusting every nested formatter and sanitized-but-
  still-sensitive error string indefinitely.
- Risk: a debug assertion, test failure, framework instrumentation, or future
  message logging could reveal the instrument under automation and exact local
  or external failure copy. Successful response internals were already
  redacted; no private key, signature, or live disclosure is claimed.
- Why existing checks did not cover it: input and OID/CLOID tests asserted only
  their dedicated wrappers. Response, status, and book tests stopped at nested
  model formatters, while advanced lifecycle tests called handlers directly.
  Message tests used ordinary error strings without asserting their absence and
  explicitly left Chase resting symbols visible.
- Implemented fix: add `RedactedOrderSymbol`, whose consuming accessor restores
  the exact `String`, and generic `RedactedOrderMessageResult<T>`, which retains
  the boxed exact result but formats only `Ok(<redacted>)` or
  `Err(<redacted>)`. Convert all 14 affected variant fields at every publication
  site and consume at the single order-update boundary. Numeric IDs, attempts,
  sigfigs, source contexts, and existing OID/CLOID wrappers remain unchanged.
- Regression coverage: format every affected variant with symbol/error
  sentinels and require type/control metadata plus redaction without either
  value. Independently format success/error wrapper states, recover the exact
  symbol, exact error, and exact successful book price, and retain existing ID,
  routing, final-exit, Chase, and TWAP controls (`src/message.rs:1710-1853`,
  `src/app_update/tests.rs:110-194`).
- Smallest behavior-preserving fix: the wrapper keeps the existing heap box and
  exact `Result<T, String>` allocation, moves values once at publication, and
  consumes them once before the same handler call. No handler signature,
  stream/subscription identity, message route, task order, result classifier,
  retry/reconciliation decision, status string, view, persistence, timing,
  signed mutation, or trading policy changed.
- Residual uncertainty: Kerosene has not type-checked on this host. Rustfmt,
  complete call-site tracing, and the mechanical publication/consumption diff
  establish the intended boundary, but exact and nearby suites cannot execute
  until ALSA development metadata is available. Non-advanced order-result,
  symbol/value, and history-navigation message diagnostics remain to audit.

### F-49 — Remaining mutation-result messages expose error payloads

- Status: addressed in Turn 42; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium account/order privacy and diagnostic-boundary hardening; the
  debug-only unrouted-order-message invariant panic remains a concrete
  formatter, but no known production misroute or emitted disclosure was found
- Scope: all remaining signed-mutation and mutation-status message results:
  leverage; wallet-cluster direct/status; shared one-shot direct/status; cancel
  direct/status; close; NUKE direct/status; quick; HUD; and move direct/status;
  every publisher, route, immediate consumer, and nearby lifecycle test
- Preconditions/event ordering:
  1. Canonical leverage, place, cancel, modify, close/NUKE, quick/HUD, and
     cluster tasks return the exact exchange `Result`; uncertain direct outcomes
     schedule exact `orderStatus` tasks whose results carry immutable request,
     account/member, context, indicator, recovery, execution, and order IDs.
  2. Publishers boxed those results directly in 14 `Message` variant fields.
     Order and wallet-cluster update routes moved the box contents once into the
     existing handlers, which classify, reconcile, refresh, settle, or retain
     uncertainty.
  3. Derived `Message::Debug` delegated successful values to already-redacted
     response/status models but emitted every `Err(String)` verbatim, creating
     the same parent-boundary bypass closed for Chase/TWAP in F-48.
- Evidence: parent commit
  `b9878f438089315d233edd9f6253113695917920` shows all 14 raw boxed result
  fields (`src/message.rs:790-793`, `src/message.rs:988-999`,
  `src/message.rs:1203-1263`, `src/message.rs:1465-1503`). Producers were traced
  through leverage, shared submit, cancel, close, NUKE, quick/HUD, move, result-
  status follow-ups, and cluster fan-out/status repair
  (`src/order_update/leverage.rs:155-170`,
  `src/order_execution/submit.rs:239-249`,
  `src/order_execution/position_actions/cancel.rs:89-104`,
  `src/order_execution/position_actions/close.rs:198-208`,
  `src/order_execution/position_actions/nuke.rs:168-184`,
  `src/order_execution/quick_order/submit.rs:404-417`,
  `src/order_update/hud.rs:207-219`,
  `src/order_execution/quick_order/move_order.rs:190-204`,
  `src/order_update/results.rs:360-376`,
  `src/order_update/results.rs:624-638`,
  `src/order_update/results.rs:832-846`,
  `src/wallet_cluster_update.rs:1055-1080`,
  `src/wallet_cluster_update.rs:1136-1153`). Both update boundaries dereferenced
  each box immediately (`src/order_update.rs:57-479`,
  `src/wallet_cluster_update.rs:143-164`).
- Violated invariant: exact mutation/status error payloads belong only in
  deliberate lifecycle classification and visible sanitized status handling.
  Generic Elm diagnostics should expose result shape, route, and correlation
  metadata without disclosing the payload or relying indefinitely on every
  upstream sanitizer and nested formatter.
- Risk: a debug assertion, test failure, framework instrumentation, or future
  message logging could expose exchange/local error copy tied to a one-shot,
  leverage update, emergency close, move, or cluster member leg. Successful
  response/status internals were already redacted; no private key, signature,
  or known live disclosure is implicated.
- Why existing checks did not cover it: response/status models and lifecycle
  tests validate nested redaction and exact handler behavior, while message
  tests focused on accounts, contexts, OID/CLOID, or leverage parameters. Their
  synthetic errors remained visible but were never asserted absent. F-48 was
  intentionally limited to the advanced-order subset.
- Implemented fix: reuse `RedactedOrderMessageResult<T>` for all 14 fields.
  Every publisher moves its original result into the same-sized boxed wrapper;
  `update_order` or `update_wallet_cluster` immediately calls `into_result()`
  before the source-identical handler. No new wrapper or policy was added.
- Regression coverage: construct every affected variant with the same unique
  error sentinel and require derived message diagnostics to contain redaction
  while rejecting the payload (`src/message.rs:1710-1819`). The exact wrapper
  success/error recovery test from F-48 remains the independent value-preserving
  control (`src/message.rs:1934-1964`), with existing message, result, cluster,
  final-exit, leverage, quick/HUD, close/NUKE, and move tests as lifecycle
  controls.
- Smallest behavior-preserving fix: fields retain one heap box and the exact
  `Result<T, String>`; each producer/consumer still performs one move at the
  same publication/handling points. No context, identifier, route, handler,
  classification, retry/status task, refresh, indicator, recovery owner,
  cluster settlement, status copy, view, schema, timing, signed request, or
  trading policy changed.
- Residual uncertainty: Kerosene has not type-checked on this host. Rustfmt,
  repository-wide raw-field absence checks, call-site tracing, and the
  mechanical diff establish the intended boundary, but exact and nearby suites
  cannot execute until ALSA development metadata is available. Raw order
  symbols, prices/fractions, and history-navigation IDs remain to audit.

### F-50 — Order symbols and history navigation identity bypass message redaction

- Status: addressed in Turn 43; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium account/order privacy and diagnostic-boundary hardening; a
  persisted history identity contains an account address, while exact position
  and order symbols reveal activity context, but no known production message
  misroute or emitted disclosure was found
- Scope: 11 direct symbol fields across outcome-sell prefill, wallet-cluster
  close, cancel intent/status, position close-menu/hide/close, and move drag/
  intent/direct-result/status-result messages; advanced-order history-window
  navigation identity; every view/task publisher, route, immediate consumer,
  and nearby lifecycle/routing test
- Preconditions/event ordering:
  1. Account/order/cluster views and chart interaction publish exact symbols;
     cancel/move reconciliation tasks republish the immutable origin symbol in
     status results; the history row publishes the selected persisted entry ID.
  2. The entry ID is deliberately stable and persisted as
     `kind:account_address:started_at_ms:source_id` for Chase and TWAP
     (`src/advanced_order_history/snapshots.rs:115-123`,
     `src/advanced_order_history/snapshots.rs:199-207`).
  3. All 12 fields were ordinary `String` values, so derived `Message::Debug`
     emitted them verbatim before the order/account/cluster update arm passed
     them unchanged to the existing handler.
- Evidence: parent commit
  `078fc1ab388288b8a82261dac6218cff1316a23f` shows the raw fields at
  `src/message.rs:784`, `src/message.rs:982-986`,
  `src/message.rs:1209-1241`, `src/message.rs:1321-1323`, and
  `src/message.rs:1480-1502`. Publishers span balance/order/position rows,
  cluster-close controls, chart cancel/drag interaction, cancel/move status
  tasks, and the history info button (`src/account_views/balances/row.rs:35-48`,
  `src/account_views/orders/row.rs:37-45`,
  `src/account_views/positions/table/close_cell/button.rs:8-58`,
  `src/account_views/positions/table/close_cell/menu.rs:42-87`,
  `src/wallet_cluster_views.rs:708-724`,
  `src/chart/interaction/press.rs:127-150`,
  `src/chart/interaction/drag.rs:204-219`,
  `src/order_update/results.rs:362-377`,
  `src/order_update/move_order.rs:12-28`,
  `src/order_views/advanced/components.rs:45-55`).
- Violated invariant: exact order/position identity may cross transient Elm
  plumbing for selection and correlation, but generic diagnostics should not
  disclose the selected symbol or an account-bearing persisted identifier.
- Risk: the debug-only unrouted-order-message assertion, a test failure,
  framework instrumentation, or future message logging could associate an
  account with a persisted Chase/TWAP record or expose which symbol is being
  sold, hidden, closed, canceled, or modified. No key, signature, signed
  payload, price, quantity, fraction, or known live diagnostic is implicated.
- Why existing checks did not cover it: F-48 wrapped only Chase/TWAP market and
  adoption symbols; F-49 wrapped result payloads but intentionally retained
  these correlation strings. OID/CLOID message tests used ordinary symbol
  literals without asserting their absence, while advanced-history tests
  redacted the persisted entry's own `Debug` but did not format the navigation
  message that cloned its raw ID.
- Implemented fix: reuse `RedactedOrderSymbol` for all 11 symbol fields and add
  `RedactedAdvancedOrderHistoryId`, an allocation-neutral string newtype whose
  diagnostic exposes only a marker. Every producer wraps at message
  construction; `update_order`, `update_account`, or `update_wallet_cluster`
  immediately calls `into_string()` before the same handler. History navigation
  does the same before the existing entry lookup/window map operation
  (`src/message.rs:310-370`, `src/order_update.rs:42-126`,
  `src/order_update.rs:281-283`, `src/order_update.rs:441-485`,
  `src/account_update.rs:15-19`, `src/wallet_cluster_update.rs:134-147`).
- Regression coverage: format all 12 affected variants with unique symbol and
  history-identity sentinels and require redaction without either exact value
  (`src/message.rs:1852-1918`). The wrapper recovery control proves the exact
  history ID and symbol survive formatting (`src/message.rs:2033-2069`), while
  existing route, exit-fence, chart-cancel, cluster, status, close, and history
  tests retain their established lifecycle assertions.
- Smallest behavior-preserving fix: wrapping moves each existing `String`
  without changing its allocation; the move-result path replaces its existing
  `&str`-to-`String` allocation with the wrapper's identical conversion. No
  message route, handler signature, history ID/schema, lookup, window identity,
  symbol normalization, move key, status owner, price, fraction, close/cancel/
  move preparation, signing, task order, visible copy, or trading policy
  changed.
- Residual uncertainty: Kerosene has not type-checked on this host. Rustfmt,
  exact raw-field absence searches, complete call-site tracing, and the
  mechanical diff establish the intended boundary, but focused and nearby
  suites cannot execute until ALSA development metadata is available. Raw
  financial values and remaining nested order-sensitive message types still
  require a separate complete inventory.

### F-51 — Direct financial message values bypass diagnostic redaction

- Status: addressed in Turn 44; focused tests added, but executable validation
  is blocked before Kerosene compilation by the missing system ALSA package
- Severity: Medium account/order privacy and diagnostic-boundary hardening; no
  incorrect preparation or known production disclosure was found, but exact
  trading intent and risk-setting values crossed a generic formatter
- Scope: all 11 remaining direct/nested financial fields in `Message`:
  order-book selected price; main sizing percentage; preset edit start/change
  strings; exact `OrderPreset`; market-slippage input; connected and cluster
  close fractions; quick-order open price and sizing percentage; move target
  price; every publisher, route, immediate consumer, and nearby interaction/
  lifecycle test
- Preconditions/event ordering:
  1. Order-book/depth views publish formatted price strings; order and quick
     sliders publish percentages; preset controls publish editable strings or a
     cloned preset; risk settings publish slippage text; close controls publish
     fractions; chart interaction publishes clicked/dragged prices.
  2. The order, preferences, or cluster update arm passed each raw value
     directly to existing storage, parsing, clamping, validation, preparation,
     or execution. Invalid/non-finite numeric inputs remained available to the
     established fail-closed checks rather than being normalized in routing.
  3. Derived `Message::Debug` emitted the input text, every numeric value, and
     nested preset label/size/offset verbatim before those checks ran.
- Evidence: parent commit
  `90b156a313e886fb43140a9b13705feb46f21ba2` shows the raw fields at
  `src/message.rs:807-814`, `src/message.rs:829-832`,
  `src/message.rs:968-971`, `src/message.rs:1013-1017`,
  `src/message.rs:1269-1273`, `src/message.rs:1484-1487`, and
  `src/message.rs:1515-1518`. Producers span the order-book/depth rows, main
  size controls, preset row, risk input, close menus, cluster controls, quick
  controls, and chart press/drag interaction
  (`src/market_views/order_book/depth/rows.rs:100-146`,
  `src/market_views/order_book/depth/dom/rows.rs:90-100`,
  `src/depth_chart/interaction.rs:24-42`,
  `src/order_views/inputs/size.rs:66-78`,
  `src/order_views/inputs/size/presets.rs:68-81`,
  `src/order_views/presets/preset_row.rs:20-95`,
  `src/settings_views/risk.rs:134-149`,
  `src/account_views/positions/table/close_cell/menu.rs:42-58`,
  `src/wallet_cluster_views.rs:708-724`,
  `src/order_views/quick_order/components/inputs.rs:54-67`,
  `src/chart/interaction/press.rs:278-293`,
  `src/chart/interaction/drag.rs:207-218`).
- Violated invariant: exact financial inputs may cross transient Elm plumbing
  so canonical handlers can validate and execute them, but generic diagnostics
  must not disclose price, size percentage, preset, slippage, or close fraction
  and must not change any float while hiding it.
- Risk: the debug-only unrouted-order-message assertion, a test failure,
  framework instrumentation, or future message logging could reveal a selected
  limit/move price, sizing intent, stored preset, configured market slippage, or
  requested close fraction. No account key, signature, wire payload, automatic
  retry, exchange outcome, or known live diagnostic is implicated.
- Why existing checks did not cover it: typed price/quantity/TWAP/leverage
  inputs already used `RedactedOrderInput`, and snapshot/form/result formatters
  hid their nested values, but book selection, sliders, preset controls,
  slippage, close fractions, and pre-form chart prices remained separate raw
  parent fields. F-50 deliberately changed only symbol/history identities.
- Implemented fix: reuse `RedactedOrderInput` for the four string fields and add
  generic `RedactedOrderValue<T>` for the six numeric fields plus the nested
  preset. It exposes only `OrderValue(<redacted>)` and returns the original `T`
  without parsing or conversion. Every publisher wraps only at `Message`
  construction; `update_order`, `update_preferences`, or
  `update_wallet_cluster` immediately restores the prior type before the same
  handler (`src/message.rs:227-278`, `src/order_update.rs:34-73`,
  `src/order_update.rs:123-130`, `src/order_update.rs:380-403`,
  `src/order_update.rs:451-460`, `src/preferences_update.rs:406-407`,
  `src/wallet_cluster_update.rs:137-146`).
- Regression coverage: format all 11 variants with unique text, preset, `f32`,
  and `f64` sentinels and reject every exact value while retaining redaction
  (`src/message.rs:1990-2063`). Independently recover a payload-bearing NaN for
  each float width, negative zero, and the exact cloned preset
  (`src/message.rs:1759-1789`); existing chart-input, route, exit-fence, order,
  preference, close/move, cluster, and preset tests remain lifecycle controls.
- Smallest behavior-preserving fix: string and preset allocations are moved as
  before; numeric wrappers are single-field moves whose recovery preserves
  exact bits. Quick-order X/Y/width/height remain unwrapped control geometry.
  No route, handler signature, parse, clamp, validation, precision, display
  formatting, preset schema/default, state timing, price/size calculation,
  preparation, signing, task, visible copy, or trading policy changed.
- Residual uncertainty: Kerosene has not type-checked on this host. Rustfmt,
  exact raw-field absence searches, complete call-site tracing, and the
  mechanical diff establish the intended boundary, but focused and nearby
  suites cannot execute until ALSA development metadata is available. Nested
  order/account state diagnostics and non-mutation external result/status
  messages remain for the final Track 9 inventory.

## Turn 1 — Baseline and Lifecycle Assurance Matrix

- Status: audited
- Severity: N/A; read-only startup turn
- Scope: architecture, all signed mutation call sites, baseline validation,
  initial assurance matrix
- Invariant: no production source changes before every current mutation surface
  has an evidence-backed lifecycle row.
- Protected behavior: all UI, order semantics, automation timing, persistence,
  and signing behavior remained unchanged.
- Evidence: required architecture/security/testing documents, prior lifecycle
  audit and campaign, repository-wide signing/task call-site searches, and the
  source references in the matrix above.
- Change: created this progress ledger only.
- Tests/checks: baseline commands and exact outcomes are recorded above.
- Compatibility/UX assessment: documentation-only; no runtime impact.
- Residual risk: matrix gaps and candidates F-01 through F-05 require focused
  adversarial verification. The Rust validation environment remains blocked by
  missing ALSA development metadata.
- Prior turn commit hash: `cbd48106a75fcbeb982f5e6ed2d53772bbf7123b`
- Next candidate: implement F-01 with a failing duplicate-child regression and
  CLOID-keyed idempotent NUKE aggregation, preserving all unique-result behavior
  and status text.

## Turn 2 — Idempotent NUKE Child Settlement

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium invariant hardening for a potentially High-consequence
  emergency-close ordering failure
- Scope: runtime NUKE parent accounting and the two terminal result boundaries
- Invariant: one logical NUKE child contributes at most one terminal outcome to
  its parent, regardless of duplicate direct or reconciliation-result delivery.
- Protected behavior: every unique child still produces the same aggregate
  counts, completion threshold, account-refresh decision, error flag, and
  visible status text. Order preparation, CLOID generation, signed payloads,
  exchange calls, UI, and persistence are untouched.
- Evidence: the result context already carries the child's immutable CLOID;
  prior aggregation discarded it and incremented an unkeyed counter. Source
  references and adversarial ordering are recorded under F-01 above.
- Change: replaced the unkeyed completed count with a runtime-only settled-CLOID
  set, made each outcome recorder atomically reject an already-settled child,
  derived completion from unique settlements, and retained the former safe
  counter-only `Debug` shape so stored CLOIDs cannot be formatted.
- Regression tests: added duplicate direct-result and duplicate status-result
  cases. Each proves one repeated CLOID leaves the parent at `1/2` and that a
  second unique CLOID is required for the unchanged `2/2` completion state.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene duplicate_nuke_` stopped in
    `alsa-sys` before compiling Kerosene because `pkg-config` could not find
    the system `alsa.pc` package.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
  - A pre-implementation focused-test attempt encountered the same ALSA
    boundary, so the new regression could not be observed failing on this host.
- Compatibility/UX assessment: internal runtime bookkeeping only; no normal
  success/failure copy or behavior changes and no schema/dependency changes.
- Residual risk: source parsing, formatting, call-site inspection, and the diff
  pass, but the new tests and Rust type-check must still execute once ALSA
  development metadata is available.
- Prior turn commit hash: `f8d2fa41abc4f6ef1456fcc2988b1ef8d280f315`
- Next candidate: audit F-02's account-refresh completeness and exact-resolution
  semantics before deciding whether one-shot status cleanup needs a guarded
  production change or only characterization coverage.

## Turn 3 — Scope-Complete One-Shot Refresh Reconciliation

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: the shared pending one-shot status record and successful account-load
  cleanup boundary
- Invariant: an account refresh may release an uncertain one-shot placement only
  if account-wide fills are complete and the refresh successfully fetched the
  open-order lane for the order's origin market.
- Protected behavior: exact `orderStatus` outcomes and complete covering refresh
  cleanup retain their existing status strings, blocker transitions, account
  refresh behavior, and timing. Order construction, signing, dispatch, pricing,
  sizing, reduce-only behavior, UI, and persistence are untouched.
- Evidence: best-effort bootstrap and scoped open-order behavior prove that
  `Ok(AccountData)` alone is not a completeness guarantee. Detailed source
  evidence and the failure ordering are recorded under F-02 above.
- Change: retained `symbol_key` in the runtime-only pending status record and
  added a per-symbol open-order completeness helper that distinguishes main and
  HIP-3 fetch lanes. Account-refresh cleanup evaluates that signal and fill
  completeness independently for every pending request.
- Regression tests: added incomplete-open-orders, incomplete-fills,
  wrong-HIP-3-scope, and later-covering-scope cases. Existing complete-refresh
  tests remain the characterization for unchanged normal cleanup. A focused
  account-data test proves an unrelated failed dex does not invalidate an
  origin lane with a successful fetch timestamp
  (`src/account/types/data/tests/freshness.rs:112-125`).
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene account_refresh_must_cover_one_shot_symbol_before_clearing_status_request`
    stopped in `alsa-sys` before compiling Kerosene because `pkg-config` could
    not find the system `alsa.pc` package.
  - `cargo test --package kerosene --bin kerosene incomplete_account_refresh_does_not_clear_one_shot_status_request`
    stopped at the same environment boundary.
  - `cargo test --package kerosene --bin kerosene complete_open_order_coverage_tracks_each_symbol_lane`
    stopped at the same environment boundary.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
  - A pre-implementation focused-test attempt encountered the same ALSA
    boundary, so the new regression could not be observed failing on this host.
- Compatibility/UX assessment: no visible copy, indicator, layout, or
  normal-path enabled/disabled behavior changes. Only an incomplete or
  origin-excluding fallback snapshot remains uncertain until exact status or a
  complete covering refresh arrives; treating that state as resolved was the
  confirmed safety defect.
- Residual risk: source parsing, formatting, call-site inspection, and the diff
  pass, but the new tests and Rust type-check must still execute once ALSA
  development metadata is available. A complete covering account snapshot
  remains the established fallback resolution boundary and does not rewrite the
  visible status into a false exact success.
- Prior turn commit hash: `5ea78f1c2aa9a1327d50093f1b382c37f48b0b28`
- Next candidate: implement F-03's zero-behavior-change CLOID redaction in
  `PendingOneShotStatusRequest::Debug` with focused regression coverage.

## Turn 4 — Redact Pending One-Shot CLOIDs

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium privacy hardening
- Scope: one custom `Debug` implementation and its focused regression
- Invariant: diagnostic formatting of pending order lifecycle state must not
  disclose account addresses or CLOIDs.
- Protected behavior: the pending record retains the exact CLOID internally for
  matching. All request IDs, lifecycle transitions, status strings, account
  refreshes, exchange payloads, views, and persistence behavior are untouched.
- Evidence: repository-wide inspection of explicit CLOID formatters found the
  raw exposure only in `PendingOneShotStatusRequest`; adjacent context, signing,
  TWAP, cluster, and order-status models use `<redacted>` or presence-only
  fields.
- Change: replaced the raw CLOID formatter value with `<redacted>` and tightened
  the existing test to reject the synthetic account and CLOID independently.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene pending_one_shot_status_request_debug_redacts_account_and_cloid`
    stopped in `alsa-sys` before compiling Kerosene because `pkg-config` could
    not find the system `alsa.pc` package.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
  - The pre-implementation focused-test attempt encountered the same ALSA
    boundary, so the strengthened regression could not be observed failing on
    this host.
- Compatibility/UX assessment: diagnostic-only output change; no user-visible,
  trading-semantic, timing, schema, or dependency impact.
- Residual risk: source parsing, formatting, call-site inspection, and the diff
  pass, but the focused test and Rust type-check must still execute once ALSA
  development metadata is available.
- Prior turn commit hash: `55263a56c1b598af20e3aaadb012948e20644e47`
- Next candidate: characterize F-04's wallet-cluster result correlation and
  idempotence under stale IDs, wrong profile/CLOID, duplicate direct results,
  and status-after-terminal ordering before changing production behavior.

## Turn 5 — Make Wallet-Cluster Leg Transitions Idempotent

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium invariant hardening
- Scope: shared order/close cluster leg result transitions and follow-up gating
- Invariant: a fully correlated direct result may transition a leg out of
  `Pending` once; only the resulting `Checking` phase may accept one
  `orderStatus` result; terminal states never transition again.
- Protected behavior: every valid unique result retains the existing classifier,
  status/message text, member refresh, CLOID repair, aggregate completion, and
  problem-count behavior. Cluster planning, sizing, signing, fan-out, UI,
  persistence, and member data streams are untouched.
- Evidence: handler and model tracing plus adversarial cases proved the void
  update helper could overwrite terminal legs and could not suppress follow-up
  work for mismatched results. Detailed ordering and source evidence are
  recorded under F-04 above.
- Change: replaced unconditional leg assignment with an expected-state
  transition that verifies the complete immutable origin context. Both handlers
  now stop immediately when correlation or phase validation fails.
- Regression tests: added exact-origin mismatch coverage, conflicting duplicate
  direct results for both cluster order and close executions, terminal-to-
  checking regression, duplicate ambiguity, status-before-checking, normal
  checking-to-confirmed settlement, and status-after-terminal replay.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene wallet_cluster_update::tests`
    stopped in `alsa-sys` before compiling Kerosene because `pkg-config` could
    not find the system `alsa.pc` package.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
  - The pre-implementation focused-test attempt encountered the same ALSA
    boundary, so the new regression could not be observed failing on this host.
- Compatibility/UX assessment: internal correlation and replay rejection only;
  no visible copy, normal result timing, controls, order semantics, schema, or
  dependency changes.
- Residual risk: source parsing, formatting, call-site inspection, and the diff
  pass, but the focused tests and Rust type-check must still execute once ALSA
  development metadata is available.
- Prior turn commit hash: `966de31f4ad2b8de6451d0ef04a54bcb8fde74a4`
- Next candidate: implement F-06's diagnostic-only cluster-leg message
  redaction, then resume F-05's Chase and TWAP duplicate/late-result audit.

## Turn 6 — Redact Wallet-Cluster Lifecycle Messages in Debug

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium privacy hardening
- Scope: `WalletClusterExecutionLeg::Debug` and one focused model regression
- Invariant: redacting explicit order fields is insufficient if a free-form
  lifecycle message can carry the same CLOID or exchange detail; diagnostic
  formatting must redact both paths.
- Protected behavior: the stored lifecycle message and the cluster execution
  view remain unchanged. No result classification, transition, refresh,
  request, signing, UI, schema, or persistence code changed.
- Evidence: the only non-view consumer of the stored message was its custom
  formatter, and unexpected-resting handlers can embed the CLOID in that message.
  Detailed source evidence is recorded under F-06 above.
- Change: replaced the formatter's raw message value with `<redacted>`.
- Regression test: formats a synthetic uncertain leg whose message repeats its
  CLOID, rejects that raw identifier in `Debug`, checks the explicit redaction
  marker, and verifies the stored message is intact.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene execution_leg_debug_redacts_lifecycle_message`
    stopped in `alsa-sys` before compiling Kerosene because `pkg-config` could
    not find the system `alsa.pc` package.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
  - The pre-implementation focused-test attempt encountered the same ALSA
    boundary, so the strengthened regression could not be observed failing on
    this host.
- Compatibility/UX assessment: diagnostic-only output change; no user-visible,
  trading-semantic, timing, schema, or dependency impact.
- Residual risk: source parsing, formatting, consumer inspection, and the diff
  pass, but the focused test and Rust type-check must still execute once ALSA
  development metadata is available.
- Prior turn commit hash: `47807a313004fec25e53377d1a5198b137bf1d34`
- Next candidate: resume F-05 with adversarial Chase and TWAP direct-result
  replay tests, adding per-attempt correlation only if existing phase guards do
  not already make duplicate and late delivery harmless.

## Turn 7 — Correlate Advanced-Order Direct Results to Exact Attempts

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium invariant hardening
- Scope: internal result messages and handlers for Chase placement, Chase
  modify, and TWAP child placement
- Invariant: a direct exchange result may transition only the exact runtime
  attempt whose task emitted it, even after the strategy returns to the same
  lifecycle phase or reuses the same OID/CLOID.
- Protected behavior: every result whose dispatch-time sequence matches the
  current pending attempt retains the existing classification, account refresh,
  reconciliation, fill accounting, retry, stop, archive, status text, and task
  timing. Order preparation, sizing, pricing, rounding, reduce-only behavior,
  Chase repricing, TWAP cadence/randomization, signing, UI, and persistence are
  untouched.
- Evidence: dispatch and handler tracing confirmed that all three state machines
  already own bounded runtime counters that distinguish recurring phases, but
  their direct-result messages discarded those counters. Exact ordering and
  source references are recorded under F-05 above.
- Change: captured `place_attempt`, `reprice_count`, or
  `slice_index`/`retry_count` in the corresponding `Message` and required it to
  equal current state before handling. This adds no state, identifiers,
  dependencies, schema fields, exchange requests, or mutation retries.
- Regression tests: added conflicting immediate duplicates plus late prior-
  attempt results while later attempts occupy the same phase. The Chase modify
  regression keeps OID 42 constant; TWAP independently exercises a prior slice
  index and a prior retry of the current slice.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - `cargo test --package kerosene --bin kerosene stale_place_result_from_prior_attempt_does_not_settle_current_attempt`
    stopped in `alsa-sys` before compiling Kerosene because `pkg-config` could
    not find the system `alsa.pc` package.
  - `cargo test --package kerosene --bin kerosene stale_modify_result_from_prior_reprice_does_not_settle_current_reprice`
    stopped at the same environment boundary.
  - `cargo test --package kerosene --bin kerosene stale_slice_result_requires_current_index_and_retry_count`
    stopped at the same environment boundary.
  - A final combined `cargo test --package kerosene --bin kerosene stale_`
    retry after the correlation guards were complete stopped at the same
    boundary.
  - `cargo check` stopped at the same pre-existing environment dependency
    boundary before checking Kerosene.
- Compatibility/UX assessment: runtime message correlation only; no visible
  copy, controls, success/failure behavior for current attempts, order
  semantics, schema, or dependency changes.
- Residual risk: source parsing, formatting, exhaustive call-site inspection,
  and the diff pass, but the new tests and Rust type-check must still execute on
  a host with ALSA development metadata. Existing Chase duration/reprice and
  TWAP slice/retry limits keep these runtime sequences below saturation; no
  counter or wraparound behavior was changed.
- Prior turn commit hash: `fda0d0ee3fe8491624a53e4c5bab3fb6d71a9c65`
- Next candidate: audit prepared-order-to-signed-wire parity and narrow
  structural validation (Track 2), starting with non-finite/negative-zero,
  market-type/asset identity, side, reduce-only, and order-kind preservation
  before deciding whether production hardening is warranted.

## Turn 8 — Validate Signed Order Wire Structure

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium invariant hardening
- Scope: Audit Track 2 across all prepared placement and modify surfaces, plus
  the shared signed-payload boundary
- Invariant: valid prepared fields must reach the signed action unchanged, while
  structurally invalid numeric values or an uncorrelatable placement must never
  be hashed, signed, or posted.
- Protected behavior: exact valid price/size strings, asset, side, TIF/order
  kind, reduce-only, OID/CLOID, JSON/msgpack field order, signature inputs,
  preparation policy, result handling, and user-visible behavior remain
  unchanged.
- Evidence: a repository-wide caller trace found that ticket, preset, Alfred,
  quick, HUD, close, cluster, NUKE, Chase, and TWAP placements all use
  `place_order_task`, while Move and Chase modify use `modify_order_task`; both
  converge on `build_signed_exchange_payload_with_nonce`. Ordinary intents use
  shared preparation; the three direct advanced/emergency constructors validate
  their planned numeric values before constructing prepared orders. Detailed
  evidence and the missing boundary invariant are recorded under F-07.
- Change: added one read-only validation pass at the single signed-action
  payload builder. Place/modify prices and sizes must parse as positive finite
  numbers; placements must carry the established 128-bit hexadecimal CLOID.
  Validation returns static redacted errors and does not mutate the action.
- Regression tests: added exact valid signed-action parity, invalid place price
  and size classes including negative zero, invalid modify size, and missing or
  malformed CLOID cases. Expanded the prepared-request characterization to
  assert the side, order-kind, and reduce-only fields copied by that handoff.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` passed.
  - `git diff --check` passed before the ledger update and is rerun during the
    final review.
  - The pre-implementation
    `cargo test --package kerosene --bin kerosene signed_order_payload_rejects_invalid_wire_numbers_before_signing`
    attempt stopped in `alsa-sys` before compiling Kerosene because
    `pkg-config` could not find the system `alsa.pc` package.
  - Post-implementation focused
    `cargo test --package kerosene --bin kerosene signed_order_payload_`,
    `cargo test --package kerosene --bin kerosene signed_modify_payload_`,
    and `cargo test --package kerosene --bin kerosene signing::tests::actions`
    attempts stopped at the same environment boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    the same pre-existing dependency boundary before checking Kerosene.
- Compatibility/UX assessment: no valid request or normal result path changes.
  Only structurally invalid internal actions that previously could reach signing
  now fail closed before hashing/posting; no schema, dependency, view, copy, or
  trading-policy change was introduced.
- Residual risk: source parsing, formatting, exhaustive call-site inspection,
  and diff checks pass, but the focused tests, Rust type-check, full suite, and
  clippy must still execute on a host with ALSA development metadata. Signing
  intentionally cannot revalidate symbol-to-asset or market capability because
  those identities are absent from the wire action and remain owned by the
  canonical preparation/state-machine boundaries.
- Prior turn commit hash: `2cda6c85c6f557c28283a7709e24ede3066b7fb1`
- Next candidate: audit mutation transport and ambiguity classification (Track
  3), distinguishing failures before serialization/connect/send from failures
  after a request may have reached the exchange without adding any automatic
  mutation retry.

## Turn 9 — Reconcile Contradictory Exchange Acknowledgements

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: Audit Track 3 parsed-response classification at the shared response
  model and all Chase/TWAP mutation handlers that bypass the shared one-shot
  classifier
- Invariant: an explicit error combined with a possible structured exchange
  effect is not a rejection or terminal success; exact CLOID/OID or account
  reconciliation must resolve it before local lifecycle state settles.
- Protected behavior: pure exchange rejection, ordinary resting/fill/cancel
  success, unstructured malformed response, transport failure, existing
  bounded cancel retry, pricing, sizing, automation timing, signed payloads,
  persistence, and normal user-visible status behavior remain unchanged.
- Evidence: the shared classifier already distinguished mixed inner statuses,
  but top-level errors hid their structured effects and advanced handlers
  consumed error/IOC/terminal-cancel branches before ambiguity. Detailed event
  ordering, source references, and risk are recorded under F-08 above.
- Change: added one response-analysis conflict predicate, removed the
  top-level-error exclusion from potential-effect analysis, and ordered every
  Chase/TWAP parsed-result handler so a conflict reaches its existing uncertain
  reconciliation path first. The patch adds no automatic place/modify retry,
  exchange request, state field, identifier, dependency, schema, or visible
  control.
- Regression tests: added top-level-envelope and shared-classifier cases; Chase
  place, modify, and cancel state-machine cases; and TWAP slice and unexpected-
  child-cancel cases. A separate TWAP characterization proves a
  non-conflicting filled status without an OID retains its prior fill-accounting
  path; existing pure-rejection and normal-success tests protect the other
  unchanged branches.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` and `git diff --check` passed before the ledger
    update and are rerun during final review.
  - Pre- and post-implementation
    `cargo test --package kerosene --bin kerosene conflicting_` attempts stopped
    in `alsa-sys` before compiling Kerosene because `pkg-config` could not find
    the system `alsa.pc` package.
  - `cargo test --package kerosene --bin kerosene signing::tests::responses` and
    `cargo test --package kerosene --bin kerosene execution_result_classifier_`
    stopped at the same environment boundary.
  - `cargo test --package kerosene --bin kerosene non_conflicting_fill_without_oid_preserves_existing_fill_accounting`
    stopped at the same boundary; this is the explicit unchanged-behavior
    characterization adjacent to the TWAP conflict guard.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
- Compatibility/UX assessment: valid and purely rejected responses take their
  former branches with identical status copy and task timing. Only an anomalous
  internally contradictory response remains uncertain instead of being
  consumed as definitive; this is lifecycle plumbing, not a product-policy or
  normal UX change.
- Residual risk: formatting, source analysis, exhaustive result-handler search,
  and diff review pass, but all Rust tests/type-check/clippy still require a
  host with ALSA development metadata. The transport client still deliberately
  flattens all failures after action construction into conservative
  transport-unknown strings.
- Prior turn commit hash: `710139d929d15fce575a4b1e056da8dc6410969f`
- Next candidate: finish Track 3 by auditing the exact HTTP client boundary for
  serialization/connect/send/body/parse phase provenance and redaction. Add
  typed internal context only if it improves diagnostics while every uncertain
  mutation remains fail-closed and no retry or user-facing behavior changes.

## Turn 10 — Isolate and Bound Signed Mutation Transport

- Status: implemented; executable Rust validation environment-blocked
- Severity: High transport replay prevention plus Medium diagnostic/privacy
  hardening
- Scope: completion of Audit Track 3 from pre-signing validation through HTTP
  policy, response parsing, task-message mapping, and immediate-result debug
- Invariant: one lifecycle attempt initiates at most one HTTP send; transport
  uncertainty returns to exact reconciliation; and both success and error
  results are diagnostically safe before they can enter derived `Message::Debug`.
- Protected behavior: valid signed payloads, endpoint/method/body, nonce and
  expiry, safe error copy, all outcome kinds, conservative unknown handling,
  CLOID/OID reconciliation, application-level target-cancel policy, ordinary
  status copy, controls, persistence, and normal request timing remain
  unchanged.
- Evidence: every signed wrapper and task caller was retraced. The exact phase
  decisions are recorded in the Mutation Transport Phase Audit, and the three
  confirmed boundary weaknesses are detailed under F-09 through F-11.
- Change: replaced shared generic HTTP policy with a dedicated no-redirect,
  no-retry exchange client; retained the existing 5/15/60-second limits and
  redundantly bound each request to 15 seconds; removed the unsafe unbounded
  fallback; applied one order-aware sanitizer to every signing exit; and made
  successful exchange-response debug metadata-only. Non-success HTTP envelopes
  can no longer confirm an apparently successful body. No downstream result
  type or handler branch changed because treating provably local errors less
  conservatively would alter UX without improving financial safety.
- Regression tests: added a loopback 307 replay trap, explicit per-request
  timeout inspection, secret/CLOID result-debug redaction, safe error and `Ok`
  preservation, 128-bit order-identifier redaction, and successful
  response-detail debug redaction, plus non-success HTTP success/rejection/
  conflict classification.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` and `git diff --check` passed before the ledger
    update and are rerun during final review.
  - Pre- and post-implementation
    `cargo test --package kerosene --bin kerosene exchange_client_does_not_replay_a_redirected_mutation`
    attempts stopped in `alsa-sys` before compiling Kerosene because
    `pkg-config` could not find the system `alsa.pc` package.
  - Focused `exchange_request_has_a_mutation_local_timeout`,
    `exchange_result_`, `parse_exchange_response_`,
    `sensitive_order_text_redacts_128_bit_cloid`,
    `exchange_response_error_status_redacts_sensitive_values`, and
    `exchange_response_debug_redacts_successful_order_details` test attempts
    stopped at the same environment boundary.
  - Focused `non_success_http_status_` test attempts stopped at the same
    boundary.
  - The protected existing
    `ambiguous_transport_results_require_account_refresh` test attempt also
    stopped at that boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
- Compatibility/UX assessment: the ordinary client used the same user agent
  and timeouts; valid exchange calls, explicit rejections, and all normal
  handler-visible strings/state are unchanged. Only hidden HTTP resend/redirect
  behavior, an unsafe client-build fallback, diagnostic exposure, and false
  confirmation from a contradictory non-2xx envelope are removed.
- Residual risk: formatting, exhaustive signed-call/result-handler inspection,
  phase analysis, and diff checks pass, but the new tests/type-check/clippy must
  execute on a host with ALSA development metadata. F-12 confirms that the
  separate `orderStatus` read-error lane needs the same pre-message CLOID
  protection in the next narrow batch.
- Prior turn commit hash: `501cfb097428987c0cd8f1dbb4f47779655db468`
- Next candidate: address F-12 at the `orderStatus` task boundary, then resume
  cancel/move ownership and repeated-attempt correlation in Track 4.

## Turn 11 — Redact Order-Status Result Diagnostics

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium privacy hardening
- Scope: F-12 across the `orderStatus` HTTP preview, parser, public task exits,
  result model, and every caller that maps those results into `Message`
- Invariant: exact OID/CLOID values remain available for request construction,
  correlation, and lifecycle handling, but neither the successful nor error
  result lane may expose sensitive order text through derived message debug.
- Protected behavior: status request JSON, exact expected/actual correlation,
  successful result fields, missing/open/filled/terminal classification, task
  routing and timing, retry/reconciliation policy, stored status text, normal
  user-visible status behavior, signing, persistence, and order semantics.
- Evidence: a complete caller trace found that one-shot, NUKE, cancel, move,
  Chase, TWAP, and wallet-cluster status tasks all use
  `fetch_order_status_by_cloid` or `fetch_order_status_by_oid`; before this turn,
  handler-time sanitization occurred only after the `Result` had entered a
  derived message. The concrete exposure and source references are recorded
  under F-12.
- Change: applied the existing order-aware redactor at both public status-result
  exits, redacted HTTP text before truncating it, changed OID/CLOID
  mismatch/missing errors to retain their diagnostic category without values,
  and sanitized the external status field only when formatting
  `OrderStatusResult::Debug`. No request, result, state-machine, message, or
  persistence type changed.
- Regression tests: added adversarial pre-message error formatting, a CLOID
  spanning the old preview cutoff, OID/CLOID mismatch and missing-field value
  checks, external parser-error CLOID redaction, and successful-result status
  debug redaction. Existing safe error text and exact successful result values
  are explicitly preserved.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` and `git diff --check` passed before the ledger
    update and are rerun during final review.
  - The pre-implementation
    `cargo test --package kerosene --bin kerosene order_status_result_error_is_redacted_before_message_mapping -- --exact`
    attempt stopped in `alsa-sys` before compiling Kerosene because
    `pkg-config` could not find the system `alsa.pc` package.
  - The post-implementation focused
    `cargo test --package kerosene --bin kerosene order_status_` attempt stopped
    at the same environment boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
- Compatibility/UX assessment: valid and safe status paths retain exact data,
  copy, state changes, and timing. Only anomalous identifier-bearing diagnostic
  strings and debug output are redacted; no view, control, schema, dependency,
  network policy, or trading behavior changed.
- Residual risk: formatting, source inspection, exhaustive public-fetch caller
  tracing, and diff review pass, but the new tests/type-check/clippy must execute
  on a host with ALSA development metadata. F-13 separately records raw
  identifier context fields on derived messages; F-12 does not claim those
  sibling fields are safe.
- Prior turn commit hash: `9123fcf33d61c01d3a168b80bea58185d503957c`
- Next candidate: address F-13 with typed redacted message-context values and
  whole-message debug regressions, then resume cancel/move ownership and
  repeated-attempt correlation in Track 4.

## Turn 12 — Redact Order-Identifier Message Context

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium privacy hardening
- Scope: F-13 across every direct exchange OID/CLOID field on `Message`, its
  publication and consumption boundaries, and the adjacent stopped-Chase cancel
  request diagnostic
- Invariant: transient message and planning diagnostics must not expose an
  exchange order identifier, while update handlers receive the exact same
  primitive value and lifecycle context as before.
- Protected behavior: all message variants and routes, exact OID/CLOID values,
  account/symbol/attempt context, handler signatures, state transitions,
  cancellation/modify/status requests, chart cancel-hover behavior, task
  timing, visible status/UI, signing, order semantics, and persistence.
- Evidence: a field-and-caller inventory traced all named OID/CLOID fields plus
  the positional chart cancel-hover OID. Fourteen order-route variants converge
  on `update_order`; the chart variant converges on `update_chart`. Every
  constructor was found by both variant-specific and repository-wide searches,
  and no alternate consumer was found. The concrete prior exposure and current
  source references are recorded under F-13.
- Change: introduced two message-only redacted wrappers with exact consuming
  accessors; wrapped at every publication site; unwrapped only at the order or
  chart update boundary; and made the stopped-Chase request's existing custom
  debug formatter value-neutral for OID. The patch adds no state field, request,
  retry, dependency, schema, or policy.
- Regression tests: added a single table-style whole-message formatter test for
  all fifteen variants, an exact wrapper round-trip test, and an OID assertion
  on the stopped-Chase request formatter. The existing chart cancel-click and
  TWAP retry-task tests remain the protected producer-path checks.
- Validation:
  - `cargo fmt` passed.
  - `cargo fmt -- --check` and `git diff --check` passed before the ledger
    update and are rerun during final review.
  - The pre-implementation
    `cargo test --package kerosene --bin kerosene order_identifier_message_debug_redacts_oid_and_cloid_fields`
    attempt stopped in `alsa-sys` before compiling Kerosene because
    `pkg-config` could not find the system `alsa.pc` package.
  - The extended pre-implementation
    `cargo test --package kerosene --bin kerosene stopped_chase_cancel_request_debug_redacts_agent_key_and_oid`
    attempt stopped at the same environment boundary.
  - Post-implementation focused
    `cargo test --package kerosene --bin kerosene order_identifier_message_` and
    `cargo test --package kerosene --bin kerosene stopped_chase_cancel_request_debug_redacts_agent_key_and_oid`
    attempts stopped at the same boundary.
  - Protected producer-path
    `cargo test --package kerosene --bin kerosene fisheye_left_click_on_order_cancel_button_cancels_order`
    and
    `cargo test --package kerosene --bin kerosene unexpected_cancel_retry_due_task_waits_for_delay`
    attempts stopped at the same boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
- Compatibility/UX assessment: the wrappers exist only while a message is in
  transit and consume back to the original primitive before any handler logic.
  All visible and trading behavior is unchanged; only derived/custom diagnostic
  output substitutes a redaction marker for the identifier.
- Residual risk: formatting, exact-value characterization, complete variant
  producer/consumer inspection, direct-field source scans, and diff review pass,
  but the new tests/type-check/clippy must execute on a host with ALSA
  development metadata. The broader Track 9 audit still needs to inspect local
  derived planning/state enums that are not carried by `Message`; no claim is
  made about those later diagnostic surfaces here.
- Prior turn commit hash: `349f532e89d33135d3abc912d994af9daf7f91b5`
- Next candidate: resume Track 4 by auditing cancel and move ownership across
  repeated attempts on the same OID, delayed direct/status results, indicator
  disappearance, and authoritative open-order/fill refresh ordering.

## Turn 13 — Correlate Cancel and Move Attempts

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: F-14 across cancel/move dispatch, direct and status messages, pending
  runtime contexts, cancel phases, account-refresh cleanup, and shared request
  sequence allocation
- Invariant: only the immutable owner of one dispatched cancel or modify may
  consume its direct/status callback or clear its pending state; neither a
  reused OID nor a presentation indicator can identify an attempt by itself.
- Protected behavior: exact cancel/modify wire mutations, prepared asset/OID,
  side, size, reduce-only and price values, captured account/key behavior,
  response classification, normal statuses and refresh scheduling, confirmed
  local move-price projection, UI, persistence, and all retry policy.
- Preconditions/event ordering: an old cancel indicator/pending slot disappears
  or an old move context/status slot is cleared; a new attempt reuses the same
  account/symbol/OID; then the old direct or status result arrives after the new
  attempt owns that tuple. A duplicate cancel result can also arrive after its
  first result has advanced the attempt into status reconciliation.
- Evidence: the complete pre-change/direct/status call graph matched cancel by
  account (direct) or account/OID/symbol (status), and move by OID-key plus
  account (direct) or account/OID/symbol (status). Cancel target recovery could
  fall through from an expired indicator to the current account-scoped pending
  request. Account refresh cleanup did not distinguish a cancel awaiting its
  direct result from one awaiting reconciliation. F-14 records the concrete
  prior behavior and current source references.
- Change: renamed the one-shot-only runtime counter into a shared lifecycle
  request allocator that skips all live one-shot/cancel/move IDs across wrap;
  allocated an ID before each cancel/move dispatch; propagated it through both
  direct and status messages; required it in every result owner match; and made
  cancel's `AwaitingResult` to `CheckingStatus` transition explicit and
  one-way. Refresh cleanup can no longer erase a cancel whose direct result has
  not arrived. No field is persisted or sent to Hyperliquid.
- Tests/checks:
  - Added stale same-OID direct/status regressions for cancel and move, duplicate
    cancel direct-result coverage, indicator-expiry ownership coverage, an
    awaiting-direct-result refresh regression, and collision-aware allocator
    wrap coverage across all four live-state families.
  - Updated nearby cancel-status fixtures to enter the status-check phase
    explicitly and updated all constructor/message/handler call sites.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - Focused `cargo test --package kerosene --bin kerosene stale_cancel_`,
    `stale_move_`,
    `duplicate_cancel_result_does_not_override_status_reconciliation`,
    `account_refresh_does_not_clear_cancel_attempt_awaiting_direct_result`, and
    `order_lifecycle_request_allocator_skips_live_ids_across_wrap` attempts all
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - Nearby/protected `cancel_order_status_`, `move_order_status_`,
    `cancel_result_ambiguous_ack_uses_pending_request_after_indicator_expires`,
    and `move_result_success_carries_confirmed_price_into_local_snapshot` test
    attempts stopped at the same dependency boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing boundary before checking Kerosene.
- Compatibility/UX assessment: the request sequence and cancel phase are
  runtime-only plumbing. The normal successful, rejected, ambiguous, and
  transport-unknown paths publish the same status text and tasks; only stale,
  duplicate, or prematurely refreshed ownership is prevented from touching a
  different attempt.
- Residual risk: formatting, exact constructor/consumer inventory, phase and
  state-transition inspection, adversarial tests, and diff review pass, but
  the crate has not type-checked and the tests/clippy must execute on a host
  with ALSA development metadata. F-15 remains: refresh fallback must prove the
  origin symbol lane, and move reconciliation must retain/compare the expected
  target price before uncertainty is released.
- Prior turn commit hash: `32b998cf9877efe3a803a320d023450151d70349`
- Next candidate: address F-15 with symbol-lane-scoped cancel/move cleanup and
  exact expected-price evidence for an open moved order, preserving the current
  fetch cadence, wire mutation, normal statuses, and UI.

## Turn 14 — Scope Cancel and Move Refresh Evidence

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: F-15 across successful account-refresh cleanup, cancel origin-lane
  evidence, move prepared-price provenance, move snapshot classification, and
  status-phase test fixtures
- Invariant: an account snapshot may release cancel/move uncertainty only when
  it successfully fetched the exact operation's open-order lane; an open move
  additionally requires a parseable live price compared with the immutable
  prepared target, while target absence is terminal evidence.
- Protected behavior: exact cancel/modify preparation and signed payloads,
  account/key/request correlation, result/status classification and messages,
  refresh scope/cadence/generations, normal valid-snapshot cleanup, confirmed
  local move projection, indicators, retries, UI, and persistence.
- Preconditions/event ordering: a cancel or move becomes uncertain; the user
  changes the selected HIP-3 universe or the target lane fails while another
  lane succeeds; alternatively, a covering snapshot contains the target with a
  malformed or differently formatted/live price; successful account loading
  reaches the shared cleanup boundary.
- Evidence: account refresh chooses scope from the current market universe and
  a scoped snapshot can be complete for only that scope. `AccountData` already
  records per-lane fetch success, but cancel/move cleanup used only the global
  completeness boolean. Move's prepared price lived in its indicator and
  prepared request, then disappeared before status reconciliation. F-15 records
  the concrete source evidence and failure ordering.
- Change: reused `has_complete_open_orders_for_symbol` for both operations;
  retained the exact prepared price in `PendingMoveOrderContext`; copied it into
  pending status state only for ambiguous/transport-unknown outcomes; redacted
  it in custom diagnostics; and classified covering move snapshots as target
  absent, expected-price open, different-valid-price open, or insufficient.
  Only the first three preserve the existing cleanup. No request, task, polling,
  schema, dependency, or view was added. Status test fixtures now remove the
  direct-result context/indicator before arming reconciliation, matching the
  production phase boundary.
- Tests/checks:
  - Added cancel and move regressions for wrong HIP-3 scope and incomplete
    target lanes, covering cancel cleanup with a still-open target, move cleanup
    with formatted expected and different valid prices, malformed live-price
    retention after a real uncertain direct-result transition, and pure
    expected/different/absent snapshot classification.
  - Extended captured move-context and pending-status diagnostic tests to prove
    exact expected-price retention and diagnostic redaction.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - Pre- and post-implementation focused
    `cargo test --package kerosene --bin kerosene cancel_refresh_requires_complete_origin_symbol_lane`,
    `move_refresh_requires_complete_origin_symbol_lane`,
    `move_covering_refresh_requires_parseable_live_target_price`, and
    `move_covering_refresh_preserves_cleanup_for_valid_live_price` attempts all
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - Post-implementation focused
    `cancel_covering_refresh_preserves_existing_cleanup_when_target_is_open`,
    `move_snapshot_reconciliation_distinguishes_expected_different_and_absent_target`,
    `pending_move_context_reuses_captured_agent_key_for_same_account`, and
    `pending_move_status_request_debug_redacts_account_and_oid` attempts stopped
    at the same dependency boundary.
  - Nearby/protected `cancel_order_status_`, `move_order_status_`,
    `move_result_failure_keeps_local_price`,
    `account_refresh_must_cover_one_shot_symbol_before_clearing_status_request`,
    and `complete_open_order_coverage_tracks_each_symbol_lane` attempts stopped
    at the same boundary.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing boundary before checking Kerosene.
- Compatibility/UX assessment: complete covering snapshots with an absent
  target, the expected price (including equivalent formatting), or a different
  valid current price retain the same cleanup and enabled/disabled timing. Only
  an unrelated/incomplete lane or malformed target evidence remains blocked.
  The captured price is runtime-only plumbing and never changes the order or
  visible status.
- Residual risk: source parsing, formatting, exhaustive constructor/consumer
  inspection, phase-order review, adversarial tests, and diff review pass, but
  Rust type-check/tests/clippy must execute on a host with ALSA development
  metadata. Track 5 still needs a fresh end-to-end Chase audit across scoped
  REST refreshes, partial open-order/fill lanes, and user-stream ordering.
- Prior turn commit hash: `8771aa8e87771d6ae3db312e01d67ba80962acf3`
- Next candidate: begin the remaining Track 5 audit by tracing Chase
  disappearance/replacement and fill reconciliation across selected HIP-3
  scopes, partial account snapshots, websocket lag/repair, and reversed REST
  versus user-stream delivery, preserving all repricing and archive behavior.

## Turn 15 — Scope Chase Reconciliation to Its Origin Lane

- Status: implemented; executable Rust validation environment-blocked
- Severity: Critical
- Scope: F-16 across Chase REST refresh, account/stream fill reconciliation,
  replacement dispatch, stop verification, fill completion, and archive
- Invariant: only a snapshot that successfully fetched the active Chase's
  origin-symbol open-order lane may prove prior exposure absent, authorize
  archive, or permit a replacement; account-wide fills are an independent
  evidence lane.
- Protected behavior: exact Chase prices, sizes, sides, reduce-only and asset
  fields, CLOID/OID identity, place/reprice limits, cadence, retries, result and
  status classification, normal covering-snapshot transitions, all visible
  strings, UI, persistence, and secret lifetime.
- Preconditions/event ordering: a Chase on one HIP-3 dex awaits verification
  after no-fill status, fill completion, or stop/cancel; the selected universe
  changes; an `Ok` refresh for a different dex returns globally complete fills
  and open orders for its own scope; the prior global open-order check treats
  absence from that unrelated result as absence from the Chase's lane.
- Evidence: account refresh derives its request scope from the current market
  universe, whereas `AccountData` exposes explicit per-symbol open-order
  authority. The pre-change Chase refresh, fill cleanup, and final replacement
  guard used only snapshot-wide completeness. Websocket disappearance was
  already correctly dex-gated, fill merging and Chase attribution deduplicate
  by fill identity and filter account/coin/side/OID/cutoff, and place/modify
  callbacks retain dispatch-attempt correlation. F-16 records the concrete
  boundaries and failure consequences.
- Change: compute an immutable set of covered active Chase symbols once per
  account snapshot; pass it into fill reconciliation; evaluate every Chase
  refresh/stop/archive/replacement branch against its own symbol; and replace
  the final known-OID placement guard's global check with the same per-symbol
  predicate. Uncovered state remains in its existing verification lifecycle
  and refreshes; no new mutation, retry, timer, request, state field, schema,
  dependency, view, or message was introduced.
- Tests/checks:
  - Added regressions proving a wrong-scope HIP-3 refresh cannot dispatch a
    no-fill replacement, archive a stopping Chase, or archive a filled Chase
    whose open orders remain unknown, plus an independent final-dispatch guard
    test for known prior OIDs.
  - Pre- and post-implementation focused attempts for
    `unrelated_hip3_refresh_does_not_place_chase_replacement`,
    `unrelated_hip3_refresh_does_not_archive_stopping_chase`,
    `unrelated_hip3_refresh_does_not_archive_filled_chase_with_unknown_open_orders`,
    and `chase_replacement_requires_open_order_coverage_for_its_symbol` all
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - Nearby/protected attempts for
    `no_fill_terminal_status_allows_clean_replacement`,
    `stopped_chase_clears_only_after_no_known_open_orders_remain`,
    `chase_fill_reconciliation_removes_fully_filled_chase`, and
    `chase_place_assigns_unique_cloid_per_place_attempt` stopped at the same
    dependency boundary. A broader `cargo test --package kerosene --bin
    kerosene chase` attempt did likewise.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing boundary before checking Kerosene.
- Compatibility/UX assessment: a complete covering lane retains the exact
  prior replacement, safety-cancel, and archive behavior. Only a snapshot that
  never observed the Chase's lane remains uncertain. All user-facing copy and
  normal enabled/disabled behavior are unchanged; this is runtime plumbing and
  no persisted type changed.
- Residual risk: source formatting, all call-site inventory, per-state ordering,
  fill-deduplication, websocket dex-scope, attempt-correlation, and diff review
  pass, but the crate has not type-checked and tests/clippy must execute on a
  host with ALSA development metadata. REST snapshot versus newer websocket
  delta ordering needs a separate Track 8 audit.
- Prior turn commit hash: `97c40f9fca2cdb9e9035e682b87a5ac5824c2ec5`
- Next candidate: audit whether an account REST request captured before a newer
  user-stream delta can complete afterward and replace that delta without
  scheduling a follow-up refresh. Trace request-start revision, generation,
  `account_refresh_followup_pending`, and every websocket mutation before
  deciding whether this is a finding; preserve refresh cadence and display
  behavior.

## Turn 16 — Preserve Newer User Data Across REST Completion

- Status: implemented; executable Rust validation environment-blocked
- Severity: Critical
- Scope: F-17 across connected account request context, successful REST result
  application, all account-bearing user-data lanes, initial loading, shared
  lifecycle cleanup, documentation, and context fixtures
- Invariant: a connected REST result may replace account state or reconcile an
  order only if no user-data event advanced that account after request
  dispatch; an event received before a mergeable base exists must force one
  post-event snapshot before lifecycle cleanup.
- Protected behavior: account addresses/providers/key generations, request
  generations, scopes/cadence, successful no-conflict refreshes, errors and 429
  backoff, existing explicit/lag follow-ups, display/status text, order
  preparation and signing, automation parameters, UI, persistence, and secrets.
- Preconditions/event ordering: one connected refresh starts; its response can
  represent pre-event state; a valid same-account open-order/fill/position/
  balance frame arrives and updates the local snapshot (or cannot merge during
  initial load); then the still-current REST result arrives without an explicit
  second refresh or lag signal. All old provider/address/generation checks pass.
- Evidence: ordinary mergeable user-data frames advance
  `account_data_revision`, but connected request context previously captured no
  revision. Same-generation success then replaced `account_data` and ran every
  shared cleanup consumer. When no account data existed, ordinary frames were
  skipped while loading without setting the existing follow-up flag. F-17
  records the complete call graph, false-safety consequences, and fixed source
  boundaries.
- Change: extended only `ConnectedSnapshot` context with
  `account_data_revision_at_dispatch`; captured it at both initial-connect and
  forced-refresh task creation; checked it after provider/generation/address
  ownership but before any successful-result mutation; preserved the newer
  websocket-merged state and pending uncertainty on mismatch; and started one
  coalesced post-frame refresh. An account frame received with no initial base
  now sets the existing follow-up/reconciliation bits without launching a
  parallel task. Off-account TWAP context and error handling are untouched.
- Tests/checks:
  - Added an adversarial request-before/open-order-frame/result-after regression
    that proves the live OID and revision survive, one-shot uncertainty remains,
    and exactly one newer request generation is active.
  - Added initial-fetch coverage proving an unmergeable account frame queues a
    follow-up while the original request remains the only in-flight task.
  - Pre- and post-implementation
    `cargo test --package kerosene --bin kerosene rest_result_started_before_ws_delta_does_not_overwrite_or_reconcile`
    attempts stopped in `alsa-sys` before Kerosene compilation because
    `pkg-config` could not find the system `alsa.pc` package. The post-change
    `websocket_account_delta_queues_followup_during_initial_fetch` attempt
    stopped at the same boundary.
  - Nearby/protected `queued_refresh_followup_result_clears_reconciliation_required`,
    `stale_same_account_refresh_result_does_not_overwrite_newer_snapshot`,
    `refresh_requested_mid_fetch_is_queued_and_runs_after_load`,
    `websocket_open_order_update_refreshes_only_matching_dex_lane`,
    `twap_reconciliation_result_after_switching_back_does_not_replace_connected_snapshot`,
    `account_load_error_redacts_account_error`, and
    `address_bearing_message_debug_redacts_values` attempts all stopped at the
    same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing boundary before checking Kerosene.
- Compatibility/UX assessment: a no-conflict response follows the identical
  application and cleanup path. Only a causally superseded success is withheld;
  the newer already-visible websocket state no longer flickers backward and
  existing uncertainty stays blocked until one sequential fresh result. During
  initial loading the first result retains the established display-population
  behavior but cannot settle orders before its queued follow-up. No copy,
  control, schema, or persisted value changed.
- Residual risk: formatting, constructor/consumer inventory, serialized Elm
  ordering, all websocket mutation lanes, initial/no-base behavior, cleanup
  reachability, error/backoff isolation, and diff review pass, but Rust
  type-check/tests/clippy must execute with ALSA development metadata.
  Continuous live deltas intentionally remain fail-closed and may defer
  reconciliation; requests stay sequential and the existing 429 backoff bounds
  pressure. Same-address stream-generation/reconnect provenance remains to be
  audited separately.
- Prior turn commit hash: `37b57f74b317a7f9d440557ac0e43ed618c57efa`
- Next candidate: continue Track 8 by tracing `WsUserDataStreamParams`, iced
  subscription replacement, manager reconnects, broadcast lag/closure, and
  queued `WsUserDataUpdate` messages for the same address. Determine whether
  address plus subscription identity proves stream provenance or whether a
  runtime generation is required; preserve reconnect behavior and stream UX.

## Turn 17 — Reject Frames From Replaced User-Data Recipes

- Status: implemented; executable Rust validation environment-blocked
- Severity: Critical
- Scope: F-18 across user-data recipe identity, account/detail/cluster message
  context, runtime lifecycle generations, source-address validation, recipe
  input mutation sites, reconnect/lag/closure audit, docs, and regressions
- Invariant: a queued user-data frame may change account, automation, wallet,
  or cluster freshness state only while its exact address/purpose/dex recipe
  incarnation is still requested by current application state.
- Protected behavior: websocket topics/payloads, shared-manager refcounts and
  reconnect backoff, lag signaling and REST repair, account and cluster update
  semantics for current frames, order preparation/signing, automation policy,
  stream cadence outside actual consumer replacement, UI/copy, persistence,
  and secrets.
- Preconditions/event ordering: an iced recipe pulls a valid frame; an account
  reconnect or consumer topology/scope change replaces/removes that recipe;
  cancellation occurs while the old task awaits its application-queue send;
  the old same-address frame arrives after the new state/recipe. Address-only
  routing previously accepted it.
- Evidence: iced 0.14's tracker selects cancellation only against
  `stream.next()`, then sends a won item without another cancellation select.
  Independent old/new recipes can therefore overlap for one queued item. The
  manager's ordinary socket reconnect is different: it retains one broadcast
  receiver and orders old-socket frames, disconnect, resubscription, and new
  frames within that stream. Lag emits a reconciliation signal and requests
  the existing gated reconnect. Broadcast closure requires singleton manager
  task/command teardown and is not a normal reconnect transition. F-18 records
  the concrete source and consequences.
- Change: retained each complete `WsUserDataStreamParams` in subscription
  output, added its runtime generation to hash/equality, and required exact
  current params plus normalized source at all three update routes. Account
  sessions use a monotonic generation. Detail windows use unique per-address
  generations across close/reopen and connected-address exclusion, without
  rotating other addresses; config clear removes the runtime address map while
  preserving the allocator. Selected-cluster topology/profile changes rotate
  one shared cluster generation. Visible-dex changes rotate only the consumers
  whose recipe inputs change. No value is persisted or sent to Hyperliquid.
- Tests/checks:
  - Added an account regression proving a canceled same-address recipe's queued
    lag frame cannot mark the new session loading/reconciling.
  - Added a cluster regression proving a canceled recipe's full position frame
    cannot clear `stale` or refresh the timestamp used by close-size preflight.
  - Added parameter/source, detail close/reopen isolation, config-clear address
    cleanup, production same-account reconnect rotation, selected-cluster
    member rotation, and diagnostic-redaction coverage.
  - Focused attempts for
    `queued_same_address_frame_from_replaced_account_stream_is_ignored`,
    `queued_position_frame_from_replaced_cluster_stream_cannot_mark_snapshot_fresh`,
    `reopened_wallet_detail_gets_new_generation_without_rotating_other_address`,
    and `address_bearing_message_debug_redacts_values` all stopped in
    `alsa-sys` before Kerosene compilation because `pkg-config` could not find
    the system `alsa.pc` package. A broader `queued_` attempt stopped at the
    same boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: this subscription change does not justify
    launching the app against an unknown local credential/live-trading config,
    and the same ALSA dependency blocks compilation first.
- Compatibility/UX assessment: every current frame follows its identical prior
  handler. Rotations coincide with a recipe input/session change that already
  removes or replaces that consumer, except same-address reconnect where the
  explicit reconnect now creates the required ownership boundary and its
  existing full REST refresh remains authoritative. No view, string, control,
  schema, stored config, order field, retry, or timer changed.
- Residual risk: formatting, message/call-site inventory, every production
  recipe-input mutation, source normalization, generation lifecycle, manager
  ordering, lag and closure, redaction, and full diff review pass. The crate
  has not type-checked; focused/full tests and clippy must execute on a host
  with ALSA development metadata. Terminal singleton-manager task failure is
  reserved for the later startup/shutdown fault-handling track.
- Prior turn commit hash: `8c4bd540d135796c2aa3a20f6757f18848047709`
- Next candidate: begin the remaining Track 6 TWAP audit by tracing scheduler
  ticks, pending child creation, deterministic CLOID/retry identity, reconnect
  and delayed result/status ordering, unexpected resting-child cancellation,
  fill attribution, stop, completion, and archive. Preserve slice sizes,
  cadence, price bounds, retry policy, and all visible lifecycle behavior.

## Turn 18 — Claim TWAP Status Results by Exact Attempt

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: F-19 across TWAP status-task dispatch, message context, retry
  ownership, fill-reconciliation cleanup, diagnostics, docs, and adversarial
  ordering regressions
- Invariant: at most one status lookup owns the current TWAP child CLOID/retry
  attempt, and only its exact result may mutate retry or reconciliation state.
- Protected behavior: deterministic child CLOIDs, IOC placement/cancel request
  construction, slice sizing, pricing, cadence, randomization, book gates,
  retry limits/delays, account-fill confirmation, stop/timeout/archive paths,
  order status text, UI, persistence, and secrets.
- Audit evidence: scheduler ticks synchronously claim `pending_op` before
  returning a place task; direct placement results require exact slice index
  and retry count; child CLOIDs remain deterministic; account fill attribution
  requires OID plus exact coin/side and aggregates by per-child maxima/sums;
  stop and deadline paths cannot schedule while pending/status reconciliation
  remains. Status repair was the uncovered recurring lane: its tasks/results
  carried only TWAP ID and CLOID, while retries and later account-fill proof
  reused mutable state under that same CLOID. F-19 records the exact ordering
  failure and consequence.
- Change: added a runtime-only optional armed status attempt. Both immediate and
  delayed status helpers claim that owner before creating a task and refuse a
  second owner. `TwapOrderStatusLoaded` carries the attempt; the handler
  requires and consumes exact CLOID/attempt ownership before applying any
  result. Retry scheduling synchronously arms the next existing retry count.
  Account-fill resolution and timeout cleanup clear ownership defensively.
  Custom debug output exposes only a boolean presence value.
- Tests/checks:
  - Added regressions proving duplicate dispatch creates one task, a duplicate
    missing result cannot consume retry budget/events twice, and a stale
    missing result cannot overwrite canceled/no-fill account reconciliation.
  - Updated filled, canceled, rejected, retry-exhaustion, stop, terminal,
    account-origin, and debug tests for exact attempt ownership and cleanup.
  - Focused attempts for
    `duplicate_status_dispatch_keeps_one_in_flight_owner`,
    `duplicate_status_result_cannot_consume_retry_budget_twice`,
    `stale_missing_status_cannot_override_no_fill_confirmation_phase`,
    `canceled_status_check_waits_for_account_fill_confirmation`,
    `running_twap_reconciliation_clears_status_metadata_after_fill`, and
    `duplicate_slice_result_cannot_consume_a_settled_attempt` all stopped in
    `alsa-sys` before Kerosene compilation because `pkg-config` could not find
    the system `alsa.pc` package.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: this internal state/message change does not
    justify launching an unknown local credential/live-trading configuration,
    and compilation is already blocked by ALSA metadata.
- Compatibility/UX assessment: every uniquely owned status result follows the
  identical classifier, retry, refresh, cancel, finish, and archive code. The
  attempt is local correlation only; it is not signed, sent to Hyperliquid,
  persisted, rendered, or used to alter timing. Only duplicate dispatches and
  duplicate/delayed results that do not own current state are ignored. No
  visible string, control, schema, order field, retry, timer, or normal task
  changed.
- Residual risk: formatting, field initialization, message producer/consumer
  inventory, status-result branches, fill/timeout cleanup, redacted debug,
  protected scheduler behavior, and full diff review pass. The crate has not
  type-checked; focused/full tests and clippy must execute on a host with ALSA
  development metadata. The target-correlated unexpected-child cancel retry
  trigger carries an attempt, but the direct cancel result does not; its
  duplicate/reversed-result behavior remains the next Track 6 candidate.
- Prior turn commit hash: `2cbd350d38f08989f21e3305e1f2b0355c2afba0`
- Next candidate: continue Track 6 by adversarially ordering TWAP unexpected-
  child cancel retry triggers and direct results for the same OID/CLOID. Prove
  whether target identity plus current retry state is sufficient, or add the
  smallest runtime attempt ownership needed to prevent an older failure or
  success from settling a newer retry; preserve the target-specific cancel,
  retry limit/delay, refresh, stop, and visible behavior.

## Turn 19 — Claim TWAP Unexpected-Cancel Attempts

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: F-20 across TWAP unexpected-resting cancellation entry, delayed retry
  dispatch, result context, bounded retry ownership, terminal guards,
  diagnostics, docs, and adversarial regressions
- Invariant: one target-specific cancellation retry attempt creates at most one
  in-flight task, and exactly one matching result may consume its retry budget
  or settle the pending child.
- Protected behavior: cancel-by-CLOID preference and OID fallback, signed
  request values, response classification, five-attempt limit, backoff delays,
  account refresh and balance invalidation, stop/archive handling, all status
  and event strings, scheduling, UI, persistence, and secrets.
- Preconditions/event ordering: an unexpected-resting cancel returns an
  ambiguous/error result and schedules the next retry; the direct result is
  duplicated or delayed past the next dispatch, or the same retry-due message
  is delivered twice while its target and shared counter still match. Target
  checks previously accepted the former and counter checks dispatched both
  copies of the latter.
- Evidence: the retry trigger carried `attempt` but did not claim in-flight
  ownership. Every cancel task result discarded that attempt, and the handler
  mutated shared `cancel_retries` using only the long-lived OID/CLOID target.
  Repeating one failure enough times could reach terminal error without the
  corresponding number of owned attempts. F-20 records the full call graph,
  ordering, consequence, and protected semantics.
- Change: added a runtime-only optional in-flight cancel attempt. A shared
  arming method atomically checks non-terminal state, current retry count,
  exact target, and absence of another owner. Both initial cancellation paths
  and delayed retry dispatch use it. `TwapUnexpectedCancelResult` now carries
  the captured attempt; its handler requires the exact target/retry/owner and
  consumes ownership before executing the unchanged classifier. Scheduled
  backoff deliberately has no owner until the due message dispatches. Debug
  output retains only a boolean presence field.
- Tests/checks:
  - Added regressions proving a duplicated retry trigger dispatches once, a
    duplicated ambiguous result consumes one retry/event/backoff transition,
    a near-limit duplicate error cannot falsely exhaust the retry budget, and a
    stale attempt-zero success cannot settle in-flight attempt one.
  - Added a current-attempt retry success characterization and asserted that
    both direct resting placement and open status reconciliation arm the
    initial cancellation owner.
  - Updated existing success, ambiguity, contradiction, transport redaction,
    exhaustion, target mismatch, stale trigger, and terminal trigger cases for
    exact ownership.
  - Focused attempts for
    `duplicate_unexpected_cancel_retry_due_dispatches_once`,
    `duplicate_unexpected_cancel_result_cannot_consume_retry_budget_twice`,
    `duplicate_prior_attempt_error_cannot_falsely_exhaust_cancel_retries`,
    `stale_unexpected_cancel_result_cannot_settle_newer_attempt`,
    `current_unexpected_cancel_retry_result_settles_attempt`,
    `unexpected_resting_slice_arms_exact_cancel_attempt`,
    `open_status_check_after_stop_keeps_twap_stopping_and_requests_cancel`, and
    `exhausted_transport_unexpected_child_cancel_redacts_error_event` all
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - The pre-edit focused
    `unexpected_cancel_retry_due_ignores_stale_attempt` attempt and baseline
    `cargo check` stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - Post-edit `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: this runtime result-correlation change does
    not justify launching an unknown local credential/live-trading setup, and
    compilation is already blocked by ALSA metadata.
- Compatibility/UX assessment: every uniquely owned initial or retry result
  follows the identical target-specific request, classifier, backoff, refresh,
  finish, and terminal path. The new attempt is local correlation only. Only a
  duplicate dispatch or a duplicate/delayed result that does not own current
  state is ignored. No visible string, control, schema, payload, retry, timer,
  or normal interaction changed.
- Residual risk: formatting, all state initializers, message producers and the
  sole consumer, both initial entry paths, retry dispatch, result branches,
  stop/terminal behavior, redacted diagnostics, and full diff review pass. The
  crate has not type-checked; focused/full tests and clippy must execute on a
  host with ALSA development metadata. A fill can reconcile before the cancel
  result and the latter currently rewrites the child status; that separate
  monotonic fill-state ordering requires the next Track 6 audit.
- Prior turn commit hash: `692cc46c6a6a517a89302ff6aaed05aaa6b00eae`
- Next candidate: continue Track 6 by ordering account/user-stream fill
  reconciliation before and after unexpected-child cancel results. Verify that
  a confirmed child fill and aggregate fill accounting remain monotonic while
  the resting remainder still reaches the identical cancel/finish path; do not
  change cancellation, partial-fill, scheduling, or visible policy.

## Turn 20 — Characterize TWAP Fill/Cancel Ordering

- Status: audited; financial invariants characterized; F-21 deferred for an
  explicit UX/history semantics decision; executable Rust validation
  environment-blocked
- Severity: Medium for F-21; no Critical/High financial-state defect confirmed
- Scope: TWAP authoritative account/user-stream fills versus confirmed
  unexpected-child cancellation, partial and full fill permutations, repeated
  fill delivery, scheduling/completion, key scrubbing, and terminal history
- Invariant: fill quantity, average price, fee, remaining target, next
  lifecycle state, completion, and archived financial metrics must be
  independent of whether fill reconciliation or cancel acknowledgement arrives
  first.
- Protected behavior: exact OID/coin/side fill attribution, stable fill
  deduplication, child/aggregate max-and-sum accounting, target-specific cancel,
  retry/refresh/stop behavior, next-slice cadence and sizing, current child
  labels and event copy, terminal history format, persistence, and secrets.
- Preconditions/event ordering: an IOC child rests; the cancel task is already
  owned; the account/user stream proves a partial or full fill; the confirmed
  cancel result and fill application are handled in both possible orders. A
  repeated fill snapshot then tests idempotence after both paths converge.
- Evidence: fill calculations depend on retained per-child metrics, not the
  child status label. Cancel success changes pending/retry state and the child
  label but never resets fill metrics. Scheduling derives its next size from
  aggregate remaining target. Terminal snapshots separately copy numeric fill
  metrics and the visible label. F-21 records the exact source and rendering
  boundaries.
- Change: added two adversarial characterization tests only. The partial test
  proves both orders converge on identical `0.25 / 0.75` child/aggregate
  accounting, price, fee, waiting state, pending cleanup, and retry state, then
  reapplies the same fill without double counting. The full test proves both
  orders converge on completion, zero remainder, key scrubbing, and identical
  archived target/fill/average/fee values. It explicitly captures the current
  order-dependent `Canceled`/`Filled` child labels. No production source or
  documentation contract changed.
- Tests/checks:
  - Pre-edit attempts for
    `confirmed_unexpected_child_cancel_clears_pending_cancel`,
    `running_twap_reconciliation_clears_status_metadata_after_fill`, and
    baseline `cargo check` stopped in `alsa-sys` before Kerosene compilation
    because `pkg-config` could not find the system `alsa.pc` package.
  - Focused attempts for
    `partial_fill_accounting_is_monotonic_across_cancel_result_order`,
    `terminal_fill_history_keeps_financial_metrics_across_cancel_result_order`,
    `twap_fill_reconciliation_deduplicates_fills_by_stable_identity`, and
    `current_unexpected_cancel_retry_result_settles_attempt` stopped at the same
    dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - Post-edit `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: this turn changes tests and the audit ledger
    only, and compilation is already blocked by ALSA metadata.
- Compatibility/UX assessment: tests and ledger only. Current live/history
  labels, event ordering, order values, cancellation behavior, timing, views,
  schema, stored data, and secrets are untouched. The explicit deferral avoids
  silently selecting a new visible meaning for a partially filled and canceled
  child.
- Residual risk: source and view/history tracing plus the new permutations
  establish the intended financial invariants, but the crate has not
  type-checked and the tests cannot execute without ALSA metadata. F-21 remains
  an approved-decision dependency, not an active production-safety gap.
- Prior turn commit hash: `351f90fe1b36316d9e0d312796c121683e472625`
- Next candidate: finish Track 6 terminalization by adversarially ordering full
  fill completion, stop requests, successful/failed in-flight status and cancel
  results, retry-due messages, repeated archive upserts, and captured-key
  scrubbing. Prove no terminal TWAP can schedule or dispatch new exchange work
  and that delayed results cannot corrupt its financial history.

## Turn 21 — Suppress Post-Terminal TWAP Cancel Retry Work

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium defense-in-depth hardening; no current financial-state
  corruption or duplicate exchange mutation was confirmed
- Scope: remaining Track 6 terminalization audit across full-fill completion,
  stop/in-flight resolution, delayed status and cancel results, cancel retry-due
  delivery, scheduler/timer eligibility, repeated history upserts, and captured
  agent-key scrubbing
- Invariant: once authoritative fills or lifecycle resolution make a TWAP
  terminal, it cannot schedule or dispatch new exchange work; delayed results
  may retain required read reconciliation but cannot regress terminal financial
  history or extend signing-key lifetime.
- Protected behavior: active-order cancel target selection, signed payload,
  retry maximum/backoff, result classification, retry counters, pending target,
  child status/summary, immediate origin-account refresh, stop handling, fill
  attribution, slice sizes/cadence/randomization/price gates, status/event copy,
  advanced-history values/schema/copy, views, persistence, and secret storage.
- Preconditions/event ordering: an unexpected-resting child has exact cancel
  attempt zero in flight; a complete account fill arrives first and completes
  the target; terminal archive persists the numeric snapshot and clears the
  key; then an ambiguous exchange response or transport error arrives and asks
  for the normal active-order cancel retry.
- Evidence: terminal status-result handlers, status and cancel arming helpers,
  the scheduler predicates, timer subscription, book handlers, finish/timeout
  paths, and account-fill reconciliation all reject or preserve terminal state.
  Repeated terminal archive calls use a stable ID upsert and re-clear the key.
  The one producer mismatch was in the cancel-result retry branch: it batched a
  delayed retry trigger even though the trigger consumer was guaranteed to
  reject terminal state. F-22 records exact source evidence and risk.
- Change: added a terminal check at the cancel-result retry-task boundary. A
  retryable result after full-fill completion now returns the same immediate
  origin-account refresh without the delayed retry trigger. Active TWAPs still
  receive the exact existing retry delay and task. Added adversarial coverage
  for both retry-producing result classes, immutable serialized terminal
  history, financial metrics, manual retry-due rejection, all terminal scheduler
  statuses, delayed terminal status-result rejection, repeated history upsert,
  and key scrubbing. Updated the internal trading architecture documentation.
- Tests/checks:
  - The pre-edit focused attempt for
    `unexpected_cancel_retry_due_ignores_terminal_twap` stopped in `alsa-sys`
    before Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` package.
  - The post-edit focused attempt for
    `terminal_fill_cancel_retry_outcomes_refresh_without_scheduling_retry`
    stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: startup, window, and subscription plumbing
    are unchanged, and compilation is already blocked by ALSA metadata.
- Compatibility/UX assessment: the only runtime difference is removal of a
  delayed retry message after the TWAP is already terminal; that message
  previously woke and no-op'd. Immediate reconciliation timing and every
  user-visible/state/persistence value are preserved. No schema, stored value,
  interaction, order payload, order timing, or copy changed.
- Residual risk: source tracing and the added assertions establish the
  terminal contract, but the crate has not type-checked and tests cannot execute
  on this host without ALSA development metadata. F-21 remains deliberately
  deferred for explicit visible/history semantics approval. No overall campaign
  completion claim is made.
- Prior turn commit hash: `7ce27516fadc2a43306e536993fe8ed680893e4f`
- Next candidate: begin Track 9 restart, shutdown, and secret-lifetime audit.
  Trace active/in-flight Chase and TWAP persistence exclusions plus disconnect,
  profile deletion, clear-config, and shutdown cleanup; verify task contexts,
  uncertainty, keys, and redacted diagnostics cannot survive or leak across
  those boundaries.

## Turn 22 — Fence Automation Progress During Final Exit

- Status: implemented; executable Rust validation environment-blocked
- Severity: High
- Scope: first Track 9 batch covering config/boot exclusion of live order state,
  main-window close plus final-save lifecycle, Chase place/reprice/size
  correction, TWAP slice dispatch, save-failure recovery, and cancellation
  cleanup during exit
- Invariant: after main-window closure transfers ownership to final persistence
  and process exit, no new exposure-progressing automation mutation may begin.
  Already-sent results must still reconcile, known exposure may still be
  canceled, and a failed save must not corrupt the exact queued automation
  state; F-24 separately owns whether that state may resume without a window.
- Protected behavior: config schema/defaults and terminal-history persistence;
  normal Chase/TWAP lifecycle, prices, sizes, cadence, randomization, limits,
  CLOIDs/OIDs, retry/cooldown rules, account and symbol gates, status/cancel
  reconciliation, active-order cleanup, save failure feedback, all UI/copy, and
  key storage/redaction.
- Preconditions/event ordering: live Chase/TWAP state is runtime-only; a place,
  modify, size correction, or slice becomes due; a config save is pending; the
  main window closes; the daemon remains alive awaiting the save; and an
  automation task/message arrives before `iced::exit()` executes.
- Evidence: `KeroseneConfig` has only terminal advanced-order history, config
  snapshots explicitly blank secret fields and omit every active/pending order
  owner, and boot initializes pending one-shot/cancel/move/nuke/leverage,
  Chase, and TWAP state empty. Account disconnect/switch, saved-profile delete,
  and config clear already block pending trading activity/active automation or
  defer destructive runtime reset. F-23 identifies the distinct live final-save
  window in which progress gates previously ignored `config_save_exit_requested`.
- Change: documented the existing exit flag as the runtime owner/fence and keep
  it armed through both successful and immediate exit tasks. Added a Chase
  progress predicate for placement, book reprice, reconciled modify, and size
  correction while retaining the broader readiness predicate for status/cancel
  cleanup. Added the exit condition to the TWAP slice dispatch gate. Queue and
  pending state are not cleared, so the existing failed-save path clears the
  flag and normal gates resume exact work. Updated internal architecture docs.
- Tests/checks:
  - The pre-edit focused attempt for
    `chase_reprice_tick_runs_queued_size_correction` stopped in `alsa-sys`
    before Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` package.
  - Focused post-edit attempts for
    `chase_place_waits_during_pending_exit_and_resumes_if_exit_aborts`,
    `due_twap_slice_waits_during_pending_exit_and_resumes_if_exit_aborts`, and
    `successful_exit_keeps_automation_fence_armed_until_runtime_exits` stopped
    at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test could not be run because compilation is already blocked
    by ALSA metadata; runtime close/exit behavior therefore remains executable-
    validation debt on a provisioned host.
- Compatibility/UX assessment: the window is already closed when the fence is
  active. Only a not-yet-dispatched Chase/TWAP progress mutation waits; existing
  reconciliation and cancellation keep their behavior. A failed save resets
  the flag and exact queued values can resume under the existing policy; F-24
  documents that policy's headless risk without changing it. No visible
  control, feedback, normal interaction timing, order payload/strategy,
  persisted value, schema, secret, or history meaning changed.
- Residual risk: Rust execution remains blocked. Track 9 still needs explicit
  audit of queued one-shot/NUKE/cluster/leverage intent after close,
  close-versus-config-clear completion, remaining disconnect/profile secret
  lifetimes, and repository-wide order/account diagnostic redaction. F-24's
  failed-save/headless policy requires approval. No overall completion claim is
  made.
- Prior turn commit hash: `10d20f18d7755010d3f145e88a5b70442de67bec`
- Next candidate: continue Track 9 at the root close boundary. Adversarially
  order queued one-shot, cancel/move, close/NUKE, wallet-cluster, and leverage
  intent messages against `WindowClosed`; inspect config-clear completion after
  a main-window close; then prove no exposure-increasing dispatch or headless
  runtime survives the successful exit owner while preserving result
  reconciliation and cleanup cancellation. Keep F-24 deferred until its visible
  failed-save policy is approved.

## Turn 23 — Preserve Final-Exit Ownership Across All Mutation Intents and Clear

- Status: implemented; executable Rust validation environment-blocked
- Severity: High (F-25 and F-26)
- Scope: root message routing for every current signed mutation entry and new
  config-clear requests; direct Alfred and wallet-cluster fan-out; one-shot,
  close/NUKE, move, leverage,
  Chase/TWAP start/adoption; explicit cancel/stop and result reconciliation;
  config-clear request/start/result; deferred save-to-clear handoff; daemon exit
- Invariant: after `WindowClosed` transfers the main daemon to final
  persistence, no fresh signed mutation intent may enter a feature route, the
  same owner must survive either save or clear until `iced::exit()`, and work
  already sent must retain exact result/status/cancel authority.
- Protected behavior: all pre-close order validation, payloads, identifiers,
  account/key capture, prices, sizes, TIF/reduce-only policy, Chase/TWAP timing,
  cancellation and retry rules, result/status reconciliation, config-clear
  success/partial/deferred/error state handling, redaction, in-app clear UX,
  persistence/schema, views, and user-visible copy.
- Preconditions/event ordering: a fresh intent or clear command is queued; the
  main-window closed event runs first; final save or clear leaves the daemon
  alive; then the queued message, save-to-clear transition, clear-start race,
  or clear result runs before the process-exit task.
- Evidence: the repository-wide signed-task inventory still has only shared
  place/cancel/modify wrappers and leverage update. Their initiating routes are
  the explicit message variants now enumerated by the root predicate, plus
  Alfred and cluster direct calls reached by their enumerated command messages.
  Cancel/stop and every result/status producer are outside the predicate. F-25
  and F-26 record the exact old ordering gaps and file/symbol evidence.
- Change: added a root fresh-mutation/destructive-persistence classifier and
  final-exit early return. Preserved exit ownership through any clear requested
  before close, direct start, save-result handoff, start-time refusal, and every
  result branch. An
  exit-owned clear result batches its unchanged normal task with `iced::exit()`;
  an exit-owned start refusal exits after retaining any already-dispatched
  trading owner. Updated the runtime field and trading architecture docs.
- Tests/checks:
  - The pre-edit focused
    `successful_exit_keeps_automation_fence_armed_until_runtime_exits` attempt
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - Post-edit focused attempts for `final_exit_fence`,
    `pending_config_clear_keeps_exit_fence_armed_until_clear_finishes`, and
    `exit_owned_config_clear` stopped at the same dependency boundary.
  - Added explicit classification coverage for every fresh mutation surface,
  complementary reconciliation/cleanup coverage, and root pre-route regressions
  for exchange and new-clear intents. Added clear ownership coverage for
  immediate/deferred start, a request helper with an existing owner, a new-
  pending-work start race, successful result, and redacted error result.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test could not be run because compilation is blocked by ALSA
    metadata; no live exchange request or credential-bearing operation ran.
- Compatibility/UX assessment: normal in-app routes and config clear start with
  no exit owner and are byte-for-byte/branch-for-branch unchanged. Result,
  status, cancel, and stop messages remain routable during final exit. Only a
  fresh exchange mutation or new clear request delivered after the main window
  has already closed is discarded. A clear already in progress when that close
  occurs now terminates the daemon as requested, including after existing
  redacted failure handling. No visible string, control, schema, stored value,
  order policy, or normal timing changed.
- Residual risk: the crate has not type-checked and focused tests cannot execute
  without ALSA development metadata. Future signed mutation message classes
  must join the explicit root classifier. Real close/clear/auxiliary-window task
  ordering remains GUI smoke-test debt. F-24's failed-final-save policy is
  unchanged and still requires approval. Remaining Track 9 secret-lifetime and
  diagnostic-redaction work is incomplete.
- Prior turn commit hash: `94b291bbed96d03036f779bbd7e653cde9cd43c6`
- Next candidate: continue Track 9 through account disconnect/switch, saved-
  profile deletion, config clear, and pending result contexts. Prove captured
  keys and account/order identities are scrubbed only after their uncertainty
  owner is safely terminal or intentionally abandoned, then audit `Debug`,
  toast/error, snapshot, and progress-log paths for sensitive order/account
  material without changing visible trading semantics.

## Turn 24 — Repair Ghost-Profile Stream Invalidation Ownership

- Status: implemented; executable Rust validation environment-blocked
- Severity: Medium (F-27)
- Scope: initial Track 9 disconnect/switch/profile/clear key-owner inventory;
  ghost-profile create/forget lifecycle; selected wallet-cluster user-data
  generation; prior Turn 17 stale-stream hardening integrity
- Invariant: a profile-removal operation must compute and apply selected-cluster
  stream invalidation before removing that exact profile; a newly created
  unrelated ghost identity must not own removal state, and the checkout must
  remain type-correct.
- Protected behavior: all account-switch/disconnect/pending-request/automation
  gates, ghost profile creation/removal, active-account fallback, cluster
  membership/config, generation arithmetic, subscription parameters, queued
  frame validation, persistence, UI/status copy, and all trading semantics.
- Preconditions/event ordering: Turn 17 added generation rotation for selected-
  cluster profile binding changes; its ghost-path predicate was inserted in the
  creation function, while its conditional use was inserted in the removal
  function. The system ALSA discovery failure prevented Rust from resolving the
  out-of-scope name.
- Evidence: pending one-shot, NUKE, leverage, cancel/move status/context, cluster
  execution, indicator, and HUD owners are included in the shared pending gate;
  account switch, disconnect, profile delete, ghost forget, and config clear
  call that gate before destructive cleanup. Chase and uncertain TWAP gates
  retain captured-key owners; safely stoppable TWAPs archive and scrub their
  captured key. During that inventory, `git blame` and the current source
  isolated F-27 to the prior user-stream generation commit.
- Change: moved `selected_cluster_profile_removed` from ghost creation to
  `forget_ghost_account_task` immediately after all refusal gates and before
  removal. Added a focused regression proving a selected-cluster ghost removal
  advances the runtime recipe generation once. No documentation contract
  changed because the already-documented Turn 17 behavior is restored, not
  extended.
- Tests/checks:
  - The pre-edit focused
    `forget_ghost_wallet_is_blocked_while_twap_order_is_active` attempt stopped
    in `alsa-sys` before Kerosene compilation because `pkg-config` could not find
    the system `alsa.pc` package.
  - Post-edit focused attempts for
    `forgetting_selected_cluster_ghost_rotates_cluster_stream_generation` and
    `account_state::switching::ghost::tests` stopped at the same dependency
    boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: this profile-generation repair does not
    change startup/window plumbing, and compilation is already blocked. No live
    exchange or credential-bearing operation ran.
- Compatibility/UX assessment: creation loses an unused always-false local
  calculation. Removal performs the exact intended pre-removal predicate and
  existing conditional wrapping increment. No visible output, interaction,
  config, profile/cluster contents, subscription address, order data, timing,
  or secret lifetime changes.
- Residual risk: Kerosene still has not type-checked on this host, so the repair
  and regression require execution where ALSA development metadata is present.
  The initial key-owner inventory found no safe source change beyond F-27, but
  Track 9 is not complete: terminal/nonterminal automation key scrubbing,
  profile deletion rollback lifetimes, and repository-wide diagnostic redaction
  need dedicated follow-up.
- Prior turn commit hash: `52451751c8545ece90f13675b95b2aa39322b2e8`
- Next candidate: resume Track 9 at captured-key terminalization. Enumerate
  every Chase/TWAP terminal assignment and removal/archive call, then
  adversarially exercise disconnect, switch, profile deletion, config clear,
  and delayed result delivery to prove keys persist only while exact cleanup or
  uncertainty still needs them. Follow with repository-wide order/account
  `Debug` and external-error redaction once key lifetime is closed.

## Turn 25 — Scrub Keys on Final TWAP Planning Skips

- Status: F-28 implemented; F-29 audited and deferred; executable Rust
  validation environment-blocked
- Severity: Medium
- Scope: complete Chase/TWAP captured-key terminal-assignment map; initial and
  retry slice skip exhaustion; nonterminal scheduling; terminal status/events;
  advanced-history and persistence compatibility
- Invariant: a terminal automation state with no in-flight mutation,
  reconciliation, or target-specific cleanup owner must not retain a usable
  signing key; a nonterminal TWAP must retain its captured key for later slices.
- Protected behavior: exact slice sizing/counts, cadence, randomization, price
  bounds, notional/precision gates, retry state, child status, events and visible
  strings, zero exchange dispatch on planning skips, terminal predicates,
  advanced-history visibility, persistence, and all other Chase/TWAP archive
  semantics.
- Preconditions/event ordering: an initial final slice or retry of the final
  slice is rejected before dispatch; the skip helper updates existing
  attempt/child/event state; `schedule_after_attempt` consumes the last slot and
  assigns a terminal status; the caller returns without the ordinary archive
  boundary.
- Evidence: repository-wide terminal-assignment and scheduler-call searches
  prove Chase terminality removes/drops its captured key and every TWAP
  result/status/fill/stop/deadline/reconciliation terminal path archives and
  scrubs. Only `record_twap_skip` and `record_twap_retry_skip` could terminalize
  through the scheduler and return without either operation. F-28 and F-29
  record exact source and behavior evidence.
- Change: added captured-key clearing to the canonical scheduler's two existing
  terminal assignments. Added end-to-end initial/retry final-skip tests plus a
  nonterminal control. Kept the current no-history/no-persistence behavior and
  documented it accurately; F-29 defers any visible/persisted history change.
- Tests/checks:
  - Focused attempts for
    `nonterminal_slice_skip_retains_key_for_the_next_slice`,
    `final_initial_slice_skip_scrubs_key_without_changing_history_visibility`,
    `final_retry_slice_skip_scrubs_key_without_changing_history_visibility`, and
    the nearby `archive_twap_if_terminal_scrubs_runtime_agent_key` regression
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup/window/subscription path changed,
    compilation is already blocked, and no live exchange or credential-bearing
    operation was run.
- Compatibility/UX assessment: all planning/scheduler/event/status values are
  unchanged. The only runtime difference is that the captured key becomes empty
  when the existing skip transition is terminal; nonterminal skips retain it.
  Tests pin the established absence of a new advanced-history entry, and no
  config save, visible copy, task count, timing, or trading policy changes.
- Residual risk: the crate has not type-checked and tests cannot execute without
  ALSA development metadata. F-29 requires a decision if complete advanced-
  history coverage is desired. Track 9 still needs profile-deletion rollback
  key-copy lifetime and repository-wide account/order diagnostic redaction
  review before a final safety verdict.
- Prior turn commit hash: `bca92498666d6ec2beef3e5122bccb23a9d6bf20`
- Next candidate: finish destructive-profile secret lifetime by tracing every
  rollback/snapshot clone through encrypted and keychain delete success/failure,
  proving removed keys are zeroized or still deliberately owned and never enter
  diagnostics. Then perform the repository-wide order/account `Debug`, external
  error, toast, snapshot, and progress-log redaction audit.

## Turn 26 — Move and Scrub Saved-Profile Delete Keys

- Status: F-30 implemented; F-31 audited and deferred; executable Rust
  validation environment-blocked
- Severity: Medium
- Scope: saved-profile deletion in both credential modes; encrypted payload
  preparation; rollback snapshots; raw agent/legacy-profile keys; config save
  success, ordinary failure, and installed-snapshot error; OS keychain cleanup;
  redacted diagnostics
- Invariant: the removed profile is the only raw-key owner during deletion. It
  may survive only while an ordinary failed durable save can restore it, must
  move back without cloning on that failure, and must be scrubbed before
  post-commit cleanup receives identity-only input.
- Protected behavior: pending-trading and automation gates; ghost routing;
  encrypted unlock/password gates; encrypted payload contents; first/second
  config-save order; pending keychain cleanup intent and retry; active/fallback
  indices; journal/hidden-position/cluster-stream cleanup and rollback; account
  switch/disconnect tasks; every success/failure toast and status; config schema;
  storage mode; and all trading semantics.
- Preconditions/event ordering: after encrypted preparation (if selected), the
  rollback captures non-secret state, OS mode stages its durable cleanup intent,
  the profile is removed by move, and the first config save runs. Ordinary
  failure restores; success commits deletion and proceeds to keychain cleanup.
  F-31 separately characterizes the marker saying replacement installed but a
  post-install filesystem step failed.
- Evidence: repository-wide saved-delete/message/caller searches found one UI
  route and one canonical deletion function. Config snapshots construct empty
  secret fields; encrypted payload members and serialized plaintext are
  zeroizing; keychain bundle/legacy cleanup needs only secret ID and redacts
  external errors. F-30/F-31 contain exact owner and failure-phase references.
- Change: removed both the target-profile cleanup clone and complete-accounts
  rollback clone. The removed profile now moves into a rollback slot, returns at
  its original index on ordinary failure, and is explicitly scrubbed after
  successful durability before ID-only cleanup. Added allocation-identity
  regression coverage, an installed-snapshot characterization, and the security
  architecture note. F-31 leaves exceptional policy unchanged.
- Tests/checks:
  - Focused attempts for
    `os_keychain_account_delete_save_failure_does_not_clear_keychain`,
    `installed_snapshot_delete_error_preserves_failure_behavior_pending_policy`,
    and the complete `account_state::switching::saved_delete::tests` module
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup/window/subscription path changed,
    compilation is already blocked, and no live exchange or credential-bearing
    operation was run.
- Compatibility/UX assessment: all branches, call ordering, persistence
  snapshots, retries, state values, tasks, and strings are unchanged. On
  ordinary failure, the same profile and key bytes return through the same
  rollback, but without a clone. On success, zeroization happens earlier and
  cleanup receives the same ID rather than an unused raw profile. The F-31
  marker retains its exact current visible and state behavior.
- Residual risk: Kerosene has not type-checked on this host. F-31 needs an
  explicit durability/feedback decision. Address-rebinding credential updates
  still use broader transient account snapshots by design and should be checked
  during the remaining secret-copy/redaction pass; repository-wide account/order
  diagnostics remain incomplete.
- Prior turn commit hash: `a2f8abf4ac71502e4a619495b6f53eb060752861`
- Next candidate: complete Track 9's repository-wide sensitive diagnostic
  review, starting with account/profile/order `Debug` implementations and
  external error-to-toast/status boundaries, while also checking address-
  rebinding snapshot scopes for unnecessary raw-key copies. Add redaction
  regressions only for concrete leaks and preserve every visible string.

## Turn 27 — Move and Scrub Address-Rebind Keys

- Status: F-32 implemented; F-31 scope expanded and deferred; executable Rust
  validation environment-blocked
- Severity: Medium
- Scope: both active-profile wallet-address rebind routes; OS-keychain and
  encrypted-config persistence; active profile/draft key ownership; ordinary
  and installed-snapshot failures; rollback save; profile/cluster identity;
  redacted status
- Invariant: an address rebind has one rollback owner for the original active
  profile key and draft key. Persistence may receive one snapshot only after the
  active key is absent; failure must restore the same allocations and success
  must scrub them before later account/stream/task work.
- Protected behavior: normalization and case-only edits; all pending request and
  Chase/TWAP gates; ghost behavior; wallet metadata and key-binding removal;
  encrypted/keychain payloads and warnings; first/rollback saves; connected
  address, stream generations, percentage state, chart/account clearing, fetch
  tasks, status copy, config schema, and every success/failure result.
- Preconditions/event ordering: the typed edit route starts from the stored
  address input, while connect normalizes the requested address first. On a true
  binding change both clear the active key before config/credential persistence;
  an ordinary failure restores before an optional rollback save, and success
  continues with established rebind/account-refresh behavior.
- Evidence: repository-wide clone and persistence-hook searches found the same
  four-copy pattern in exactly these two rebind transactions. The shared helper
  and scoped snapshots now make owner transitions explicit. F-31/F-32 record
  parent-source, failure-phase, and regression evidence.
- Change: added a private account-update rollback helper, replaced complete
  account/profile/key-input staging clones with moved owners, bounded the one
  required persisted-account snapshot to the persistence call, and scrubbed
  rollback-only key and identity copies at commit. Added allocation-identity
  assertions across encrypted/keychain failure and installed-snapshot
  characterizations across both routes. Updated security documentation.
- Tests/checks:
  - Pre-edit focused attempts for
    `wallet_address_edit_os_keychain_failure_rolls_back_saved_metadata` and
    `connect_wallet_os_keychain_failure_rolls_back_saved_metadata_and_does_not_connect`
    stopped in `alsa-sys` before Kerosene compilation because `pkg-config`
    could not find the system `alsa.pc` package.
  - Post-edit focused attempts for the `wallet_address` profile tests and
    `connect_wallet` connection tests stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup/window/subscription route changed,
    compilation is already blocked, and no live exchange or credential-bearing
    operation was run.
- Compatibility/UX assessment: the same canonical profile becomes keyless
  before the same persistence calls. Failure restores identical bytes at the
  same state boundary, now via the original allocations; success reaches the
  same storage/status/stream/account tasks after earlier scrubbing. No payload,
  call order, task, timing policy, visible string, state value, or stored format
  intentionally changes. F-31 behavior remains characterized and unchanged.
- Residual risk: Kerosene has not type-checked on this host. F-31 still requires
  a durability/feedback decision. Explicit agent-key saving retains a separate
  staged-account clone graph, and repository-wide account/order `Debug` plus
  external-error/toast redaction remain incomplete.
- Prior turn commit hash: `696f467b4487488e691afca8e224ed1f21fa4963`
- Next candidate: finish runtime credential-copy ownership in
  `save_active_account_credentials` so unrelated saved keys are not cloned more
  than the one synchronous storage snapshot requires. Then begin the repository-
  wide account/order `Debug`, external error, toast/status, snapshot, and
  progress-log redaction pass.

## Turn 28 — Commit One Staged Agent-Key Snapshot

- Status: F-33 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: explicit active-profile credential saving; old committed key, draft
  key input, saved/ghost profile snapshot, both storage modes, identity-safe
  commit, captured signing authority, and config-save scheduling
- Invariant: storage receives exactly one caller-owned saved-profile snapshot
  containing the draft key while the prior canonical key remains authoritative.
  Failure leaves both canonical and draft allocations unchanged; success moves
  the exact persisted key into the originating profile and promptly scrubs every
  other caller-staged key. Backend-required zeroizing buffers remain separate.
- Protected behavior: ghost rejection; changed-key pending/Chase/TWAP gates;
  unchanged-key saves; profile order and active selection; keychain/encrypted
  payloads, migration flags, and status text; captured signing context;
  canonical commit only after storage success; debounced config save; config
  schema; and all visible interactions.
- Preconditions/event ordering: after existing gates, build one caller-owned,
  ghost-filtered snapshot directly from runtime profiles, substituting the draft
  only at the active runtime index. Call the same synchronous storage boundary
  while canonical state stays old. Apply or discard the staged key from the
  returned boolean, destroy the snapshot, then request the same config save on
  success.
- Evidence: repository-wide call-site search found one explicit-save caller of
  `persist_active_profile_secrets_from_accounts`; same-address connect uses the
  separate one-snapshot wrapper. F-33 records the parent clone graph, backend
  contract, current source, and allocation assertions.
- Change: added a caller-owned persisted-snapshot constructor that substitutes
  the active draft without cloning the old committed key, refactored explicit
  save behind a testable synchronous persistence boundary, moved the successful
  staged key into canonical state, and dropped the remaining snapshot before
  config-save scheduling. Added success/failure allocation and authority
  regressions plus the security architecture note.
- Tests/checks:
  - The pre-edit focused attempt for
    `agent_key_save_commits_the_exact_persisted_snapshot_allocation` stopped in
    `alsa-sys` before Kerosene compilation because `pkg-config` could not find
    the system `alsa.pc` package.
  - The post-edit focused `account_update::profile::tests::agent_key_save`
    attempt stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup/window/subscription path changed,
    compilation is already blocked, and no live exchange or credential-bearing
    operation was run.
- Compatibility/UX assessment: persistence sees the same ordered profile set
  and exact key values. Canonical signing state remains old during the same
  backend call and changes only on the same success result. Failure state,
  statuses, tasks, config timing, and payloads are unchanged; successful state
  differs only in allocation provenance. No visible copy or stored format
  changes.
- Residual risk: Kerosene has not type-checked on this host. F-31 remains
  deferred. Account switching/add-account key-owner copies and repository-wide
  account/order `Debug`, external-error, toast/status, snapshot, and progress-
  log redaction remain to audit before Track 9 or the campaign can close.
- Prior turn commit hash: `fe49d17a57683d8d83bf56479fbe42cb2a661ec5`
- Next candidate: begin the repository-wide sensitive diagnostic audit. Enumerate
  every account/order/credential-bearing `Debug` implementation and derived
  `Debug` owner, then trace external errors into messages, statuses, toasts,
  snapshots, and logs. Fix only concrete leakage with redaction tests; include
  switching/add-account key copies in that owner inventory.

## Turn 29 — Close Nested Response Diagnostic Bypass

- Status: F-34 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: all `ExchangeResponse` model formatting layers, externally supplied
  response-type text, type-only summary, and unchanged classification consumers
- Invariant: exact nested exchange values remain available to lifecycle code,
  but no public `Debug` entry point may emit raw statuses, identifiers, order
  details, or arbitrary external error text. Recognized response-type summaries
  must remain exact; an unrecognized type must become value-neutral before
  status state.
- Protected behavior: JSON deserialization; raw fallback handling; error/effect,
  fill, cancel, modify, default, ambiguity, and IOC classifiers; OID/fill
  extraction; ordinary summaries and status copy; task/message shape; all
  reconciliation; wire requests; persistence; and every normal interaction.
- Preconditions/event ordering: deserialize first so classification retains the
  exact response. A diagnostic can then select the top-level value, inner value,
  or data value independently. Each formatter must summarize without traversing
  raw statuses. The no-data summary may display only a recognized protocol type
  or a value-neutral marker.
- Evidence: source/derive inventory and call-site tracing found nested field use
  only in response analysis, tests, and TWAP fill extraction; none depends on
  raw `Debug`. F-34 records the parent source, bypass, current source, and
  adversarial response shape without storing any real account/order material.
- Change: added redacted custom formatters to both public nested models, removed
  the private raw wire formatter, allowlisted response-type metadata at the
  top-level and summary boundaries, added the adversarial regression, and
  documented the response-layer policy.
- Tests/checks:
  - The pre-edit exact regression attempt stopped in `alsa-sys` before Kerosene
    compilation because `pkg-config` could not find the system `alsa.pc` file.
  - Post-edit exact and `signing::tests::responses` attempts stopped at the same
    dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup, subscription, window, or view
    path changed, compilation is already blocked, and no live exchange or
    credential-bearing operation was run.
- Compatibility/UX assessment: recognized top-level and type-only summaries
  remain byte-for-byte identical. Exact response fields and every consumer are
  unchanged. Only direct diagnostics and an unrecognized anomalous external
  type are replaced with a value-neutral marker; no normal copy, timing, state,
  payload, or behavior changes.
- Residual risk: Kerosene has not type-checked on this host. Local planner/state
  `Debug`, direct order-intent message payloads, other external status/error
  paths, snapshots, and progress logs remain to inspect. The switch-account
  pre-gate profile clone and add-account staged-key copies are confirmed owner
  candidates for later narrow allocation regressions.
- Prior turn commit hash: `a1a2f0ce202aadd7b33cd43165476f826f1a21fa`
- Next candidate: remove the full credential-profile clone performed before
  same-index/pending/Chase/TWAP switch gates, proving blocked attempts create no
  raw key owner and successful switching retains exactly its intentional
  profile plus key-input owners. Then audit the add-account staging graph and
  continue the remaining local planner/message diagnostic inventory.

## Turn 30 — Gate Account Credential Capture

- Status: F-35 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: ordinary account-switch target ownership across every caller, all
  pre-switch blockers, saved/ghost targets, old-account cleanup, key-input
  transfer, and unchanged post-switch tasks
- Invariant: an invalid, same-profile, pending, Chase-blocked, or uncertain-TWAP
  switch captures no target credential. Once all gates pass, a saved switch
  creates one moveable key-input allocation from the canonical profile; a ghost
  switch creates no key copy and still scrubs any stray key.
- Protected behavior: index validation; all gate ordering and feedback; active
  TWAP stopping; connected/account/chart/portfolio/journal reset; active
  selection; address and key inputs; ghost status; deferred legacy loading;
  stream resets; config save timing; ConnectWallet/DisconnectWallet
  publication; hotkey, picker, add-account, ghost, and deletion-fallback
  callers; and every visible interaction.
- Preconditions/event ordering: validate index and no-op/gate conditions first,
  then stop ordinary active TWAPs and clear old connected state exactly as
  before. Only after that synchronous cleanup can the still-existing target
  profile be captured. Apply its identity/address and move or discard its
  zeroizing key before the unchanged stream/config/task tail.
- Evidence: repository-wide call-site search found one switch implementation
  used by every UI/internal caller. Parent/current source comparison proves the
  full pre-gate profile clone and second key clone are gone. F-35 records the
  exact owner graph and focused capture/allocation assertions.
- Change: introduced a minimal non-cloneable switch target and testable capture
  boundary, moved target capture after every blocker and old-account cleanup,
  omitted ghost keys, moved the saved target key into the input owner, added
  three focused ownership regressions, and documented the lifecycle rule.
- Tests/checks:
  - The pre-fix exact rejected-switch capture attempt stopped in `alsa-sys`
    before Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` file.
  - Post-fix exact rejected/success/ghost allocation tests and the full
    `account_state::switching::tests` module stopped at the same dependency
    boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup, subscription, window, or view
    route changed, compilation is already blocked, and no live exchange or
    credential-bearing operation was run.
- Compatibility/UX assessment: the same gates run in the same order with the
  same status/toast output. Once authorized, the same old-account stop/clear
  work precedes the same target values, stream reset, config request, and
  connect/disconnect message. Saved and ghost values are identical; only raw
  owner count and allocation provenance change. No visible copy, timing,
  trading, persistence, or interaction change is intentional.
- Residual risk: Kerosene has not type-checked on this host. Add-account and
  deferred legacy-key loading retain separate copy graphs; local planner/state
  and direct-message diagnostic redaction plus other external status/snapshot
  paths remain incomplete.
- Prior turn commit hash: `051b06c43524b73f3a051db1d1142d9056da3187`
- Next candidate: harden add-account submission so the window draft remains the
  failure authority, credential storage receives only the necessary saved-
  profile snapshot, and success moves rather than reclones the authorized key
  into canonical/first-account input state. Include locked/encrypted and
  keychain failure plus switch-on-add behavior before returning to the remaining
  diagnostic inventory.

## Turn 31 — Keep New Accounts Keyless Until Credential Commit

- Status: F-36 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: the complete add-account submission owner graph from draft validation
  through credential storage, canonical installation, first-account input, and
  ordinary/blocked switch-on-add
- Invariant: failed storage leaves the exact draft as authority and no installed
  keyed profile; storage sees exactly one caller-staged new key while canonical
  metadata remains unable to sign. Success moves that staging allocation into
  canonical state and reuses the verified draft only for the intentional first-
  account input owner.
- Protected behavior: exact address/key validation and errors; trimming and
  default name; saved/ghost profile counting; watch-only storage bypass;
  encrypted/keychain backend selection and payload; immediate encrypted config
  metadata; failure state/status; profile order; config scheduling; first-
  account journal/input/connect behavior; switch-on-add gates, state clearing,
  tasks, active source, and toasts; no schema, view, or normal interaction
  change.
- Preconditions/event ordering: validate while borrowing the draft, then create
  one caller-staged profile. For a keyed submission, clone existing saved
  profiles once for the required storage bundle, append the staged profile by
  move, and push only its keyless metadata shell into canonical state. Storage
  resolves synchronously. Remove/drop on failure or identity-check and replace
  the shell with the staged profile on success; only then schedule the
  established config save and close the draft window. The first-account special
  case claims the verified draft; ordinary switch-on-add continues through
  `switch_account_task` after the window draft is dropped.
- Evidence: F-36 records the parent/current owner graphs, atomic encrypted-save
  constraint, source boundaries, and focused allocation assertions. Call-site
  tracing reconfirmed `Message::AddAccountSubmit` is the only production entry
  and both storage modes converge on the injected synchronous boundary.
- Change: added a non-cloneable validated submission owner and private storage
  hook, installed a keyless provisional metadata shell, replaced it with the
  successful staged profile by move, moved the verified normalized draft into
  the first-account input, removed clone-only default counting, expanded
  failure and switch characterization tests, and documented the ownership
  contract.
- Tests/checks:
  - The pre-fix exact first-account allocation regression stopped in `alsa-sys`
    before Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` file.
  - Post-fix exact first-account success, keychain failure, draft-retarget, and
    Chase-blocked switch-on-add tests plus the full
    `account_update::add_window::tests` module stopped at the same dependency
    boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup, subscription, view, or window-
    routing behavior changed, compilation is already blocked, and no live
    exchange or credential-bearing external operation was run.
- Compatibility/UX assessment: the same normalized profile/key values reach the
  same backend and final state. Canonical metadata remains present during the
  immediate encrypted save exactly as before, but its skipped/non-serialized
  key field stays empty until acceptance. Every success/failure branch returns
  the same task, status/error/toast copy, selection, connection behavior, and
  config timing; only internal allocation provenance and destruction timing
  differ.
- Residual risk: Kerosene has not type-checked on this host. The deferred legacy
  account-key load is the remaining confirmed profile/key clone owner. The
  repository-wide local planner/state/message, external-error/status,
  snapshot, and progress-log diagnostic inventory remains incomplete.
- Prior turn commit hash: `cd89a56c4ce57370f00c6be585830d55eb78bc55`
- Next candidate: trace and harden deferred legacy-profile key loading without
  changing migration/storage outcomes, then resume the remaining account/order
  diagnostic inventory and Track 9 completion audit.

## Turn 32 — Move Deferred Legacy Keys Into Runtime Owners

- Status: F-37 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: the runtime deferred-legacy account-key migration reached after an
  authorized OS-keychain profile switch, including Hydromancer migration and
  bundle persistence outcomes
- Invariant: legacy lookup receives only the profile identity and any existing
  legacy Hydromancer fallback. The loaded agent and normalized Hydromancer
  allocations must become their canonical runtime owners by move, with one
  intentional input copy each; conflict/read failure creates no runtime key
  owner.
- Protected behavior: mode/ghost/index/existing-key gates; lookup identity and
  legacy fallback precedence; agent-required migration; exact agent and
  Hydromancer values; trimming/equality/conflict rules; key generation and
  journal cache clearing; bundle persistence and legacy cleanup timing;
  persistence-failure runtime values; all statuses; switch/connect behavior;
  no view, schema, or trading change.
- Preconditions/event ordering: the normal switch installs the target profile
  and discovers an empty bundled input, then calls the deferred loader. Build a
  narrow lookup shell, synchronously read legacy fields, require a nonempty
  agent, resolve Hydromancer conflict before installing either agent owner,
  move/copy only the final owners, then invoke the unchanged active-profile
  bundle persistence boundary.
- Evidence: F-37 records the parent clone graph, exact keychain reader inputs,
  current source, and allocation-sensitive controls. Repository search confirms
  `switch_account_task` is the only production caller and invokes this path only
  after its target is active with an empty input.
- Change: replaced the full profile clone with an identity/fallback shell,
  narrowed loader/persistence callbacks to `FnOnce`, moved loaded agent and
  normalized Hydromancer buffers into canonical state, retained one input copy
  for each, expanded fallback/conflict/trim/failure tests, and documented the
  ownership contract.
- Tests/checks:
  - The pre-fix exact primary migration regression stopped in `alsa-sys` before
    Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` file.
  - Post-fix exact primary migration, pre-existing Hydromancer fallback, and
    bundle-failure tests plus the full `account_state::switching::tests` module
    stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup, subscription, view, or window
    path changed, compilation is already blocked, and no live exchange or
    credential-bearing external operation was run.
- Compatibility/UX assessment: identical legacy values reach the same runtime
  fields and persistence bundle in the same order. Existing fallback and
  whitespace behavior are explicitly retained; conflict/read/storage failure
  leaves the same state and status. No copy, task, prompt, timing, persistence,
  trading, or interaction change is intentional.
- Residual risk: Kerosene has not type-checked on this host. Startup active-
  legacy profile merging and storage-selection snapshots remain separate owner
  graphs to inspect before closing Track 9. Local planner/state/message and
  other diagnostic paths also remain incomplete.
- Prior turn commit hash: `47061c87f5b0b06acba2a614f6141e39e93c6407`
- Next candidate: audit and, if safe, harden the startup active-legacy profile
  merge in `config/files/storage.rs`, including binding, partial-bundle failure,
  and cleanup semantics; then resume the remaining diagnostic inventory.

## Turn 33 — Preserve Both Active Legacy Secrets Before Startup Cleanup

- Status: F-38 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: startup hydration of an active profile missing from an otherwise valid
  keychain bundle, including two-field legacy read, store/repair, and cleanup
- Invariant: when the candidate bundle lacks a global Hydromancer value, every
  unambiguous legacy value the profile cleanup can delete must be in the
  attempted bundle, and exact loaded plaintext owners must survive until that
  bundle is durably accepted. Existing bundle-global disagreement precedence
  must remain unchanged pending approval.
- Protected behavior: active-profile selection; exact-versus-trimmed secret-ID
  use; wallet binding and mismatch gates; plaintext/bundle precedence; loader
  invocation; success and failure warnings; save blocking; original-payload
  repair; bundle-global disagreement behavior; cleanup payload/scope/timing;
  final values and schema; no order, view, prompt, or normal startup behavior
  change.
- Preconditions/event ordering: normalize plaintext secrets and bind any
  unbound payload entries first. If the active wallet still lacks a bound key,
  load its legacy two-field record into a narrow shell. Require the agent key,
  merge an unambiguous missing Hydromancer value, then copy both into the
  candidate payload and move both into plaintext config as failure authority.
  Store the candidate; only success proceeds through authoritative payload
  application and cleanup. An existing bundle global remains authoritative.
- Evidence: F-38 records the exact loss ordering, parent/current sources,
  cleanup behavior, established bundle-global precedence, and adversarial
  success/precedence/store-failure assertions. Call-site review confirms this
  helper is exclusive to the valid-bundle startup branch.
- Change: merged both loaded fields transactionally, added an identity-only
  lookup shell, retained exact/trimmed ID roles, moved loaded fallback buffers
  into config, preserved existing bundle-global precedence, added focused
  preservation/precedence/allocation tests, and documented the startup
  migration contract.
- Tests/checks:
  - The pre-fix exact helper regression stopped in `alsa-sys` before Kerosene
    compilation because `pkg-config` could not find the system `alsa.pc` file.
  - Post-fix exact helper, integrated success, store-failure, and bundle-global
    precedence tests plus the full `config::files::storage::tests` module
    stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no app runtime, subscription, view, or
    window path changed, compilation is already blocked, and no live exchange
    or credential-bearing external operation was run.
- Compatibility/UX assessment: unambiguous migration retains the same agent
  value and now preserves the accompanying Hydromancer value before the same
  cleanup. Store failure keeps the same block/warning and now retains both exact
  loaded buffers. A pre-existing bundle global remains authoritative with the
  same store, cleanup, warning, and final-state behavior; F-39 records why
  changing that disagreement policy requires approval. No schema, normal copy,
  task, prompt, trading rule, or UI flow changes.
- Residual risk: Kerosene has not type-checked on this host. Storage-selection
  hydration constructs a full saved-profile snapshot and reads every legacy
  profile; its ownership/correlation needs remain to audit. The broader
  diagnostic inventory also remains incomplete.
- Prior turn commit hash: `fc17cf69f613c0f4038d8066bc4cf4a71c2f9104`
- Next candidate: audit `encrypted_storage_selection_payload` and its callers
  for profile/key ownership, binding/conflict rollback, and cleanup authority;
  then return to the remaining account/order diagnostic inventory.

## Turn 34 — Narrow Storage Migration and Cleanup Secret Owners

- Status: F-40 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: OS-keychain-to-encrypted credential selection, current/bundle/legacy
  payload assembly, reader inputs, allocation ownership, cleanup snapshots,
  unlock cleanup retry, and clear-config task capture
- Invariant: the durable candidate payload may own required secret copies, but
  legacy readers must receive only exact identity and non-secret field-presence
  state, and keychain cleanup must receive only profile identities. Newly
  loaded missing values, or required normalized buffers, should move into their
  payload owners.
- Protected behavior: payload schema/order/values; profile IDs and wallet
  binding; ghost exclusion; exact legacy field reads and ordering; bundle and
  runtime precedence; Hydromancer trimming/conflict behavior; mismatch and
  failure handling; encryption/save rollback; cleanup scope/count/order/retry;
  all strings, settings behavior, config compatibility, and trading behavior.
- Preconditions/event ordering: build the current candidate, merge any matching
  keychain bundle values, read unresolved legacy globals, then visit each
  persisted profile in order for mismatch gating and unresolved legacy fields.
  Encrypt and save the completed candidate before clearing keychain state. A
  later unlock retry or clear-config task may reuse the cleanup profile list.
- Evidence: F-40 records the parent/current ownership graphs, production reader
  emptiness contract, cleanup consumers, async capture, and focused controls.
  Repository search confirms `encrypted_storage_selection_payload` has one
  production caller and `keychain_cleanup_profiles_snapshot` feeds only full
  keychain cleanup, unlock retry, config clear, and its tests.
- Change: built payload profiles from filtered references, removed the second
  full migration snapshot, introduced identity-only lookup and cleanup shells,
  used non-secret guards to preserve field-level reads, moved loaded or required
  normalized fallback allocations into the payload, and narrowed single-use
  readers to `FnOnce`.
- Tests/checks:
  - The pre-fix exact allocation-owner regression stopped in `alsa-sys` before
    Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` file.
  - Post-fix identity/presence, allocation, bundle-precedence, and cleanup-
    identity tests plus the full `secret_storage::selection::tests` and
    `config::secrets::model::tests` modules stopped at the same dependency
    boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no view, subscription, window, or startup
    behavior changed, compilation is already blocked, and no live exchange or
    credential-bearing external operation was run.
- Compatibility/UX assessment: the same profiles and globals reach the same
  encrypted schema in the same order. The production keychain reader sees the
  same empty/non-empty decision for every field and therefore performs the
  same reads; mismatch/conflict errors, bundle authority, save rollback, and
  cleanup behavior are unchanged. No schema, copy, prompt, task timing, trading
  rule, or UI flow changes.
- Residual risk: Kerosene has not type-checked on this host. F-41 records the
  storage-selection authority decision for disagreeing bundle and legacy
  values. Required encryption/serialization buffers remain, and the broader
  local order planner/state/message diagnostic inventory is incomplete.
- Prior turn commit hash: `e7335ef5dac019f52286c734967928bc37e26bfe`
- Next candidate: audit local order-planning, state, and `Message` debug/error
  paths for sensitive account/order material, then continue remaining external-
  status and Track 9 shutdown/restart diagnostics.

## Turn 35 — Redact Leverage Mutation Diagnostics

- Status: F-42 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: leverage input, submission snapshot, pending-result context, derived
  `Message` diagnostics, signed leverage-action diagnostics, and exact-value
  preservation through the existing view/update and serde boundaries
- Invariant: leverage validation, correlation, signing, reconciliation, and
  visible status require exact values, but generic diagnostics must not expose
  account identity or financial mutation parameters.
- Protected behavior: enabled/disabled input timing; exact text passed to the
  unchanged sanitizer; snapshot equality and stale-submit gate; leverage
  parsing/constraints; symbol and account/key selection; margin-mode policy;
  pending equality and current-account guard; task timing; response
  classification and refresh; form/status updates; serde names/order/values;
  action hashing/signing/posting; all persistence, prompts, and visible copy.
- Preconditions/event ordering: the view wraps each input value only when
  publishing its message, and the order update route restores the exact string
  before sanitizing it. Apply captures the same immutable snapshot; dispatch
  builds the same pending context and action; result handling compares and
  consumes their exact values. Only an explicit `Debug` request receives the
  redacted representations.
- Evidence: F-42 records the complete producer/consumer graph, parent-source
  diagnostic exposure, prior signing-redaction omission, absence of a current
  production log sink, signed serialization control, and adversarial sentinel
  assertions.
- Change: changed the leverage-input message payload to `RedactedOrderInput`,
  added value-redacting `Debug` implementations for the submission snapshot,
  pending result context, and `UpdateLeverageAction`, retained margin mode and
  optional-dex shape as non-value diagnostic structure, and documented the
  leverage diagnostic boundary.
- Tests/checks:
  - The pre-fix exact message regression stopped in `alsa-sys` before Kerosene
    compilation because `pkg-config` could not find the system `alsa.pc` file.
  - Post-fix exact message and signing-action regressions plus the complete
    `message::tests` and `signing::tests` modules stopped at the same dependency
    boundary. The existing leverage JSON/msgpack equivalence test therefore
    could not execute on this host.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: this batch changes only diagnostic
    formatting and an exact-value internal message wrapper, compilation is
    already blocked, and no live exchange or credential-bearing external
    operation was run.
- Compatibility/UX assessment: `RedactedOrderInput::into_string` returns the
  original allocation, and a regression assertion characterizes that exact
  handoff. Snapshot/context equality and every consumer still use the original
  field types and values. Removing only `Debug` derives does not change serde;
  the pre-existing JSON/msgpack equivalence test characterizes the signed wire.
  No visible state, status, error, timing, persistence, or trading semantic
  changed.
- Residual risk: Kerosene has not type-checked on this host. F-42 is source-
  hardened, but local planner/state types such as TWAP events, planning skips,
  and execution outcomes plus remaining external-status paths still need
  diagnostic audit before Track 9 can close.
- Prior turn commit hash: `fed00b0ca77642dd54acdf1076e61c69b8ec12cc`
- Next candidate: continue the local planner/state diagnostic inventory at
  `TwapEvent`, `TwapPlannedSliceSkip`, `ExecutionOutcome`, and nearby order
  correlation helpers; then resume remaining external-status and shutdown/
  restart diagnostics.

## Turn 36 — Redact Shared Execution-Outcome Diagnostics

- Status: F-43 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: shared normalized execution results and every one-shot, cancel, move,
  quick/HUD, NUKE, and wallet-cluster consumer of classification status and
  refresh/error controls
- Invariant: exact normalized order status must remain available to deliberate
  lifecycle and visible-status consumers, while generic diagnostics expose only
  the state-machine classification/control metadata they need.
- Protected behavior: response summaries; secret-shaped external-error
  sanitation; classifier ordering and all outcome kinds; exact status strings;
  clone/equality; cancellation heuristic; move prefix/correlation; quick-form
  recovery; one-shot/NUKE/cluster visible feedback; unexpected-resting handling;
  pending transitions; refresh choice; task construction/order; all message,
  persistence, view, and trading behavior.
- Preconditions/event ordering: parse or receive the same result, normalize it
  through the same classifier, and retain the same exact status/kind/flags.
  Existing handlers read or move those fields exactly as before. Only a direct
  `Debug` request substitutes a redaction marker for the status field.
- Evidence: F-43 records the parent source, response-summary construction,
  complete shared-classifier producer/consumer inventory, absence from
  messages/persistence and current production formatters, earlier redaction-
  coverage boundary, and exact-value diagnostic regression.
- Change: removed only the derived `Debug` implementation from
  `ExecutionOutcome` and added an explicit formatter that preserves kind,
  `is_error`, and `refresh_account` while redacting status; documented the
  normalized-result diagnostic boundary.
- Tests/checks:
  - The pre-fix exact diagnostic regression stopped in `alsa-sys` before
    Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` file.
  - Post-fix exact regression, the focused `execution_result_classifier_`
    family, and the full `order_update::results::tests` module stopped at the
    same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup, subscription, window, view, or
    task behavior changed, compilation is already blocked, and no live exchange
    or credential-bearing external operation was run.
- Compatibility/UX assessment: stored status is the identical `String` from the
  identical classifier and remains exact in the regression before it is
  formatted. All consumers, messages, error/status copy, response parsing,
  equality, task branches, and refresh decisions are source-identical. No
  visible behavior, timing, schema, signed bytes, or trading semantics changed.
- Residual risk: Kerosene has not type-checked on this host. F-43 is source-
  hardened, but TWAP event/planning-skip diagnostics, move-order correlation
  keys, and other local/external-status types remain to audit before Track 9
  can close.
- Prior turn commit hash: `d3b24fd77aedec777172a3c9348f403a0952274c`
- Next candidate: audit the complete TWAP activity-message path from
  `TwapPlannedSliceSkip` and result/error producers through `TwapEvent`, visible
  activity/history consumers, and both independent `Debug` surfaces; then
  inspect `MoveOrderKey` and remaining local correlation helpers.

## Turn 37 — Redact TWAP Activity Diagnostics

- Status: F-44 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: planned-slice skip values; every TWAP activity/error producer and
  sanitizer; exact order-status/live-activity/history consumers; independent
  planning-skip and event diagnostics
- Invariant: exact TWAP activity belongs in the deliberate visible and terminal-
  history fields, while generic formatters retain only state-machine metadata
  and cannot reveal strategy/order values.
- Protected behavior: planning size/range/notional calculations and established
  skip copy; initial/retry accounting; event timestamps/kinds/error flags and
  retention limit; external-error sanitization; slice result/status/fill/cancel
  text; order status/toasts; live activity rows; child summaries; terminal
  history logs/summary and serialization; scheduling, retry, terminalization,
  archive, task ordering, exchange requests, and every visible string.
- Preconditions/event ordering: the planner constructs and returns the same
  exact skip; recording moves it into the same event/status owners. Runtime
  producers sanitize external failures where required, then push exact events.
  Live/history consumers continue reading the message field directly. Only a
  direct `Debug` request receives a redaction marker in place of that field.
- Evidence: F-44 records the entire producer/sanitizer/consumer path, parent
  source, earlier runtime-redaction omission, no current production formatting
  sink, exact planner/event characterizations, and the separately discovered
  persisted-history diagnostic graph.
- Change: replaced derived formatters for `TwapPlannedSliceSkip` and
  `TwapEvent` with explicit structural diagnostics that preserve kind, error
  state, and event timestamp while redacting only the message; documented the
  live activity boundary.
- Tests/checks:
  - Both pre-fix exact diagnostic regressions stopped in `alsa-sys` before
    Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` file.
  - Post-fix exact regressions, the complete planning/TWAP-state suites, the
    terminal history-snapshot suite, and the existing TWAP execution suite
    stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no view, startup, subscription, window,
    task, or persistence behavior changed, compilation is already blocked, and
    no live exchange or credential-bearing external operation was run.
- Compatibility/UX assessment: the tests require exact message storage before
  formatting, and the existing planner/history assertions continue to protect
  established copy. Only `Debug` derives changed; fields, constructors, clone,
  direct UI/history reads, serialized history models, and all lifecycle code are
  source-identical. No visible behavior, timing, schema, signed bytes, or
  trading semantics changed.
- Residual risk: Kerosene has not type-checked on this host. F-44 is source-
  hardened, but the persisted advanced-history model graph independently
  derives raw `Debug` over account/order/log/child fields. `MoveOrderKey` and
  remaining local/external-status types also still require audit before Track 9
  can close.
- Prior turn commit hash: `7e923e1332f330cea4b0b72f6f30d9d500d91dcc`
- Next candidate: audit and, if safely cohesive, harden the complete persisted
  `AdvancedOrderHistoryEntry`/child/log diagnostic graph while preserving serde
  and every history view; then inspect `MoveOrderKey` and remaining correlation
  helpers.

## Turn 38 — Redact Persisted Advanced-History Diagnostics

- Status: F-45 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: complete persisted Chase/TWAP entry/log/child model graph, pre-snapshot
  Chase fill metrics, snapshot/config/view consumers, and unchanged serde
- Invariant: terminal history must retain exact values for persistence and
  deliberate views, but generic formatting of any independently reachable
  layer must not disclose account-linked historical order data.
- Protected behavior: Chase/TWAP snapshot construction and all financial
  calculations; entry IDs and upsert/pruning; every field/default/order/value in
  JSON; legacy empty default; config snapshot/load; clone/copy/equality; history
  list/detail formatting and child IDs; window routing; all visible strings,
  terminalization, archive timing, and trading behavior.
- Preconditions/event ordering: terminalize and snapshot through the unchanged
  constructor, clone the same entry into config, and deserialize/render its
  exact fields as before. Formatting an entry, nested record, or temporary fill
  metric now reports only allowlisted structural metadata and redaction markers;
  it does not traverse or mutate persisted values.
- Evidence: F-45 records the complete constructor/model/config/view graph,
  parent raw derives, absence of a current production sink, free-form persisted-
  string risk, all four new formatter policies, and nested serde/diagnostic
  characterization.
- Change: replaced derived `Debug` on `AdvancedOrderHistoryEntry`, child, log,
  and `ChaseHistoryFillMetrics` with explicit non-recursive redacted formatters;
  retained typed history-kind formatting and narrow structural booleans,
  presence flags, source/index values, and nested record counts; documented the
  persisted-history diagnostic boundary.
- Tests/checks:
  - The pre-fix exact nested diagnostic/serde regression stopped in `alsa-sys`
    before Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` file.
  - Post-fix exact regression, the complete advanced-history suite, the exact
    config history round-trip/default test, and the history-detail view tests
    stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no view, startup, subscription, window,
    task, config, or persistence behavior changed, compilation is already
    blocked, and no live exchange or credential-bearing operation was run.
- Compatibility/UX assessment: serde derives and every serialized field remain
  source-identical; the regression compares the exact nested JSON value before
  and after all four formatting calls. Existing snapshot/config/view controls
  remain unchanged. Only diagnostics differ, with no visible behavior, schema,
  defaults, timing, signed bytes, or trading semantics changed.
- Residual risk: Kerosene has not type-checked on this host. F-45 is source-
  hardened, but `MoveOrderKey`, TWAP form/book helpers, and other local/external-
  status model diagnostics remain to audit before Track 9 can close.
- Prior turn commit hash: `1354c94b2a2fe6a77a3afd0ce7b1969128ae5042`
- Next candidate: audit `MoveOrderKey` plus its map/drag/cancel/modify consumers
  and other nearby order-correlation model diagnostics; then continue TWAP form/
  book and remaining external-status boundaries.

## Turn 39 — Redact Cancel and Move Correlation Diagnostics

- Status: F-46 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: `MoveOrderKey`; cancel/move pending status owners; map, active-drag,
  chart-overlay, direct/status-result, refresh, disconnect, and config-clear
  consumers; captured-key move context boundary
- Invariant: exact order identity must remain available to local correlation
  and reconciliation, while generic diagnostics expose only safe lifecycle
  control metadata.
- Protected behavior: stored account/symbol/OID/expected-price/request values;
  key clone/equality/hash and map lookup; active drag and chart overlays; cancel
  phase transitions; stale request/account/OID/symbol rejection; move-context
  ownership and captured key; result/status classification; refresh cleanup;
  task/message ordering; visible order state and copy; all signed values,
  persistence, timing, and trading behavior.
- Preconditions/event ordering: dispatch and drag creation retain the same exact
  values in the same owners. Map/overlay consumers continue hashing the key,
  while pending requests compare every exact field in the same order. Only a
  direct formatter substitutes markers for the key fields and status-record
  symbol; the captured-key context still has no formatter.
- Evidence: F-46 records the parent source, complete key/request consumer
  inventory, absence of a current production formatting sink, intentional
  non-formattability of the captured-key context, prior partial-redaction gap,
  and exact correlation plus diagnostic characterizations.
- Change: removed only `MoveOrderKey`'s derived `Debug` and added an explicit
  formatter that redacts coin/OID; changed only the symbol fields in cancel and
  move status formatters to redaction markers; documented the local correlation
  diagnostic boundary.
- Tests/checks:
  - The pre-fix exact pending-cancel correlation/diagnostic regression stopped
    in `alsa-sys` before Kerosene compilation because `pkg-config` could not
    find the system `alsa.pc` file.
  - Post-fix exact key/hash and pending cancel/move correlation regressions, the
    complete order-result suite, and the move-order update suite stopped at the
    same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no view, startup, subscription, window,
    task, persistence, or runtime behavior changed, compilation is already
    blocked, and no live exchange or credential-bearing operation was run.
- Compatibility/UX assessment: the new tests require exact key values,
  equality/hash distinctions, and request matcher behavior before formatting.
  The production diff changes only three diagnostic field renderings; every
  consumer and state-machine branch is source-identical. No visible behavior,
  schema, timing, signed bytes, or trading semantics changed.
- Residual risk: Kerosene has not type-checked on this host. F-46 is source-
  hardened, but TWAP form/book helpers, remaining external-status types, and
  other Track 9 diagnostics still require audit before a final verdict.
- Prior turn commit hash: `4b6fbf5c42fda63648e2ed50b40b93913367fea4`
- Next candidate: audit the complete TWAP form/book/planning-helper diagnostic
  graph and its scheduling/result consumers, then continue remaining external-
  status boundaries without changing visible automation behavior.

## Turn 40 — Redact Transient TWAP Helper Diagnostics

- Status: F-47 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: editable TWAP form, parsed start schedule, TWAP-owned book snapshot,
  direct-response fill summary, authoritative-account fill summary, and their
  complete validation/planning/result/reconciliation/parent diagnostic graph
- Invariant: exact TWAP strategy, market-timing, order-identity, and fill values
  must remain available to deliberate runtime consumers but cannot bypass the
  already-redacted order/start/result layers through helper formatting.
- Protected behavior: form fields/default/equality and stale-start rejection;
  schedule parsing, interval and aggregate-capacity checks; exact book levels,
  timestamp, source/freshness gates, and slice planning; response parsing and
  OID fallback; fill OID/coin/side matching and deduplication; base/quote fee
  conversion; child settlement and account reconciliation; clone/copy/default
  traits; scheduling, randomization, history/UI copy, tasks, signed values,
  persistence, and all trading semantics.
- Preconditions/event ordering: input/update/start, market-stream/cache/plan,
  direct-result/child-settlement, and account-fill/reconciliation paths retain
  the same exact fields in the same owners and consume them in the same order.
  Only direct formatters substitute markers; optional response metadata retains
  its presence shape, and captured-key initialization remains non-formattable.
- Evidence: F-47 records all five raw parent derives, exact producer/consumer
  paths, existing parent and nested redaction boundaries, absence of a current
  production formatting sink, diagnostic bypass risk, and exact-value
  characterizations.
- Change: replaced only the five derived `Debug` implementations with explicit
  non-recursive formatters. Form input, cadence, book/timestamp, OID, size,
  price, and fee values are redacted; the form's established randomization
  boolean and fill optionality remain structural diagnostics.
- Tests/checks:
  - The pre-fix exact form/book diagnostic regression stopped in `alsa-sys`
    before Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` file.
  - Post-fix exact helper diagnostic family, complete `twap_state::tests`, and
    complete TWAP start tests stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup, subscription, view, window,
    task, persistence, or runtime behavior changed, compilation is already
    blocked, and no live exchange or credential-bearing operation was run.
- Compatibility/UX assessment: regressions require every exact field, form
  equality, response optionality, and parsed cadence to remain unchanged before
  and after formatting. Production changes are limited to formatters; all
  calculations, consumers, strings, serde/config behavior, timing, signed
  bytes, and trading branches remain source-identical. No user experience or
  trading semantic changed.
- Residual risk: Kerosene has not type-checked on this host. F-47 is source-
  hardened, but raw symbol/error context in advanced-order Elm messages and
  remaining external-status diagnostic types still require Track 9 audit.
- Prior turn commit hash: `05402699c29f45ac8fe69bfcf57f97ca8f43377d`
- Next candidate: audit the complete Chase/TWAP market/result `Message`
  diagnostic graph, including raw symbol fields, initial-book errors, nested
  response/status formatters, producers, routing, and handler unwrapping; keep
  every stream identity and automation transition unchanged.

## Turn 41 — Redact Advanced-Order Message Diagnostics

- Status: F-48 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: complete Chase/TWAP market/result Elm-message graph; five symbol-
  bearing market/adoption variants; nine initial-book, exchange-mutation,
  cancellation, and status-result variants; all publishers, routing, update
  consumption, nested formatters, and final-exit classification controls
- Invariant: the Elm boundary must transport exact keyed automation identity
  and task outcomes without making symbols, nested payloads, or error strings
  available to generic message diagnostics.
- Protected behavior: exact symbols, books, external/local errors, exchange and
  status responses; Chase/TWAP IDs, slice/place/reprice/cancel/status attempts,
  OID/CLOID wrappers, sigfigs, provider/key generations, stream subscription
  identity; handler signatures and call order; placement/repricing/cancellation,
  status checks, result classification, retries, reconciliation, terminalization,
  status/history/UI copy, final-exit fencing, tasks, timing, persistence, signed
  values, and trading semantics.
- Preconditions/event ordering: each existing subscription/view/task producer
  wraps only when constructing `Message`; routing remains pattern-only; the
  order-update arm consumes the wrapper immediately and passes the original
  `String` or `Result<T, String>` to the same handler at the same point. No
  wrapper persists in feature state or crosses another lifecycle transition.
- Evidence: F-48 records the full variant/producer/consumer inventory, parent
  raw fields, safe nested model policies, concrete debug-only catch-all sink,
  prior coverage gap, and exact wrapper/message characterizations.
- Change: added one exact-value redacted symbol wrapper and one generic boxed
  result wrapper that exposes only success/error shape; converted the 14 fields
  and their publishers/consumer arms without altering feature handlers or
  routes; documented the advanced-order Elm diagnostic boundary.
- Tests/checks:
  - The pre-fix exact advanced-order message diagnostic regression stopped in
    `alsa-sys` before Kerosene compilation because `pkg-config` could not find
    the system `alsa.pc` file.
  - Post-fix exact message/wrapper regressions, the complete message and app-
    update suites, and the complete Chase and TWAP execution suites stopped at
    the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: wrapper publication/consumption does not
    change startup, subscriptions, views, windows, tasks, or runtime behavior;
    compilation is already blocked, and no live exchange or credential-bearing
    operation was run.
- Compatibility/UX assessment: the wrapper regression recovers the exact
  symbol, exact error, and exact successful nested book value after formatting.
  Every handler continues receiving its prior type and every route/producer
  retains the same correlation fields. Only derived diagnostics and test
  construction syntax differ; no visible copy, behavior, timing, schema,
  signed bytes, or trading semantic changed.
- Residual risk: Kerosene has not type-checked on this host. F-48 is source-
  hardened, but one-shot/cancel/move/close/NUKE/quick/HUD/cluster result errors,
  raw order symbols/prices/fractions, and advanced-history navigation IDs remain
  in the broader Elm-message audit.
- Prior turn commit hash: `cb361da193d340bd2443e448a491fc0e1c60ea33`
- Next candidate: audit and cohesively harden the remaining one-shot, cancel,
  move, close/NUKE, quick/HUD, and wallet-cluster mutation-result `Message`
  fields using the proven result wrapper; separately inventory raw symbol,
  price/fraction, and history-navigation values before changing their types.

## Turn 42 — Redact Remaining Mutation-Result Messages

- Status: F-49 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: complete non-advanced mutation-result Elm-message graph across
  leverage, wallet cluster, shared one-shot, cancel, close, NUKE, quick/HUD,
  move, and their status-reconciliation messages; all publishers, two immediate
  update consumers, nested response/status diagnostics, and lifecycle controls
- Invariant: every exchange mutation/status result must cross the Elm boundary
  exactly once without exposing its payload through generic `Message::Debug`.
- Protected behavior: exact success/error results; account/member/request/
  execution/context/indicator/recovery identity; routing and handler call order;
  response classification, ambiguous-outcome status checks, refresh decisions,
  optimistic state, NUKE aggregation, cluster leg settlement, leverage state,
  quick/HUD recovery, move correlation, visible status copy, tasks, timing,
  persistence, signed values, and trading semantics.
- Preconditions/event ordering: each existing task mapper moves the same result
  into the wrapper at `Message` construction; the order or wallet-cluster update
  arm immediately restores the original result and calls the same handler. No
  wrapper enters feature state, changes correlation ownership, or survives a
  lifecycle transition.
- Evidence: F-49 records the 14 parent raw fields, complete producer/consumer
  inventory, prior nested redaction, concrete debug-only sink, synthetic-error
  coverage gap, exact wrapper control, and repository-wide absence of remaining
  raw exchange/order-status result fields in `Message`.
- Change: reused the Turn 41 boxed result wrapper for all 14 remaining fields;
  mechanically changed every publisher from `Box::new(result)` to
  `result.into()` and both update consumers from `*result` to
  `result.into_result()`; documented the complete mutation-result boundary.
- Tests/checks:
  - The pre-fix exact 14-variant message diagnostic regression stopped in
    `alsa-sys` before Kerosene compilation because `pkg-config` could not find
    the system `alsa.pc` file.
  - Post-fix exact regression, complete message and shared order-result suites,
    and the wallet-cluster suite stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup, subscription, view, window,
    task, or runtime behavior changed; compilation is already blocked, and no
    live exchange or credential-bearing operation was run.
- Compatibility/UX assessment: the previously committed generic wrapper test
  recovers exact success and error payloads; this turn changes only the set of
  message fields using it. Every context/control field and handler signature is
  source-identical, and each task still allocates one result box at the same
  point. No visible behavior, timing, schema, signed bytes, or trading semantic
  changed.
- Residual risk: Kerosene has not type-checked on this host. F-49 is source-
  hardened, but raw symbols in cancel/close/move/cluster and related view-state
  messages, raw move price and close fractions, and advanced-history navigation
  IDs remain in the broader Elm-message audit.
- Prior turn commit hash: `b9878f438089315d233edd9f6253113695917920`
- Next candidate: inventory every remaining order-sensitive non-result field in
  `Message`, then close the cohesive raw symbol/history-ID subset with exact
  wrappers while preserving move price/fraction handling for its own complete
  financial-value audit.

## Turn 43 — Redact Order Symbols and History Navigation Identity

- Status: F-50 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: 11 direct symbol fields on outcome prefill, cluster close,
  cancel/status, position hide/close, and move drag/intent/result/status
  messages; the account-bearing advanced-history navigation ID; all publishers,
  routes, immediate consumers, and diagnostic/lifecycle controls
- Invariant: transient Elm diagnostics must not expose an exact order/position
  symbol or persisted history identity, while every handler receives the
  source-identical string at the source-identical lifecycle point.
- Protected behavior: exact symbol and history-ID bytes; outcome sell prefill;
  cluster-close member sizing; cancel and move origin correlation/status
  ownership; close-menu and hidden-position selection; close fractions and
  market/limit choice; move price and drag state; history entry lookup/window
  focus/open behavior; routing, exit fencing, task order, signing, persistence,
  visible copy, and trading semantics.
- Preconditions/event ordering: views or chart interaction move their existing
  symbol/history string into a wrapper only when constructing `Message`;
  cancel/move status tasks do the same with captured origin strings. The order,
  account, cluster, or history update arm consumes the wrapper immediately and
  invokes the unchanged handler. No wrapper enters feature state, persistence,
  preparation, or signing.
- Evidence: F-50 records the parent raw fields, account-bearing history-ID
  construction, complete producer/consumer inventory, concrete generic
  diagnostic sinks, prior coverage gap, exact recovery control, and
  repository-wide absence of those raw direct fields after the patch.
- Change: reused `RedactedOrderSymbol` for all 11 direct symbol fields; added an
  exact-value `RedactedAdvancedOrderHistoryId`; mechanically wrapped every
  publisher and restored each string at its immediate update boundary; left
  every financial value and nested model unchanged.
- Tests/checks:
  - The pre-fix exact 12-variant diagnostic regression stopped in `alsa-sys`
    before Kerosene compilation because `pkg-config` could not find the system
    `alsa.pc` file.
  - Post-fix exact diagnostic and wrapper-recovery tests, complete message and
    app-update suites, chart-input tests, advanced-history tests, and wallet-
    cluster tests stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: no startup, subscription, window, task, or
    rendering behavior changed; compilation is already blocked, and no live
    exchange or credential-bearing operation was run.
- Compatibility/UX assessment: the exact recovery control proves symbol and
  history-ID identity is unchanged; existing chart cancel, routing, final-exit,
  cluster, close/move/status, and history-window paths still receive their
  original types immediately after the message boundary. No visible behavior,
  timing, schema, signed bytes, financial input, or trading semantic changed.
- Residual risk: Kerosene has not type-checked on this host. F-50 is source-
  hardened, but direct financial values (price, percentage/fraction, preset and
  slippage input) and remaining nested order-sensitive message types need a
  complete diagnostic inventory before any further wrapper change.
- Prior turn commit hash: `078fc1ab388288b8a82261dac6218cff1316a23f`
- Next candidate: inventory every remaining direct and nested financial/order-
  sensitive `Message` field, classify UI geometry separately from trading
  values, and close one exact-value subset without changing parsing, precision,
  preparation, or visible behavior.

## Turn 44 — Redact Direct Financial Message Values

- Status: F-51 implemented; executable Rust validation environment-blocked
- Severity: Medium
- Scope: complete direct financial-value graph in `Message`: book-selected
  price, main/quick percentages, preset edit/execute, market-slippage input,
  connected/cluster close fractions, quick-open price, and move price; all
  publishers, immediate consumers, diagnostic controls, and UI geometry
  classification
- Invariant: every transient financial value must reach the established parser,
  validator, clamp, preparation, or state update bit-for-bit while generic
  message diagnostics expose no exact trading value.
- Protected behavior: exact string bytes, float bits (including non-finite and
  negative-zero inputs), preset label/size/offset, order-book formatting,
  percentage sizing, slippage parsing, connected/cluster close sizing,
  quick-form placement geometry, move target validation, routing, final-exit
  fencing, persistence, visible copy, preparation, signing, and trading
  semantics.
- Preconditions/event ordering: views move existing strings, numeric values, or
  cloned presets into wrappers only when constructing `Message`; order,
  preferences, and cluster update arms immediately recover the original type
  before any existing branch runs. No wrapper enters state, config, preparation,
  signing, or a task result.
- Evidence: F-51 records the complete parent field/publisher/consumer inventory,
  prior coverage gap, exact float/preset controls, UI-geometry classification,
  generic debug sink, and repository-wide absence of the targeted raw field
  types after the patch.
- Change: reused `RedactedOrderInput` for four financial strings and added
  `RedactedOrderValue<T>` for six numeric values plus `OrderPreset`; wrapped
  every producer and restored every value at its immediate update boundary.
- Tests/checks:
  - The pre-fix exact 11-variant financial diagnostic regression stopped in
    `alsa-sys` before Kerosene compilation because `pkg-config` could not find
    the system `alsa.pc` file.
  - Post-fix exact diagnostic and bit/preset recovery tests, complete message,
    app-update, order-update, wallet-cluster, chart-input, and preferences
    suites stopped at the same dependency boundary.
  - `cargo fmt`, `cargo fmt -- --check`, and `git diff --check` passed.
  - `cargo check`, full `cargo test`, and
    `cargo clippy --all-targets --all-features -- -D warnings` each stopped at
    that same pre-existing dependency boundary before checking Kerosene.
  - The GUI smoke test was not run: startup, subscriptions, windows, rendering,
    and geometry are unchanged; compilation is already blocked, and no live
    exchange or credential-bearing operation was run.
- Compatibility/UX assessment: recovery tests prove exact float bit patterns
  and preset equality; the existing string wrapper proves exact text recovery;
  chart interaction still verifies positive clicked price plus exact X/Y/canvas
  dimensions. All handlers retain their original signatures and receive values
  before existing parsing/validation. No visible behavior, timing, schema,
  precision, signed bytes, or trading semantic changed.
- Residual risk: Kerosene has not type-checked on this host. F-51 is source-
  hardened, but nested formattable order/account state and raw non-mutation
  external result/status messages require a final Track 9 inventory. Public
  market-data values and UI geometry must remain separately classified rather
  than blanket-wrapped.
- Prior turn commit hash: `90b156a313e886fb43140a9b13705feb46f21ba2`
- Next candidate: inventory every remaining formattable nested order/account
  type and raw external-result message (including `PnlCardTarget`, ticket sizing
  provenance, preset config diagnostics, account refresh, and user-data paths),
  distinguish deliberately public market data, and close one evidenced subset.

## Deferred Findings

- F-21: the live and persisted child label for a filled unexpected-resting
  order depends on fill-versus-cancel delivery order. Financial state is safe,
  but choosing fill-dominant, cancel-dominant, or combined visible semantics
  requires explicit approval. Existing stored history strings remain unchanged.
- F-24: a failed final config save clears exit ownership after iced has already
  removed the main window, so automation can resume headlessly. Fixing it
  requires choosing delayed close, reopen-on-error, or exit-with-unsaved-config
  behavior; current window/save-error UX is unchanged pending approval.
- F-29: final pre-dispatch TWAP skip exhaustion remains absent from advanced
  history. Archiving it would add a visible persisted row; the captured key is
  independently scrubbed pending a product decision.
- F-31: a config save that reports its profile/credential snapshot installed
  but a post-install durability step failed restores saved-delete or
  OS-keychain-rebind runtime state while disk may retain the transition and
  cleanup intent. Choosing installed-snapshot authority or a second durable
  rollback changes exceptional behavior and requires approval.
- F-39: when a valid partial bundle already has a global Hydromancer key that
  differs from the active profile's legacy Hydromancer entry, startup currently
  keeps the bundle global authoritative, migrates the missing agent, and runs
  profile-wide cleanup that deletes the differing legacy entry. Treating the
  disagreement as uncertain would add a startup warning/save block; preserving
  both would require field-specific cleanup ownership or new durable conflict
  state. Choose explicitly between current bundle authority, conflict blocking,
  or a separate retention policy. Turn 33 preserves current behavior and fixes
  only the unambiguous missing-global loss path.
- F-41: during OS-keychain-to-encrypted selection, a current/bundled credential
  retains established authority over a differing legacy per-profile/global
  entry, and successful full keychain cleanup removes the legacy entry. Some
  already-resolved fields deliberately skip the legacy read; a bundle-filled
  profile whose runtime field is empty still performs its established legacy
  read but ignores a different result. Detecting or retaining disagreement would
  add keychain reads/prompts, migration blocking/status behavior, or field-
  specific durable cleanup state. Choose explicitly between current/bundle
  authority, conflict blocking, or separate retention. Turn 34 preserves and
  characterizes current authority while narrowing secret ownership only.

## Validation Summary

- Passing this turn: `cargo fmt`, `cargo fmt -- --check`, `git diff --check`.
- Environment-blocked this turn: pre-fix 11-variant financial diagnostics;
  post-fix exact diagnostic/bit/preset recovery tests, complete message, app-
  update, order-update, chart-input, wallet-cluster, and preferences suites,
  `cargo check`, full `cargo test`, and strict clippy at
  `alsa-sys` system dependency discovery, before Kerosene was compiled.
- No live exchange mutation or credential-bearing operation was run.

## Residual Risk

- The remaining audit tracks are incomplete; no overall safety-completion claim
  is made.
- F-01 through F-20, F-22/F-23, F-25 through F-28, F-30, F-32 through F-38,
  F-40, and F-42 through F-51
  have source fixes and regression coverage but await executable validation on
  a host with ALSA development metadata.
- F-21 is explicitly deferred for a visible/history semantics decision; its
  financial invariants have characterization coverage.
- F-24 is explicitly deferred for a main-window/final-save failure policy; its
  successful save/clear intervals are independently fenced by F-23/F-25/F-26.
- F-29 is explicitly deferred for final-skip advanced-history visibility; its
  secret-lifetime risk is independently addressed by F-28.
- F-31 is explicitly deferred for post-install saved-profile deletion/rebind
  authority; ordinary failure and successful key ownership are independently
  hardened by F-30/F-32.
- F-39 is explicitly deferred for partial-bundle versus legacy-profile
  Hydromancer conflict authority; F-38 independently preserves unambiguous
  missing-global migration before cleanup.
- F-41 is explicitly deferred for storage-selection current/bundle versus
  legacy credential authority; F-40 independently narrows migration and cleanup
  secret owners without changing that policy.
- TWAP terminalization plus successful save/clear final-exit fencing across all
  current mutation intents are source-audited with focused coverage but cannot
  be executed on this host. Ghost-profile cluster stream invalidation is
  source-repaired but likewise uncompiled. TWAP captured-key terminalization is
  source-complete apart from the deferred history decision. Saved-profile
  delete and address-rebind key ownership are source-hardened apart from F-31;
  explicit credential-save ownership is source-hardened by F-33, and nested
  exchange-response diagnostics are source-hardened by F-34. Ordinary switching
  key ownership is source-hardened by F-35, and add-account ownership by F-36;
  deferred runtime legacy-key ownership is source-hardened by F-37, and startup
  partial-bundle cleanup by F-38. Storage-selection and cleanup ownership are
  source-hardened by F-40. Leverage input/submission/result/action diagnostics
  are source-hardened by F-42, and shared normalized execution-outcome
  diagnostics by F-43. TWAP planning/live-event diagnostics are source-hardened
  by F-44, persisted advanced-history diagnostics by F-45, and cancel/move
  correlation diagnostics by F-46. Transient TWAP form/schedule/book/fill
  helper diagnostics are source-hardened by F-47, and advanced-order Elm
  message diagnostics by F-48. All remaining mutation-result message payloads
  are source-hardened by F-49; direct order/position symbols and advanced-
  history navigation identity are source-hardened by F-50, and all direct
  financial message values by F-51. Remaining nested order/account and external-
  status paths, plus the rest of Track 9, require completion before a final
  verdict.
