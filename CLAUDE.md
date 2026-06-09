# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Kerosene is a GPU-accelerated desktop trading terminal for Hyperliquid, built in Rust (edition 2024) with the **iced** 0.14 GUI framework. This is trading software: never log, print, snapshot, or commit private keys, API keys, or other secret material outside the established `secret_storage` paths.

**`AGENTS.md` is the authoritative, detailed agent guide for this repo** (code style, naming, derives, UI conventions, error handling, testing expectations). Read it before making non-trivial changes. This file covers the essentials; AGENTS.md governs where the two overlap.

## Commands

```sh
cargo check                                          # fast type-check
cargo fmt                                            # always run after Rust changes
cargo test test_name                                 # run tests matching a name
cargo test --package kerosene --bin kerosene module::path::tests   # focused module tests
cargo test                                           # full suite
cargo clippy --all-targets --all-features -- -D warnings           # CI-level lint
cargo run                                            # launch the app (debug)
timeout 20s xvfb-run -a cargo run                    # Linux headless GUI smoke test
                                                     # (timeout after window start is OK; a panic is not)
```

No custom `rustfmt.toml` or `clippy.toml` — Rust defaults apply. Packaging is via `./scripts/package.sh` (Linux/macOS) and `pwsh ./scripts/package-windows.ps1`. Harnesses in `tests/manual/` are dev references, not part of `cargo test`.

Note: `src/lib.rs` is empty — all modules are declared in `src/main.rs` and compiled into the binary, which is why focused tests target `--bin kerosene`.

## Architecture

Single-binary Elm-style app launched through `iced::daemon`:

- **State** — `TradingTerminal` in `src/app_state.rs` is the central state container. Feature-specific state lives in `*_state.rs` modules or feature `model.rs` files, not in new top-level fields.
- **Messages** — every user action, network result, timer tick, and persistence result is a variant of `Message` in `src/message.rs`.
- **Update** — `src/app_update.rs` is the root update. `src/app_update/routing.rs` maps each `Message` to an `UpdateRoute` that dispatches to a feature update module (`*_update.rs`). **When you add a `Message` variant, you must also route it in `routing.rs`.** Synchronous changes return `Task::none()`; async work uses `Task::perform(async_fn, Message::Variant)` with async fns returning `Result<T, String>`.
- **View** — `*_views.rs` modules render UI as pure functions of state: no I/O, no task spawning, no mutation, no clock reads in view code.
- **Subscriptions** — websocket, timer, user-data, integration, and window subscriptions are assembled in `src/subscription_state.rs` and children. Keep subscription identity stable so iced can manage long-lived streams.

### Feature module pattern

Each feature follows the same layout: `foo_state.rs` (+ `foo/model.rs`) for domain state, `foo_update.rs`/`foo_update/` for message handling and side effects, `foo_views.rs`/`foo_views/` for rendering. Tests live beside the code as `tests.rs` or `tests/` modules under `#[cfg(test)]`. Larger features use `foo.rs` plus a `foo/` directory of submodules.

Multi-instance widgets (charts, order books, watchlists, spaghetti charts) are keyed by typed IDs (`ChartId`, `OrderBookId`, …) in maps — pass IDs through messages rather than relying on a global "active" instance.

### Cross-cutting flows to keep in sync

- **Panes/windows**: pane types are defined in `PaneKind`. Adding one touches pane creation, view dispatch, layout persistence, default widget config, and tests. Detached windows carry `window::Id` in messages.
- **Persistence**: persisted state goes through `config*` modules (JSON at the platform config dir). Wire-format changes need serialization tests and backwards-compatible defaults to preserve saved layouts.
- **Secrets**: keychain/encrypted-secret handling lives in `secret_storage/`; use `SensitiveString`/`Zeroizing` patterns. Order placement and signing (`signing/` — EIP-712 + MessagePack action payloads) changes require focused tests for request construction, response parsing, and failure handling.
- **Transport**: REST types/fetchers in `api/`, WebSocket streams/managers in `ws/`; optional integrations in `hydromancer_api/`, `hyperdash_api/`, and the Telegram/X feed modules.
- **Charts**: iced `canvas::Program<Message>` rendering with state in `chart_state`/`chart`. Keep expensive geometry bounded to the visible range; invalidate caches on data, theme, scale, or viewport changes; keep interaction math in chart update modules with focused tests.

## Conventions that bite

- Avoid `unwrap()`; prefer `?`, `match`, `if let`, or explicit fallbacks. `expect()` only for true invariants, with a useful message.
- Match local style: section banner comments (`// ---- Section Name ----` style where present), `view_` prefix for view helpers, imports grouped at the top, no `use std::*`.
- Add or update tests when changing calculations, parsing, routing, persistence, order planning, signing, risk filtering, chart geometry, or state machines; for bug fixes add a regression test that fails without the fix.
- Prefer the standard library or existing dependencies before adding a crate; never hand-edit `Cargo.lock`.
