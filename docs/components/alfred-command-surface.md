# Alfred Command Surface

Alfred is Kerosene's command palette. It opens panes/windows and drafts or
submits trading actions from short typed commands.

For user-facing command examples and safety notes, see [Alfred](../alfred.md).
This document describes implementation boundaries.

## Component Map

| Component | Key files | Responsibility |
| --- | --- | --- |
| State | `src/alfred_state.rs`, `src/alfred_state/` | Query, selection, catalog, parsed trading intents, close-position helpers. |
| Update | `src/alfred_update.rs` | Open/close, query changes, selection movement, submit, command dispatch. |
| Views | `src/alfred_views.rs`, `src/alfred_views/` | Overlay, result rows, disabled-state explanations. |
| Routing | `src/message.rs`, `src/app_update/routing.rs` | Alfred messages and route ownership. |
| Execution targets | `pane_update.rs`, `order_update.rs`, `order_execution/`, `risk_state/` | Pane/window commands and trading actions reuse normal app paths. |

## Runtime Behavior

Alfred is an overlay rendered by `main_view.rs` when the Alfred state is open.
The overlay is above the pane grid and below the highest priority modal states
such as credential unlock.

Input flow:

```text
ToggleAlfred
  -> open overlay and focus query
AlfredQueryChanged
  -> rebuild result rows from command catalog or trading parser
AlfredSelectionMoved
  -> update highlighted row
AlfredSubmit / AlfredCommandSelected
  -> dispatch selected command
```

Escape closes Alfred without action.

## Command Catalog

The catalog includes non-trading commands such as:

- add panes
- open windows
- navigate app surfaces

Command rows should emit existing messages where possible. For example, adding
a pane should go through the same pane update path as the add-widget menu.

## Trading Parser

Trading-style queries can parse into a single preview row. Examples include:

- market buy/sell
- limit buy/sell
- Chase order draft/start
- close-position
- NUKE

Alfred does not bypass order validation. It should reuse:

- active symbol resolution
- market-type checks
- hidden-symbol/risk filters
- stale-account checks
- order sizing
- close/NUKE planning
- standard order submission path

If a trading query is incomplete or unsafe, the row should be disabled and
explain the missing condition.

## Preview And Safety

Alfred is intentionally preview-first. Trading rows should make the action
clear before submission and should be disabled for:

- missing side
- unknown or hidden symbol
- missing account data
- missing agent key
- stale account data
- unavailable mid/price reference
- unsupported outcome/spot/perp behavior
- invalid size or percentage
- no matching open position

Market-order actions can execute immediately after submit, so preview text and
disabled states are part of the safety boundary.

## Hotkey Integration

The Alfred hotkey is configured through Settings > Hotkeys. Keyboard events are
handled through hotkey subscriptions and preferences update logic, then emit
the same Alfred messages as any other input path.

## Tests To Check

Use focused tests in:

- `src/alfred_state/**/tests`
- `src/alfred_views/rows/tests.rs`
- `src/alfred_update.rs` tests where present
- order execution tests for trading command behavior
- risk-state tests for hidden symbol behavior
- routing tests when adding Alfred messages

For new trading commands, add parser and disabled-state tests plus an execution
path test when practical.
