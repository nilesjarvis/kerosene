# System Overview

Kerosene is a GPU-accelerated desktop trading terminal for Hyperliquid. It is a
single Rust binary using iced's daemon API, async tasks, pane grids, canvas
programs, and multiple windows.

The application is organized around one central state object:

- `src/main.rs` wires modules and starts `iced::daemon`.
- `src/app_state.rs` defines `TradingTerminal`, the top-level mutable state.
- `src/message.rs` defines every user, network, timer, persistence, and window
  event as a `Message`.
- `src/app_update.rs` and `src/app_update/routing.rs` dispatch messages to
  feature-specific update modules.
- `src/main_view.rs` and `src/main_view/` render the main shell and route panes
  and windows to feature views.
- `src/subscription_state.rs` and `src/subscription_state/` build websocket,
  timer, keyboard, and window subscriptions.

The codebase favors explicit feature boundaries. State lives in feature modules
where possible, feature update modules own side effects, and view modules stay
pure.

## Runtime Shape

```text
main.rs
  loads config
  builds iced settings
  starts iced::daemon

TradingTerminal::boot_from_config
  normalizes config
  restores pane/window/widget state
  allocates feature instances
  queues initial REST fetches and window tasks

iced runtime
  calls subscription()
  receives Message values
  calls update(message)
  runs returned Task<Message>
  calls view_window(window_id)
```

The binary starts with a persisted `KeroseneConfig`, but runtime state is richer
than the config schema. For example, charts hold visible candle geometry and
transient fetch status, order execution holds pending request contexts, and
account state holds merged REST and websocket data. Persistence is therefore a
snapshot operation, not direct serialization of `TradingTerminal`.

## Startup Lifecycle

Startup is implemented in `src/app_boot/`:

1. `config::load_config()` reads `config.json` from the platform config
   directory and collects config or secret warnings.
2. `TradingTerminal::boot_from_config` registers the last layout and determines
   layout ratios.
3. Symbol selection, muted tickers, read-data provider, active account profile,
   address book, wallet tracker, and widget configs are normalized.
4. Chart and spaghetti instances are created from persisted widget configs.
5. A default pane layout is created when no compatible saved pane tree exists.
6. Order books, positioning info panes, session data panes, detached chart
   windows, settings, journal, wallet tracker, and other auxiliary state are
   restored.
7. Initial tasks are batched for exchange symbols, order books, positioning
   data, session data, account auto-connect, windows, calendar/feed data, HYPE
   widgets, watchlists, and ticker tape context refresh.
8. Theme and chart preference values are synchronized into chart instances.

Boot intentionally does not perform blocking I/O in view code. All network work
is returned as iced `Task<Message>` values.

## Main Architectural Boundaries

