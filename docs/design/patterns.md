# Screen Patterns

[Design language](README.md) · [Foundations](foundations.md) · [Components](components.md)

These patterns compose the shared components into Kerosene's existing feature
families. They prescribe a visual target while preserving feature behavior,
instance IDs, saved layouts, and user appearance preferences.

## Workspace and pane chrome

The workspace has three visual levels: a quiet application shell, bounded panes,
and the data inside them. Keep global account/market/layout controls in the top
shell; keep pane-specific controls in their own pane. The status bar reports
application-wide connectivity and time without competing with current prices.

A pane follows this order:

```text
24px title bar      Title                         refresh · detach · close
optional toolbar   symbol / mode / filters / contextual actions
body               chart, table, form, or feed
optional footer    local summary / pagination / local status
```

The title bar uses 13px secondary text, 8px horizontal inset, and compact 24px
icon targets. Title-bar height can grow with font metrics. Keep the title in
the same place whether optional actions are visible or unavailable. Reserve
the right edge for actions; do not interleave closing controls with content
filters. Active-pane focus uses a restrained accent boundary or marker, not a
full-color header.

Pane bodies use `surface.panel`; title bars share it or use a closely related
neutral surface. One continuous outline frames the pane when dividers are
enabled. Header/body joins must not double the border. Preserve existing pane
corner and divider preferences. Drop targets use a soft accent region and
clear outline; they must remain visibly different from ordinary selection.

The grid owns outer widget padding. Feature bodies own cell and group spacing.
Charts and data tables can fill the content area without another inset card.
Detached panes retain the same toolbar, data typography, and control geometry;
window chrome supplies only window-level actions.

## Chart and drawing tools

Retain the existing useful hierarchy: symbol and current price, compact market
metrics, toolbar, plot. Use 16px for featured price, 13px for symbol/title, and
11px labels with 12–13px numeric metadata. Secondary metrics may wrap or hide
before the symbol, price, or essential chart controls do.

Use compact ghost buttons for reset, reload, screenshot, and utility actions.
Use selected-toggle treatment for drawing tools and overlays. Use the same
24px height for timeframe pickers and adjacent tools. Separate meaningful
groups by 8px or one subtle rule; avoid a rule after every button.

The drawing toolbar may remain a separate collapsible row. At narrow widths,
wrap complete tool groups or use an explicit overflow surface. Preserve active
tool visibility. A selected drawing tool keeps its indicator while hovered.

The plot uses the current chart color adapter and user overrides. Grid and axes
are quiet; candles, prices, orders, and active drawings dominate. Crosshair
labels use readable paired backgrounds. Buy/sell order overlays carry side and
order-state information as well as color. Pending, open, filled, and failed
orders must not become visually interchangeable.

Keep controls consistent for candlestick, line, comparison, spread, depth,
positioning-flow, session, and analytics charts. Canvas marks can have specialized
geometry; their menus, tooltips, legends, and settings use shared components.
Chart background effects remain opt-in preferences and must not lower control
or axis-label readability.

## Order entry and quick orders

The order ticket has a stable visual sequence:

```text
Symbol / account context
Available balance                              margin used
Market | Limit | Chase | TWAP                   segmented mode
Price                                          unit
[value                                         ] [Mid]
Size                                           denomination
[value                                         ]
25%       50%       75%       Max               compact presets
Algorithm / execution options                  only when applicable
Fees / margin / validation                     concise local context
[BUY SYMBOL                ] [SELL SYMBOL                ]
```

This sequence describes hierarchy; show fields and modes according to current
product behavior. It does not introduce a new order planner or preset policy.

Use 28px fields and segments, 24px presets/options, 36px buy/sell controls, and
8px between related groups. Keep price/size labels, unit labels, and values on
the same grid. Fee estimates are secondary, validation critical, unavailable
values neutral unless they represent a real failure.

