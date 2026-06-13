# Integrations And Feeds

Kerosene integrates with Hyperliquid, Hydromancer, HyperDash, Telegram, X,
ForexFactory-style calendar data, SEC APIs, HYPE ETF endpoints, and
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

Hydromancer state appears in:

- `hydromancer_api.rs`
- `ws/hydromancer/`
- `feed_state/`
- `feed_update/connection.rs`
- `subscription_state/hydromancer.rs`
- `subscription_state/market/`

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
- `src/feed_update/liquidations/tests.rs`
- Telegram tests in `src/feed_update/` and `src/telegram_*`
- `src/screener_*` tests
- `src/calendar_*` tests

For integration changes, test both missing-key and configured-key paths.
