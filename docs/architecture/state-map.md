# Application State Map

`TradingTerminal` in `src/app_state.rs` is the central mutable state for the
application. It owns cross-feature state directly and delegates richer feature
models to dedicated modules.

This document groups the state by responsibility so contributors know where a
new field belongs and whether it should be persisted.

## Top-Level State Principles

- Put feature-specific models in the matching `*_state` module before adding
  broad top-level fields.
- Use typed IDs and maps for multi-instance widgets.
- Keep runtime-only status separate from persisted config snapshots.
- Store secret-bearing strings as `SensitiveString` or `Zeroizing<String>`.
- Do not derive broad traits for `TradingTerminal`; keep state behavior
  explicit.

## Shell And Layout State

Representative fields:

- `panes: pane_grid::State<PaneKind>`
- `focus`, `dragging_pane`
- `saved_layouts`, `active_layout_name`, `layout_input`
- `app_onboarding_dismissed`, which gates the first-run welcome screen
- `add_widget_menu_open`, `layout_menu_open`, `add_widget_placement`
- `main_window_id`, window size/position fields, auxiliary window IDs
- pane chrome settings such as border thickness, corner radius, widget padding,
  custom chrome, and UI scale

Primary modules:

- `pane_state.rs`
- `pane_update/`
- `pane_interaction_update/`
- `pane_management.rs`
- `layout_update/`
- `layout_persistence/`
- `window_update.rs`
- `main_view/`

Persisted through:

- `config::SavedLayout`
- `config::PaneLayoutConfig`
- widget config snapshots in `layout_persistence/`
- window size/position fields in `KeroseneConfig`
- first-run app onboarding dismissal in `KeroseneConfig`

## Theme, Preferences, And Chrome State

Representative fields:

- `active_theme`, `custom_themes`
- chart visual preferences: dotted and gradient backgrounds, hollow candles,
  fisheye, chromatic aberration, edge blur, crosshair style/guides/scale
- chart HUD preferences: sound, volume, readout elements, UI sounds
- font preferences and imported custom fonts
- runtime preference-asset import sequence and per-target completion owners;
  prepared completion messages temporarily own clone-safe staging-file leases
- `hotkeys`, `chart_timeframe_hotkey_prefix`, `recording_hotkey_for`
- toast, sound, desktop notification, status bar, and muted ticker state

Primary modules:

- `preferences_update.rs` and children
- `settings_state.rs`, `settings_update.rs`, `settings_views/`
- `app_theme/`
- `app_fonts.rs`
- `hotkey_state/`
- `sound/`
- `status_bar/`
- `toast_overlay.rs`

Persisted through:

- `config::KeroseneConfig`
- `config::CustomThemeConfig`
- `config::DisplayFontConfig`
- `config::HotkeyConfig`
- safe imported asset paths under the platform config directory

## Market And Symbol State

Representative fields:

- `active_symbol`, `active_symbol_display`
- `exchange_symbols`, symbol search query/sort/filter state
- market universe and HIP-3/outcome filters
- favourites, muted tickers, ticker tape state
- `all_mids`, `all_mids_updated_at_ms`, live price flashes
- `order_books: HashMap<OrderBookId, OrderBookInstance>`
- `live_watchlists: HashMap<LiveWatchlistId, LiveWatchlistInstance>`
- positioning info and session data instances
- HYPE ETF and HYPE unstaking queue state

Primary modules:

- `market_state/`
- `market_update/`
- `market_views/`
- `api/exchange_symbols/`
- `api/watchlist/`
- `api/order_book.rs`
- `api/outcome_volume.rs`
- `api/hype_etfs/`
- `api/hype_unstaking_queue.rs`

Persisted through:

- active symbol and symbol search settings
- `config::OrderBookConfig`
- `config::LiveWatchlistConfig`
- `config::PositioningInfoConfig`
- `config::SessionDataConfig`
- favourites, muted tickers, market universe, display denomination

## Chart State

Representative fields:

- `charts: HashMap<ChartId, ChartInstance>`
- `next_chart_id`, `primary_chart_id`, and the runtime chart-incarnation
  generation
- the chart asset-context REST request allocator and coalesced spot owner
- `chart_surface_viewports`
- detached chart window state
- `spaghetti_charts: HashMap<SpaghettiChartId, SpaghettiChartInstance>`
- screenshot settings/window state plus the runtime capture owner and sequence

`ChartInstance` owns:

- symbol, display name, timeframe, and `CandlestickChart`
- asset context and price flash state
- editor/search state
- quick-order form state
- persisted annotations
- liquidation levels, heatmap, SEC earnings, funding requests, and macro
  indicator settings