Buy/sell actions have equal width, a stable position, and explicit direction.
Keep them reachable when a ticket is short; scroll the form region before
clipping the action row. Preserve existing sticky action behavior where present.
With insufficient horizontal room, stack the actions at full width in their
existing order rather than shrinking text or overlapping targets.

Use the same action family for normal, unavailable, and pending orders. Pending
retains the directional label with a spinner. Do not swap to a differently
sized container or generic unrelated disabled style. Keep account identity and
reason for disabled trading visible where relevant. No new confirmation or
activation behavior is implied by a style change.

Chase/TWAP use the same fields, badges, and controls, with explicit Running,
Paused, Completed, or Failed state where the model supplies it. Progress and
child-order metadata are secondary to the primary status. Stop/cancel actions
are visually distinct from starting an algorithm. Quick-order/HUD surfaces use
the same roles at compact sizes; reduced size must not hide account, side, or
the executable value.

## Order book and trade feed

The book keeps its dense ladder: ask rows, spread/mid separator, then bid rows.
Use a shared column grid for price, size, and total; headers and numbers align
right. Keep the book/DOM/depth mode segmented and tick-size choices compact.
The active-symbol indicator, pinned state, and stale state must have labels or
tooltips with distinct meanings.

Depth bars sit behind values at a bounded opacity; text uses a contrast-checked
foreground over the maximum fill. User orders use an additional marker/outline
so they remain recognizable on either side of the book. The mid/spread row is
visually anchored by a neutral inset band and stronger price, not a large button.

The dense 20px row exception is appropriate here. Inline actions must remain
usable without overlapping neighboring levels. Never move the hit target for a
price level as a consequence of hover styling. Existing execution gestures
remain unchanged.

## Account, wallet, and position tables

Use one table family for positions, open orders, balances, trade/funding history,
transfers, wallet detail, and wallet clusters. Keep their domain-specific
columns, but share header height, numeric alignment, row selection, badges,
loading states, and action geometry.

Positions/Open Orders/Balances/History are content tabs with an underline.
Counts use quiet badges. `Journal` is a ghost action with the shared external
window icon, not another active content tab.

Use neutral text for size, entry, mark, and notional; semantic text for signed
PnL/change and explicitly labeled side. Zero is neutral across all tables.
Liquidation/margin warnings require meaningful thresholds from existing domain
logic. Color alone does not establish that a position is safe or unsafe.

Close, cancel, edit, and details actions occupy stable end columns. Avoid a
different tinted mini-button for every action. Multi-account and cluster
contexts keep the account label visible near the actionable row or ticket.
Respect privacy toggles and address truncation consistently in all windows.

## Search, watchlists, and market summaries

Symbol search, live watchlists, screeners, and market selectors share the search
field, filter row, sortable header, and selectable-row styles. Favorite stars
have fixed icon targets. Show the symbol first, venue/category as secondary
metadata, and right-aligned change/value columns where applicable.

Selected rows use a marker plus soft fill. A category such as Crypto or Stocks
is neutral metadata, not an action or a warning. Use the accent only for an
interactive or selected state. Keep tickers distinct when different venues
share a display symbol.

Positioning, session data, portfolio, income, HYPE ETFs, and unstaking summaries
use the metric-pair pattern. Group related metrics with spacing/rules, avoiding
an independent rounded card per cell. Calendars and outcomes may retain their
event/contract grouping, but filters, time labels, status badges, and action
buttons conform to the same component recipes.

## Liquidations and tracked trades

Use a compact control row, aligned table header, continuous data rows, and
optional summary/footer. Preserve existing responsive column priorities;
symbol, side/event meaning, and notional should remain identifiable at narrow
widths. Keep aggregation counts visible with the event even if an optional
method column disappears.

