# Foundations and Themes

[Design language](README.md) · [Components](components.md) · [Screen patterns](patterns.md)

## Theme invariants

A theme changes resolved colors, never control sizes, spacing, type hierarchy,
icon shapes, action order, or state semantics. Changing from Papertrade to
Hyperliquid or a user-defined palette must leave the same controls in the same
places. Font selection and UI scale remain separate preferences.

The current system exposes a six-color base palette and an extended palette in
[app_theme.rs](../../src/app_theme.rs). Some presets supply hand-authored
extended pairs; other themes use generated shades. The words `weak` and `strong`
do not guarantee a consistent elevation or luminance order across these paths.
For example, Papertrade deliberately maps `background.strong` to cream, while
Hyperliquid maps it to a raised dark gray.

Introduce a shared semantic resolution layer over those existing palettes.
The following token names describe the target contract, not existing Rust APIs.
Resolve once per styling context and pass the roles to shared component styles.

## Surface and text roles

| Role | Use | Existing starting point and resolution rule |
| --- | --- | --- |
| `surface.workspace` | Space behind panes and auxiliary window roots. | `palette.background`. |
| `surface.panel` | Pane bodies, data tables, chart chrome. | `extended.background.strong.color` preserves today's pane mapping; explicit preset mapping may override. |
| `surface.inset` | Fields, recessed wells, inactive segment tracks. | Start at `background.weak.color`; choose a restrained distinct surface for the theme. |
| `surface.raised` | Neutral buttons, hover rows, small floating sections. | Start at `background.neutral` or `stronger`; validate distinction from the parent rather than blindly taking the next shade. |
| `surface.overlay` | Menus, popovers, tooltips, dialogs. | Opaque resolved panel/raised pair, with an outline. Never depend on the underlying chart for readability. |
| `text.primary` | Values, actionable labels, active content. | `palette.text`, contrast-checked against its actual surface. |
| `text.secondary` | Field labels, headers, useful supporting data. | Start at `background.weak.text`; resolve an opaque readable color. |
| `text.muted` | Timestamps, tertiary descriptors, optional metadata. | A separate readable neutral role. Do not repeatedly reduce `text.secondary` alpha in views. |
| `text.disabled` | Unavailable controls. | Neutral muted role paired with a disabled surface; retain readable labels. |
| `border.subtle` | Pane outlines and decorative dividers. | Start from an 8–16% blend of primary text over the surface, adjusted per theme. |
| `border.control` | Field edges and boundaries needed to identify controls. | Stronger than decorative rules; contrast-checked on the actual adjacent surfaces. |
| `border.focus` | Keyboard focus. | Contrast-checked accent, or a two-color indicator if one color cannot work. |

Blend to an opaque final color for ordinary UI surfaces and labels. Alpha is an
input to token resolution, not an invitation for each view to invent a new
opacity. Reserve runtime translucency for deliberate overlays, chart fills,
heatmaps, and existing window-transparency behavior.

## Semantic colors

Each role below supplies `fill`, `on_fill`, `soft_fill`, `on_soft`, and
`foreground` pairs, plus hover and pressed variants for actions. A foreground
used as small text can differ in lightness from the corresponding filled button.

| Role | Meaning | Starting point |
| --- | --- | --- |
| `accent` | Primary non-trading action, active navigation, focus, selection. | `extended.primary` pairs. |
| `positive` | Positive PnL/change, buy actions, successful results. | `extended.success` pairs. |
| `negative` | Negative PnL/change, sell actions. | `extended.danger` pairs. |
| `warning` | Stale, delayed, attention needed, or a recoverable limitation. | `extended.warning` pairs. |
| `critical` | Invalid fields, failed requests, destructive actions. | Danger pairs initially; distinct role even when it shares the negative hue. |
| `info` | Neutral progress, explanations, provider information. | Neutral surface with text and an informational icon; accent only when interactive. |

Buy and sell are directional actions. Neither is a generic success/error
message. Use explicit `BUY`, `SELL`, `Long`, `Short`, plus/minus signs, or state
labels so shared hues do not carry the entire meaning. A sell button and a
delete button can share a base hue but differ in label and visual prominence.
Destructive actions are outlined or quiet until their existing confirmation
surface; ticket buy/sell actions are filled and equally prominent.