- fetch request and error guards for candles/funding/overlays

Primary modules:

- `chart_state/`
- `chart/`
- `chart_update/`
- `chart_views/`
- `chart_screenshot/`
- `spaghetti_state.rs`, `spaghetti/`, `spaghetti_update/`, `spaghetti_views/`
- `spread_chart/`

Persisted through:

- `config::ChartConfig`
- `config::SpaghettiChartConfig`
- `config::DetachedChartWindowConfig`
- chart annotations and macro indicator config
- screenshot settings

## Order And Trading State

Representative fields:

- order form fields: price, quantity, denomination, percentage, kind,
  reduce-only, leverage input, leverage mode
- pending status/action contexts
- pending order indicators and move-order contexts
- order presets
- `chase_orders`, `next_chase_id`, `selected_chase_id`
- `twap_orders`, `next_twap_id`, `selected_twap_id`, `twap_form`
- `advanced_order_history`
- close-menu, nuke confirmation, pending nuke execution
- wallet cluster execution legs, which block account changes while pending

Primary modules:

- `order_execution/`
- `order_update/`
- `order_views/`
- `signing/`
- `twap_state/`
- `advanced_order_history/`

Persisted through:

- order kind, reduce-only, presets, preset denomination
- advanced order history snapshots
- completed advanced-order history, not active in-flight request contexts

Security notes:

- Agent keys must remain in `SensitiveString`/`Zeroizing` paths.
- Pending move-order context captures the original account/key so replacement
  placement cannot silently switch accounts after canceling an order.

## Account, Wallet, And Portfolio State

Representative fields:

- saved account profiles and active account index
- wallet address input, agent key input, ghost account IDs
- connected address and `account_data`
- account loading/error/backoff/reconciliation status
- hidden positions, positions sorting, PnL privacy, account picker state
- wallet tracker and wallet detail windows
- `wallet_clusters` runtime/window state and selected cluster member snapshots
- portfolio and income state
- account analytics snapshots

Primary modules:

- `account/`
- `account_state/`
- `account_update/`
- `account_views/`
- `wallet_state/`
- `wallet_update/`
- `wallet_views/`
- `wallet_cluster_state.rs`
- `wallet_cluster_update.rs`
- `wallet_cluster_views.rs`
- `portfolio_state/`
- `portfolio_update.rs`
- `account_analytics/`
- `account_metrics/`
- `pnl_card/`

Persisted through:

- account profile metadata and secret IDs
- wallet address, labels, tracker entries, wallet cluster definitions, hidden positions
- portfolio/income alert toggles
- PnL card preferences per window where applicable

Secrets are not serialized into plaintext config snapshots.

## Journal And Analytics State

Representative fields:

- `journal` state, active account scope, loaded fills, notes, filters, sorting
- journal cache status and snapshot state
- portfolio history and income snapshots
- PnL card windows and image export state

Primary modules:

- `journal/`
- `journal_update.rs`
- `journal_views/`
- `account_analytics/`
- `portfolio_state/`
- `pnl_card/`

Persisted through:

- journal notes and per-account entries
- journal cache files per wallet
- read-data provider and chart snapshot settings

## Integration And Feed State

Representative fields:

- Hydromancer key input and feed connection state
- liquidation feed and summary buckets
- tracked trades and de-duplication state
- HyperDash key and liquidation/heatmap state
- OpenRouter key generation plus runtime key-check sequence and exact owner
- X OAuth credentials, exact credential/List/source/image owners, and feed
  instances
- Telegram feed channels, fast-mode auth state, notifications
- calendar events and retry state
- screener state and loaded contexts/history

Primary modules:

- `feed_state/`, `feed_update/`, `feed_views/`
- `hydromancer_api/`
- `hyperdash_api/`, `hyperdash_update/`
- `openrouter_api.rs`, `openrouter_update.rs`
- `x_feed.rs`, `feed_update/x.rs`, `feed_views/x.rs`
- `telegram_feed.rs`, `telegram_fast_feed.rs`
- `calendar_state.rs`, `calendar_update.rs`, `calendar_views/`
- `screener_state.rs`, `screener_update.rs`, `screener_views.rs`

Persisted through:

- integration enablement, alert toggles, channels, handles, non-secret settings
- secret values via keychain or encrypted-config paths

## Transient State

Do not persist:

- in-flight REST request contexts
- websocket subscription handles
- live book snapshots unless explicitly part of a saved widget setting
- current loading spinners and stale-response guards
- temporary UI dropdown/editor state unless the feature intentionally restores it
- raw secret input buffers into plaintext config

If a transient value is needed after restart, define a config wire type and add
compatibility tests.
