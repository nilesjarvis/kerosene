# Integrations And Feeds

Kerosene integrates with Hyperliquid, Hydromancer, HyperDash, Schwab, OpenRouter,
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
| HyperDash | `src/hyperdash_api.rs`, `src/hyperdash_update/` | GraphQL liquidation heatmaps, liquidation levels, positioning info, liquidation distribution. |
| Schwab | `src/schwab.rs`, `src/account_update/schwab.rs`, `src/settings_views/integrations.rs`, chart backfill paths | User-supplied OAuth credentials, linked brokerage account summaries, and Schwab price-history candles. |
| OpenRouter | `src/openrouter_api.rs`, `src/openrouter_update.rs` | AI chat-completion client foundation for news and TradFi filing summaries; key validation and default-model selection. |
| Feeds | `src/feed_state/`, `src/feed_update/`, `src/feed_views/` | Liquidation feed, tracked trades, Telegram feed, aggregation, alerts, rendering. |
| Telegram | `src/telegram_feed.rs`, `src/telegram_fast_feed.rs` | Public channel scraping and optional MTProto fast/private feed. |
| Calendar and screener | `src/calendar_*`, `src/screener_*` | Economic calendar, market screener contexts/history. |

## Hyperliquid REST

`src/api.rs` owns:

- shared `reqwest::Client`
- user agent
- request/connect/idle timeouts
- `API_URL = https://api.hyperliquid.xyz/info`

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

## HyperDash

HyperDash integration is GraphQL-based and covers:

- current liquidation levels for chart overlays
- historical liquidation heatmaps
- liquidation distribution pane
- positioning info

Update modules live under `hyperdash_update/`:

- `key.rs`
- `heatmap.rs`
- `liquidations.rs`
- `liquidations_distribution.rs`

Requests use keys and pending maps for dedupe/stale protection. Saving a new
HyperDash key clears relevant pending/cached overlay state and refreshes enabled
views.

The HyperDash key is secret-bearing.

## Schwab

Schwab is optional and authenticated through user-supplied Schwab developer app
credentials or a pasted access token. The current integration is read-only:

- OAuth refresh-token exchange and access-token persistence
- automatic access-token refresh at boot and on a one-minute timer while
  refresh credentials are stored (Schwab access tokens expire in ~30 minutes),
  with a retry cooldown so a failing refresh cannot hammer the token endpoint
- linked account discovery through the Schwab trader API
- account summaries, balances, buying power, and position counts
- `schwab:` chart symbols backed by Schwab price-history candles
- account switcher entries that show Schwab details only while a Schwab account
  is the active account source

Schwab state appears in:

- `src/schwab.rs`
- `src/account_update/schwab.rs`
- `src/account_views/picker/`
- `src/account_views/summary.rs`
- `src/settings_views/integrations.rs`
- `src/api/candles.rs` and chart candle request paths

Schwab app keys, app secrets, access tokens, refresh tokens, account numbers,
and account hashes are secret or sensitive. They should be kept in
`SensitiveString`, `Zeroizing`, redacted message wrappers, OS keychain, or
encrypted-config paths. Do not write Schwab credentials or account identifiers
to plaintext config snapshots or logs.

Schwab order placement is intentionally disabled in the current build. Enabling
trading should be handled as a separate high-risk order-execution project with
request construction tests, stale-account guards, confirmation/error states, and
approval for the relevant Schwab API scopes.

## OpenRouter

OpenRouter is the foundation for AI-assisted features (news and TradFi filing
summaries). The user supplies an API key in Settings > Integrations; saving it
persists the key through the selected secret storage backend and validates it
against `GET /api/v1/key`, surfacing usage/limit status in the settings UI.

`src/openrouter_api.rs` owns:

- a dedicated `reqwest::Client` with a long completion timeout (chat
  completions outlive the shared 15s client budget)
- `chat_completion` — non-streaming `POST /api/v1/chat/completions` with
  `ChatCompletionRequest`/`ChatMessage` request builders
- `fetch_key_status` — key validation and credit/limit reporting
- typed error-envelope parsing with status-code hints (401/402/429/...)