Market-direction colors describe execution side. A long liquidation is normally
a sell-side event; a short liquidation is a buy-side event. Explicit labels are
necessary because a positive-colored buy is not necessarily good news for the
liquidated account. Magnitude may strengthen a background tint, but never
overpower the row's text or selected marker. Test the current largest tint,
not only quiet small events.

A healthy empty feed, a paused view, a disconnected feed, and a stale snapshot
have separate labels. Retain actual existing follow/scroll behavior. A new
follow button, new-items indicator, or sorting control is behavior work and
must not appear in a mockup as though already supported.

## Telegram, X, listings, and updates

Reading feeds can use 13px body type and variable-height entries, with 11px
source/time metadata. Keep message hierarchy and media aspect ratio. Grouping
cards may use 6px corners, but controls inside them keep 4px control corners.
Channel/source identities may retain their marks; connection controls, search,
filter chips, menus, and link actions use the shared language.

External content colors and images must not become the source for app control
colors. Bound selection highlights behind readable text. Keep loading,
authentication-required, error, and no-matching-content states explicit.

## Settings and account setup

Settings use a quiet sidebar and a single scrolling content region. Use 18px
window headings, 13px section labels/body, 11px descriptions, and 28px controls.
Align labels and values in a responsive form grid. At narrow widths, move the
control below its label before truncating important configuration text.

Theme options retain their useful swatches and selected checkmark. The option's
chrome belongs to the active application theme; only the swatches preview the
candidate theme. Selection must be visible for a candidate whose accent is
similar to the current surface. Font, scale, corner, padding, and transparency
settings remain independent.

Use one preference-row pattern for theme, risk, network, integrations, storage,
and notifications. Put help text only where the decision needs explanation.
Provider errors and save status sit near the relevant section. Account-add,
credential, approval, and account-switcher windows reuse the same fields and
feedback patterns while retaining their existing security and privacy flows.

## Alfred, journal, assistant, and console

Alfred uses a focused standard search field over two-line result rows. Keep
the keyboard-selected result visibly different from hover, and retain explicit
disabled reasons. Category tags use shared badges. Preserve the existing popup
scale preference and bounded results; do not multiply popup scale twice.

Journal retains its useful master/detail structure, compact metric groups,
and restrained rules. Replace local button/surface variants with shared roles.
Reflection input is a multi-line field; Save is primary; filters are segments;
trade rows follow the common table pattern. Do not derive light-theme wells by
unconditionally darkening the workspace or selected text by whitening accent.

Assistant remains a reading surface with 13px body text, a responsive session
sidebar, and a multi-line composer. Bubbles, tool trace groups, and code blocks
may use 6px container corners. Send/stop, model menus, follow-up actions, copy,
and session controls use the same buttons and selectors as the rest of the app.
Keep streaming, tool progress, and errors explicit without decorative motion
that competes with reading. Code remains monospace.

Console uses the standard table, filters, status badge, and pagination patterns.
Its diagnostic purpose does not justify another set of default-styled buttons.
Keep the existing sanitized metadata and privacy boundaries.

## Onboarding and exports

Onboarding may retain a display-size wordmark and a bounded decorative backdrop.
Its Start action still follows the prominent primary-button recipe. It is the
exception to dense trading composition, not a second application theme.

Screenshot and PnL-card controls share the standard form language. Exported
artwork can have presentation-specific typography and layout, but must use
readable pairs, preserve privacy options, and exclude unintended account data.
Synthetic examples should never resemble an actual connected user's account.

## Narrow panes and resizing

Use available pane width, not whole-window width, to choose layout. Priority is:
preserve actionable values and state, collapse optional metadata, wrap complete
groups, then use supported scrolling/overflow. Never shrink all text, clip
trade actions, or hide errors to force a layout to fit.

Review representative panes at 280px, 400px, and 640px width. These are review
fixtures, not universal new breakpoints. At insufficient width for safe trading,
show a clear bounded layout or minimum-width requirement rather than overlapping
controls. Scrollbars and overlays remain inside their owning pane/window.
