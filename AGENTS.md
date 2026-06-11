# AGENTS.md - Coding Agent Instructions for Kerosene

Last audited against the repository: 2026-05-31.

## Project Overview

Kerosene is a GPU-accelerated desktop trading terminal for Hyperliquid, built in
Rust with the **iced** GUI framework.

- **Package:** `kerosene` 0.1.8
- **Rust edition:** 2024
- **Framework:** `iced` 0.14 with `default`, `tokio`, `canvas`, `svg`, `advanced`,
  and `image` features
- **Architecture:** Elm-style state/messages/update/view, launched through
  `iced::daemon`
- **Domain:** live market data, charting, account/wallet state, order placement,
  client-side Chase/TWAP automation, layouts, themes, settings, and optional
  integrations

This is trading software. Protect private keys, API keys, wallet addresses where
appropriate, and any generated config or secret material. Never print or commit
secrets.

## External Rule Files

No Cursor or Copilot instruction files were present when this file was last
updated. Treat this `AGENTS.md` as the repository-level source of agent guidance.

## Current Repository Map

Entry and application shell:

- `src/main.rs` declares the binary modules and starts `iced::daemon`.
- `src/app_state.rs` defines `TradingTerminal`, the central mutable state.
- `src/app_boot/` builds initial state from persisted config.
- `src/message.rs` defines the `Message` enum.
- `src/app_update.rs` is the root update method.
- `src/app_update/routing.rs` maps each `Message` to a feature update route.
- `src/main_view.rs` and `src/main_view/` render the main window shell.
- `src/subscription_state.rs` and `src/subscription_state/` assemble websocket,
  timer, user-data, integration, and window subscriptions.

Common feature layout:

- `*_state.rs` and `*/model.rs` files hold domain state and pure model helpers.
- `*_update.rs` and `*_update/` files handle messages and side effects.
- `*_views.rs` and `*_views/` files render UI.
- Tests live beside code as `tests.rs` or `tests/` modules.

Important feature areas:

- Account and wallet: `account*`, `wallet_*`, `account_analytics*`,
  `account_metrics*`, `portfolio_*`, `journal*`
- Market data and widgets: `market_state*`, `market_update*`, `market_views*`,
  `feed_*`, `hype_etf_state*`, `hype_unstaking_state*`
- Charts: `chart*`, `chart_state*`, `chart_update*`, `chart_views*`,
  `chart_screenshot*`, `spaghetti*`, `spread_chart*`
- Orders and signing: `order_execution*`, `order_update*`, `order_views*`,
  `signing*`, `twap_state*`, `advanced_order_history*`
- App chrome and configuration: `config*`, `settings_*`, `preferences_update*`,
  `layout_*`, `pane_*`, `window_*`, `status_bar*`, `toast_overlay*`
- Integrations and transport: `api*`, `ws*`, `hydromancer_api*`,
  `hyperdash_api*`, `hyperdash_update*`
- Command and tools: `alfred_*`, `screener_*`, `calendar_*`, `pnl_card*`,
  `annotations*`, `sound*`

Assets, docs, and packaging:

- `assets/` contains icons, bundled fonts, screenshots, and sounds.
- `docs/` contains feature guides for Alfred, advanced orders, and the journal.
- `scripts/package.sh`, `scripts/package-macos.sh`, and
  `scripts/package-windows.ps1` handle release packaging.
- `packaging/` contains platform packaging templates.

## Build, Run, Lint, and Test Commands

No custom `rustfmt.toml` or `clippy.toml` was present when this file was updated.
Use Rust defaults.

Fast feedback:

- `cargo check` - type-check the app
- `cargo test test_name` - run tests matching a name
- `cargo test pattern -- --exact` - run an exact test target
- `cargo test --package kerosene --bin kerosene module::name::tests` - run a
  focused module test target

Standard validation:

- `cargo fmt` - format after code changes
- `cargo fmt -- --check` - CI-style formatting check
- `cargo test` - run all Rust tests
- `cargo clippy --all-targets --all-features -- -D warnings` - lint with CI-level
  strictness

Run and smoke test:

- `cargo run` - run debug build
- `cargo run --release` - run release build
- `timeout 20s xvfb-run -a cargo run` - Linux headless GUI smoke test; a timeout
  after the window starts is acceptable, but a panic is not

