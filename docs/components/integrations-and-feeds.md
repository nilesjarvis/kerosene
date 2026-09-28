# Integrations And Feeds

Kerosene integrates with Hyperliquid, Hydromancer, HyperDash, OpenRouter,
Telegram, X, ForexFactory-style calendar data, SEC APIs, HYPE ETF endpoints, and
Hypurrscan-style unstaking data. Integrations enter the app as REST tasks,
websocket subscriptions, timer-driven refreshes, or optional authenticated
streams.

## Component Map

| Component | Key files | Responsibility |
| --- | --- | --- |
| Hyperliquid REST | `src/api.rs`, `src/api/` | Shared HTTP client, info API calls, candles, symbols, books, fills, order status. |
| Hyperliquid websocket | `src/ws.rs`, `src/ws/manager.rs`, `src/ws/market_streams/`, `src/ws/user_streams/` | Singleton exchange websocket, subscriptions, coalescing, routed market/user streams. |
| Hydromancer | `src/hydromancer_api.rs`, `src/ws/hydromancer/`, `src/feed_update/connection.rs` | Funding history, authenticated liquidation/tracked-trade feeds, optional read-data provider. |
| HyperDash | `src/hyperdash_api.rs`, `src/hyperdash_api/`, `src/hyperdash_update/` | GraphQL liquidation heatmaps, liquidation levels, positioning info, liquidation distribution. |
| OpenRouter | `src/openrouter_api.rs`, `src/openrouter_api/`, `src/openrouter_update.rs`, `src/agent_*` | AI completion foundation plus the Pi-backed Kerosene Assistant; key validation and default-model selection. |
| Feeds | `src/feed_state/`, `src/feed_update/`, `src/feed_views/` | Liquidation feed, tracked trades, Telegram feed, aggregation, alerts, rendering. |
| Telegram | `src/telegram_feed.rs`, `src/telegram_fast_feed.rs` | Public channel scraping and optional MTProto fast/private feed. |
| Calendar and screener | `src/calendar_*`, `src/screener_*` | Economic calendar, market screener contexts/history. |
| SEC earnings | `src/api/sec.rs`, `src/api/sec/`, `src/chart_update/earnings.rs` | EDGAR earnings events, filing summaries, and chart request/cache state. |

## Remote Wallet Database