Primary and success are identical in some themes. Preserve that choice: shape,
placement, and wording distinguish an active tab from a profitable position.
Do not change financial meaning to manufacture more colors.

### Market visualization roles

Keep the existing [chart color adapter](../../src/app_theme/chart_colors.rs)
and its overrides. Define `market.up`, `market.down`, line, area, grid,
crosshair, and series roles separately from generic action colors. An existing
user candle override may legitimately differ from the button palette; buy/sell
labels must still identify direction. Depth and trade/liquidation feed direction
should follow the same market direction adapter where they already do so.

Data series may use multiple hues. Distinguish them by labels and, where needed,
line style or markers. Keep series colors stable across refreshes and pane
resizes. Threshold highlights must not obscure text or hide a selected row.

### Resolution order

1. Resolve the existing built-in/custom palette and any exact preset adapter.
2. Resolve semantic surfaces and color pairs centrally.
3. Composite soft fills over the intended surface, then measure contrast.
4. Adjust derived foreground lightness as needed; prefer preserving the chosen
   hue. If no usable hue-preserving color exists, use a readable neutral with
   a semantic icon/label.
5. Apply user chart overrides to plot roles, and existing opacity preferences
   to permitted surfaces. Recheck text on the final composited background.

Never silently rewrite stored custom-theme colors. Runtime correction belongs
in derived roles. Theme editing must preserve existing preset matching and
fallback behavior.

## Typography

Use the configured display font for interface labels and reading text, and
[monospace_font](../../src/app_fonts.rs) for aligned numeric data. The current
font selector permits proportional families in the numeric slot, so a helper
name does not guarantee tabular digits. Align numeric cells by shared column
geometry regardless of font. Verify tabular figures when the chosen renderer
and font support them; do not assume that feature is already enabled.

| Role | Size / line box | Weight | Application |
| --- | --- | --- | --- |
| `type.micro` | 10 / 14 | Regular | Optional compact chart annotation or badge only. |
| `type.meta` | 11 / 16 | Regular | Table headers, units, timestamps, secondary labels. |
| `type.control` | 12 / 16 | Regular; medium when supported | Buttons, selectors, tabs, table data. |
| `type.body` | 13 / 20 | Regular | Descriptions, forms, assistant and journal prose. |
| `type.pane_title` | 13 / 16 | Medium | Pane and section titles. |
| `type.featured` | 16 / 20 | Medium | Current price and compact summary values. |
| `type.window_title` | 18 / 24 | Medium | Settings and auxiliary-window page headings. |
| `type.summary` | 24 / 32 | Medium | A small number of portfolio/journal headline metrics. |

The onboarding wordmark and export artwork are explicit display-type exceptions.
Do not use 9–10px controls simply to fit more actions. Hide optional metadata or
overflow a toolbar before reducing primary controls below the control scale.
Badges containing essential status use 11px, not the optional micro size.

Selection changes color and indicators, not font size. Avoid changing weight
if the chosen font causes width movement. Use sentence case for ordinary labels;
retain uppercase ticker symbols and the established `BUY`/`SELL` ticket labels.

## Spacing and dimensions

| Token or pattern | Logical pixels | Use |
| --- | --- | --- |
| Micro gap | 2 | Indicator/text and tightly related sub-lines. |
| Base gap | 4 | Neighboring toolbar controls and dense row spacing. |
| Control content gap | 4 compact, 6 standard | Icon to label; spinner to label. |
| Group gap | 8 | Buttons, field label groups, paired ticket actions. |
| Section gap | 12 or 16 | Form groups, settings sections, reading surfaces. |
| Dialog inset | 16 | Modal and auxiliary form content. |
| Panel content inset | 8 | Default interior for forms and summaries. |
| Data cell inset | 8 horizontally | Header and row alignment. |
| Compact control | 24 minimum high; 8 horizontal inset | Toolbars, inline actions, filters. |
| Standard control | 28 minimum high; 10 horizontal inset | Inputs and routine form actions. |
| Prominent control | 36 minimum high; 12 horizontal inset | Ticket actions and dialog commitment. |
| Icon control | 24 × 24, or 28 × 28 in forms | 14–16px artwork centered in the target. |
| Data row | 24 minimum high | Watchlists, positions, fills, orders. |
| Dense book row | 20 minimum high | Order book and passive tick stream exception. |
| Two-line list row | 40 minimum high | Account switcher, Alfred, multi-line identities. |