Packaging:

- `./scripts/package.sh all` - package Linux targets where tools are available
- `./scripts/package.sh deb|rpm|appimage|macos` - package one Linux/macOS target
- `pwsh ./scripts/package-windows.ps1` - Windows package workflow

Manual harnesses in `tests/manual/` are development references and are not part
of `cargo test`.

## Code Style Guidelines

### Formatting and Imports

- Run `cargo fmt` after every Rust code change.
- Keep imports at the top of each file.
- Prefer grouped imports with braces when importing several items from the same
  path.
- Alias imports only for collisions or established local convention, such as
  `use iced::widget::container as container_style;`.
- Do not use `use std::*`.
- Avoid unrelated import churn.

### Naming Conventions

| Item           | Convention           | Example                                  |
|----------------|----------------------|------------------------------------------|
| Structs/Enums  | PascalCase           | `TradingTerminal`, `PaneKind`, `Message` |
| Enum variants  | PascalCase           | `AccountSummary`, `PaneResized`          |
| Functions      | snake_case           | `fetch_account_data`, `view_chart`       |
| Variables      | snake_case           | `pane_grid_widget`, `is_focused`         |
| Constants      | SCREAMING_SNAKE_CASE | `API_URL`, `HIP3_DEXES`, `ZOOM_SPEED`    |

- Prefix modular view functions with `view_`.
- Name update helpers after the action they perform, not the widget that
  triggered them.
- Keep IDs explicit in names when a feature has multiple pane/window instances,
  such as `ChartId`, `OrderBookId`, and `SpaghettiChartId`.

### Types and Lifetimes

- Methods returning UI from `&self` generally return `Element<'_, Message>`.
- Free UI helpers that borrow inputs should expose the needed lifetime; helpers
  that borrow nothing may return `Element<'static, Message>`.
- Use explicit style closures like `|theme: &Theme|` when it improves clarity.
- Prefix intentionally unused closure arguments with `_`.
- Use named lifetimes only when tying multiple input lifetimes to an output.

### Derives

- `Message` must derive `Debug, Clone`.
- Small value enums should usually derive `Debug, Clone, Copy, PartialEq, Eq`.
- Enums with owned data should derive `Debug, Clone`; do not force `Copy`.
- API/config wire types should derive the needed `serde` traits.
- Keep application state derives minimal. Do not add broad derives to
  `TradingTerminal`.

### File Organization

When editing or creating modules, match the local style and use section banners
where they already appear:

```rust
// ---------------------------------------------------------------------------
// Section Name
// ---------------------------------------------------------------------------
```

- Prefer `foo.rs` plus `foo/` submodules for larger features.
- Keep tests close to the module under test with `#[cfg(test)] mod tests;`.
- Use doc comments for non-obvious semantics or public crate-facing helpers.
- Do not move code across feature boundaries unless the change genuinely needs
  it.

## Architecture Rules

### State

- `TradingTerminal` is the central state container for cross-feature state.
- Feature-specific state belongs in the relevant `*_state` module or feature
  model before adding more top-level fields.
- Multi-instance widgets should use typed IDs and maps, following the existing
  chart, order book, live watchlist, positioning, and spaghetti patterns.
- Persisted state must be represented in `config*` modules and covered by
  serialization tests when the wire format changes.

### Messages and Updates

- Every user action, network result, timer tick, and persistence result should
  flow through a well-named `Message` variant.
- After adding a `Message`, update `src/app_update/routing.rs` so it reaches the
  correct feature update module.
- Keep `TradingTerminal::update` routed through feature methods. Do not grow a
  large catch-all branch in `app_update.rs`.
- Return `Task::none()` for synchronous state changes.
- Use `Task::perform(async_fn, Message::Variant)` for asynchronous work.
- Async functions should return `Result<T, String>` unless a typed error is
  already established nearby.
- Map errors with context using `.map_err(|e| format!(...))`.

### Views

- Views must be pure functions of state. Do not perform I/O, spawn tasks, mutate
  state, or read the clock from view code.
- Put feature UI in the matching `*_views` module or submodule.
- Keep pane and window views instance-aware; pass IDs through messages instead
  of relying on global active state unless the existing feature does so.
