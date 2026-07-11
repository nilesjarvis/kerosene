# Security And Secrets

Kerosene is trading software. It handles agent private keys, API keys, wallet
addresses, signatures, and account data. Secret material must stay out of logs,
plain config snapshots, screenshots, docs examples, and commits.

## Secret Types

Secret-bearing values include:

- Hyperliquid agent private keys
- Hydromancer API key
- HyperDash API key
- X OAuth access token, Client ID, and refresh token
- Telegram fast-mode login code/password/API hash while in memory
- Telegram API hash embedded at build time through
  `KEROSENE_TELEGRAM_API_HASH`
- encrypted secret password/confirmation inputs
- any future API token or signing key

Wallet addresses are not private keys, but they can identify a user. Avoid
printing real wallet addresses in tests/docs unless explicitly anonymized.

## Runtime Secret Handling

`app_state.rs` defines:

```rust
pub(crate) type SensitiveString = Zeroizing<String>;
```

Secret buffers and payloads use `Zeroizing<String>` so memory is cleared on
drop where practical.

Saved-account deletion moves the removed profile into a narrow rollback owner
instead of cloning profile credentials. A failed durable config save moves that
same owner back into account state; after a successful save, its agent and
legacy per-profile integration keys are scrubbed before keychain cleanup, which
receives only the profile secret ID. Encrypted deletion staging continues to
use zeroizing payload and plaintext buffers.

Wallet-address rebinding likewise moves the active profile key and key-input
buffer into one rollback owner. Credential persistence receives one short-lived
account snapshot after the active key has been removed. Failure restores the
original allocations; success scrubs the rollback keys and sensitive identity
copies immediately.

Explicit agent-key saving builds one caller-owned persisted-profile snapshot
with the draft key substituted directly, while the previous committed key
remains the signing authority. Failed storage drops that caller-staged copy;
successful storage moves the exact staged key buffer into committed profile
state rather than cloning it again. Backend-required payload and serialization
buffers remain short-lived zeroizing scopes inside the synchronous call.

Account switching performs same-profile, pending-request, Chase, and uncertain-
TWAP checks before capturing target credentials. A successful saved-profile
switch clones the canonical key once into a narrow target and moves that exact
allocation into the key-input owner. Rejected and ghost switches create no
target-key copy; ghost cleanup still scrubs any stray canonical key.

Keyed add-account submission keeps the window draft as failure authority and
places its one caller-owned key copy only in the credential-storage snapshot.
The provisional canonical profile contains metadata but no signing key, so an
immediate encrypted-config save can persist the profile atomically without
making it trading-capable before credential storage accepts it. Failure drops
the staged copy and preserves the exact draft allocation; success moves the
staged profile into canonical state. A first saved account also moves the
verified, normalized draft allocation into the key-input owner, while ordinary
switch-on-add continues through the post-gate account-switch capture.

Deferred legacy keychain loading constructs a lookup shell from the profile
secret ID and any existing per-profile Hydromancer fallback; it does not clone
the account's name, address, or canonical agent-key owner. A loaded agent-key
allocation moves into the canonical profile and is copied once for the active
key input. A newly migrated, normalized Hydromancer allocation moves into
global runtime state and is copied once for its input. Conflict, trimming,
persistence, and legacy-cleanup behavior remain owned by the existing migration
path.

Startup hydration treats an active profile's partial-bundle legacy fallback as
one agent/Hydromancer transaction. Its keychain lookup shell retains the exact
profile ID but omits unrelated metadata and canonical secret owners. When the
bundle has no global Hydromancer key, every unambiguous loaded value that
profile-wide cleanup may delete is placed in the attempted bundle first. Until
that write succeeds, the loaded buffers remain the exact plaintext config
fallbacks. When the bundle already has a global Hydromancer key, it retains its
established authority; choosing a different conflict policy requires an
explicit migration-behavior decision.

