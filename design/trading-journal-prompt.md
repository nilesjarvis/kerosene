# Design Brief — Kerosene Trading Journal (redesign)

Recreate the **Trading Journal window** for **Kerosene**, a GPU-accelerated, BYOK desktop trading terminal for Hyperliquid. Two reference screenshots accompany this brief:

- `journal-01-overview.png` — default state (no trade selected → analytics cockpit on the right)
- `journal-02-detail.png` — a trade is selected (master–detail inspector on the right)

Build it as a single, interactive desktop-app window. Match the screenshots closely.

---

## 1. Product & aesthetic

Kerosene is an engineer's instrument: technical, private, fast, a little retro-terminal — **never glossy consumer-fintech**. A warm near-black workspace lit by a single flame-orange accent, editorial serif headlines set against dense monospace data. Information density is a feature.

**Design tokens**

| Role | Value |
|---|---|
| Page background | `#090a0c` |
| Panel surface | `#101114` |
| Raised / hover | `#15161a` / `#1a1b20` |
| Sunken well | `#07080a` |
| Accent (flame orange) | `#ff8a1f` (soft `#ffd1a0`, ink-on-orange `#140d07`) |
| Primary text (cream) | `#f4f0ea` (heading `#fff2e4`) |
| Muted / dim | `#aaa59d` / `#746f68` |
| Up / long / positive | `#2fbd85` |
| Down / short / negative | `#ed7088` |
| Warn / fees | `#ffb648` |
| Hairline border | `rgba(255,255,255,0.10)` (orange focus `rgba(255,138,31,0.34)`) |

**Type**
- **Display / serif** (`Georgia`/`ui-serif`) — window title, pane titles like "Performance Overview", the selected asset name. Editorial counterweight to the technical UI.
- **UI / sans** (`Inter`) — reflection body copy, button labels.
- **Data / mono** (`Roboto Mono`, tabular figures) — **every** number, ticker, %, timestamp, label, status chip. The terminal is mono-forward; treat mono as the default for data.

**Geometry & feel**
- Radii: 3px chips, 4px buttons/inputs, 5px primary buttons, 6px cards/panels.
- Structure drawn with **hairline rules and shared edges**, not heavy containers. No drop-shadow-heavy cards inside the window.
- Status/side chips: mono UPPERCASE, ~10px, letter-spacing ~0.07em, 3px radius, tinted fill + tinted border (`LONG` green, `SHORT` red-pink, `SPOT` neutral, `CLOSED`/`FILLED` neutral).
- No emoji, no gradients-as-decoration, no stock imagery. Line icons only if any. The only non-mono glyphs are `←`, `✕`, `□`, `–`.

---

## 2. Window structure (top → bottom, all states)

1. **Title bar** (h≈42, hairline bottom): orange rounded-square **K** logo · "Trading Journal" (serif) · `TRADING` green badge · `Account 6 · 0x17ed…567d` (mono, dim) · right-aligned window controls `–  □  ✕`.
2. **Toolbar** (h≈46, hairline bottom): left — `13 trades` · `495 fills` · `Synced Jun 19, 15:16` (sync in green), separated by short vertical hairlines. Right — `SORT` label + segmented (`Recent` active / `PnL ↓` / `PnL ↑`), then `FILTER` label + segmented (`All` active / `Perp` / `Spot`). Active segment = orange outline + orange-soft text + faint orange fill.
3. **KPI strip** (always visible, hairline bottom): a single row of **8 equal cells** divided by vertical hairlines. Each cell = mono uppercase micro-label over a larger mono value. Cells: **Net PnL** `+$2,384.94` (green) · **Win Rate** `50.0%` · **Profit Factor** `3.29` · **Expectancy** `+$198.74` (green) · **Avg R** `+0.49R` · **Avg Win** `+$570.80` (green) · **Avg Loss** `-$173.31` (red) · **Fees** `$373.01` (amber).
4. **Body** — two columns: **trade list** (~404px, hairline right) | **right pane** (fills rest). This is the master–detail core.

---

## 3. Trade list (left column, both states)

