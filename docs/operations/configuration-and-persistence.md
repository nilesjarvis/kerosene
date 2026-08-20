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
- Assistant sessions: `assistant_sessions.json`
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
| `src/agent_persistence.rs` | Bounded, owner-only Assistant session side-file load and atomic save. |

## KeroseneConfig

`KeroseneConfig` is the durable contract for the app. It stores:

- saved layouts and active layout name
- first-run app onboarding dismissal
- pane layout and legacy layout ratios
- Canvas workspace trees, labels, open state, and window geometry
- widget configs
- detached chart windows
- active symbol and order defaults
- UI scale, pane chrome, fonts, themes, chart display preferences
- accounts, active account index, hidden positions
- wallet tracker, Combined Portfolio, wallet clusters, and address book
- favourites, muted tickers, market universe, denomination
- feed and notification preferences
- Telegram/X channel/source lists
- journal entries and per-account journal entries, including reflection fields
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
- Canvas runtime state is captured as `CanvasConfig`; runtime `window::Id`
  values are never serialized.
- Canvas pane trees that contain only unknown future pane types are preserved
  even though the current build cannot open them.
- Account profiles are converted through persisted account snapshots.
- Hidden positions are scoped to persisted accounts.
- Journal entries are scoped by account and omit ghost account data where
  appropriate.
- Secret fields such as `agent_key`, `hydromancer_api_key`,
  `hyperdash_api_key`, `x_access_token`, `x_oauth_client_id`, and
  `x_refresh_token` are written as empty values.
- Wallet cluster config stores cluster names, selected cluster/window state, and
  account profile secret-id references, not private agent keys.
- Combined Portfolio stores only watch-only addresses, optional labels, open
  state, and window geometry. Fetched portfolio histories remain runtime-only.
- Read-data provider controls the persisted chart backfill source.
- Widget configs come from layout/widget snapshot helpers, not direct runtime
  maps.

This design prevents transient websocket/task state and secret material from
leaking into config.

## Save Lifecycle

Config saves are debounced and run off the main update path:

- `persist_config()` schedules a save.
- save lifecycle tracks due time, in-flight status, and exit-requested state.
- the app onboarding dismissal flag defaults to visible only for brand-new
  default configs; older serialized configs without the field load as dismissed.
- final-save-before-exit behavior prevents losing recent changes.
- file writes use temporary files and backup paths for resilience.

If the primary config disappears during an upgrade or reinstall, startup loads
the valid backup before falling back to defaults. If neither config copy is
readable but an OS-keychain credential bundle remains, startup reconstructs the
saved account identifiers and wallet bindings from that bundle, assigns generic
names to the recovered profiles, and persists the recovered metadata.
Encrypted-config credentials require a surviving primary or backup config
because the encrypted blob is stored in those files.

OS-keychain credential changes use a scoped read-modify-write transaction. Each
account or integration flow updates only its own fields in the durable bundle;
an unreadable bundle blocks the change instead of allowing an incomplete
runtime snapshot to overwrite other saved credentials. Explicit clears remove
only their named profile or integration fields.

When changing any persisted preference or layout state, call `persist_config()`
from the owning update module after state changes.

## Layout Persistence

Saved layouts are separate from the app's global config snapshot. They capture a
workspace:

- pane tree
- Canvas workspace trees and best-effort window placement
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
data-backed widgets. It also replaces the current Canvas set, closes obsolete
Canvas windows, and reopens those saved as open. Layout loading should be
tolerant of unsupported or older pane config. Older configs deserialize with an
empty Canvas list. Imported layouts run the same Canvas geometry and
multi-instance ID normalization used for loaded config before they are stored
or applied.

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

## Assistant Sessions

Assistant chats are stored in a separate, schema-versioned
`assistant_sessions.json` side-file. This keeps potentially large conversation
history out of `KeroseneConfig` and lets Assistant saves follow their own
lifecycle. Writes use a temporary file and replacement, owner-only permissions
on Unix, an owner-only ACL on Windows, and a 32 MiB file limit. Clear All Config
removes the durable file and its interrupted-save temp file.

The side-file contains bounded drafts and user/assistant messages, but never the
OpenRouter API key, tool activity cards, Pi process state, or raw tool payloads.

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