Switching from OS-keychain storage to encrypted config builds the candidate
payload directly from borrowed persisted-profile references, without a second
full profile snapshot. Legacy readers receive the exact profile ID plus
non-secret field-presence guards, not canonical names, wallet addresses, or
credential contents. Newly loaded agent and integration buffers move into the
candidate payload when no normalization is required; a required normalized
integration buffer becomes the owner instead. Keychain-cleanup and clear-config
tasks receive only identity shells. Existing bundle precedence, field-read
decisions, conflict handling, cleanup scope, and persistence ordering remain
unchanged.

Secret-bearing state includes:

- `wallet_key_input`
- `hydromancer_api_key`
- `hydromancer_key_input`
- encrypted secret password/confirmation buffers
- X OAuth token/client/refresh input and runtime state
- profile secret payloads

Do not clone secrets unnecessarily. When a task must own a key, keep the
ownership scope narrow.

## Storage Modes

Credential storage supports:

- OS keychain
- encrypted config

OS keychain mode stores profile/global secrets outside plaintext config.

Encrypted config mode stores an encrypted blob in `KeroseneConfig` using:

- Argon2id key derivation
- XChaCha20Poly1305 encryption
- random salt and nonce
- schema/version/cipher metadata

Encrypted mode requires unlock before secrets are available for use or update.

## Config Snapshot Rules

Plain config snapshots intentionally write empty secret fields:

- `agent_key`
- `hydromancer_api_key`
- `hyperdash_api_key`
- `x_access_token`
- `x_oauth_client_id`
- `x_refresh_token`
- `schwab_client_id` / `schwab_client_secret` / `schwab_access_token` /
  `schwab_refresh_token`
- `openrouter_api_key`

Saved account profiles persist secret IDs and wallet metadata, not raw agent
keys. Secret payloads map secret IDs to agent keys and global integration
tokens inside the selected secret storage backend.

## Ghost Wallets

Ghost wallets are in-memory only. They should not cause agent keys or ghost
secret state to be persisted. If a ghost account is active, journal/account
snapshot logic should avoid writing ghost-only secret-linked data where
appropriate.

## Signing Boundary

`src/signing/` is the only implementation boundary for signed Hyperliquid
exchange actions.

Rules:

- Do not implement ad hoc signing in feature modules.
- Do not log signing payloads, signatures, nonces with key context, or raw
  exchange requests if they could expose sensitive material.
- Order execution modules should pass keys into signing tasks through
  zeroizing-owned values.
- Tests should use known dummy keys or fixtures, not real keys.

Parsed exchange responses retain exact nested status values for lifecycle
classification, but every response-model `Debug` layer exposes only allowlisted
response-type metadata, status counts, and explicit redaction markers. A
type-only response summary emits the recognized protocol label or a
value-neutral marker; ordinary protocol copy remains unchanged.

The shared order-result classifier likewise retains its exact normalized status
for existing UI, cancellation checks, and reconciliation decisions. Formatting
an `ExecutionOutcome` exposes only its outcome kind and control flags; the
status is represented by a redaction marker so OIDs, fill size, fill price, and
sanitized-but-still-sensitive external copy do not become diagnostic output.
The stored status and every downstream consumer remain unchanged.

Cancel and move correlation state likewise retains exact account, symbol, OID,
request-sequence, and expected-price values for map identity, stale-result
rejection, status reconciliation, and chart interaction. Its independently
formattable key and pending status records expose only request sequence, phase,
type shape, and redaction markers. The captured-key move context remains
non-formattable. Formatting does not alter equality, hashing, matching, cleanup,
or any order action.

TWAP planning and activity messages also remain exact for the order-status UI,
live activity log, and terminal advanced-order history. Their independently
formattable runtime layers do not expose that text: `TwapPlannedSliceSkip` and
`TwapEvent` diagnostics retain kind, timing where applicable, and error state,
but represent the message with a redaction marker. This does not change event
storage, display, history serialization, scheduling, or retry behavior.