- Column header row: `ASSET · POSITION` (left) / `NET PNL` (right), mono uppercase dim.
- Scrolling rows. Each row: a 30px rounded-square **mono monogram** of the asset (2 letters, neutral fill) · stacked **ticker** (orange-soft mono) + inline side chip + a dim sub-line `Position · Date` · a small **sparkline** (green for winners, red for losers) · right-aligned stacked **PnL** (green/red, mono) over **R-multiple** (dim).
- Selected row: **3px orange left bar + faint orange row wash**. Hover: subtle light wash.
- Sample rows (in order): `xyz:BTC` Long 0.40 `+$1,240.55` +2.0R · `xyz:ETH` Long 12.5 `+$892.30` +3.1R · `xyz:NVDA` Long 800 `+$545.12` +1.8R · `xyz:DRAM` Short 3000 `+$458.17` +2.4R · `xyz:TSLA` Short 300 `+$210.45` +1.2R · `HYPE` Long 5 `+$78.20` +0.7R · `xyz:META` Short 150 `-$96.40` −0.5R · `xyz:NVDA` Short 1000 `-$103.13` −0.6R · `HYPE/USDC` Spot 2617.92 `-$133.47` −0.8R · `HYPE/USDC` Spot 3654.25 `-$185.64` −0.9R · `xyz:SPCX` Short 100 `-$208.41` −1.1R · `xyz:SOL` Short 220 `-$312.80` −1.4R.

---

## 4. Right pane — state A: **Cockpit** (no trade selected) — see `journal-01-overview.png`

Header row: **"Performance Overview"** (serif) · timeframe segmented `1D / 1W(active) / MTD / 1M / 3M / YTD / ALL` · dim hint "Select a trade for detail →".

Below, a padded grid of hairline panels (each panel = 30px mono title bar over body):

- **Equity Curve** (wide) — mono title + "Cumulative realized PnL" meta + green total at right. Area-filled green line chart rising to +2.38k, faint horizontal gridlines, dashed zero line, end-point dot + value tag.
- **Win / Loss** (narrow, right) — a **donut**: 50% green / 50% red ring, center reads `50%` / `WIN RATE`. Beside it: Wins `6` (green), Losses `6` (red), Expectancy `+$198.74` (green).
- **4 KPI tiles**: Expectancy / Trade `+$198.74` · Avg Win : Avg Loss `3.29 : 1` · Avg R Multiple `+0.49R` · Total Fees `$373.01`, each with a dim sub-caption.
- **Long vs Short vs Spot** — three horizontal bars from a shared track: Long `+$2,756.17` (green, full) · Spot `-$319.11` (red) · Short `-$52.12` (red); each with a `N trades · X% win` sub-line.
- **Edge by Time of Day** — a heatmap: rows MON–FRI × columns 00/04/08/12/16/20 (UTC), green/red intensity cells.
- **PnL by Asset** — per-asset **diverging** bars centered on a midline (green right / red left), label + signed value: BTC +1240.55, ETH +892.30, DRAM +458.17, NVDA +441.99, TSLA +210.45, HYPE +78.20, META −96.40, SPCX −208.41, SOL −312.80, HYPE/USDC −319.11.

---

## 5. Right pane — state B: **Trade detail** (a trade is selected) — see `journal-02-detail.png`

Replaces the cockpit when a row is clicked.

- **Detail header**: `← Overview` button (returns to cockpit) · asset monogram · asset name (serif) · side chip (`LONG`) · status chip (`CLOSED`/`FILLED`) · right-aligned large mono PnL (green/red).
- Sub-line: `Opened {date} · Held {duration} · {fills} fills`.
- **Chart snapshot**: "CHART SNAPSHOT · ENTRY → EXIT" label + `1m / 5m(active) / 1h` segmented. A candlestick chart (green/red candles, faint gridlines) with **two dashed vertical markers** — an `OPEN` tag + orange dot at entry and a `CLOSE` tag + orange dot at exit. Caption row: `TF 5m · Candles 64 · Entry $… · Exit $… · Dir +X.XX%`.
  - **Spot trades have no chart** — instead show a dashed orange-bordered frame reading "Chart snapshots are currently available for perp trades only." (Entry shows `@ price`, Exit shows `—`.)
- **Stat grid**: 4-column hairline grid — Entry · Exit · Size · Duration · Fills · Fees (amber) · Net PnL (signed color) · R Multiple (signed color).
- **Reflection**: two blocks, each an orange-soft mono uppercase label over Inter body copy — **Entry thesis** and **Exit reflection**. Then a row of `#tag` chips and a primary orange **Save reflection** button.

---

## 6. Interaction

- Click any list row → right pane swaps to that trade's detail; the row gets the orange selected treatment.
- `← Overview` (or deselect) → right pane returns to the cockpit.
- KPI strip, toolbar, and trade list persist across both states.
- Each trade carries its own data, candle chart, entry/exit, stats, thesis & reflection copy, and tags.

## 7. Goals this redesign must hit

1. **Kill vertical scrolling** — the old journal stacked everything; this is a fixed-chrome master–detail with only the list and the right pane scrolling.
2. **Stronger analytics** — replace a thin equity-curve + win-rate block with the full cockpit (profit factor, expectancy, R-multiple, long/short/spot, time-of-day, per-asset).
3. **First-class reflection** — structured entry-thesis / exit-reflection per trade with tags, not a cramped inline note box.
4. Stay 100% within the Kerosene visual system above.
