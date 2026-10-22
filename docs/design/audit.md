# Current Application Design Audit

[Design language](README.md) · [Implementation checklist](implementation.md)

The audit uses source revision `8963856a` and repository images reviewed on
2026-10-22. The source package is 0.2.2, newer than the overview version recorded
in the repository's AGENTS.md. Findings below describe the inspected source or
identified captures; the target specifications describe proposed convergence.

## Evidence and coverage

Inspected code includes shared UI helpers, palette construction and preset
adapters, pane grid styles/actions, chart toolbars, order actions, account tabs
and tables, wallet and journal styles, settings navigation/theme choices,
watchlist rows, liquidation styles, Telegram styles, assistant styles/composer,
Alfred rows, onboarding, toasts, and console views.

Visual review used the recent [Papertrade application preview](../../assets/screenshots/papertrade/kerosene-preview.png)
and [Papertrade UwU application preview](../../assets/screenshots/papertrade/kerosene-uwu-preview.png).
Their capture context is recorded in the [theme guide](../components/settings-preferences-and-theme.md)
as isolated, wallet-free previews from the October 2026 theme work. Older
[dark terminal](../../assets/screenshots/themes/dark.png) and
[add-widget menu](<../../assets/screenshots/add widget.png>) captures provide
historical visual context. The filename `dark.png` is not proof that its active
palette is the built-in Dark theme.

No fresh current-revision desktop capture or interactive sweep was performed.
The available release binary predates the newer source/theme work, so it was
not used as evidence of current rendering. Stateful behavior, actual text
contrast across every theme, and narrow-pane clipping still require the native
review matrix in the implementation guide. The new SVG sheets are target
specimens, not evidence that the application passes that matrix.

## Existing strengths to preserve

| Existing pattern | Why it works | Carry into the shared language |
| --- | --- | --- |
| Compact pane grid and restrained chart tools | Large amounts of market information fit in one workspace. | Keep density, but regularize control heights and hit areas. |
| Theme and chart adapters | Palettes and market colors already have shared entry points. | Add semantic roles above them instead of duplicating a theme engine. |
| Shared trade buttons and text inputs | Some cross-feature reuse already exists. | Complete their state contract and adopt them consistently. |
| Theme option swatches and checkmarks | Selection is recognizable without depending only on a candidate color. | Standard selected-row behavior. |
| Journal rules and grouped metrics | Restraint creates hierarchy without filling every area with cards. | Reuse the composition while replacing dark-biased color derivation. |
| Explicit pending order presentation | In-flight actions are distinguishable from idle actions. | Keep the same geometry and action context across states. |
| Alfred disabled explanations | The user can understand why a command is unavailable. | Reuse that clarity for genuinely ambiguous disabled controls. |
| User corner, padding, scale, font, and chart settings | Appearance already adapts to different workflows. | Preserve them during consolidation. |

## Concrete inconsistencies

| ID | Evidence | Visible consequence | Target |
| --- | --- | --- | --- |
| D01 | [Shared order-type buttons](../../src/helpers/ui/buttons.rs) use 13px active text and 12px inactive text. | Selection changes type size and can disturb alignment. | Fixed 12px labels; selected fill plus outline. |
| D02 | [Account action buttons](../../src/account_views/style.rs) use 10px text, `[1, 6]` padding, 3px radius, and ignore widget status. | Tiny bespoke controls lack consistent pointer feedback. | Compact 24px shared action variant with complete states. |
| D03 | [Chart toolbar](../../src/chart_views/toolbar/sections.rs), [book mode controls](../../src/market_views/order_book/controls/chrome.rs), and [feed controls](../../src/feed_views/controls.rs) each define local sizes/styles. | Similar choices and actions have different density and selection grammar. | One compact size; segments for modes, ghost controls for actions. |
| D04 | [Account tabs](../../src/account_views/tabs.rs) use square buttons and separators; [settings navigation](../../src/settings_views/sidebar.rs) uses filled rounded rows; order modes use outlined buttons. | Navigation, mode choice, and ordinary actions are not governed by an explicit family. | Underlined content tabs, marked sidebar rows, outlined selected segments. |
| D05 | [Order actions](../../src/order_views/actions.rs) switch between shared trade buttons, local pending containers, and `button::secondary` for unavailable states. | State changes can change visual family and layout metrics. | One trade action component with idle, busy, and disabled presentation. |
| D06 | [Shared trade button](../../src/helpers/ui/buttons.rs) handles hover with alpha 0.9; [account action](../../src/account_views/style.rs) ignores status; [journal segments](../../src/journal_views/style.rs) group hover and press. | Pointer feedback is inconsistent; pressed states are often indistinct. | Explicit rest, hover, press, focus, disabled, selected, and busy recipes. |
| D07 | [Shared text-on-background helper](../../src/helpers/ui/colors.rs) selects light/dark text at luminance 0.4; [journal primary](../../src/journal_views/style.rs) uses workspace color as text. | Neither strategy guarantees readable text for every custom fill. | Paired foregrounds, checked against final composited backgrounds. |
| D08 | [Shared input](../../src/helpers/ui/inputs.rs) uses alpha text-derived surfaces and extended-palette fields; Telegram/assistant have local input treatments. | Form surfaces and focus treatment vary by feature and theme. | Shared field frame and states, with slots and multi-line variants. |
| D09 | [Journal styles](../../src/journal_views/style.rs) darken workspace by 0.35 for a sunken well and lighten accent by 0.34 for selected text. | Derivation assumes a dark visual direction and can produce inappropriate light-theme contrast. | Resolve inset and accent-foreground roles centrally. |
| D10 | [Pane title](../../src/main_view/grid/styles.rs) applies alpha 0.46; [journal dim text](../../src/journal_views/style.rs) applies another alpha to muted text. | Useful labels can become too faint depending on their final background. | Opaque, contrast-checked secondary/muted text roles. |
| D11 | [Shared signed values](../../src/helpers/ui/colors.rs) render zero as primary text; [wallet values](../../src/wallet_views/style.rs) use secondary text; [history values](../../src/account_views/history_tables/style.rs) treat zero as success. | The same numeric result has different semantic emphasis. | Neutral zero; separate missing value from error state. |
| D12 | [Assistant styles](../../src/agent_views/styles.rs) use 5/6/7/8/12/14/99px radii; [journal](../../src/journal_views/style.rs) separates 4px ordinary and 5px primary buttons. | Controls appear to belong to different products. | 4px controls, 6px containers, 3px badges; explicit geometric exceptions. |
| D13 | [Pane actions](../../src/main_view/grid/components.rs) and [settings sidebar](../../src/settings_views/sidebar.rs) use text glyphs such as `x`, refresh arrows, `[]`, and `<>`. | Shape and optical weight vary with the selected font. | Shared monochrome vector action icons and stable icon targets. |
| D14 | [Pane styles](../../src/main_view/grid/styles.rs) centralize borders, but [wallet tables](../../src/wallet_views/style.rs) add separate translucent boxed surfaces. | Nested outlines and extra insets can fragment one workspace. | One pane frame; tables fill it; nested cards only for meaningful groups. |
| D15 | [Liquidation rows](../../src/feed_views/liquidations/rows/row/style.rs) range from 0.02 to 0.6 direction-color opacity. | High-magnitude states need their own contrast/selection review. | Bounded visualization fills with readable text at the strongest state. |
| D16 | [Theme adapters](../../src/app_theme.rs), [Papertrade](../../src/app_theme/papertrade.rs), and [Hyperliquid](../../src/app_theme/hyperliquid.rs) assign different meanings to extended background shades. | Reading `strong` as a universal elevation level is unreliable. | Named semantic surfaces resolved per palette path. |
| D17 | [Theme choice buttons](../../src/settings_views/themes/button.rs) reserve a border only when active; similar selected-row implementations recur. | Selected geometry/border logic is duplicated and can diverge. | Reserve border space and share selected-row styling. |
| D18 | [Console](../../src/console_views.rs) uses default button/picker styling while custom families exist elsewhere. | New or secondary windows can bypass the visual system. | Require shared components across auxiliary windows too. |

