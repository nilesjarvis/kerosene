# Component Specifications

[Design language](README.md) · [Foundations](foundations.md) · [Screen patterns](patterns.md)

Use these recipes for new UI and when consolidating existing styles. Token
names refer to the proposed semantic roles in the foundations guide. The sizes
are logical pixels at 100% UI scale and grow with font metrics when needed.

## Buttons

Choose a semantic role and a size independently. Do not create a special button
family for each feature. A toolbar refresh action, journal export action, and
settings reset action should draw from the same roles.

| Role | Resting appearance | Use |
| --- | --- | --- |
| Primary | Accent fill and its paired `on_fill` text; 4px radius. | One leading non-trading action per local decision group, such as Save or Connect. |
| Secondary | Raised neutral surface, primary text, 1px subtle outline. | Ordinary actions such as Edit, Export, Reset, or Cancel. |
| Ghost | Transparent surface, secondary text, reserved transparent outline. | Low-emphasis toolbar, navigation, and row utilities. |
| Destructive | Critical foreground with soft fill or a control outline. | Remove account, delete saved layout, cancel all, or other consequential removal. Use a filled critical button in an existing confirmation surface. |
| Buy | Positive fill and paired text. | A buy submission or explicit long-side trading action. |
| Sell | Negative fill and paired text. | A sell submission or explicit short-side trading action. |

Buy and sell have identical dimensions and label treatment. Keep both labels
explicit even when their colors are familiar. Closing a pane is a neutral ghost
action; removing saved content is destructive. A neutral Cancel button dismisses
a dialog; Cancel order is an explicit trading action whose existing semantics
and confirmation behavior remain intact.

Compact buttons are 24px high with 12px text and 8px horizontal inset. Standard
buttons are 28px high with 12px text and 10px inset. Prominent ticket buttons
are 36px high with 13px text and 12px inset. Icon-only controls use a matching
square target. Reserve icon/spinner space when a state change would otherwise
shift or resize the label.

### Button states

| State | Required treatment |
| --- | --- |
| Rest | Role recipe above. |
| Hover | Theme-resolved hover fill; ghost acquires a neutral raised fill. Keep label, border width, and geometry stable. |
| Pressed | A distinct pressed fill or inset indicator. Do not shrink or translate the button. |
| Keyboard focus | Persistent 2px focus indicator, combined with the current role/state. |
| Selected | Soft accent fill plus a persistent outline/marker; used only for controls that represent a choice or toggle. |
| Selected and hovered | Retain the selected indicator while changing the surface. Hover never erases selection. |
| Disabled | Neutral disabled fill/text, no hover/pressed affordance, no activation. Explain an otherwise ambiguous restriction nearby or in a discoverable tooltip. |
| Busy | Stable-size control, spinner plus retained action context, activation unavailable. Keep the directional identity of a pending trade. |

Resolve availability first: busy or disabled prevents activation. Resolve
selection next, then pointer state, then add focus without erasing a selected
marker or validation state. Busy is distinct from disabled: the former says an
action is in progress, the latter says it cannot currently be performed.

For small foreground labels, choose the resolved semantic foreground rather
than the raw fill color. For filled buttons, always use the foreground paired
with the final background. Never assume white, the workspace color, or a fixed
luminance threshold produces readable button text.

## Text and numeric inputs

A standard input has a 28px minimum height, inset surface, 1px control border,
4px radius, 12px value, and 8px horizontal text inset. Its visible label is 11px
secondary text, 4px above the field. A placeholder is an example or hint; it
does not replace the label. Numeric entry uses the configured numeric font.

| State | Appearance |
| --- | --- |
| Rest | Inset surface, primary value, secondary placeholder. |
| Hover | Stronger control border; no change in dimensions. |
| Focus | 2px focus indicator, visible caret and selection, same surface. |
| Invalid | Critical border and a concise 11px error directly below. Retain the entered value. |
| Invalid and focused | Keep the critical indicator/error and add a distinguishable focus ring. |
| Disabled | Neutral disabled surface; no caret or edit affordance. |
| Read only | Readable normal text and subdued boundary; still selectable/copyable where supported. |
| Pending validation | Small trailing progress indicator; preserve value and unit position. |

Use leading/trailing slots for units, search icons, reveal controls, and clear
actions. A unit is secondary text, not a detached button unless changing it is
an action. `Mid`, `Max`, denomination switches, and other appended actions share
the field height and align with its boundary. Reserve their widths so changing
the value does not move them.

Mask secret inputs by default, with a consistent reveal control if supported.
Use synthetic values in documentation and captures. Retain existing redacted
message types and sensitive-input behavior when replacing the visual shell.

Search fields use the same component with a search icon and optional clear
button. Multi-line notes and assistant composers use body type, 8–12px inner
padding, and the same border/focus rules; their height grows for content.