Heights are minimum outer dimensions including borders. Grow them when font
metrics or accessibility scaling require it. For a 16px line box, reserve 24,
28, or 36px outer height and center the content; do not add enough vertical
padding to accidentally exceed that size. All controls in one row share a
height. Keep dimensions unchanged across hover, selection, and loading.

Fixed dimensions are appropriate for compact controls and table column
contracts. Pane bodies and forms use fill/proportional sizing. Respect the
existing global and per-pane widget padding: the grid applies that outer inset
once, while a feature applies only its own internal cell/group spacing. Avoid
adding two layers of generic pane padding.

The 20px book-row exception is for dense market scanning. Keep its inline
controls explicit and separated; unrelated toolbar icons still receive 24px
targets. Any expansion of hit areas must avoid overlap with adjacent rows.

## Corners, borders, and depth

Use 3px badge corners, 4px control corners, and 6px panel/overlay corners. A
standard button does not gain a larger radius because it is primary. Tables
and internal separators are square. Circular avatars, status dots, and slider
thumbs are geometric exceptions; ordinary filter/status chips are rectangles.

Preserve the configured pane radius and divider settings. Apply the existing
[effective_radius](../../src/config/schema/defaults.rs) behavior consistently
to controls and containers affected by square-corner mode. Semantic circles
remain circles. This is not a proposal to reset saved preferences.

Use 1px decorative dividers and control outlines. Reserve border space even
when the border is transparent. Draw a 2px focus ring inside the reserved
control frame or in a reserved outer region so focus never moves content.

Ordinary panes, buttons, tables, and cards have no shadows. Menus, dialogs,
Alfred, and toasts may share one subtle overlay shadow: 0px horizontal offset,
4px vertical offset, 16px blur, with theme-resolved opacity. An outline must
still identify the overlay when the shadow is invisible on a dark background.

## Icons and motion

Use a consistent monochrome vector family for app actions, with 14px compact
and 16px standard artwork and a consistent optical stroke. Reuse existing
vector assets where possible. Replace font-dependent `x`, arrows, gears, and
textual icon substitutes gradually through a shared icon helper. Ticker logos
retain their identity; external images do not define control geometry.

Icon-only controls need a concise tooltip and an accessible name where the
widget supports it. A tooltip is not a substitute for a keyboard-reachable
control. Communicate external links with the same external-link icon everywhere.

Pointer feedback is immediate. Optional hover transitions are at most 100ms;
small popovers/toasts may settle within 120–160ms. Numeric cells, ticket
geometry, and table rows never animate position on price updates. Reserve
spinner space during requests. Disable decorative motion when reduced-motion
support is available; adding that preference is separate behavior work.

## Readability criteria

Adopt 4.5:1 as the project target for all informative text, including muted
metadata. The W3C text criterion uses 4.5:1 for ordinary text and allows 3:1 for
qualifying large text; Kerosene's compact scale should use the stricter target
throughout. This is a design acceptance target, not a claim of current app-wide
conformance. [W3C text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)

Aim for 3:1 against adjacent colors for indicators needed to identify controls
and states, such as field boundaries, checkmarks, and focus rings. Decorative
pane separators can be quieter when they carry no essential information.
Check composited colors, including selected rows and hover fills.
[W3C non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html)

Keyboard focus must remain visible for as long as the element has focus. Do
not substitute a brief flash or a hover treatment. The 2px ring is Kerosene's
chosen treatment, not a size requirement attributed to this criterion.
[W3C focus visibility](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html)

Contrast thresholds are not font-size recommendations. The type and hit-area
sizes above are project decisions for a desktop terminal. Test at 100%, 125%,
and 150% UI scale with both the default font and an alternate bundled font.
