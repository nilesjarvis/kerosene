# Alfred

Alfred is Kerosene's command palette for adding panes, opening windows, and drafting fast trading actions from typed commands. It is designed for short intent-style input: type a command, review the single preview row, then press Enter or click the row.

Configure the Alfred hotkey from Settings > Hotkeys. Escape closes Alfred without taking action.

## Command Results

Alfred shows normal command matches for panes and windows when the query is a general search. When the query parses as a trading action, Alfred switches to a single preview result so the screen shows only the trade or position action that would be submitted.

Disabled rows explain what is missing or unsafe, such as missing account data, unknown symbols, hidden tickers, missing side, invalid size, stale account data, or unavailable mid prices.

## Natural-language Orders

Use `buy` or `sell` to draft an order for the selected symbol. Alfred resolves the ticker, sets the active symbol, fills the order form, and submits through the normal order-entry execution path.

Examples:

- `buy 1k HYPE` drafts a market buy for `1,000` HYPE.
- `sell 250 HYPE` drafts a market sell for `250` HYPE.
- `buy $1k HYPE` drafts a `$1,000` USD-notional market buy.
- `buy $1k HYPE at 43` drafts a `$1,000` USD-notional limit buy at `43`.
- `chase $1k HYPE` drafts a `$1,000` USD-notional Chase order and leaves the side selection to the order-entry controls.
- `chase buy $1k HYPE` starts a `$1,000` USD-notional Chase buy.
- Spell spot markets as an explicit pair, such as `buy $1k HYPE/USDC`. A
  bare ticker with the `spot` qualifier is accepted only when exactly one
  spot pair exists for that ticker; ambiguous tickers must name the quote
  asset so Alfred cannot route an order to the wrong book. An unqualified
  bare ticker resolves only to a perpetual market and never falls back to
  spot.

Supported size suffixes:

- `k` = thousand.
- `m` = million.
- `b` = billion.

Dollar-prefixed sizes, such as `$1k`, are treated as USD notional for non-outcome markets. Outcome markets force coin-size input.

If the query omits a side, for example `$1k HYPE`, Alfred may still show the interpreted market-order draft, but it remains disabled until the user adds `buy` or `sell`. Side-less Chase commands are allowed because they only prepare the Chase order form; the user must still choose `CHASE BUY` or `CHASE SELL`.

## Close Position

Use `close` to close an existing open position at market. The command submits through the same reduce-only market close flow as the positions table.

Examples:

- `close HYPE` closes 100% of the open HYPE position.
- `close HYPE 25` closes 25% of the open HYPE position.
- `close 25 HYPE` also closes 25%.
- `close 100% HYPE` closes 100%.

Percentages must be greater than `0` and no more than `100`. If no open position matches the ticker, the command is disabled.

## NUKE Positions

Type `nuke` or `close all` to close all visible open perpetual positions at market. Alfred previews how many positions are routable and lists skipped positions when applicable.

NUKE uses reduce-only market orders and the same safety checks as the positions-table NUKE control:

- Wallet and agent key must be available.
- Account data must be fresh.
- Hidden and muted positions are not routed.
- Each position must resolve to a perpetual market.
- Each routed market needs a usable mid price.

If every position is skipped, Alfred disables the command instead of submitting a no-op.

## Safety Notes

Alfred trading actions reuse the existing order-entry and position-action code paths. They do not bypass validation, risk-hidden ticker checks, outcome read-only restrictions, stale-account checks, or market-data requirements.

Always read the preview row before pressing Enter. Market-order actions can execute immediately once submitted.