The transient TWAP input-to-reconciliation pipeline likewise keeps exact form
inputs, parsed cadence, cached book/freshness data, direct-response OID and fill
metrics, and authoritative account-fill metrics for runtime decisions only.
Direct diagnostics for the form, parsed schedule, TWAP-owned book snapshot, and
both fill-summary layers expose only structural booleans/presence where useful
and redaction markers. Their values, defaults, equality, calculations,
scheduling, and reconciliation behavior remain unchanged.

Chase/TWAP market, mutation-result, cancellation, and status-result messages
retain exact symbols and `Result` payloads until the order update boundary.
Their transient wrappers make derived `Message` diagnostics value-neutral:
symbols are redacted and task results expose only `Ok`/`Err` shape without
traversing nested responses or external error text. Update immediately restores
the original types before unchanged handlers run, so stream identity,
correlation, reconciliation, and visible error handling remain exact.

The same result wrapper covers every other signed-mutation and `orderStatus`
message: leverage, wallet-cluster legs, one-shot placement, cancel, close,
NUKE, quick/HUD, and move. No raw exchange-response or order-status result box
remains in `Message`; diagnostics expose only outcome shape. The order or
wallet-cluster update route restores the original result immediately, retaining
all exact errors, responses, contexts, indicators, recovery owners, and member
correlation for existing lifecycle handling.

Direct order/position symbols on outcome-sell prefill, cluster close,
cancel/status, position hide/close, and move drag/intent/result/status messages
use the same exact-value symbol wrapper. Advanced-history navigation uses a
dedicated wrapper because its persisted identity embeds the originating account
address. Views and tasks wrap only at message construction; the order, account,
cluster, or history-window update arm immediately restores the original
`String`. Diagnostics are value-neutral while selection, preparation,
correlation, prices, fractions, persistence, and window identity remain exact.

Direct financial values on order-book price selection, sizing percentages,
preset edit/execute, market-slippage input, connected and cluster close
fractions, quick-order open/percentage, and move price messages are likewise
value-neutral in diagnostics. String inputs reuse `RedactedOrderInput`; numeric
values and the exact nested preset use `RedactedOrderValue<T>`, which moves the
original value without conversion and preserves floating-point bits. Update
restores each value before existing parsing, clamping, preparation, or state
changes. Quick-order canvas coordinates remain ordinary control geometry; no
price, size, preset, slippage, fraction, persistence, or UI behavior changes.

Percentage-derived ticket sizing retains a runtime provenance fence containing
the originating account, account/balance revisions, symbol, denomination,
percentage, order kind, reference price, reduce-only state, and market
universe. Its custom `Debug`, like the quick-order counterpart, hides the
account, symbol, percentage, and exact reference price while retaining only
safe revision, mode, boolean, universe, and price-presence metadata. Stale-size
validation still compares the untouched strings and exact floating-point bits;
quantity calculation, invalidation, recalculation, submission, and visible
errors remain unchanged.

Order presets and saved layouts remain exact persisted configuration, but their
diagnostics are structural only. `OrderPreset` hides its label, size, and price
offset value; `OrderPresetsConfig` reports only category counts; `SavedLayout`
hides names, symbols, order/risk values, widget contents, and other nested
configuration while reporting only collection counts and presence. Layout
import/export result messages expose only `Ok`/`Err` shape and restore the
original layout or error before existing normalization and toast handling.
Serde fields, defaults, JSON wire output, layout application, preset execution,
and visible import/export behavior remain unchanged.

Account refresh results for the connected account, wallet-cluster members,
wallet details, wallet tracker, portfolio history, and income snapshots retain
their exact success payloads and upstream errors until the established update
handler runs. `RedactedAccountMessageResult<T>` keeps the existing boxed result
ownership while making derived `Message` diagnostics expose only `Ok`/`Err`
shape. Each update path restores the original result at its prior box-
consumption point; unchanged request-identity and staleness checks still govern
state application. Account reconciliation, cluster sizing, tracker state,
portfolio/income data, retry bookkeeping, and user-visible error sanitization
remain exact.

