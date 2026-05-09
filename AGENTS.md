# AGENTS.md — Coding Agent Instructions for Kerosene

## Project Overview

Kerosene is a trading terminal GUI built in Rust using the **iced** framework (v0.14).
It follows the Elm Architecture (state, messages, update, view) as prescribed by iced.
The application logic is broken down into various modules inside `src/` (e.g., `main.rs`, `api.rs`, `account.rs`, `chart.rs`, `spaghetti.rs`, `ws.rs`).

- **Rust edition:** 2024
- **Framework:** iced 0.14 (features: `default`, `tokio`, `canvas`, `svg`)
- **Key dependencies:** `reqwest` (HTTP), `serde`/`serde_json` (serialization), `tokio-tungstenite` (WebSocket), `k256` + `sha3` (signing), `rodio` (audio).
- **Architecture:** Elm Architecture — `TradingTerminal` struct holds all core state, `Message` enum defines events, `update()` handles mutations, and pure `view()` methods render the UI.

## Cursor and Copilot Rules

There are no external cursor/copilot rule files in this repository. All rules are contained in this `AGENTS.md` file.

## Build, Run, Lint, and Test Commands

*Note: No custom `rustfmt.toml` or `clippy.toml` config exists. All formatting uses Rust defaults.*

### Building & Running
- `cargo build` — Build (debug mode)
- `cargo build --release` — Build (release mode, optimized)
- `cargo run` — Run the application
- `cargo check` — Type-check only (fastest feedback loop)

### Linting & Formatting
- `cargo clippy` — Lint (must pass with zero warnings)
- `cargo clippy --fix` — Automatically fix linting issues
- `cargo fmt -- --check` — Format check (CI-style, no changes)
- `cargo fmt` — Apply format (Run this after every code change)

### Testing
- `cargo test` — Run all tests (inline and test modules)
- `cargo test test_name` — Run a single test by name (e.g., `cargo test my_test_fn`)
- `cargo test pattern -- --exact` — Run tests matching a pattern exactly
- `cargo test --package kerosene --bin kerosene -- module::name::tests` — Run specific module tests
- `cargo test -- --nocapture` — Run tests and show standard output (for debugging)

## Code Style Guidelines

### Formatting and Imports
- **Formatter:** Use `cargo fmt` defaults (max line width 100 chars, 4 spaces indentation).
- **Imports:** Place at the top of the file, grouped sequentially by path depth:
  1. Single-item deep imports and aliases (`use iced::widget::container as container_style;`).
  2. Grouped widget imports (alphabetical).
  3. Top-level crate imports (`use iced::{...}`).
  4. Third-party crates (alphabetical).
- Prefer multi-item `use` with braces over separate `use` lines. Alias only for collisions. No `use std::*`.

### Naming Conventions
| Item           | Convention           | Example                                    |
|----------------|----------------------|--------------------------------------------|
| Structs/Enums  | PascalCase           | `TradingTerminal`, `PaneKind`, `Message`   |
| Enum variants  | PascalCase           | `AccountSummary`, `PaneResized`            |
| Functions      | snake_case           | `fetch_account_data()`, `view_chart()`     |
| Variables      | snake_case           | `pane_grid_widget`, `is_focused`           |
| Constants      | SCREAMING_SNAKE_CASE | `API_URL`, `HIP3_DEXES`, `ZOOM_SPEED`      |

- Prefix UI helper constructors descriptively (e.g., `buy_button`, `tab_button`).
- Prefix modular view components with `view_` (e.g., `view_order_book`).

### Types and Lifetimes
- Methods returning `Element` from `&self` use `Element<'_, Message>`.
- Free functions not borrowing anything return `Element<'static, Message>`.
- Use explicit closures for styling: `|theme: &Theme|`. Prefix unused parameters with an underscore (`_theme`).
- Named lifetimes (e.g., `'a`) only when tying multiple input lifetimes to the output.

### Derive Macros
- **Message enums:** Must have `#[derive(Debug, Clone)]` (required by iced runtime).
- **Small value enums:** Apply `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`.
- **Enums with non-Copy data:** Use `#[derive(Debug, Clone)]` — no `Copy`.
- **API types:** Apply `#[derive(Debug, Clone, Serialize, Deserialize)]`.
- **State structs:** Keep minimal. Do not use unneeded derives on application state.

### File Organization
When editing or creating modules, organize them with comment banners:

```rust
// ---------------------------------------------------------------------------
// Section Name
// ---------------------------------------------------------------------------
```

- Use sub-section banners (slightly shorter dashes) within `impl` blocks.
- ASCII layout diagrams encouraged for complex UI structures.
- Doc comments (`///`) on methods with non-obvious semantics; not required for straightforward private items in a binary crate.

### Error Handling
- Use `Result<T, Error>` for functions handling fallible external states (I/O, Parsing).
- Propagate iced errors in `main()` using `?`.
- Async functions return `Result<T, String>`, mapping errors with `.map_err(|e| format!(...))` where appropriate instead of unwrapping.
- **Strict rule:** Avoid `unwrap()`. Prefer `if let`, `match`, or `?`. `expect()` may only be used when invariants are strictly guaranteed. `.unwrap_or_default()` is acceptable.
- Model UI loading/error states explicitly via typed enums (e.g., `ChartStatus::Error(String)`).
- Stub unimplemented handlers with: `// No-op -- will wire to API later`.

### Architecture & Iced Framework Patterns
- **State:** Centralize primary mutable state in `TradingTerminal`. Pane-specific state goes in `PaneKind` variants or as `TradingTerminal` fields for cross-pane state.
- **Messages:** Ensure every distinct user action maps to a well-named `Message` variant.
- **Update:** Use exhaustive pattern matching in `update()`. Return `Task::none()` for synchronous changes and `Task::perform(async_fn, Message::Variant)` for asynchronous work.
- **View:** `view()` MUST be pure. No side effects. Factor each pane into its own `view_*` function.
- **Canvas:** Implement `canvas::Program<Message>` with `type State = ChartState`. Use `canvas::Cache` for render caching; clear on state changes.
- **Layout:** Rely heavily on `Length::Fill` and proportions for scalable grids. Avoid fixed pixel widths outside of dividers (`rule::horizontal(1)`). Use `[vertical, horizontal]` padding.
- **PaneGrid:** `pane_grid::Configuration` tree defines initial layout. `Axis::Horizontal` for top/bottom splits, `Axis::Vertical` for left/right. Ratios in [0.0, 1.0].

### UI Patterns
- **Pane wrapper:** Most panes end with `container(scrollable(content)).width(Fill).height(Fill).padding(10).into()`.
- **Table/list views:** header row → `rule::horizontal(1)` → data rows built with `.fold(Column::new().spacing(N), ...)`.
- **Colors:** Green (`color!(0x50fa7b)`) for bullish/buy, red (`color!(0xff5555)`) for bearish/sell — consistent everywhere.
- **Text sizing:** 12px for data, 11px for muted/secondary (with `color!(0x888888)`), 13px for titles, 16px for featured prices.
