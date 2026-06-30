# Settings, Preferences, And Themes

Settings and preferences translate user configuration into runtime state,
persisted config, and chart/UI synchronization. This area includes themes,
fonts, pane chrome, chart visuals, hotkeys, sounds, read-data provider, risk
filters, secret storage controls, integrations, layouts, and notifications.

## Component Map

| Component | Key files | Responsibility |
| --- | --- | --- |
| Settings state/update | `src/settings_state.rs`, `src/settings_update.rs` | Settings window tab selection and lifecycle. |
| Settings views | `src/settings_views/` | Themes, layouts, risk, integrations, storage, hotkeys, sidebar. |
| Preferences update | `src/preferences_update.rs`, `src/preferences_update/` | Persist preference changes and synchronize affected runtime systems. |
| Theme system | `src/app_theme.rs`, `src/app_theme/`, `src/config/themes.rs` | Built-in and custom themes, extended palettes, chart colors. |
| Fonts | `src/app_fonts.rs`, `src/config/fonts.rs`, `src/preferences_update/fonts.rs` | Bundled fonts, imported fonts, iced font settings. |
| Hotkeys | `src/hotkey_state/`, `src/preferences_update/hotkeys/`, `src/settings_views/hotkeys.rs` | Action catalog, recording, matching, execution, display. |
| Sound | `src/sound/`, `src/preferences_update/sounds.rs` | Built-in/generated/custom sounds, HUD sounds, volume, import. |
| Risk settings | `src/risk_state/`, `src/settings_views/risk/` | Muted tickers, market universe, denomination, market slippage, optimistic updates. |

## Settings Window

The settings window is routed through `main_view/windows.rs` and
`settings_update.rs`.

Current settings tabs include:

- Themes
- Layouts
- Risk
- Integrations
- Storage
- Hotkeys

View modules are split by tab. The deprecated settings pane remains available
for compatibility but the window is the primary settings surface.

## Preferences Update Flow

Preference changes usually follow this shape:

```text
view emits Message
  -> app_update/routing.rs routes to Preferences or Settings
  -> preferences_update.rs normalizes value
  -> runtime state changes
  -> affected charts/windows/widgets are synchronized
  -> persist_config()
  -> optional Task for import, refresh, or window sizing
```

Normalization functions live in `config/schema.rs` and related config modules.
Use them before storing values such as UI scale, chart effect strength, pane
border thickness, widget padding, popup scale, and market slippage.

## Theme System

Themes are split into:

- built-in theme definitions under `app_theme/`
- custom theme config under `config/themes.rs`
- runtime mapping in `app_theme.rs`

Theme changes call:

- `apply_chart_theme_colors`
- chart visual sync helpers
- config persistence

Chart colors are derived from the active theme through
`app_theme/chart_colors.rs` so bullish/bearish semantics stay consistent across
charts and market widgets.

## Chart Visual Preferences

Chart preferences include:

- dotted background and opacity
- theme-aware gradient background
- hollow candle mode
- fisheye effect and strength
- chromatic aberration and strength
- edge blur and strength
- crosshair style, guides, and scale
- HUD readout elements
- HUD order sound and volume
- HUD UI sounds

Preference updates modify top-level state, call chart sync helpers, and persist
config. The sync helpers push the preference values into existing chart
instances so changes are visible without restarting.

## Fonts

Kerosene embeds bundled display and monospace fonts and supports imported
custom fonts.

Key paths:

- `app_fonts.rs`
- `config/fonts.rs`
- `preferences_update/fonts.rs`
- `settings_views/themes/fonts.rs`

Imported fonts are copied to the platform config directory and referenced by a
safe stored file name. Do not persist arbitrary user-supplied paths.

## Pane Chrome And UI Scale

User-adjustable chrome includes:

- UI scale
- pane border thickness
- pane corner radius
- outer widget border
- default widget padding
- focused widget padding
- custom window chrome

Changes that affect minimum usable layout dimensions should call
`sync_main_window_min_size`.

## Hotkeys

Hotkeys are configured in settings and executed through keyboard subscriptions.

Key modules:

- `hotkey_state/display.rs`
- `hotkey_state/groups.rs`
- `hotkey_state/matching.rs`
- `preferences_update/hotkeys/`
- `settings_views/hotkeys.rs`

Hotkeys can execute actions such as toggling Alfred, changing chart timeframes,
placing commands, or focusing tools. Recording state is stored in
`recording_hotkey_for`.

Keyboard events become messages first; actions should still route through the
same update modules as button clicks.

## Sound And Notifications

Sound state includes:

- global sound enabled
- HUD order sound
- custom HUD order sound file
- HUD order sound volume
- chart HUD UI sounds

`sound.rs` queues audio, uses `rodio` where available, and falls back to
platform sounds where needed. Imported sounds are copied to the config sound
directory and referenced safely.

Desktop notifications use `notify-rust` and are controlled by notification
toggles. Notification text should not include secrets.

## Risk Preferences

Risk settings include:

- muted tickers
- market universe
- display denomination
- market slippage
- optimistic account updates

Risk preferences feed into symbol search, account views, market widgets, and
order automation. Hidden/muted symbols should be filtered before subscriptions,
rows, or trading automation are created.

## Read Data Provider

`ReadDataProvider` controls whether read paths use Hyperliquid or Hydromancer
where supported.

Changing provider:

- updates `read_data_provider`
- updates chart backfill source
- clears journal chart snapshot cache
- persists config
- warns if Hydromancer is selected without an API key
- reloads chart backfills
- refreshes account data

This setting is a runtime behavior change, not just a cosmetic preference.

The integrations settings also include `hydromancer_realtime_position_pnl_enabled`.
When enabled and a Hydromancer API key is saved, Kerosene subscribes to
Hydromancer `l2Book` ticks for currently visible open perp positions and uses
the same book-mid prices as Tick candles for the positions widget's mark, value,
uPnL, and total PnL. This is independent of the global read-data provider
selection.

## Storage Settings

Storage settings include:

- credential storage mode
- encrypted secret unlock/apply controls
- config clearing
- credential status

See [Security And Secrets](../operations/security-and-secrets.md) for the
secret-storage model.

## Tests To Check

Use focused tests in:

- `src/config/tests/**`
- `src/config/fonts/tests.rs`
- `src/config/hotkeys/tests/**`
- `src/config/themes/**/tests`
- `src/preferences_update/hotkeys/keyboard/tests.rs`
- `src/preferences_update/**/tests` where present
- `src/sound/tests.rs`
- `src/risk_state/**/tests`
- `src/app_theme/**/tests` where present

For UI-only settings changes, run `cargo check`. For config schema or
normalization changes, add serialization/default tests.