Trading-journal fill and chart-snapshot results follow the same boundary with a
dedicated `RedactedJournalMessageResult<T>`. Derived message diagnostics expose
only `Ok`/`Err` shape, not account activity, candle timing/values, pagination
warnings, or upstream error text. Snapshot-request diagnostics likewise hide
trade identity, symbol, and exact trade/fetch windows while retaining safe
source/generation/timeframe structure. Fill handlers restore the exact result
only after the existing request/account/address stale guards accept it;
snapshot handling recovers the exact candles or error for the unchanged request
and provider-generation checks. Pagination, fill aggregation, snapshot
coverage, cache behavior, and visible sanitized errors remain unchanged.

Wallet-label import/export keeps its exact schema, timestamp, addresses, labels,
colors, and tags for the established JSON wire format and merge rules, but the
export model's diagnostics report only schema compatibility, a redacted time,
and entry count. `RedactedWalletLabelsMessageResult<T>` makes import/export
completion diagnostics expose only `Ok`/`Err` shape; the layout update path
recovers the original payload or error before the existing config-clear fences,
merge/sync/persistence work, cancellation checks, and toast sanitizer. File
contents, normalization and precedence, tracked-address refreshes, both silent
cancellation strings, and visible success/error behavior remain unchanged.

The same identity boundary applies after import. Persisted tracker/address-book
entries and live address-book entries report only redacted label/color presence
and tag/address counts; `WalletDisplay` hides both its primary and secondary
strings while retaining the `has_label` shape. Tracker label input/edit messages
use `RedactedWalletLabel`, restoring the exact string before existing edit-state
storage or trim-on-commit behavior. Serde data, label/color/tag lookup, short-
address rendering, tooltips, subscription refresh, tracker rows, and config
persistence remain exact and user-visible output is unchanged.

Saved-account and wallet-cluster identity follows the same rule. Account profile
and picker diagnostics hide names and addresses; cluster config/live/execution
diagnostics hide cluster/member names, stable cluster/profile IDs, and fan-out
weights while retaining safe mode, status, count, and lifecycle structure.
Account-label, cluster-name, cluster-ID, and required profile-ID messages use
exact-value redacted wrappers and restore their strings in the first account or
cluster update arm. Profile/cluster serde, drafts, generated IDs, selection,
membership, weight bits, stream rotation, persistence, execution correlation,
history rendering, and all visible names remain unchanged.

HyperDash positioning diagnostics distinguish public market aggregates from
wallet-level data. Ticker/market/timeframe, aggregate notionals and counts,
pagination shape, and timestamps remain available on the aggregate models, but
wallet rows are represented only by count. Row diagnostics hide addresses,
ordinary names/labels/tags, verification and copy-score metadata, exact
positions, prices, PnL, account values, and deltas. Positioning completion
messages use `RedactedPositioningMessageResult<T>`, so generic Elm diagnostics
also avoid traversing either a success payload or external error. Response
parsing, size caps, request keys/generations, coalescing, stale-result handling,
stored values, widget errors, and every rendered aggregate and wallet row remain
exact and unchanged.

HyperDash liquidation-level and heatmap models contain public market data, so
their standalone diagnostics retain exact coin, range, timestamp, amount, and
cell structure. Their three asynchronous completion messages instead use
`RedactedHyperdashMarketMessageResult<T>` and expose only `Ok`/`Err` shape,
preventing generic Elm diagnostics from recursively formatting large public
payloads or pre-handler external errors. Public request/cache keys and key
generations remain available for correlation. Fetch parameters, response
parsing and caps, shared-request fan-out, stale-result guards, cache contents,
distribution validation, status/toast sanitization, rendering, and visible data
remain exact and unchanged.