Components should take the key via
`TradingTerminal::openrouter_api_key_for_task()`, the model via
`openrouter_model_for_task()` (falls back to the `openrouter/auto` router), and
gate features on `openrouter_configured()`. Results returned from tasks should
be checked against `openrouter_key_generation_is_current` so responses that
arrive after a key change are dropped.

There is currently no production chat-completion caller. A future component
must capture the key generation plus its own logical request ID at dispatch,
carry both through a value-neutral result message, and reject a non-current
owner before recovering completion content or errors. Chat work must not reuse
the key-validation request owner because the two operations have independent
lifecycles.

Key validation has an additional runtime-only request owner containing the key
generation and a separate wrapping check ID. Each successful nonempty save
replaces that owner, including repeated saves of the same key, so only the exact
newest check may publish credit/limit or error status. Settings-window close
does not cancel this app-global check; key change, key clear, config clear, and
accepted completion invalidate it. The owner is not persisted and does not
change request timing or visible status text.

The OpenRouter key is secret-bearing. The default model slug is plain,
non-secret config (`openrouter_model`). Key-check messages and standalone
credit/limit status diagnostics redact values, while the accepted update path
recovers and renders the exact values. Result errors receive a second redaction
pass before entering visible runtime status. Chat request serialization and
completion parsing remain exact, but generic diagnostics do not traverse prompt
messages, returned provider values, generated content, or token-usage counts;
safe request model/options and message count remain available for correlation.

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

## Telegram Feed

Telegram has two modes:

- Public web fetch through `telegram_feed.rs`.
- Fast/private feed through `telegram_fast_feed.rs` using `grammers`.

Public mode fetches `https://t.me/s/<channel>` pages and does not require a
secret. Fast mode can use Telegram API ID/hash, code, password, and session
storage. Session files are stored under the platform config directory with
restricted permissions where supported.

Fast-feed subscriptions require:

- Telegram pane open
- fast mode enabled
- API ID available
- at least one public or private channel configured

Credentials used during login should not be persisted as plaintext input
buffers.

## X Feed

X Feed uses local BYOK user-context access for the authenticated account's
following timeline and Lists. Users can provide a user access token directly or
provide a Client ID plus refresh token so Kerosene can refresh the access token
locally. Runtime state lives in `x_feed.rs`, update logic in `feed_update/x.rs`,
and rendering in `feed_views/x.rs`.

The pane is multi-instance through `PaneKind::XFeed(XFeedId)`. Persisted layout
config stores widget IDs and selected non-secret sources in `x_feeds`. Raw X
access tokens, Client IDs, and refresh tokens are stored only in the selected
credential store (OS keychain or encrypted config) and are omitted from
plaintext config snapshots.

Low-latency behavior is REST polling while an X Feed pane is open. Following and
List timelines are user-context REST endpoints, so X Filtered Stream is not a
drop-in replacement for these sources; it is app-context public filtering and
should only be added as an optional public watch source.

## Calendar

Calendar state covers economic events, impact/window filters, loading/error
state, retry attempts, and next retry time.

Key modules:

- `calendar_state.rs`
- `calendar_update.rs`
- `calendar_views/`
- `api/calendar.rs`

Calendar fetches are one-shot tasks triggered by pane open, manual refresh, or
timer/retry behavior. One terminal-wide request ID and an active-loading flag
own the completion across pane close/reopen and runtime layout reconstruction;
stale or duplicate results cannot replace cached events or alter retry state.
The owner is runtime-only. Generic Elm diagnostics retain its request ID and
`Ok`/`Err` shape without traversing event fields or an upstream error, while the
Calendar update path receives the exact result for unchanged storage, error
sanitization, retry scheduling, and rendering.

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
- `src/openrouter_api/tests.rs`
- `src/openrouter_update/tests.rs`
- `src/feed_update/liquidations/tests.rs`
- Telegram tests in `src/feed_update/` and `src/telegram_*`
- `src/screener_*` tests
- `src/calendar_*` tests

For integration changes, test both missing-key and configured-key paths.