- Main shell overlays belong in `main_view` or the existing overlay modules.

### Subscriptions

- Add websocket, timer, and window subscriptions through
  `subscription_state.rs` or its child modules.
- Keep subscription identity stable so iced can manage long-lived streams.
- Convert stream events into `Message` values and handle them in update modules.

### PaneGrid and Windows

- Pane definitions live in `PaneKind` and pane management/update modules.
- When adding a pane type, update pane creation, view dispatch, layout
  persistence, default widget config, and tests as needed.
- Detached or auxiliary windows must carry `window::Id` in messages where the
  target matters.
- Preserve saved layout compatibility when changing pane config wire types.

### Canvas and Charts

- Chart rendering uses iced canvas with `canvas::Program<Message>` and chart
  state in `chart_state`/`chart`.
- Keep expensive geometry bounded to visible ranges, as the current candle and
  overlay drawing code does.
- If adding caches, clear or invalidate them on data, theme, scale, or viewport
  changes that affect rendering.
- Keep interaction math in chart interaction/update modules and cover it with
  focused tests.

## Error Handling and Security

- Avoid `unwrap()`. Prefer `?`, `match`, `if let`, or explicit fallbacks.
- `expect()` is only acceptable for invariants that are truly guaranteed and
  should include a useful message.
- `.unwrap_or_default()` is acceptable when the default behavior is intentional.
- Model loading, empty, stale, disconnected, and error states explicitly in
  state types.
- Use `SensitiveString`/`Zeroizing` patterns for secret-bearing strings.
- Never log, print, snapshot, or serialize private keys/API keys outside the
  intended secret-storage path.
- Keep keychain, encrypted secret, and config persistence changes covered by
  tests.
- Order placement and signing changes require focused tests for request
  construction, response parsing, stale-account behavior, and failure handling.

## UI Guidelines

- Follow existing iced widget patterns and local helper modules in `helpers/ui`.
- Use `Length::Fill`, proportional layout, and stable dimensions for scalable
  panes.
- Avoid fixed pixel widths except for dividers, compact controls, and values
  already fixed by local convention.
- Most pane bodies should follow the established pattern of content wrapped in a
  `container` and usually a `scrollable`, with width/height set to fill.
- Tables and lists generally use: header row, `rule::horizontal(1)`, then rows
  folded into a `Column`.
- Use theme palette values or existing helper colors. Do not introduce one-off
  color constants when a theme value exists.
- Keep bullish/buy and bearish/sell color semantics consistent with existing
  success/danger usage.
- Text sizes should stay close to local convention: small data text around 12px,
  muted metadata around 11px, pane titles around 13px, featured prices around
  16px unless nearby code uses a different established size.
- Do not add explanatory in-app text for obvious controls. Prefer clear labels,
  icons, tooltips where already used, and predictable layout.

## Testing Expectations

- Add or update tests when changing calculations, parsing, routing, persistence,
  order planning, signing, risk filtering, chart geometry, or state machines.
- Prefer focused tests in the module closest to the behavior.
- For bug fixes, add a regression test that fails without the fix when practical.
- For config schema changes, test serialization and backwards-compatible
  defaults.
- For UI-only layout changes, at minimum run `cargo check`; run targeted tests
  for any helper logic touched.
- Do not rely on manual harnesses as the only validation for core logic.

## Dependency and Asset Changes

- Keep dependencies conservative and aligned with the existing stack.
- Prefer the standard library or existing dependencies before adding a crate.
- If changing `Cargo.toml`, let Cargo update `Cargo.lock`; do not edit lockfile
  entries by hand.
- Keep release-impacting dependencies compatible with Linux, macOS, and Windows
  packaging paths.
- Place bundled fonts, sounds, images, and icons under `assets/` and update any
  embedding or packaging code that must include them.
## Practical Workflow for Agents

1. Read the relevant state, update, view, and test modules before editing.
2. Make the smallest change that fits the existing feature boundary.
3. Update message routing, subscriptions, persistence, and tests when the change
   crosses those boundaries.
4. Run `cargo fmt`.
5. Run the narrowest meaningful tests or checks first, then broader validation
   when the blast radius is larger.
6. Report what changed and which commands passed or could not be run.