Live-watchlist, ticker-tape, screener, and symbol-search context/history
completions follow the same public-market boundary with
`RedactedPublicMarketMessageResult<T>`. Generic Elm diagnostics retain request
IDs, requested symbols, timestamps, and `Ok`/`Err` shape without traversing
context/history maps, financial values, partial-family errors, or top-level
upstream errors. Each market update arm restores the exact result before its
request-ownership checks and unchanged partial-data, refresh, cache, or
sanitized-status handling. Symbol-search completion additionally requires the
retained loading flag, request ID, and exact requested-symbol snapshot before it
may consume request ownership or replace volume context. API requests, public
market values, refresh cadence, stale/duplicate behavior, visible statuses, and
all rendered watchlist, tape, screener, and symbol-search data remain exact and
unchanged.

Outcome-volume aggregation is likewise public market data: standalone
`OutcomeVolume24h` diagnostics retain exact contract and notional volume. Its
completion message uses the same value-neutral result wrapper, while runtime
state retains the latest requested-symbol snapshot beside loading and request
ID. Every newer metadata refresh replaces that owner; only a completion matching
all three fields may settle it, and accepted or empty-universe terminal paths
clear the snapshot. Concurrent newest-request-wins behavior, 24-hour candle
aggregation, requested/current-symbol intersection, retained data on error,
sanitized error state, outcome grouping, and rendered volume remain unchanged.

Exchange-symbol metadata follows the same message boundary while retaining a
structural standalone diagnostic summary of symbol-family counts, cache
provenance, and failure flags. `SymbolsLoaded` carries a runtime request
generation plus a value-neutral result wrapper, so generic Elm diagnostics show
correlation and `Ok`/`Err` shape without traversing symbol labels, outcome
details, or an upstream error. The generation spans cached startup, immediate
live verification, and periodic refreshes; only the current loading/in-flight
owner may settle, and acceptance advances the generation before the unchanged
metadata merge. Cache policy, partial-family retention, fail-closed spot and
outcome orderability, aliases, refresh cadence, visible statuses, and dependent
market refreshes remain exact and unchanged.

Order-book snapshots retain structural standalone diagnostics—side counts and
best-level presence—while individual prices and sizes remain redacted.
`BookLoaded` uses the public-market result wrapper, preserving pane/request,
symbol, selected-tick, and server-sigfig correlation without traversing a
snapshot or upstream error in generic Elm diagnostics. Its numeric request
owner is allocated for the terminal lifetime and skips active IDs across wrap,
so runtime layout reconstruction cannot make an old pane task indistinguishable
from the replacement pane's request. REST/stream parsing, exact book values,
precision planning, merge/scope behavior, sanitized UI errors, rendered rows,
and click-to-limit behavior remain unchanged.

Historical primary, comparison, and macro candle completions use the same
public-market result boundary. Generic Elm diagnostics retain chart,
incarnation, request, symbol, timeframe, provider/key-generation, range, and
attempt correlation while exposing only `Ok`/`Err` shape; they do not traverse
OHLCV payloads or pre-handler external errors. A runtime chart-incarnation
generation spans all three completion families and advances before saved-layout
chart reconstruction, so an old task cannot become the owner of an identical
replacement chart. Exact candle values, cache/backfill/retry policy, chart
errors, macro calculations, rendering, interaction, and order-price behavior
remain unchanged.

Session Data and spaghetti-chart historical completions also use the
public-market result boundary. Generic Elm diagnostics retain their public
pane/chart, request, symbol, timeframe/lookback, source-generation, session,
and timestamp correlation while exposing only `Ok`/`Err` shape; daily,
intraday, or per-series OHLCV collections and upstream errors are not traversed.
Session requests use a terminal-lifetime allocator across pane reconstruction.
Spaghetti requests combine a per-instance per-series owner with the shared
runtime chart incarnation, preserving all existing source/provider/key/
timeframe/session guards. Exact public calculations, caches, visible errors,
rendering, and interaction remain unchanged.

Economic-calendar completions use the same public-market result boundary.
Generic Elm diagnostics retain the runtime request ID and `Ok`/`Err` shape but
do not recursively format event titles, dates, impact/forecast values, or a
pre-handler external response excerpt. The Calendar update arm immediately
recovers the exact result after its active-owner check, preserving event data,
error sanitization, retry cadence, filters, status copy, and rendering.

