# Implementation and Review

[Design language](README.md) · [Audit](audit.md) · [Components](components.md)

The implementation goal is one coherent visual pass across the application.
Build shared styles first, migrate feature families against the same reference,
then review complete screens. A pass is incomplete if the main ticket is polished
but settings, auxiliary windows, or loading states retain unrelated controls.

## Shared implementation boundaries

Retain the existing Elm architecture and feature ownership. Shared UI helpers
render and style widgets; feature views still supply labels, messages, instance
IDs, availability, values, and domain meaning. Shared styles must not read the
clock, perform I/O, access account state, or initiate trades.

The following names are suggested implementation boundaries, not existing APIs:

| Location | Proposed responsibility |
| --- | --- |
| `src/app_theme/` | Resolve semantic surface/text/action pairs from existing palettes; retain chart adapters and preset matching. |
| `src/helpers/ui/tokens.rs` | Geometry, type roles, spacing, size variants, and corner-resolution helpers. |
| `src/helpers/ui/buttons.rs` | `ButtonRole`, `ControlSize`, common pointer-state styles, selected-toggle and busy content variants. |
| `src/helpers/ui/inputs.rs` | Shared field frame, units/actions, error/read-only appearance, picker consistency. |
| `src/helpers/ui/` additional focused modules | Tabs, badges, icon actions, menu rows, table headers, and local status messages when real reuse warrants them. |
| Existing feature `*_views` | Composition, domain-specific columns/content, and message binding. |
| `src/main_view/` and `window_chrome.rs` | Workspace/pane/window frames and overlay placement. |

Use existing dependencies and widgets. Avoid building a generic component
framework that duplicates iced. Centralize recipes and small reusable wrappers;
keep specialized chart geometry and message flow in their current modules.

### State and focus limitations

The installed iced 0.14 button status enum exposes Active, Hovered, Pressed, and
Disabled. Selected, busy, invalid, and keyboard-focus state are not all supplied
by that enum. Pass domain-selected/busy state explicitly, and implement or reuse
focus tracking where the widget/runtime supports it. Do not write a nonexistent
`button::Status::Focused` match arm or claim keyboard coverage because a hover
style exists. Inputs have their own focus status and can use it directly.

A style pass can complete pointer states without changing domain state
machines. New keyboard traversal, focus management, overflow controls, inline
validation messages, or freshness timestamps may require messages/state/update
work. Track those separately and add route/state tests as required by AGENTS.md.

## Ordered convergence pass

| Stage | Concrete work | Completion evidence |
| --- | --- | --- |
| 1. Establish roles | Implement semantic palette resolution, type/spacing tokens, contrast helpers, square-corner handling. | Token tests across generated and preset palettes; source palette and user config remain intact. |
| 2. Complete primitives | Unify buttons, fields, selected rows, badges, tabs, icon targets, menu surfaces, and all states. | A native component gallery or isolated demo showing the state matrix in every review theme. |
| 3. Align the shell | Apply pane headers, outline joins, toolbar heights, account/layout controls, menus, status bar, toasts. | Complete workspace captures, including pane drag and detached windows. |
| 4. Align trading surfaces | Ticket, quick order/HUD, book, positions, open orders, Chase/TWAP, and wallet/cluster actions. | Stable pending/disabled geometry, no clipped values or action rows, focused regression checks. |
| 5. Align data panes | Search/watchlists, feeds, positioning, sessions, summaries, calendar, outcomes, HYPE panels. | Shared headers/rows, neutral zero/missing states, strongest heatmap contrast reviewed. |
| 6. Align auxiliary surfaces | Settings, account setup, journal, assistant, Alfred, console, screenshots and PnL controls. | Same control family and keyboard/focus expectations in each window. |
| 7. Remove redundant recipes | Replace migrated local styles with shared helpers; retain documented visualization/display exceptions. | Inventory shows no unexplained local variant; all acceptance checks complete. |

Stages are dependency order, not permission to ship an unfinished visual mix.
Small internal commits can make review safer while the final screen review
covers the complete pass. Do not migrate a trading action's behavior simply
because its button style changes.

## Migration checklist by family

- [ ] Shared order-type text stays the same size in every state; compact,
  standard, and prominent sizes have one definition.
- [ ] Primary, secondary, ghost, destructive, buy, and sell styles have distinct
  responsibilities and complete pointer/availability states.
- [ ] Normal, busy, and disabled order actions occupy the same frame.
- [ ] All fields, pickers, segmented modes, checkboxes, switches, and sliders
  use the common geometry and focus vocabulary.
- [ ] Account/content tabs use the shared navigation treatment; count badges
  and sidebar selection are consistent.
- [ ] Pane title bars, refresh/detach/close controls, and shell actions share
  icon size, target area, and hover treatment.