## Pickers, segments, and binary controls

| Component | Geometry and treatment | Interaction contract |
| --- | --- | --- |
| Pick list | Same height and border as a text input; trailing chevron in a reserved slot. Compact variant is 24px. | Opened state retains focus; selected option has a checkmark. Menu matches the shared overlay recipe. |
| Segmented choice | Inset track; 24px compact or 28px standard segments; 4px group radius. Selected segment uses soft accent fill, accent foreground, and visible outline. | For mutually exclusive choices such as Market/Limit/Chase/TWAP or Book/DOM/Depth. Equal widths for a short balanced set. |
| Filter toggle | Compact secondary/ghost control; selected soft fill plus checkmark or outline. | For independent filters. Does not look like navigation to another page. |
| Checkbox | 14px visible box inside a minimum 24px target; 3px corners; 12px label. Checked accent fill with paired checkmark; mixed state uses a dash. | Both label and box toggle it. Used for options and multi-selection. |
| Switch | 28 × 16px track inside a minimum 24px target; visible thumb position plus label. | For an immediate on/off preference. Avoid coloring every enabled preference as market-positive. |
| Radio | 14px circle in a minimum 24px target; inner dot identifies selection. | For mutually exclusive form choices that need explanatory text. |
| Slider | 4px rail, 12px thumb, minimum 24px interaction height; label and numeric readout. | Current value remains readable and keyboard-adjustable where supported; focus is visible. |

All controls have disabled and focused treatments. Do not depend on reduced
opacity alone for an unchecked, off, or unselected state. Keep segment text at
12px in both selected and unselected states. Order type selection must not grow
from 12px to 13px as it does in the current shared helper.

## Tabs and navigation rows

Content tabs switch the visible panel. They use a 28px strip, 12px labels, 10px
horizontal inset, a quiet background, and a 2px accent underline on the active
tab. Inactive labels are secondary; active labels use primary or readable accent
foreground. No vertical separator between every tab. Keep one baseline divider
under the strip. Count badges use the shared badge component.

Segments select a mode within the current panel. Keep that distinction visible:
Order type is segmented; Positions/Open Orders/Balances is tab navigation.

Sidebar navigation uses 32px minimum rows, 12px labels, an optional consistent
16px icon, and 8px content gap. The selected item has a soft accent fill and a
2px leading marker. Hover retains that marker. Do not rely on color-only
selection or theme-dependent contrast between two nearly identical surfaces.

When tabs do not fit, retain existing horizontal scrolling or use a deliberate
overflow control. Do not compress labels into unreadable text. New overflow
interaction is separately scoped behavior work.

## Menus and popovers

Menus share an opaque overlay surface, 1px outline, 6px radius, optional overlay
shadow, and 4px outer padding. A single-line item is at least 28px high with
8px horizontal inset, a stable icon/check column, 12px label, and optional
right-aligned shortcut. Two-line results are at least 40px high.

Selected options have a checkmark. Keyboard-highlighted items have a persistent
focus marker. Hover uses a neutral raised surface. Disabled items retain readable
text and the reason when it materially helps the user. Group labels use 11px
metadata, separated from selectable rows by spacing or a single rule.

Anchor menus to the triggering control and clamp them within the window. Long
menus scroll inside a bounded height. Opening a menu must not move pane content.
Keep destructive items separated at the end of the relevant group. Preserve
existing Escape, outside-click, and focus-restoration behavior; add missing
keyboard interaction as an explicit implementation task.

Tooltips use the same surface family with 6 × 8px padding, 11px text, 4px radius,
and an outline. Their content is a short explanation or shortcut; essential
status and field errors remain visible without hover.

## Tables and selectable rows

Use one shared column definition for header and body. Headers are 24px high,
11px secondary text, with one baseline divider. Rows are at least 24px high
with 12px values and 8px horizontal cell inset. Dense order-book rows may use
the documented 20px exception. Center content vertically.

| Content | Alignment and formatting |
| --- | --- |
| Symbol, account name, event label | Left. Preserve an identifiable symbol; truncate optional descriptors first. |
| Price, size, value, PnL, percentage | Right, including the header. Fixed column contract and consistent precision within a column. |
| Timestamp | Consistent width and format within the table; timezone in header/tooltip when relevant. |
| Side/status | Short explicit label or rectangular badge; consistent position. |
| Row actions | End of row, compact and stable. Visibility may increase on hover/focus without moving cells. |

Rows have a transparent/panel resting surface and neutral hover fill. A selected
row has a soft accent fill plus a 2px leading marker. Selected-and-hovered rows
keep the marker. Avoid a rounded card around every row in ordinary data tables.
Zebra striping is optional only when it improves a particularly wide table;
use one faint resolved neutral surface, never another semantic hue.