| Boundary | Modules | Responsibility |
| --- | --- | --- |
| Application shell | `main.rs`, `app_state.rs`, `app_boot/`, `app_update/`, `main_view/`, `subscription_state/` | Runtime boot, central state, message routing, shell rendering, subscriptions. |
| Config and persistence | `config/`, `config_persistence/`, `layout_persistence/` | Config schema, load/save, saved layouts, widget snapshots, secret references. |
| Market data | `api/`, `ws/`, `market_state/`, `market_update/`, `market_views/` | REST models, websocket streams, symbols, books, mids, watchlists, market widgets. |
| Charting | `chart/`, `chart_state/`, `chart_update/`, `chart_views/`, `chart_screenshot/`, `spaghetti*`, `spread_chart/` | Candle/funding state, canvas rendering, interaction, overlays, screenshots, comparison charts. |
| Trading | `order_execution/`, `order_update/`, `order_views/`, `signing/`, `twap_state/`, `advanced_order_history/` | Order forms, validation, sizing, signing, submissions, Chase/TWAP lifecycle, order feedback. |
| Account and wallet | `account/`, `account_state/`, `account_update/`, `account_views/`, `wallet_state/`, `wallet_update/`, `wallet_views/` | Account profiles, account data fetch/merge, user stream application, wallet tracker/details. |
| Portfolio and analytics | `portfolio_state/`, `portfolio_update/`, `account_analytics/`, `account_metrics/`, `pnl_card/` | Portfolio history, income data, PnL calculations, exportable PnL cards. |
| Journal | `journal/`, `journal_update.rs`, `journal_views/` | Fill cache, trade aggregation, notes, summary charts, chart snapshots. |
| Integrations and feeds | `feed_state/`, `feed_update/`, `feed_views/`, `hydromancer_api/`, `hyperdash_api/`, `hyperdash_update/`, `telegram_*`, `calendar_*`, `screener_*` | Optional external data sources, feed panes, alerts, external API keys. |
| Chrome and preferences | `settings_*`, `preferences_update/`, `app_theme/`, `status_bar/`, `toast_overlay/`, `hotkey_state/`, `sound/`, `window_chrome.rs` | Settings UI, themes, fonts, sounds, hotkeys, notifications, status bar, custom chrome. |
| Layout and panes | `pane_state.rs`, `pane_update/`, `pane_interaction_update/`, `pane_management.rs`, `layout_update/`, `layout_preview/` | Pane definitions, add/remove/split, resize, layout import/export, layout compatibility. |

## External Service Boundaries

Kerosene talks to several external systems:

- Hyperliquid REST info API through `src/api/`.
- Hyperliquid exchange websocket through `src/ws/manager.rs` and market/user
  stream helpers.
- Hyperliquid exchange signing endpoints through `src/signing/`.
- Hydromancer read-data, liquidation, and tracked-trade APIs when configured.
- HyperDash liquidation level and heatmap APIs when configured.
- Telegram HTTP/MTProto feed paths when configured.
- X recent-post and streaming paths when a bearer token is configured.
- SEC company submissions for optional chart earnings markers.

External API keys are secret-bearing state. They must not be logged, committed,
or serialized into plaintext config.

## Rendering Model

Most UI is ordinary iced widgets returned from `view_*` functions. Charts and
some visual exports use canvas or image-oriented rendering:

- Main window: `TradingTerminal::view_main`.
- Pane content routing: `main_view/panes.rs`.
- Auxiliary windows: `main_view/windows.rs`.
- Chart pane: `chart_views.rs` and `chart/`.
- Screenshot window and bitmap export: `chart_screenshot/`.
- PnL card image generation: `pnl_card/image/`.

Views should be pure functions of state. If a UI interaction needs I/O or state
mutation, it should emit a `Message` and let an update module handle it.

## Persistence Model

Config persistence is handled through snapshots:

- Runtime state is converted to `config::KeroseneConfig` by
  `config_persistence/save/snapshot.rs`.
- Layout-specific state is captured by `layout_persistence/`.
- Secrets are persisted through `secret_storage/` and `config/secrets/`; config
  snapshots intentionally blank secret fields.
- Journal fill cache is stored separately per wallet through
  `config::journal_cache_path`.
- Imported fonts and sounds are copied into platform config subdirectories and
  referenced by safe file names.

This separation makes it possible to evolve runtime state without exposing
secret material or serializing transient websocket/task state.

## Testing Shape

Tests are colocated with the modules they protect. The highest-risk areas have
focused unit or integration-style tests:

- Order execution, Chase, TWAP, close/nuke, order status, and signing.
- Account data parsing, merge, fee handling, and websocket repair.
- Chart geometry, viewport, overlays, candle/funding data, and screenshots.
- Journal fill aggregation and cache behavior.
- Config schema normalization, layout snapshots, secret crypto, hotkeys, and
  persistence.
- Websocket manager reconnect/coalescing behavior.

Use focused tests for behavior changes, then run broader validation when the
change crosses routing, persistence, signing, or shared model boundaries.