PnL-card runtime diagnostics must not reproduce the card itself. The target,
account-derived metrics, formatted render text, and rendered image use custom
`Debug` implementations that hide the symbol, prices, size/context, PnL,
percentages, direction-derived colors, pixels, PNG bytes, and export filename
while retaining only safe variant, mode, presence, dimension, and buffer-length
metadata. Copy/save result messages likewise expose only success/error shape;
the account update path recovers the exact external error or saved path before
the unchanged toast handling. On-screen rendering, privacy toggles, styles,
image bytes, filenames, saved-path toast text, and export behavior remain
unchanged.

Terminal Chase/TWAP history intentionally persists exact account, symbol,
financial, timing, child-identifier, status, and activity fields for its
existing views. The persisted entry, child, and log types have independently
redacted diagnostics, as does the pre-snapshot Chase fill-metrics helper.
Formatting exposes only allowlisted kind/source IDs, booleans, presence flags,
record counts, and redaction markers; serde fields/defaults and view access stay
exact and unchanged.

Leverage updates retain their exact input, symbol, account-correlation, asset,
margin-mode, and leverage values for validation, stale-result checks, signing,
and user feedback. Those values must not become a diagnostic payload: the Elm
input message uses `RedactedOrderInput`, the submission and pending-result
contexts implement value-redacting `Debug`, and the serialized leverage action
redacts its asset and leverage when formatted. This changes only diagnostic
representations; the signed wire fields and normal leverage behavior remain
unchanged.

## API Key Boundaries

Hydromancer, HyperDash, and X keys are only needed in:

- request tasks
- subscription setup
- secret persistence
- settings input/update flows

Saving or replacing keys should update secret storage and clear stale
connection/cache state when required. Hydromancer key rotation should evict old
websocket managers so old-key tasks stop.

## Release-Time Embedded Credentials

Kerosene can be built with optional Telegram fast-mode defaults through
`KEROSENE_TELEGRAM_API_ID` and `KEROSENE_TELEGRAM_API_HASH`. The API hash is
compiled into the binary when set. Public release builds should leave these
variables unset unless the bundled Telegram application credentials are
explicitly approved as public, non-user-specific, and rotation-safe. Without
bundled values, users can enter their own Telegram developer API ID and hash
when enabling fast mode.

## UI And Output Safety

Do not display secrets in:

- settings status messages
- toasts
- logs
- screenshots
- PnL card images
- chart screenshots
- test snapshots
- docs examples

When showing credential status, say where credentials are stored or what failed
without echoing the value.

## Filesystem Safety

Config paths use platform config directories. Imported asset file names are
validated before being referenced. Journal caches and Telegram session files
should use restrictive permissions where supported.

Do not accept arbitrary stored paths for future secret or asset features without
normalization and tests.

## Trading Risk Boundaries

Security also includes preventing unintended trades:

- close-position and NUKE require fresh account data
- hidden/muted positions should not be silently routed
- move-order replacement must not switch account/key after canceling the
  original order
- Chase/TWAP must respect account/key availability and market-type checks
- ambiguous order results require verification or refresh

Do not weaken these checks for UI convenience.

## Logging And Debugging

Safe to log:

- high-level status
- anonymized request IDs
- non-secret error strings
- counts and durations
- synthetic test addresses/keys

Do not log:

- private keys
- API keys
- bearer tokens
- encrypted secret passwords
- Telegram login codes/passwords
- real account dumps
- signed payloads or signatures from real accounts

When in doubt, redact.

## Tests To Check

Use focused tests in:

- `src/config/secrets/**/tests.rs`
- `src/secret_storage/**` tests where present
- `src/config/tests/**` for credentials omission
- `src/signing/tests/**`
- `src/order_execution/**/tests` for key/account safety
- `src/order_update/**/tests` for result verification
- `src/pnl_card/tests/privacy.rs`
- `src/journal/cache/tests.rs` for cache file behavior

For any storage or signing change, inspect generated config output and ensure
secret fields remain empty or encrypted.
