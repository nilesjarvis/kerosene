# Kerosene Technical Documentation

This directory is the technical map for Kerosene. Start from the project
[README](../README.md), then use this page to move from the application-wide
architecture into the implementation details for each major component.

Kerosene is a single Rust binary built with iced. The runtime follows the Elm
Architecture:

```text
configuration -> boot -> TradingTerminal state
user/network/timer/window event -> Message -> update route -> Task<Message>
TradingTerminal state -> view_window/view_main -> iced widgets and canvas
subscriptions -> Message -> update route
```

The most important rule for reading or changing the code is that state,
messages, updates, views, subscriptions, and persistence are intentionally
separate. The docs below describe where each concern belongs.

## Recommended Reading Path

1. [System Overview](architecture/system-overview.md)
2. [Elm Runtime And Message Flow](architecture/elm-runtime.md)
3. [Application State Map](architecture/state-map.md)
4. [Subscriptions And Tasks](architecture/subscriptions-and-tasks.md)
5. [Configuration And Persistence](operations/configuration-and-persistence.md)
6. [Security And Secrets](operations/security-and-secrets.md)
7. Component guides for the feature area you are changing.
8. [Testing, Validation, And Packaging](operations/testing-validation-packaging.md)

## Architecture Guides

| Guide | Purpose |
| --- | --- |
| [System Overview](architecture/system-overview.md) | High-level binary layout, startup lifecycle, architectural boundaries, and dependency shape. |
| [Elm Runtime And Message Flow](architecture/elm-runtime.md) | How `Message`, `TradingTerminal::update`, update routing, `Task`, and pure views work together. |
| [Application State Map](architecture/state-map.md) | The top-level `TradingTerminal` fields grouped by feature ownership and persistence behavior. |
| [Subscriptions And Tasks](architecture/subscriptions-and-tasks.md) | Websocket, timer, keyboard, window, REST, and async side-effect flow. |

## Component Guides

| Guide | Primary coverage |
| --- | --- |
| [Layouts, Panes, And Windows](components/layouts-windows-and-panes.md) | `PaneKind`, pane grid routing, detached windows, saved layouts, add-widget flow. |
| [Market Data And Symbols](components/market-data-and-symbols.md) | Symbol universe, mids, books, watchlists, ticker tape, positioning info, session data, HYPE widgets. |
| [Volatility Metrics](components/volatility-metrics.md) | Realized volatility, ATR/NATR, ATR-distance, and proposed chart/watchlist/screener integration. |
| [Charting And Canvas](components/charting-and-canvas.md) | Chart instances, candle/funding data, canvas rendering, viewport, overlays, screenshots, spaghetti charts. |
| [Trading And Order Execution](components/trading-and-order-execution.md) | Order entry, quick/HUD orders, Chase, TWAP, close/nuke, move-order, signing boundaries. |
| [Account, Wallet, And Portfolio](components/account-wallet-portfolio.md) | Account profiles, REST/user-stream data, positions, balances, wallet tracker, portfolio and income views. |
| [Journal And Analytics](components/journal-and-analytics.md) | Fill cache, trade aggregation, notes, chart snapshots, account analytics, PnL card. |
| [Integrations And Feeds](components/integrations-and-feeds.md) | Hydromancer, HyperDash, Telegram, X, calendar, screener, feed rendering, notifications. |
| [Settings, Preferences, And Themes](components/settings-preferences-and-theme.md) | Settings window, theme system, fonts, hotkeys, sounds, risk preferences, UI scaling. |
| [Alfred Command Surface](components/alfred-command-surface.md) | Command palette architecture and links to the detailed Alfred feature guide. |

## Operations Guides

| Guide | Purpose |
| --- | --- |
| [Configuration And Persistence](operations/configuration-and-persistence.md) | Config schema, snapshots, saved layouts, credentials references, imported assets, journal cache. |
| [Security And Secrets](operations/security-and-secrets.md) | Secret-bearing state, OS keychain and encrypted-config flows, signing risks, logging rules. |
| [Testing, Validation, And Packaging](operations/testing-validation-packaging.md) | Focused tests, full validation, smoke tests, packaging scripts, manual harness scope. |
| [Development Guide](operations/development-guide.md) | Checklist for adding panes, messages, subscriptions, persisted state, and high-risk trading changes. |

## Existing Feature Guides And Audits

These documents remain useful when working in their specific feature areas:

- [Alfred](alfred.md)
- [Telegram Feed](telegram-feed.md)
- [X Feed](x-feed.md)
- [Trading Journal](journal.md)
- [Chase Orders](advanced-orders/chase-orders.md)
- [TWAP Orders](advanced-orders/twap-orders.md)
- [Chase Metrics Audit](advanced-orders/chase-metrics-audit.md)
- [Liquidation Feed UI Audit](liquidation-feed-ui-audit.md)
- [Order Lifecycle Refactor Audit](order-lifecycle-refactor-audit.md)
- [Journal Chart Snapshots Plan](journal-chart-snapshots-plan.md)

## Documentation Maintenance Rules

- Update these docs when adding a new top-level module, `PaneKind`, `Message`
  route, subscription family, config schema field, secret type, or external
  integration.
- Keep user-facing behavior in feature guides and implementation boundaries in
  component guides.
- Document persisted wire-format changes with the compatibility expectations and
  tests that protect them.
- Do not include private keys, API keys, wallet-private material, or real user
  account data in examples.