Settings → Integrations accepts a custom PocketBase base URL under
`remote_wallet_database.url`. The wallet feature owns its read-only client,
runtime mirror, and update messages (`RemoteWalletDatabaseUrlChanged`,
`SaveRemoteWalletDatabase`, `DisconnectRemoteWalletDatabase`,
`RemoteWalletDatabaseSync`, `RemoteWalletDatabaseLoaded`), all routed through
`WalletTracker`. Boot and an independent 30-second timer request full paginated
snapshots with entity expansion. One request runs at a time per active source;
unique request IDs reject late results after switching/disconnecting/resetting.
No admin credentials or remote records are persisted. Read failures retain the
last complete in-memory snapshot and surface stale status in settings/tracker.
The [README](../../README.md#remote-wallet-label-database) defines the collection
schema, permissions, polling behavior, bounds, and local/remote precedence.

## Hyperliquid REST

`src/api.rs` owns:

- shared `reqwest::Client`
- user agent
- request/connect/idle timeouts
- `API_URL = https://api.hyperliquid.xyz/info`

Build requests directly from the shared client; reqwest's request builder retains
its own client handle. OpenRouter keeps its separate client and longer timeout.
Account, wallet, analytics, and symbol read helpers borrow the client while their
caller awaits them, including concurrent request groups. Public fetch tasks still
own their account and request inputs.

Submodules cover:

- candles and chart backfill
- exchange symbols
- order books
- order status by CLOID/OID
- user fills
- watchlist/screener contexts and history
- economic calendar
- SEC earnings events
- HYPE ETF and unstaking data
- outcome volumes

REST work uses `Task::perform` and returns typed result messages with enough
context for stale-response guards.

## Hyperliquid Websocket

`src/ws/manager.rs` implements a singleton multiplexed websocket manager for:

```text
wss://api.hyperliquid.xyz/ws
```

The manager:

- stores active subscriptions
- sends subscribe/unsubscribe commands
- replays subscriptions after reconnect
- sends periodic pings
- detects stale reads
- coalesces high-frequency frames
- broadcasts routed channel messages
- records telemetry

Feature streams under `ws/market_streams/` and `ws/user_streams/` convert routed
JSON into typed data:

- candles
- L2 books
- asset context
- all-mids
- open orders
- fills
- account/user data

Subscriptions are added from `subscription_state/`, not from views.

## Hydromancer

Hydromancer is optional and authenticated. It provides:

- funding history over REST
- liquidation websocket feed
- tracked-trade websocket feed
- alternative candle/book/asset-context streams when selected as read provider
- optional real-time open-position PnL from Hydromancer `l2Book` ticks,
  matching Tick candle book-mid prices

Hydromancer state appears in:

- `hydromancer_api.rs`
- `ws/hydromancer/`
- `feed_state/`
- `feed_update/connection.rs`
- `subscription_state/hydromancer.rs`
- `subscription_state/market/`
- `account_update/position_pnl.rs`

The Hydromancer key is secret-bearing. Key rotation should evict old websocket
managers so stale key tasks do not keep running.

Connection errors and connect timeouts share the manager's retry path: record
the failure, broadcast the redacted error or timeout label, then wait while
processing commands. Failed attempts double the delay from one second up to
30 seconds; a successful connection resets it. Shutdown can interrupt both
connection attempts and retry waits.

The socket frame parser removes top-level `cursor` and `sessionId` fields before
broadcasting the JSON. String values move directly into zeroizing resume fields;
malformed values are removed and ignored. Control-message errors borrow their
source text while applying the existing authentication labels and redaction.
Fill parsing borrows the tuple's address; only tracked-trade events copy it into
their owned output. Liquidation events still validate that address as a string
before reading the separate liquidated-user field.

Feed updates share control-message status and heartbeat transitions in
`feed_update/hydromancer_status.rs`, after validating the stream generation and
scope. Data-event filtering and freshness remain feed-specific: hidden
liquidations refresh receipt time, while filtered tracked trades do not. Lag
clears liquidation summary/chart buckets while retaining both feeds' rows.

## HyperDash

HyperDash integration is GraphQL-based and covers:

- current liquidation levels for chart overlays
- historical liquidation heatmaps
- liquidation distribution pane
- positioning info

Liquidation levels, heatmaps, and ticker positions share observed send, body-read,
and HTTP status handling in `hyperdash_api/http.rs`. Their request builders,
parsers, and error contexts remain endpoint-specific. Perp deltas retain the
bounded chunk reader and strict UTF-8 decoding in `positioning/response.rs`.
Heatmap bucket inference sorts its temporary timestamp buffer in place and
ignores zero-length gaps, preserving duplicate and missing-timestamp behavior.

Update modules live under `hyperdash_update/`:

- `key.rs`
- `heatmap.rs`
- `liquidations.rs`
- `liquidations_distribution.rs`

Requests use keys and pending maps for dedupe/stale protection. Saving a new
HyperDash key clears relevant pending/cached overlay state and refreshes enabled
views.

Chart invalidation updates matching heatmaps directly and clears liquidation
waiters while retaining displayed liquidation data. Distribution responses take
ownership of their pending request only after generation and key matching;
unmatched results preserve current state. Key and distribution regression tests
live in `hyperdash_update/key/tests.rs` and
`hyperdash_update/liquidations_distribution/tests.rs`.

Heatmap responses move into the bounded cache before updating waiting charts.
Cache hits and fresh responses borrow that stored data for chart updates, which
retain independent render cells and lightweight loaded markers. Cache admission
order, muted/request guards, status text, and canvas invalidation are unchanged.
Heatmap lifecycle tests live in `chart_state/heatmap/tests.rs` and
`hyperdash_update/heatmap/tests.rs`.

The HyperDash key is secret-bearing.

## OpenRouter

OpenRouter is the foundation for AI-assisted features (the Kerosene Assistant,
news summaries, and TradFi filing summaries). The user supplies an API key in Settings > Integrations; saving it
persists the key through the selected secret storage backend and validates it
against `GET /api/v1/key`, surfacing usage/limit status in the settings UI.

`src/openrouter_api.rs` and its child modules provide:

- a dedicated `reqwest::Client` with a long completion timeout (chat
  completions outlive the shared 15s client budget)
- `chat_completion` — non-streaming `POST /api/v1/chat/completions` with
  `ChatCompletionRequest`/`ChatMessage` request builders
- `fetch_key_status` — key validation and credit/limit reporting
- `fetch_tool_models` — the bounded `GET /api/v1/models` catalog used by the
  Assistant model picker, filtered to text-output models that advertise the
  `tools` parameter
- typed error-envelope parsing with status-code hints (401/402/429/...)

`models.rs` keeps catalog wire types, filtering, pricing, and model labels
together. `key.rs` owns key-status responses. The root retains completion
requests and shared observed HTTP transport; endpoint builders retain their
own headers and timeout overrides. Public entry points remain available from
`openrouter_api`.

The Assistant footer model name opens a searchable model picker. Catalog rows
show the OpenRouter name/slug, context window, and current prompt/completion
prices (plus reasoning-token or per-request charges when present) normalized
from USD per token to USD per million tokens. Conditional
pricing overrides are flagged as variable rates rather than presented as a
single guaranteed price. The catalog is fetched on first open, can be refreshed
manually, and is discarded whenever the OpenRouter key generation changes.
Late catalog results are ignored using the same key-generation guard as key
validation.

Components should take the key via
`TradingTerminal::openrouter_api_key_for_task()`, the model via
`openrouter_model_for_task()` (falls back to the `openrouter/auto` router), and
gate features on `openrouter_configured()`. Results returned from tasks should
be checked against `openrouter_key_generation_is_current` so responses that
arrive after a key change are dropped.

The OpenRouter key is secret-bearing. The default model slug is plain,
non-secret config (`openrouter_model`). Selecting a model in the Assistant
updates this default, persists the config, and invalidates the current Pi
runtime so a later turn cannot accidentally continue on the previous model.

The assistant's Pi RPC process, read-only snapshot contract, and packaging
requirements are documented in [Kerosene Assistant And Pi](assistant-and-pi.md).

## Liquidation Feed

The liquidation feed uses Hydromancer websocket data. State includes:

- raw liquidation event deque
- aggregation settings
- chart/summary bucket toggles
- following/autoscroll state
- reconnect nonce
- stale status
- alert settings and thresholds

Key modules:

- `feed_state/liquidations/`
- `feed_update/liquidations.rs`
- `feed_views/liquidations/`
- `subscription_state/hydromancer.rs`

The feed is subscribed only when the pane is open and a Hydromancer key is
available.

Live updates and history rebuilding use the same minute/second bucket
accumulation helper. Live updates prune old buckets and cap the event deque;
rebuilding clears and replays all retained events without age pruning.

## Tracked Trades

Tracked trades are Hydromancer feed events filtered by tracked addresses and
deduplicated with seen-key state.

State includes:

- tracked trade deque
- seen keys/order
- aggregation toggle
- settings menu
- reconnect nonce
- alert settings

Tracked trade subscription addresses come from configured tracked wallets and
related feed settings. Empty address sets should not open a stream.

Row aggregation and alert suppression share the order/hash/time-span merge
predicate. Suppressed alerts check borrowed event data before constructing an
owned row. Each feed retains its own grouping keys, scan limits, and render
limits. Tracked-trade cells reuse their row view's theme, and optional PnL, fee,
and intent labels are formatted only for visible columns.

The two feeds share header text, settings-dropdown containers, and toggle
widgets in `feed_views/controls.rs`; labels, toggle messages, and text sizes
remain in each caller. Tracked-trade wallet counts are computed during the
empty-state checks and passed through responsive rendering to the top bar.

## Telegram Feed

Telegram has two modes:

- Public web fetch and HTML parsing through `telegram_feed/client.rs`, with
  fetch functions re-exported from `telegram_feed.rs`.
- Fast/private feed through `telegram_fast_feed.rs` using `grammers`.

Public mode fetches `https://t.me/s/<channel>` pages and does not require a
secret. Fast mode can use Telegram API ID/hash, code, password, and session
storage. Session files are stored under the platform config directory with
restricted permissions where supported.

The public client shares avatar/media request, status, size, and raster-signature
checks while retaining separate size limits and error labels. HTML extraction
borrows attributes and text fragments until normalization constructs owned
model fields. Feed state, redacted debug output, channel normalization, and
shared plain-text/image helpers remain in `telegram_feed.rs`. Model tests live
in `telegram_feed/tests.rs`; parser tests and local HTTP regressions live under
`telegram_feed/client/tests.rs`.

Fast-feed subscriptions require:

- Telegram pane open
- fast mode enabled
- API ID available
- at least one public or private channel configured

Credentials used during login should not be persisted as plaintext input
buffers.

Fast-mode update handlers live in `feed_update/telegram/fast.rs`. Code requests,
code/password submissions, and sign-out share generation, pending-state, status,
and completion-message setup. Their validation and credential cleanup stay at
the call sites; code/password submissions capture the existing challenge ID
before allocating the next result ID. Auth results and stream events retain
their separate request/reconnect generation guards. Login and private-channel
scan admission reuse the state's `signed_in()` predicate.

Feed updates borrow channel candidates and prior ticker mentions. Mention
refreshes replace match metadata while retaining captured reference prices and
their timestamps. Alert preparation keeps at most three formatted messages plus
an overflow count, preserving arrival order even if those posts are later
pruned from the rendered feed. Disabled alerts skip that preparation.

Avatar updates merge cached state with one profile lookup; avatar and media
request strings are copied only when a fetch is eligible. Media scheduling keeps
its target snapshot and existing result guards. Update tests live under
`feed_update/telegram/tests.rs`, with ownership regressions in `tests/ownership.rs`.

Telegram views borrow posts, profiles, candidate records, titles, and status text
from feed state. Private candidate selection retains scan order and excludes
selected peers; collapsed lists do not clone candidate strings or image handles.
Avatar rendering shares the loaded-image/initials path. Clipboard and channel
actions retain owned messages, image widgets retain cloned handles, and impact
chips move their prepared ticker/symbol strings and sparkline buffers into the
widgets after computing tooltip text. Styles and padding/color helpers live in
`feed_views/telegram/styles.rs`; canvas geometry and layout values are unchanged.
Sign-in views borrow the fixed dialing-code list, selected code, and informational
text. The country-change message owns its selected string, and code cells iterate
over characters without collecting a temporary vector.

Fast-feed authentication and pending challenges live in `telegram_fast_feed/auth.rs`;
session files, permissions, client-operation serialization, and pool shutdown live
in `telegram_fast_feed/session.rs`. Each module keeps its focused tests nearby.
Challenge removal and restoration share one registry guard, which leaves scope
before network work. A wrong challenge type retains its original request ownership
and returns the existing input error without acquiring the same lock twice.
The root retains private-channel scans, stream orchestration, channel
resolution/backfill, and cursor generations. Scans cache the ASCII-folded title
and peer ID for sorting while retaining stable order and adjacent-peer deduplication.
The four session file paths use a fixed array with the same cleanup order.

Media classification, bounded downloads, and follow-up events live in
`telegram_fast_feed/media.rs`, preserving byte limits, thumbnail selection,
timeout, semaphore admission, and failure events. Live delivery still records its
cursor before scheduling a media follow-up, while backfill schedules its media
jobs before recording the cursor.

## X Feed

X Feed uses local BYOK user-context access for the authenticated account's
following timeline and Lists. Users can provide a user access token directly or
provide a Client ID plus refresh token so Kerosene can refresh the access token
locally. Runtime state lives in `x_feed.rs`, REST requests and response parsing
in `x_feed/client.rs`, update logic in `feed_update/x.rs`, and rendering in
`feed_views/x.rs`.

Direct-token and OAuth commits share credential replacement and input cleanup.
Clearing credentials uses the same cleanup for editable and pending inputs,
while retaining its unconditional request invalidation. Candidate validation
borrows the input until it is accepted; task and persistence snapshots own
zeroizing buffers independently of runtime state.

Auth, list, and timeline requests use the same token-refresh decision. Complete
refresh credentials are required; a missing access token, unknown expiry, or
expiry within 60 seconds triggers refresh. Pending refreshes are suppressed,
and token results are persisted before runtime credentials are committed.

Author profiles update in place through one map-entry path. Posts without an
image URL still refresh author metadata while preserving cached or pending
images. A changed URL invalidates the old image request; unchanged URLs retain
the existing cache and retry backoff. Source options sort borrowed lists using
cached ASCII-folded names, and timeline refreshes copy only the selected newest
post ID into the request.

The pane is multi-instance through `PaneKind::XFeed(XFeedId)`. Persisted layout
config stores widget IDs and selected non-secret sources in `x_feeds`. Raw X
access tokens, Client IDs, and refresh tokens are stored only in the selected
credential store (OS keychain or encrypted config) and are omitted from
plaintext config snapshots.

Low-latency behavior is REST polling while an X Feed pane is open. Following and
List timelines are user-context REST endpoints, so X Filtered Stream is not a
drop-in replacement for these sources; it is app-context public filtering and
should only be added as an optional public watch source.

## SEC earnings

`api/sec.rs` owns the request/result types and filing-summary orchestration.
Its child modules keep the read-only EDGAR pipeline separated by responsibility:

- `http.rs`: endpoint URLs, configured user agent, observed GET requests, status
  checks, and JSON/text decoding. Both readers share request/status handling and
  retain distinct decode errors.
- `submissions.rs`: ticker lookup, company submissions, and dated 8-K item 2.02
  earnings events.
- `earnings.rs`: company-facts decoding, periodic filing selection, metric
  matching, and year-over-year formatting.
- `documents.rs`: safe archive/document URLs, submission-package parsing, and
  summary-document selection.
- `summary.rs`: HTML-to-text conversion and headline/highlight extraction. The
  headline and highlight searches share one ASCII-folded copy of the filing;
  candidate windows borrow the original text. Duplicate matching compares the
  first 80 ASCII alphanumerics without building intermediate strings.

Chart request generations, pending readers, and in-memory caches remain in
`chart_update/earnings.rs`. Summary text is retained when optional company-facts
data is unavailable. Module tests use fixtures; HTTP reader tests use a loopback
server without contacting EDGAR.

## Calendar

Calendar state covers economic events, impact/window filters, loading/error
state, retry attempts, and next retry time.

Key modules:

- `calendar_state.rs`
- `calendar_update.rs`
- `calendar_views/`
- `api/calendar.rs`

Calendar fetches are one-shot tasks triggered by pane open, manual refresh, or
timer/retry behavior.

The API sorts events by whole-second timestamps and then raw date text, computing
each key once. The view shares one temporary set of parsed timestamps between
filtering and the next-event summary, retaining full timestamp precision. Invalid
dates remain visible under time filters but are excluded from the next-event
summary; both sorts preserve input order for equal keys.

The calendar uses a compact table at pane widths of 640px and above, with a
grouped date gutter, local event times, impact pills, and right-aligned forecast
and previous values. Narrower panes stack event details and omit empty value
lines. Time and event-title tooltips retain relative timing and full titles in
the table. Window/impact filters and refresh share a toolbar; the next release
and data freshness appear in the footer. Refreshes preserve the scroll position
instead of estimating an offset from unfiltered rows and fixed row heights.

## Screener

The screener uses watchlist context/history data and displays market scans in a
separate window.

Key modules:

- `screener_state.rs`
- `screener_update.rs`
- `screener_views.rs`
- `api/watchlist/`

Screener refreshes are separate from live watchlist and ticker tape caches.

## Alerts And Notifications

Alerts can use:

- in-app toasts
- sounds through `sound.rs`
- desktop notifications through `notify-rust`

`notification_state.rs` shares toast, sound, and desktop delivery for trade,
error, interest, tracked-trade, and Telegram feed alerts. Each alert keeps its
own desktop title and sound kind; the global sound and desktop toggles are
independent. Toasts are queued first, with errors retained ahead of informational
toasts, and only desktop delivery needs a second copy of the message.

Feed-related alert toggles include:

- income alerts
- liquidation alerts
- tracked trade alerts
- Telegram notifications
- X notifications

Alerts should avoid printing wallet-private data or API keys.

## Tests To Check

Use focused tests in:

- `src/api/**/tests.rs`
- `src/ws/manager/**/tests.rs`
- `src/ws/manager/integration_tests.rs`
- `src/ws/hydromancer/**/tests.rs`
- `src/hydromancer_api/tests.rs`
- `src/hyperdash_api/**/tests.rs`
- `src/hyperdash_update/**/tests.rs`
- `src/openrouter_api/**/tests.rs`
- `src/openrouter_update/tests.rs`
- `src/feed_update/liquidations/tests.rs`
- Telegram tests in `src/feed_update/` and `src/telegram_*`
- `src/screener_*` tests
- `src/calendar_*` tests

For integration changes, test both missing-key and configured-key paths.