- [ ] Theme swatches, account rows, watchlists, and Alfred share persistent
  selection indicators that survive hover.
- [ ] Account, wallet, history, and journal agree on neutral zero and missing
  values, without changing domain calculations or legitimate warning states.
- [ ] Tables align header/body cells and retain usable rows at narrow widths.
- [ ] Journal, assistant, Telegram, and console consume shared controls;
  specialized reading and chart content keeps only documented exceptions.
- [ ] Menus, tooltips, banners, toasts, dialogs, and empty/error/loading states
  use one surface and typography family.
- [ ] Fonts, UI scale, square corners, pane radius/dividers, widget padding,
  transparency, chart overrides, and saved layouts are preserved.

## Theme review matrix

Every built-in theme, bundled custom preset, and user-custom palette path needs
token-level validation. Visual review starts with these cases because they
exercise different failure modes:

| Case | What it tests |
| --- | --- |
| Built-in Dark and Light | Generated dark/light surfaces and action pairs. |
| Hyperliquid | Hand-authored surfaces; primary and positive share a hue. |
| Papertrade | Warm light surfaces and dark semantic fills. |
| Papertrade UwU | Pastel fills requiring dark foregrounds and dark navy panels. |
| Bloomberg or another strong-accent preset | Saturated accent restraint and market-direction distinction. |
| A muted built-in theme such as Nord | Selection, borders, and text when palette differences are small. |
| An edited bundled custom theme | Generated fallback after source-palette matching no longer applies. |
| A deliberately low-contrast custom palette | Derived foreground correction without rewriting user colors. |

The SVG specimens cover three visual directions but do not replace this matrix.
In particular they do not execute iced, font fallback, pointer states, or account
logic. Never approve the whole system from a single attractive dark screenshot.

## State and layout review

For each shared control, exercise rest, hover, press, focus, selected,
selected-plus-hover, disabled, and busy where applicable. For fields also
exercise invalid, focused-invalid, read-only, and text selection. For data
panes cover initial load, populated, refreshing, empty, filtered empty,
disconnected, stale, partial, and error where the model supports them.

Review these complete compositions with synthetic data:

- [ ] Standard workspace with chart, order book, watchlist, positions, and ticket.
- [ ] Populated position/order rows with long symbols, large values, zero,
  negative, missing, and stale values.
- [ ] Ticket in idle, invalid, unavailable, and pending states; Chase/TWAP state
  transitions; quick-order and wallet-cluster contexts.
- [ ] Narrow feeds at 280/400/640px with the largest magnitude highlight and
  an active selection; wide tables with consistent numeric columns.
- [ ] Settings form, account picker/add-account form, add-widget menu, Alfred,
  journal, assistant, console, and detached chart.
- [ ] Overlay near a window edge, tooltip over a chart, persistent banner, and
  multiple toasts without hiding critical trading controls.
- [ ] 100/125/150% UI scale; default and alternate bundled fonts; square corners,
  custom padding, disabled dividers, and supported transparency settings.

Capture comparisons at the same window/pane size with the same synthetic data.
Use isolated configuration with no wallet or credentials. A successful smoke
launch is not evidence of every state or of visual consistency.

## Verification appropriate to the change

Documentation and SVG-only changes require link validation, SVG parsing/render
inspection, and a diff review. They do not need Rust compilation.

When implementation begins, run `cargo fmt` and `cargo check` for Rust UI
changes. Add focused tests for token contrast, color-pair resolution, neutral
zero/missing logic, geometry helpers, and any changed routing/state/persistence.
Do not create tests that merely reproduce constants without checking behavior.
Theme resolution tests should measure final pairs and cover custom fallback.

Run the repository headless GUI smoke test for shell/window/canvas changes when
feasible. If order construction, account binding, or execution lifecycle changes,
run the relevant focused order/signing tests; styling alone must not alter those
paths. Broader tests and strict Clippy belong to the final implementation pass
according to its actual blast radius, not to this documentation delivery.

## Definition of complete

The visual pass is complete when the same component and state have the same
geometry and meaning everywhere, every feature family above has been reviewed,
all review themes have readable resolved colors, and the narrow-pane and scale
fixtures pass. There are no hover-erased selections, label-size jumps, unrelated
disabled styles, unexplained radius variants, or accidental dark-only surfaces.

Every intentional exception has a reason and scope recorded here or in its
feature guide. Documented exception categories are dense book rows, plot geometry,
market heatmaps, circular identity/slider elements, onboarding display type,
and export artwork. An exception does not create an alternate button family.

Update these guides when a shared token or component contract changes. Update
the SVG references in the same change if appearance changes. Keep the audit as
dated evidence; mark resolved items in implementation tracking rather than
rewriting historical observations as though the inconsistency never existed.