Sorting uses a small consistent arrow in a reserved header slot. Only sortable
headers receive a hover/focus affordance. Keep headers visible during vertical
scrolling where the view can support it. Empty/loading/error content occupies
the body without moving the header or column grid.

### Financial values

Positive signed values use the positive foreground and `+`; negative values use
the negative foreground and a minus sign. Zero is neutral. Missing data is `—`
in secondary text; it is not zero. Use warning/critical treatment only when
missingness represents an actual freshness or error condition. Avoid rendering
negative zero after display rounding. Keep neutral quantities and volume neutral.

Preserve existing instrument precision, denominations, and calculation rules.
Compact `K`/`M`/`B` values are appropriate for scanning columns, with full values
available through existing detail/tooltip affordances. Executable order inputs
and confirmation values retain the precision needed to understand the order.
Color reflects the meaning of the metric, not an arbitrary positive number:
funding, fees, and liquidation side require their domain labels.

## Badges, identity, and metrics

Badges are 18px high, with 11px text, 6px horizontal inset, and 3px corners.
Use a quiet neutral surface for counts/categories, and paired soft semantic
colors for status/side. Optional dot or icon sits 4px from the label. A clickable
badge receives a 24px target and shared toggle states; it must not look passive.

Use labels such as `Live`, `Stale`, `Watch only`, `Pending`, `Long`, and `Short`.
Status dots alone are insufficient. Reserve pill geometry for avatars and
semantic circular controls, not arbitrary categories. Count badges use the
same family as status badges.

Identity rows put logo/avatar, primary name, optional secondary identifier, then
actions in a stable order. Use one logo box size per context, preserving aspect
ratio. Copy and external-link actions are neutral ghost controls. Follow the
existing privacy/redaction state for accounts and addresses.

Metric pairs use an 11px secondary label and 13px numeric value. Featured price
uses 16px; portfolio/journal headlines may use 24px. A borderless group separated
by spacing or one rule is preferred to a separate filled card for every metric.

## Loading, empty, stale, and error states

| State | Visual treatment | Example |
| --- | --- | --- |
| Initial loading | Quiet placeholder rows or small spinner in the content region; structure retained. | `Loading positions…` |
| Refreshing with data | Keep the data; compact progress indicator near refresh/status. | `Refreshing` |
| Successful empty result | Neutral one-line explanation; optional relevant action. | `No open orders` |
| Filtered empty result | Neutral explanation and clear-filter action if available. | `No symbols match these filters` |
| Not connected/configured | Neutral explanation with one appropriate action. | `Connect an account to view positions` |
| Stale or delayed | Warning badge and last-update context; data remains legible. | `Stale · last update 14:32:08` |
| Failure | Critical icon/text, concise context, and recovery action when supported. | `Could not load positions` / `Retry` |
| Partial result | Warning/neutral limitation next to the affected section. | `Some history is unavailable` |

These are distinct domain states, not interchangeable empty arrays. The visual
pass must use state already available and identify missing state explicitly for
follow-up; it must not invent a healthy status from silence. Skeletons do not
imitate actual prices. Avoid replacing a populated chart or table with a large
blocking spinner during background refresh.

## Banners, toasts, and dialogs

An inline notice belongs next to its field or section. A persistent banner
belongs above affected content for a continuing issue. Toasts report completed
actions and transient outcomes. Use an icon plus a short title/body; color
supports the message. Avoid making the entire workspace red for a local issue.

Toasts share a 6px radius, 1px outline, overlay surface, 12px text, and 12px
inset. Use a small semantic marker or icon rather than an opaque full-color
slab. Retain existing stacking, duration, position, copy, and dismissal settings.
Dismiss/copy buttons follow the compact ghost recipe. Repeated background
events should not drown out actionable messages.

Dialogs use the overlay surface, 6px radius, 16px inset, 18px title, 13px body,
and a clearly separated action row. Keep Cancel secondary and the commitment
action explicit. Destructive confirmation uses critical styling and names the
affected object. Do not add or remove trade confirmations as a side effect of
visual consolidation. Fit the dialog within the window and allow content to
scroll while actions remain reachable.

## Scrollbars, dividers, and progress

Decorative rules are 1px `border.subtle` and align with the content grid. Avoid
double borders where parent and child containers meet. Reserve scrollbar space
so appearing/disappearing bars do not move numeric columns. Target a 6px visible
thumb within a 10px interaction gutter; use a stronger hover/focus treatment.
Honor platform behavior where a native equivalent is used.

Progress bars use a 4px track and an accent fill for process progress. Risk or
margin meters use documented thresholds and labels, never the same styling as
a generic loading bar. Show actual completion percentages only when meaningful
progress is known; otherwise use an indeterminate indicator.
