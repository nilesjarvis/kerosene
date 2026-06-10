# Development Guide

This guide is a practical checklist for changing Kerosene without violating its
state, routing, persistence, UI, or security boundaries.

## Before Editing

1. Identify the owning feature boundary.
2. Read the relevant state, update, view, subscription, config, and tests.
3. Check whether the behavior is single-instance or multi-instance.
4. Check whether the behavior is persisted.
5. Check whether secrets, signing, account freshness, or hidden-symbol risk
   filters are involved.

Avoid broad refactors while making feature changes.

## Adding A Message

1. Add a focused `Message` variant in `src/message.rs`.
2. Include IDs for target chart, pane, order, window, wallet, or instance.
3. Add it to `message_route` in `src/app_update/routing.rs`.
4. Handle it in the owning update module.
5. Emit it from the view/subscription/task that owns the event.
6. Add routing or behavior tests when the message crosses important boundaries.

Do not grow a catch-all branch in `app_update.rs`.

## Adding A Pane

Update:

- `PaneKind` in `src/pane_state.rs`
- persisted `PaneKindConfig`
- add-widget menu section
- pane creation in `pane_update/`
- `main_view/panes.rs`
- layout persistence conversion
- boot restoration/defaults
- widget config snapshots if the pane owns instance state
- tests for layout compatibility and pane routing

For multi-instance panes, define a typed ID and store instances in a map.

## Adding A Window

Update:

- state field for the `window::Id`
- open-window update task
- `main_view/windows.rs` view dispatch
- `window_title`
- `window_update.rs` close/resize/move handling
- config snapshot/boot restoration if size or open state persists

Messages that affect an auxiliary window should carry `window::Id` when the
target matters.

## Adding Persisted State

1. Add a wire field to the appropriate config type.
2. Provide a default for old configs.
3. Normalize loaded values if bounded or user-editable.
4. Include the field in config snapshots.
5. Restore it during boot or layout application.
6. Persist after updates.
7. Add serialization/default/backward-compatibility tests.

Do not serialize runtime maps directly if they contain transient state.

## Adding Secret State

1. Use `SensitiveString` or `Zeroizing<String>`.
2. Decide whether the secret is profile-scoped or global.
3. Add it to `SecretPayload` or the relevant secret storage path.
4. Keep plaintext config snapshot fields empty or skipped.
5. Add storage, unlock, and omission tests.
6. Redact all UI/log/status output.

Do not put new API keys into normal `KeroseneConfig` fields as plaintext.

## Adding A Subscription

1. Put stream construction under `subscription_state/` or `src/ws/`.
2. Gate it on visible/open state and required credentials.
3. Choose a stable identity tuple.
4. Convert stream items into typed messages.
5. Route messages to the owning update module.
6. Add stale-response or parsing tests where practical.

Do not open sockets from views or update methods directly.

## Adding A REST Task

1. Add typed API request/response models under `src/api/` or the integration
   module.
2. Return `Result<T, String>`.
3. Add request context to the result message when stale responses are possible.
4. Start the task from an update module with `Task::perform`.
5. Apply the result only if context still matches current state.
6. Test success, parse failure, and stale-response behavior where practical.

## Changing Order Execution

Read:

- `order_update.rs`
- `order_execution/core.rs`
- relevant `order_execution/*` module
- `signing/`
- `risk_state/`
- relevant tests

Preserve:

- account/key identity
- stale account checks
- reduce-only behavior
- hidden-symbol filters
- market-type restrictions
- order status verification for ambiguous results
- zeroizing secret handling

Add focused regression tests before broad validation.

## Changing Charts

Read:

- `chart_state/model.rs`
- relevant `chart_update/` module
- `chart/` rendering or interaction module
- `chart_views/` surface composition
- relevant cache/viewport tests

Check:

- visible range bounds
- cache invalidation
- detached surface behavior
- chart ID and surface ID in messages
- theme/style synchronization
- overlay sync after account/order updates

For canvas changes, focused tests plus a GUI smoke test are useful.

## Changing Account Or Wallet Data

Read:

- `account/types/`
- `account/data/`
- `account_update/connection/`
- `account_update/stream.rs`
- wallet modules if watch-only views are affected

Check:

- HIP-3 normalization
- spot fallback behavior
- stale/freshness metadata
- websocket repair behavior
- optimistic updates
- account-scoped hidden positions and journal entries

High-risk account changes should include tests for stale snapshots and merge
behavior.

## Changing Settings Or Config

Read:

- `preferences_update.rs`
- relevant `settings_views/` tab
- `config/schema.rs`
- feature-specific config module
- config tests

Check:

- normalization
- persistence
- boot restoration
- runtime sync into existing instances
- window min-size updates if layout changes
- secret omission if credentials are nearby

## Documentation Updates

Update docs when changing:

- top-level architecture
- message route ownership
- `PaneKind`
- persisted config schema
- secret storage behavior
- order execution or signing flow
- websocket subscription identity
- external integrations
- packaging scripts

Use `docs/README.md` as the navigation spine.

## Final Validation

Pick the narrowest meaningful tests first. For broad or risky changes, finish
with:

```sh
cargo fmt -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

For GUI startup or rendering changes, also run:

```sh
timeout 20s xvfb-run -a cargo run
```
