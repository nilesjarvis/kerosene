# Configuration And Persistence

Kerosene persists user configuration as JSON, but runtime state is not serialized
directly. Instead, the app creates safe config snapshots that omit secrets and
translate feature instances into compatibility-focused wire types.

## Config Location

The main config file is:

- Linux: `~/.config/kerosene/config.json`
- macOS: `~/Library/Application Support/kerosene/config.json`
- Windows: `%APPDATA%\kerosene\config.json`

Related storage:

- backup config: `config.json.bak`
- journal cache: `journal_cache_<address>.json`
- imported fonts: `fonts/`
- imported sounds: `sounds/`
- Telegram fast-feed session files in platform config storage

## Main Modules

| Module | Responsibility |
| --- | --- |
| `src/config.rs` | Public config module exports and warning collection. |
| `src/config/schema.rs` | `KeroseneConfig`, defaults, normalization bounds, core persisted fields. |
| `src/config/files/` | Platform paths, JSON load/save, temp files, backup fallback, path safety. |
| `src/config/layouts.rs` | Saved layout and pane tree wire types. |
| `src/config/panes.rs` | Widget config wire types for charts, order books, positioning info, session data, detached charts. |
| `src/config/secrets/` | Secret payload model, keychain bridge, encrypted secret crypto. |
| `src/config/themes.rs` | Custom theme config and defaults. |
| `src/config/hotkeys.rs` | Hotkey wire config. |
| `src/config/live_watchlist.rs` | Live watchlist columns and sort config. |
| `src/config_persistence/` | Debounced saves, snapshot creation, clear-config flow. |
| `src/layout_persistence/` | Saved layout application and widget snapshot conversion. |

## KeroseneConfig

`KeroseneConfig` is the durable contract for the app. It stores:

- saved layouts and active layout name
- pane layout and legacy layout ratios
- widget configs
- detached chart windows
- active symbol and order defaults
- UI scale, pane chrome, fonts, themes, chart display preferences
- accounts, active account index, hidden positions
- wallet tracker and address book
- favourites, muted tickers, market universe, denomination
- feed and notification preferences
- Telegram/X channel/source lists
- journal entries and per-account journal entries
- order presets and advanced-order history
- hotkeys
- credential storage mode and encrypted secret blob

It should not store raw active secret values.

## Snapshot Model

`config_persistence/save/snapshot.rs` converts `TradingTerminal` to
`KeroseneConfig`.

Important snapshot behavior:

- If config was cleared this session, snapshot returns default config.
- Layout state is captured through `saved_layout_snapshot("current")`.
- Account profiles are converted through persisted account snapshots.
- Hidden positions are scoped to persisted accounts.
- Journal entries are scoped by account and omit ghost account data where
  appropriate.
- Secret fields such as `agent_key`, `hydromancer_api_key`, `hyperdash_api_key`,
  and `x_bearer_token` are written as empty values.
- Read-data provider controls the persisted chart backfill source.
- Widget configs come from layout/widget snapshot helpers, not direct runtime
  maps.

This design prevents transient websocket/task state and secret material from
leaking into config.

## Save Lifecycle

Config saves are debounced and run off the main update path:

- `persist_config()` schedules a save.
- save lifecycle tracks due time, in-flight status, and exit-requested state.
- final-save-before-exit behavior prevents losing recent changes.
- file writes use temporary files and backup paths for resilience.

When changing any persisted preference or layout state, call `persist_config()`
from the owning update module after state changes.

## Layout Persistence

Saved layouts are separate from the app's global config snapshot. They capture a
workspace:

- pane tree
- chart configs
- spaghetti configs
- order book configs
- live watchlists
- positioning info panes
- session data panes
- order defaults
- theme and custom themes
- favourites/ticker tape
- alerts and slippage
- widget padding

Applying a layout rebuilds runtime instances and queues refresh tasks for
data-backed widgets. Layout loading should be tolerant of unsupported or older
pane config.

## Backward Compatibility

Config compatibility is maintained through:

- serde defaults
- normalization after load
- unsupported-pane pruning
- legacy field handling
- default theme/font/hotkey repair
- tests under `src/config/tests/**`

When adding a config field:

1. Add the field to the wire type with a default if existing configs may lack
   it.
2. Normalize loaded values if user-editable or bounded.
3. Include it in snapshots.
4. Include it in boot restoration.
5. Add serialization/default tests.

## Imported Assets

Imported fonts and sounds are copied into platform config storage. Stored file
names are checked to reject:

- empty names
- path separators
- `..`

Persist config references, not arbitrary original user paths.

## Journal Cache

Journal fill cache is per wallet and separate from `config.json`. It is
market/account history, not secret material. Writes should remain atomic and
restrictive on Unix where supported.

Do not put journal cache payloads inside `KeroseneConfig`; they can be large and
wallet-specific.

## Runtime-Only State

Do not persist:

- active Chase/TWAP automation state
- pending order indicators
- in-flight request contexts
- websocket subscription state
- current account snapshots
- all-mids and live order books
- toasts and loading spinners
- raw key/API token input buffers

If the user needs it after restart, define a config wire type and tests first.

## Tests To Check

Use focused tests in:

- `src/config/tests/**`
- `src/config/files/**/tests.rs`
- `src/config/secrets/**/tests.rs`
- `src/config_persistence/save/tests.rs`
- `src/layout_persistence/snapshots/**/tests.rs`
- `src/layout_update/layouts/tests.rs`
- feature tests for any widget config you change

Run `cargo test config` or a more specific module test for schema changes.