These findings are inspection-based risks and concrete implementation
differences. No measured contrast failure is claimed without a rendered color
pair measurement. The proposed visual system addresses both actual divergence
and the missing shared contracts that allow it to recur.

## Feature coverage map

Every current pane family and major auxiliary surface has a target pattern.
This inventory is broader than the individually inspected controls above.

| Surface family | Main implementation entry points | Shared target |
| --- | --- | --- |
| Main shell, pane headers, add-widget menus | `main_view/`, `account_views/summary/`, `window_chrome.rs`, `status_bar/` | Pane frame, ghost icons, menu rows, status badges. |
| Chart, comparison, spread and depth plots | `chart_views/`, `spaghetti_views/`, `market_views/order_book/` | Compact toolbar, plot roles, overlay labels. |
| Order entry, quick orders, Chase, TWAP, advanced orders | `order_views/`, `chart_views/quick_trade.rs` | Fields, modes, directional actions, lifecycle badges. |
| Order book, trade feed | `market_views/order_book/`, `trade_feed_views.rs` | Dense numeric rows, mode segments, market fills. |
| Symbol search, live watchlists, screener | `market_views/watchlist/`, `market_views/live_watchlist.rs`, `screener_views.rs` | Search/filter bar, selectable numeric rows. |
| Positions/history, account management | `account_views/` | Content tabs, financial tables, identity rows, account forms. |
| Wallet tracker, compact wallets, wallet details/clusters | `wallet_views/`, `wallet_cluster_views/` | Identity, row actions, tables, ticket context. |
| Portfolio, income, journal | `account_views/portfolio*`, `account_views/income/`, `journal_views/` | Metric groups, shared charts/tables, reading inputs. |
| Positioning and session data | `market_views/positioning_info/`, `market_views/session_data/` | Metrics, column grids, compact controls. |
| Liquidations/distribution and tracked trades | `feed_views/liquidations/`, `feed_views/tracked_trades/`, `market_views/liquidations_distribution/` | Feed table, magnitude/side encoding, summaries. |
| Telegram and X | `feed_views/telegram/`, `feed_views/x.rs` | Reading feed, shared filters, connection notices. |
| Calendar, outcomes, listings, updates, HYPE ETFs/unstaking | `calendar_views.rs`, `market_views/outcomes/`, `market_views/listings.rs`, `market_views/updates.rs`, `market_views/hype_*` | Grouped events/contracts, neutral metadata, shared actions. |
| Settings, including legacy settings pane | `settings_views/` | Preference rows, sidebar, swatch choices, fields. |
| Alfred | `alfred_views.rs`, `alfred_views/rows.rs` | Focused search and selectable command rows. |
| Assistant and console | `agent_views/`, `console_views.rs` | Shared controls around specialized reading/diagnostic bodies. |
| Onboarding, notifications, exports | `main_view/onboarding.rs`, `toast_overlay.rs`, `pnl_card/`, `chart_screenshot*` | Deliberate display exceptions, consistent controls, privacy. |

## What this pass should avoid

Do not change all themes to the same hue, reset saved appearance preferences,
inflate every table into a card list, or turn trade actions into generic accent
buttons. Do not mechanically replace all alpha values in chart visualizations.
Do not carry old screenshots' layout or earlier feature-audit suggestions into
the specification without checking the current source and chosen compact
direction. Consolidate the shared language while retaining domain information.
