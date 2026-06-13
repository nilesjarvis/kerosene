# Subscriptions And Tasks

Kerosene receives data from two side-effect mechanisms:

- `Subscription<Message>` for continuous streams such as websockets, timers,
  keyboard input, and window events.
- `Task<Message>` for one-shot async work such as REST fetches, signing/order
  calls, file dialogs, imports, exports, and opening windows.

Both mechanisms convert their results back into `Message` values. Update modules
then apply those messages to `TradingTerminal`.

## Subscription Assembly

`src/subscription_state.rs` builds a batch:

```text
push_market_subscriptions
push_user_data_subscriptions
push_hydromancer_subscriptions
push_telegram_subscriptions
push_timer_subscriptions
push_window_subscriptions
push_post_window_timer_subscriptions
```

Each child module contributes only subscriptions that are relevant to the
current state. For example, liquidation feed streams are only added when the
Hydromancer key is present and the liquidation pane is open.

## Market Subscriptions

Market subscriptions live under `src/subscription_state/market/`.

They cover:

- chart candle streams and chart asset-context streams
- spaghetti/comparison chart candle streams
- order-book L2 book and asset-context streams
- positioning-info asset-context streams
- Chase book streams and reprice ticks
- TWAP book streams
- live watchlist refresh ticks
- ticker tape context refresh ticks

The order-book subscription path chooses Hyperliquid or Hydromancer stream
helpers based on the configured read-data provider and available Hydromancer
key. Hidden or unsupported symbols are skipped before subscriptions are added.

L2 book subscriptions use one canonical live precision per coin across
order-book panes, Chase, and TWAP. Stream helpers filter frames when a provider
echoes `nSigFigs` or `mantissa`, and the coalescers keep echoed precision
variants in separate pending slots. When a provider omits those precision
fields, individual frames cannot be positively attributed after they arrive on
the shared manager, so new L2 consumers should continue to subscribe through
the canonical precision path or use an isolated stream before introducing
same-coin multi-precision live subscriptions.

## User Data Subscriptions

`src/subscription_state/user_data.rs` always pushes a user-data stream with
`WsUserDataStreamParams`.

The stream covers:

- connected account private user data when a wallet is connected
- all-mids subscriptions for visible dexes
- wallet detail windows for addresses different from the connected address

Results become:

- `Message::WsUserDataUpdate`
- `Message::WalletDetailsWsUpdate`

The stream filters private subscriptions internally when no address is
connected.

## Integration Subscriptions

Hydromancer subscriptions:

- `ws_hydromancer_liquidations`
- `ws_hydromancer_tracked_trades`
- Hydromancer candle/book/asset-context streams when selected as read provider

Telegram subscriptions:

- `telegram_fast_feed_stream` when fast mode is enabled, a feed pane is open,
  an API ID is available, and channels/private channels are configured.

X subscriptions:

- X stream events when streaming is enabled and a bearer token/source list is
  configured.

Integration streams should avoid creating subscriptions when required keys or
visible panes are missing. This prevents unnecessary external connections.

## Timer And Input Subscriptions

`src/subscription_state/timers.rs` delegates to:

- app/UI timers
- keyboard subscriptions
- account timers
- HyperDash timers
- feed timers
- wallet tracker timers
- analytics timers

Timer messages drive:

- spinner/status-bar animation
- toast cleanup
- chart price flashes and HUD safety
- account refresh/backoff
- Chase and TWAP scheduling
- feed refreshes
- watchlist refreshes
- liquidation/heatmap refreshes

Keyboard events are routed through preferences and hotkey logic so global
commands use the same `Message` path as button-driven commands.

## Window Subscriptions

`subscription_state.rs` adds iced window events:

- close events -> `Message::WindowClosed`
- resized -> `Message::WindowResized`
- moved -> `Message::WindowMoved`
- unhandled window event -> `Message::Tick`

Window messages let `window_update.rs` persist auxiliary window state, remove
closed windows from maps, and keep layout min sizes synchronized.

## Hyperliquid Websocket Manager

`src/ws/manager.rs` owns the shared Hyperliquid websocket multiplexer.

Key properties:

- A global `OnceLock` starts one manager task.
- Subscribers send `WsCommand::Subscribe`, `Unsubscribe`, and `Ping`.
- Active subscriptions are replayed after reconnect.
- Incoming text frames are parsed into channel/data pairs.
- A coalescer batches high-frequency channel updates before broadcast.
- Telemetry records connect/disconnect, RX/TX bytes, API latency, and websocket
  ping latency.
- Stale read timeouts force reconnects to recover from half-open sockets.
- `SubscriptionGuard` unsubscribes topics when a stream is dropped.

Feature stream helpers in `src/ws/market_streams/` and `src/ws/user_streams/`
subscribe to manager channels and convert routed payloads into typed app data.

## Hydromancer Websocket Paths

Hydromancer streams live under `src/ws/hydromancer/`. They are separate from
the Hyperliquid manager because they use a different service, API key, and
stream semantics.

Hydromancer covers:

- liquidation feed
- tracked trades
- alternative candle/book/asset-context streams

Hydromancer keys are secret-bearing values and should only be passed into
stream setup or request tasks, never logged.

## One-Shot Tasks

Common `Task::perform` uses:

- `api::fetch_exchange_symbols`
- `api::fetch_order_book`
- `api::fetch_watchlist_contexts`
- `api::fetch_candles` and `api::fetch_chart_backfill_candles`
- `api::fetch_user_fills`
- account data fetches and portfolio/income analytics
- order placement/cancel/modify/status checks
- Hydromancer and HyperDash HTTP calls
- SEC earnings fetches
- layout/wallet-label import/export
- screenshot and PnL card image I/O
- font/sound imports

Tasks should produce result messages that carry enough context to reject stale
responses. Examples include chart ID, request key, cloid/oid, account address,
timeframe, symbol, and fetch timestamp.

## Stable Identity Requirements

Subscriptions need stable identities because iced uses identity to decide
whether a stream is the same stream or a new one. Good identity keys include:

- chart ID + symbol + timeframe
- order book ID + symbol + sigfigs
- account address + visible mids dex list
- integration key + reconnect nonce + tracked addresses
- TWAP/Chase ID + symbol

When the state changes, identity should change only when the underlying stream
must change.

## Stale Response Guards

Many modules keep the request that is currently in flight:

- `ChartInstance::candle_fetch_request`
- `ChartInstance::funding_fetch_request`
- liquidation/heatmap pending keys
- journal fill request state
- order status context
- account refresh follow-up state
- positioning/session data request keys

When a result arrives, the update module compares the result context with the
latest expected context before applying it. This prevents old network responses
from overwriting newer state.

## Adding A Subscription

1. Add the stream helper in `src/ws/`, an integration module, or an appropriate
   `subscription_state` child module.
2. Choose a stable identity tuple.
3. Convert stream items into specific `Message` values.
4. Route the messages in `app_update/routing.rs`.
5. Apply updates in the owning feature update module.
6. Add tests for identity, gating, parsing, stale-response handling, or
   reconnect behavior when practical.

Do not start sockets or timers from views.
