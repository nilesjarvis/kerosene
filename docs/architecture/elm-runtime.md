# Elm Runtime And Message Flow

Kerosene uses iced in an Elm-style architecture. The code is easier to reason
about when each event is followed through the same path:

```text
source event -> Message -> message_route -> update_* -> Task<Message> -> state
state -> view_* -> iced widgets/canvas
state -> subscription() -> stream events -> Message
```

## Messages

`src/message.rs` defines `Message`, a large `Debug, Clone` enum. It is the
typed event bus for the app. Messages cover:

- User input: button clicks, form input, pane interactions, hotkeys, chart
  gestures, window actions.
- Network results: REST responses, websocket updates, order placement results,
  order-status checks, integration fetches.
- Timer ticks: UI animation, account refresh, Chase/TWAP scheduling,
  watchlist/feed refreshes, status bar refresh.
- Persistence results: config save, layout import/export, wallet label I/O,
  credential apply flows.
- Window events: open, close, move, resize, detached chart/window-specific
  actions.

Every new behavior should add a precise message variant rather than overloading
an unrelated one. When a message targets a particular pane/window/chart/order,
the ID should travel in the message.

## Update Routing

`src/app_update.rs` exposes the root update method:

```rust
pub(crate) fn update(&mut self, message: Message) -> Task<Message>
```

It delegates to `src/app_update/routing.rs`, which maps each message to an
`UpdateRoute`. The route then calls a feature update method such as
`update_order`, `update_market`, `update_chart`, or `update_account`.

Current route groups:

| Route | Update module | Main responsibility |
| --- | --- | --- |
| `Account` | `account_update.rs` | Account profiles, connect/disconnect, account data, user stream updates, PnL card actions. |
| `Alfred` | `alfred_update.rs` | Command palette query, selection, and command execution. |
| `Annotations` | `annotation_update.rs` | Chart drawing tools and persisted annotations. |
| `Calendar` | `calendar_update.rs` | Economic calendar fetches and filters. |
| `Chart` | `chart_update.rs` | Chart symbol/timeframe/editing, candle/funding data, HUD, overlays, detached charts. |
| `ChartScreenshot` | `chart_screenshot/update.rs` | Screenshot capture, bitmap output, screenshot window lifecycle. |
| `Chrome` | `chrome_update.rs` | Toasts, status bar, sound toggles, top-level UI cleanup. |
| `Feed` | `feed_update.rs` | Hydromancer liquidation/tracked-trade feeds, Telegram, X. |
| `Hyperdash` | `hyperdash_update.rs` | HyperDash key, liquidation overlays, heatmap, liquidation distribution. |
| `Journal` | `journal_update.rs` | Fill loading, cache, notes, chart snapshots, journal window. |
| `Layout` | `layout_update.rs` | Saved layouts, import/export, wallet label import/export. |
| `Market` | `market_update.rs` | Symbols, watchlists, ticker tape, order books, positioning info, session data, HYPE widgets. |
| `Order` | `order_update.rs` | Order form, submit/cancel, quick/HUD orders, close/nuke, Chase, TWAP, move-order. |
| `PaneInteractions` | `pane_interaction_update.rs` | Pane grid resize/drag/click and minimum sizing. |
| `Panes` | `pane_update.rs` | Add/remove panes and add-widget menu behavior. |
| `PortfolioIncome` | `portfolio_update.rs` | Portfolio and income fetches for portfolio panes/windows. |
| `Preferences` | `preferences_update.rs` | Themes, UI scale, chart preferences, fonts, sounds, hotkeys, risk preferences. |
| `Screener` | `screener_update.rs` | Screener window, sort/filter, context/history loads. |
| `Settings` | `settings_update.rs` | Settings tab and settings-window lifecycle. |
| `Spaghetti` | `spaghetti_update.rs` | Comparison and pair-ratio charts. |
| `WalletTracker` | `wallet_update.rs` | Wallet tracker and wallet detail windows. |
| `Window` | `window_update.rs` | Window size/position/open/close state. |

Routing is intentionally explicit. After adding a `Message`, update
`message_route` so the message reaches the correct feature update module.

## Update Modules

Update modules mutate `TradingTerminal` and return `Task<Message>`. They should
be named after behavior, not after widgets. Common patterns are:

- Synchronous state change: mutate state and return `Task::none()`.
- Async REST or filesystem work: return `Task::perform(async_fn, Message::...)`.
- Multiple side effects: collect tasks and return `Task::batch`.
- Delayed or chained behavior: return a `Task` that produces the next message.

Examples:

- `order_update.rs` routes order form changes, submission results, quick orders,
  Chase/TWAP lifecycle messages, close-position actions, and move-order actions.
- `market_update.rs` further routes market messages into symbols, order books,
  live watchlist, ticker tape, positioning info, session data, and HYPE modules.
- `chart_update.rs` further routes chart messages into candles, editor,
  detached charts, macro indicators, and earnings.
- `feed_update.rs` routes feed connection, liquidation, tracked-trade,
  Telegram, and X messages.

Update modules are the only place that should intentionally mutate application
state or start side effects.

## Tasks

iced `Task<Message>` values are Kerosene's async side-effect channel. They are
used for:

- Hyperliquid REST fetches.
- Hydromancer/HyperDash fetches.
- Order placement, cancel, modify, leverage updates, and status verification.
- Layout and wallet-label import/export.
- Font and sound imports.
- Chart screenshots and PnL card image export.
- Opening or closing auxiliary windows.

Async functions should return `Result<T, String>` unless a nearby typed error is
already established. Errors should include context close to the failing boundary
and become user-visible status, toasts, or disabled states where appropriate.

## Views

View functions produce iced widgets from state:

- `TradingTerminal::view_window` dispatches by `window::Id`.
- `TradingTerminal::view_main` renders the main shell.
- `main_view/panes.rs` maps each `PaneKind` to the corresponding feature view.
- Feature view modules usually use `view_*` prefixes and return
  `Element<'_, Message>`.

Views should not:

- Perform I/O.
- Spawn tasks.
- Mutate `TradingTerminal`.
- Read the clock directly for business logic.

When a view needs new data, it emits a message and lets an update module produce
a task or mutate state.

## Subscriptions

Subscriptions are the continuous event sources:

- Websocket streams for candles, books, asset context, user data, Hydromancer,
  Telegram fast feed, X stream, Chase, TWAP, positioning info, and spaghetti
  charts.
- Timers for UI animation, account refresh, feed refresh, order automation,
  status bar, and other scheduled work.
- Window close/move/resize events.
- Keyboard events for hotkeys and shortcuts.

Subscriptions must have stable identities so iced can keep long-lived streams
alive and drop stale streams when the corresponding state disappears.

## Adding A New Action

Use this checklist for a new user action or network result:

1. Add a focused `Message` variant with all target IDs it needs.
2. Add the variant to `message_route`.
3. Implement handling in the owning update module.
4. Return `Task::none()` for pure state changes or `Task::perform` for async
   work.
5. Emit the message from a view, subscription, or task result.
6. Persist state only when the action changes persisted configuration.
7. Add focused tests for parsing, routing, state transitions, persistence, or
   order/signing behavior when those surfaces are touched.

Do not handle unrelated messages in `app_update.rs`; the root update method
should remain a router.
