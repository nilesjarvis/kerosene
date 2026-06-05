# Liquidation Feed Widget — UI/Design Audit

## 1. Layout & Spacing

**Good:**
- Responsive columnar layout with `responsive` wrapper adapting to available width.
- Graceful progressive column-hiding (method at 680px, user at 590px, price at 500px, size at 410px, side at 330px).
- Consistent 8px spacing between major sections and 4px between rows.

**Issues:**
- **No column resizing or manual reordering.** Users cannot customize which columns appear/disappear — it's entirely width-driven. A long widget in a tall pane hides columns that the user may want to see.
- **Fixed column widths** (e.g., `TIME_WIDTH = 60`, `COIN_WIDTH = 80`) are inflexible. A symbol like `PEPE2W` could overflow the 80px coin column since `Wrapping::None` is set.
- **No minimum/maximum pane sizing** — the widget fills whatever space it gets, which can make it too wide (wasteful) or too narrow (columns disappear) without user control.

## 2. Top Bar / Controls

**Good:**
- Connection status indicator with status dot and tooltip is clear and conventional.
- Settings dropdown uses `float` for proper z-indexing over the feed.
- Threshold input is compact and contextually placed.

**Issues:**
- **Redundant threshold display.** The `>$` prefix label next to a `Min $` placeholder in the text input is visually noisy — you get `>$ [Min $]` which reads as "more than dollar sign min dollar sign." Pick one signal, not both.
- **Threshold input is a free-form text field** with no validation visible. A user can type "abc" and the feed silently shows nothing — no error feedback.
- **Settings button (⚙️) is semantically clear** but the gear icon at 13px is borderline too small for touch/small-cursor targets. Consider 14px or a slightly larger padding.
- **"Clear" button** has no tooltip explaining what "clear" means (clear the feed? clear all liquidations?).
- **No filter for symbols** — at narrow widths, the column collapse is the only filtering. Users with long hidden-symbol lists may want to see only their watched symbols regardless of width.

## 3. Header Row

**Good:**
- Muted text color for headers creates good visual hierarchy.
- "Avg Px" label changes when aggregation is enabled — good contextual awareness.

