# Layouts, Panes, And Windows

Kerosene's main workspace is an iced `pane_grid` plus a set of auxiliary
windows. The layout system is responsible for creating panes, routing panes to
views, saving pane trees, restoring widget instances, and keeping window state
compatible across releases.

## Runtime Components

| Component | Key files | Responsibility |
| --- | --- | --- |
| Pane definitions | `src/pane_state.rs` | Runtime `PaneKind` enum and pane defaults. |
| Pane creation | `src/pane_update/`, `src/pane_management.rs`, `src/add_widget_menu/` | Add-widget menu, insert/split helpers, pane focus behavior. |
| Pane interactions | `src/pane_interaction_update.rs`, `src/pane_interaction_update/min_size.rs` | Resize, drag/drop, click/focus, close cleanup, minimum sizing. |
| Pane rendering | `src/main_view/grid.rs`, `src/main_view/panes.rs` | Pane grid chrome and `PaneKind` to feature-view routing. |
| Saved layouts | `src/layout_update/`, `src/layout_persistence/`, `src/config/layouts.rs` | Save/load/import/export layout snapshots and restore widget configs. |
| Windows | `src/main_view/windows.rs`, `src/window_update.rs`, `src/window_chrome.rs` | Multi-window view routing, titles, size/position persistence, custom chrome. |
| Boot restoration | `src/app_boot/panes.rs`, `src/app_boot/windows.rs`, `src/app_boot/widget_configs.rs` | Rebuild panes, windows, and feature instances from config. |

## PaneKind

`PaneKind` is the runtime routing enum for the main pane grid. It includes
instance-aware variants for multi-instance widgets:

- `Chart(ChartId)`
- `OrderBook(OrderBookId)`
- `LiveWatchlist(LiveWatchlistId)`
- `PositioningInfo(PositioningInfoId)`
- `SessionData(SessionDataId)`
- `SpaghettiChart(SpaghettiChartId)`

It also includes singleton pane types:

- `Watchlist`
- `Portfolio`
- `Income`
- `BottomTabs`
- `OrderEntry`
- `AdvancedOrders`
- `Settings`
- `Calendar`
- `Liquidations`
- `LiquidationsDistribution`
- `TrackedTrades`
- `TelegramFeed`
- `XFeed`
- `Outcomes`
- `HypeEtfs`
- `HypeUnstakingQueue`

When adding a pane type, update all of these surfaces:

- runtime `PaneKind`
- persisted `PaneKindConfig`
- add-widget menu section
- pane creation/update route
- `main_view/panes.rs` view dispatch
- layout persistence conversion
- boot default/widget config restoration if it owns instance state
- tests for layout compatibility or pane creation

## Main Pane Rendering

`TradingTerminal::view_main` composes:

1. Account summary/top bar.
2. Optional ticker tape.
3. Main pane grid.
4. Toast overlay.
5. Alfred overlay.
6. Encrypted-credentials unlock overlay.
7. Status bar.

`main_view/grid.rs` builds the pane grid. It supplies pane title bars, close
controls, chart-specific buttons, add-widget affordances, drag styling, resize
events, and pane click messages.

`main_view/panes.rs` is the final dispatch point:

```text
PaneKind::Chart(id) -> view_chart(id, chart_count)
PaneKind::OrderBook(id) -> view_order_book(id)
PaneKind::LiveWatchlist(id) -> view_live_watchlist(id)
...
```

Feature views should not infer target instance from global focus when a pane ID
exists. Pass IDs through messages.

## Add-Widget Flow

The add-widget menu uses:

- `add_widget_menu/` for body, sections, and UI components.
- `pane_management.rs` for insertion helpers and placement.
- `pane_update/` for message handling.
- `AddWidgetPlacement` to control where a new pane is inserted relative to the
  active pane.

Adding a multi-instance pane usually allocates an ID, creates an instance in the
feature map, inserts the pane, optionally queues initial fetch tasks, and then
persists config.

## Pane Interaction Flow

`pane_interaction_update.rs` handles:

- `PaneResized`
- `PaneDragged`
- `PaneClicked`
- pane close messages

Important side effects:

- Chart click can update focus/primary chart and active symbol.
- Closing a pane must remove or preserve related feature instance state
  intentionally.
- Closing a chart can require detached-window cleanup or primary-chart
  reassignment.
- Closing order book/watchlist/session/positioning panes should remove the
  corresponding instance when it is no longer referenced.
- Resizes can require window minimum-size synchronization.

Minimum sizing lives under `pane_interaction_update/min_size.rs`. It prevents
important surfaces such as order entry from collapsing below usable dimensions.

## Saved Layouts

Saved layouts are user-named snapshots. They include:

- pane tree and split ratios
- active symbol and order form defaults
- active theme and custom themes
- ticker tape, favourites, alert toggles, and slippage settings
- order presets
- chart configs
- spaghetti chart configs
- order book configs
- live watchlists
- positioning info panes
- session data panes
- widget padding overrides

`layout_persistence.rs` applies a saved layout by:

1. Normalizing active symbol and order kind.
2. Applying theme/order/favourite/alert settings.
3. Restoring chart and spaghetti instances.
4. Closing detached chart windows for missing charts.
5. Restoring the pane tree.
6. Applying widget padding.
7. Restoring order books, live watchlists, positioning info, and session data.
8. Queuing refresh tasks for open data-dependent panes.
9. Syncing chart colors and chart display preferences.

Saved layout application should preserve compatibility with old configs. If a
new pane cannot be restored safely, prune it rather than panicking.

## Config Wire Types

Runtime layout types are not serialized directly. Persisted wire types live in
`src/config/layouts.rs` and `src/config/panes.rs`:

- `SavedLayout`
- `PaneLayoutConfig`
- `PaneKindConfig`
- `AxisConfig`
- `WidgetPaddingConfig`
- `ChartConfig`
- `OrderBookConfig`
- `PositioningInfoConfig`
- `SessionDataConfig`
- `SpaghettiChartConfig`
- `DetachedChartWindowConfig`

This split allows runtime structs to change while preserving config
compatibility.

## Windows

Kerosene is an iced daemon app, so the runtime can render more than one window.
`main_view/windows.rs` routes `window::Id` values to:

- main trading terminal
- settings
- screener
- journal
- wallet tracker
- wallet details
- TWAP details
- advanced-order history details
- chart screenshot
- PnL cards
- detached charts

`window_title` mirrors this routing and creates context-specific titles such as
`Kerosene TWAP #...` or `Kerosene Chart - HYPE 1h`.

`window_update.rs` owns size, move, close, and open-window state. When opening a
new window, the update module should store the returned `window::Id` in the
owning feature state so later messages can target it.

## Detached Charts

Detached charts use the same chart instance model as inline chart panes but
render through a `ChartSurfaceId::Detached(window_id)`. This lets one
`ChartInstance` maintain per-surface interaction state while sharing symbol,
timeframe, candle data, overlays, and preferences.

Detach/close behavior must keep these maps consistent:

- `charts`
- `detached_chart_windows`
- `chart_surface_viewports`
- persisted detached chart configs

## Custom Window Chrome

`window_chrome.rs` and `main_view/title_bar.rs` handle platform-aware chrome.
Settings can enable custom chrome where supported. The setting may require
platform-specific behavior:

- Linux can apply decoration changes live.
- macOS may require restart for OS bar preference changes.
- Windows uses `windows_subsystem = "windows"` and platform resources for
  packaged builds.

## Tests To Check

Use focused tests when changing this area:

- `src/app_boot/chart_instances/tests.rs`
- `src/layout_persistence/snapshots/layout_tree/tests.rs`
- `src/layout_persistence/snapshots/widgets/**`
- `src/pane_interaction_update/tests.rs`
- tests in `src/pane_interaction_update/min_size.rs`
- `src/config/tests/**` for serialization and legacy compatibility
- `src/app_update/routing/tests/**` when adding pane/window messages

At minimum, run `cargo check` after UI-only pane changes.