**Issues:**
- **Header row is not sticky/fixed.** As the feed scrolls, the header scrolls away. This is the single biggest UX flaw. The user loses context of what each column means after scrolling even a few rows.
- **No sort indicators.** The header looks like it could be clickable (it's just text, not a button) but doesn't signal whether sorting is available or not. If sorting isn't implemented, the header shouldn't look interactive. If it is, it should have sort state indicators (▲/▼).
- **Header alignment inconsistency.** Columns use hardcoded widths, but the "Method" column has `Space::new().width(Fill)` pushed before it, pushing it right. The header and data rows both do this, so they align, but the visual asymmetry is confusing.

## 4. Feed Rows

**Good:**
- **Heatmap-style row highlighting** based on notional value is excellent — subtle color coding from muted (<$1K) to vibrant (>$500K) with 6 tiers.
- **Monospace font** throughout for data columns — good for alignment and readability of numbers.
- **Symbol button** with hover-highlight and click-to-select is a nice interaction.
- **User cell** supports copy-to-clipboard with tooltip showing full address.
- **Relative time display** ("2s ago") instead of absolute timestamps.

**Issues:**
- **Row padding is tight (4px vertical, 8px horizontal).** At 12px font, 4px padding feels cramped. Consider 5-6px vertical for breathing room.
- **Relative time can be imprecise.** "2s ago" at a glance doesn't convey as much as "14:32" for a feed where the user may want to correlate with other events. Consider a hybrid: relative under 60s, absolute time after.
- **The BUY/SELL label width (50px) feels generous** for a 4-character string. The space is underutilized.
- **No row separator.** Rows rely entirely on background color + 4px gap. With low-opacity colors (<$1K rows have 0.02 opacity), rows blend together on some themes. A subtle left-border accent or 1px bottom border would help.
- **Row corner radius** (via `liquidation_row_style`) is nice, but combined with 4px vertical padding, the curves feel disproportionate — small pill-shaped rectangles.

## 5. Row Styling / Color Semantics

**Issues:**
- **The color semantics are inverted for the heatmap.** Long liquidations (SELL side — `is_buy = false`) get the "danger" (red) color, and short liquidations (BUY side — `is_buy = true`) get "success" (green). This is technically correct from the position-holder's perspective (longs getting liquidated = danger), but the background glow is on the *row*, not the side label. A row with a strong green glow could be misread as "good news" at a glance before the user parses the SELL/Buy detail. The row-level color should be about magnitude, and the side indicator should carry the semantic color.
- **Brightness scaling is arbitrary.** The `brightness` multiplier (0.4–1.2) can push values above 1.0 (capped to 1.0), meaning the >$500K tier uses the same red as the <$500K tier — just with higher opacity. The 1.2x brightness is a dead operation.
- **No visual distinction for aggregation count** in the row itself. The `x3` count only appears in the Method column. At narrow widths where Method is hidden, the aggregation indicator is lost entirely.

## 6. Chart

**Good:**
- 60-bar histogram with buy/sell split and tooltip is a compact, informative view.
- Tooltip shows buy/sell breakdown per bar.
- Bars are normalized to the max value in the window.

**Issues:**
- **No axis labels, time markers, or scale indicators.** The chart is 60 bars but there's no way to tell which bar corresponds to "now" vs. "60 seconds ago." Consider a subtle time axis or at minimum anchoring "now" on the right with a marker.
- **Bar height is fixed at 24px max.** With `max_bar_height * 2` for the total column height (up to 48px), the chart can look very short and squat at low volumes.
- **The buy/sell stacked-bar approach** makes it hard to compare the two sides visually. A diverging bar (buy up, sell down from a center line) would be more readable.
- **Tooltips for every bar** could cause visual clutter if the user hovers quickly. A shared tooltip or hover range would reduce noise.
- **No y-axis scale** — the chart only shows relative heights. A small "$10K" label at the max height would provide useful context.

## 7. Summary Bar

**Good:**
- Four timeframes (1m, 5m, 15m, 1H) give good temporal context.
- Ratio bars visualize long/short split.
- Muted state for no-data is handled gracefully.

**Issues:**
- **Summary bar is dense.** Four stacked ratio bars with labels at 10px and 11px is information-dense without much visual breathing room. Consider a more compact format or horizontal layout with less vertical stacking.
- **The ratio bars (3px height) are very thin.** At 3px, the long/short split is hard to perceive — especially the subtle differences. A height of 4-5px would be more legible.
- **No total count or number-of-events metric.** The summary shows notional but not how many individual liquidation events contributed, which could be useful context.
- **"L" and "S" abbreviations** require the user to parse them. A small color dot before "L" and "S" would reinforce the long=red, short=green mapping visually.

## 8. Scrollable Area

**Issues:**
- **Scroll behavior isn't configured.** No `snap_to_grid` or `scroll_to_start/end` behavior. As new liquidations arrive, the feed doesn't auto-scroll unless the user is at the bottom. This is a critical UX gap for a live feed — there should be a "follow" mode.
- **No "new items" indicator.** If the feed grows while the user is scrolled up, there's no badge or prompt to scroll down to see new data.
- **Scrollbar width is 4px with 0 margin.** This is minimal and may be hard to grab on some systems.

## 9. Accessibility & Readability

**Issues:**
- **10px font size** in the top bar controls and dropdown is below WCAG recommended minimums and strains readability, especially at the edges of the screen.
- **Color contrast on low-opacity rows.** The < $1K rows have 0.02 background opacity — essentially invisible. The text sits on whatever the scrollable background is. If that's dark, light text works; if there's a subtle gradient or texture, readability drops.
- **No keyboard navigation hints.** The symbol button and user button are interactive but discoverability is visual-only (hover highlight).

## 10. Missing UI Features

- **No sort functionality** (by time, notional, symbol, side).
- **No search/filter** within the feed (e.g., filter by specific symbol).
- **No row expansion** — clicking a row doesn't reveal more detail.
- **No "mark as read" or "pin" functionality** for notable liquidations.
- **No animation or transition** when rows are added/removed — changes are instant.
- **No loading state** visible while the feed is populating.

---

**Summary:** The liquidation feed is well-structured with good responsive design and a thoughtful heatmap approach. The most impactful fixes are: (1) make the header row sticky during scroll, (2) add auto-scroll/follow mode for the live feed, (3) reduce the double-signaled threshold input, and (4) add time axis markers to the chart. The color-inversion risk on row backgrounds is a subtle but important semantic issue to address.
