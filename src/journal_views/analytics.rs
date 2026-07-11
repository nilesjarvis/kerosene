use crate::journal::AggregatedTrade;
use std::{collections::HashMap, fmt};

// ---------------------------------------------------------------------------
// Journal cockpit analytics
//
// Pure aggregations over a filtered trade slice that feed the KPI strip and the
// analytics cockpit. "Scored" trades — the denominator for win rate, profit
// factor, expectancy and R-multiple — are closed perp trades whose opening
// basis is within loaded history (matching the legacy win-rate definition).
// PnL totals span every filtered trade so spot/outcome activity still shows up
// in net PnL and per-asset bars.
// ---------------------------------------------------------------------------

const TIME_OF_DAY_BUCKET_HOURS: u64 = 4;
const TIME_OF_DAY_BUCKETS: usize = 6;
const TIME_OF_DAY_WEEKDAYS: usize = 5;
const MS_PER_DAY: u64 = 86_400_000;
const MS_PER_HOUR: u64 = 3_600_000;

pub(crate) fn journal_is_non_perp(coin: &str) -> bool {
    // Spot index (`@`), outcome (`#`), or a named spot pair (`PURR/USDC`).
    coin.starts_with('@') || coin.starts_with('#') || coin.contains('/')
}

fn journal_is_scored(trade: &AggregatedTrade) -> bool {
    trade.status == "CLOSED" && !journal_is_non_perp(&trade.coin) && trade.basis_complete
}

pub(crate) fn journal_effective_pnl(trade: &AggregatedTrade, include_fees: bool) -> f64 {
    if include_fees {
        trade.pnl - trade.fee
    } else {
        trade.pnl
    }
}

#[derive(Clone)]
pub(crate) struct JournalKpis {
    pub net_pnl: f64,
    pub total_fees: f64,
    pub win_rate: f64,
    pub wins: usize,
    pub losses: usize,
    pub scored: usize,
    pub profit_factor: Option<f64>,
    pub expectancy: Option<f64>,
    pub avg_win: Option<f64>,
    pub avg_loss: Option<f64>,
    pub avg_r: Option<f64>,
    /// Risk unit: mean absolute loss across scored losers. `None` with no losers.
    pub r_unit: Option<f64>,
}

impl fmt::Debug for JournalKpis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalKpis")
            .field("metrics", &"<redacted>")
            .field("wins", &self.wins)
            .field("losses", &self.losses)
            .field("scored", &self.scored)
            .field("has_profit_factor", &self.profit_factor.is_some())
            .field("has_expectancy", &self.expectancy.is_some())
            .field("has_avg_win", &self.avg_win.is_some())
            .field("has_avg_loss", &self.avg_loss.is_some())
            .field("has_avg_r", &self.avg_r.is_some())
            .field("has_r_unit", &self.r_unit.is_some())
            .finish()
    }
}

pub(crate) fn journal_kpis(trades: &[&AggregatedTrade], include_fees: bool) -> JournalKpis {
    let mut net_pnl = 0.0;
    let mut total_fees = 0.0;
    let mut win_sum = 0.0;
    let mut loss_sum = 0.0; // magnitude (positive)
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;

    for trade in trades {
        net_pnl += journal_effective_pnl(trade, include_fees);
        total_fees += trade.fee;

        if journal_is_scored(trade) {
            let pnl = journal_effective_pnl(trade, include_fees);
            if pnl > 0.0 {
                wins += 1;
                win_sum += pnl;
            } else if pnl < 0.0 {
                losses += 1;
                loss_sum += pnl.abs();
            } else {
                flats += 1;
            }
        }
    }

    let scored = wins + losses + flats;
    let win_rate = if scored > 0 {
        (wins as f64 / scored as f64) * 100.0
    } else {
        0.0
    };
    let avg_win = (wins > 0).then(|| win_sum / wins as f64);
    let avg_loss = (losses > 0).then(|| -(loss_sum / losses as f64));
    let r_unit = (losses > 0).then(|| loss_sum / losses as f64);
    let profit_factor = (loss_sum > 0.0).then(|| win_sum / loss_sum);
    let expectancy = (scored > 0).then(|| (win_sum - loss_sum) / scored as f64);
    let avg_r = match (expectancy, r_unit) {
        (Some(expectancy), Some(unit)) if unit > 0.0 => Some(expectancy / unit),
        _ => None,
    };

    JournalKpis {
        net_pnl,
        total_fees,
        win_rate,
        wins,
        losses,
        scored,
        profit_factor,
        expectancy,
        avg_win,
        avg_loss,
        avg_r,
        r_unit,
    }
}

/// Per-trade R-multiple using the cockpit risk unit. `None` when no risk unit is
/// available (no scored losers) or the trade is not a scored perp.
pub(crate) fn journal_trade_r_multiple(
    trade: &AggregatedTrade,
    r_unit: Option<f64>,
    include_fees: bool,
) -> Option<f64> {
    let unit = r_unit.filter(|unit| *unit > 0.0)?;
    if !journal_is_scored(trade) {
        return None;
    }
    Some(journal_effective_pnl(trade, include_fees) / unit)
}

#[derive(Clone, Default)]
pub(crate) struct JournalSegmentStats {
    pub pnl: f64,
    pub count: usize,
    pub closed: usize,
    pub wins: usize,
}

impl fmt::Debug for JournalSegmentStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalSegmentStats")
            .field("pnl", &"<redacted>")
            .field("count", &self.count)
            .field("closed", &self.closed)
            .field("wins", &self.wins)
            .finish()
    }
}

impl JournalSegmentStats {
    pub fn win_rate(&self) -> Option<f64> {
        (self.closed > 0).then(|| (self.wins as f64 / self.closed as f64) * 100.0)
    }
}

#[derive(Clone, Default)]
pub(crate) struct JournalDirectionSplit {
    pub long: JournalSegmentStats,
    pub short: JournalSegmentStats,
    pub spot: JournalSegmentStats,
}

impl fmt::Debug for JournalDirectionSplit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalDirectionSplit")
            .field("long", &self.long)
            .field("short", &self.short)
            .field("spot", &self.spot)
            .finish()
    }
}

pub(crate) fn journal_direction_split(
    trades: &[&AggregatedTrade],
    include_fees: bool,
) -> JournalDirectionSplit {
    let mut split = JournalDirectionSplit::default();
    for trade in trades {
        let pnl = journal_effective_pnl(trade, include_fees);
        let segment = if journal_is_non_perp(&trade.coin) {
            &mut split.spot
        } else if trade.is_long {
            &mut split.long
        } else {
            &mut split.short
        };
        segment.pnl += pnl;
        segment.count += 1;
        if trade.status == "CLOSED" {
            segment.closed += 1;
            if pnl > 0.0 {
                segment.wins += 1;
            }
        }
    }
    split
}

#[derive(Clone)]
pub(crate) struct JournalAssetPnl {
    pub coin: String,
    pub pnl: f64,
}

impl fmt::Debug for JournalAssetPnl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalAssetPnl")
            .field("coin", &"<redacted>")
            .field("pnl", &"<redacted>")
            .finish()
    }
}

/// Per-asset net PnL, sorted from most positive to most negative.
pub(crate) fn journal_asset_pnls(
    trades: &[&AggregatedTrade],
    include_fees: bool,
) -> Vec<JournalAssetPnl> {
    let mut by_coin: HashMap<String, f64> = HashMap::new();
    for trade in trades {
        *by_coin.entry(trade.coin.clone()).or_insert(0.0) +=
            journal_effective_pnl(trade, include_fees);
    }
    let mut assets: Vec<JournalAssetPnl> = by_coin
        .into_iter()
        .map(|(coin, pnl)| JournalAssetPnl { coin, pnl })
        .collect();
    assets.sort_by(|a, b| {
        b.pnl
            .partial_cmp(&a.pnl)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.coin.cmp(&b.coin))
    });
    assets
}

#[derive(Clone, Copy, Default)]
pub(crate) struct JournalHeatCell {
    pub count: usize,
    pub pnl: f64,
}

impl fmt::Debug for JournalHeatCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalHeatCell")
            .field("count", &self.count)
            .field("pnl", &"<redacted>")
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct JournalTimeOfDay {
    /// `[weekday Mon..Fri][bucket 00/04/08/12/16/20 UTC]`.
    pub cells: [[JournalHeatCell; TIME_OF_DAY_BUCKETS]; TIME_OF_DAY_WEEKDAYS],
    pub max_abs_pnl: f64,
}

impl fmt::Debug for JournalTimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalTimeOfDay")
            .field(
                "populated_cells",
                &self
                    .cells
                    .iter()
                    .flatten()
                    .filter(|cell| cell.count > 0)
                    .count(),
            )
            .field("max_abs_pnl", &"<redacted>")
            .finish()
    }
}

impl Default for JournalTimeOfDay {
    fn default() -> Self {
        Self {
            cells: [[JournalHeatCell::default(); TIME_OF_DAY_BUCKETS]; TIME_OF_DAY_WEEKDAYS],
            max_abs_pnl: 0.0,
        }
    }
}

/// Weekday index where Monday = 0 … Sunday = 6 (UTC), from an epoch timestamp.
fn weekday_index(time_ms: u64) -> usize {
    let days = time_ms / MS_PER_DAY;
    // 1970-01-01 (day 0) was a Thursday; shift so Monday maps to 0.
    ((days + 3) % 7) as usize
}

fn hour_bucket(time_ms: u64) -> usize {
    let hour = (time_ms % MS_PER_DAY) / MS_PER_HOUR;
    ((hour / TIME_OF_DAY_BUCKET_HOURS) as usize).min(TIME_OF_DAY_BUCKETS - 1)
}

pub(crate) fn journal_time_of_day(
    trades: &[&AggregatedTrade],
    include_fees: bool,
) -> JournalTimeOfDay {
    let mut grid = JournalTimeOfDay::default();
    for trade in trades {
        let weekday = weekday_index(trade.start_time);
        if weekday >= TIME_OF_DAY_WEEKDAYS {
            continue; // skip weekends
        }
        let bucket = hour_bucket(trade.start_time);
        let cell = &mut grid.cells[weekday][bucket];
        cell.count += 1;
        cell.pnl += journal_effective_pnl(trade, include_fees);
    }
    grid.max_abs_pnl = grid
        .cells
        .iter()
        .flatten()
        .map(|cell| cell.pnl.abs())
        .fold(0.0_f64, f64::max);
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trade(coin: &str, is_long: bool, pnl: f64, fee: f64, start_time: u64) -> AggregatedTrade {
        AggregatedTrade {
            id: format!("{coin}-{start_time}-{pnl}"),
            legacy_note_ids: Vec::new(),
            coin: coin.to_string(),
            start_time,
            end_time: Some(start_time + 1),
            max_position: 1.0,
            volume: 100.0,
            fee,
            pnl,
            status: "CLOSED".to_string(),
            fill_count: 1,
            avg_entry_price: 100.0,
            total_entry_notional: 100.0,
            total_entry_size: 1.0,
            is_long,
            basis_complete: true,
        }
    }

    #[test]
    fn kpis_compute_profit_factor_expectancy_and_r() {
        let trades = [
            trade("BTC", true, 300.0, 0.0, 1),
            trade("ETH", true, 100.0, 0.0, 2),
            trade("SOL", false, -100.0, 0.0, 3),
            trade("DOGE", false, -100.0, 0.0, 4),
        ];
        let refs: Vec<&AggregatedTrade> = trades.iter().collect();
        let kpis = journal_kpis(&refs, false);

        assert_eq!(kpis.wins, 2);
        assert_eq!(kpis.losses, 2);
        assert_eq!(kpis.scored, 4);
        assert!((kpis.win_rate - 50.0).abs() < 1e-9);
        assert!((kpis.net_pnl - 200.0).abs() < 1e-9);
        assert!((kpis.profit_factor.unwrap() - 2.0).abs() < 1e-9); // 400 / 200
        assert!((kpis.avg_win.unwrap() - 200.0).abs() < 1e-9);
        assert!((kpis.avg_loss.unwrap() + 100.0).abs() < 1e-9);
        assert!((kpis.expectancy.unwrap() - 50.0).abs() < 1e-9); // 200 / 4
        assert!((kpis.r_unit.unwrap() - 100.0).abs() < 1e-9);
        assert!((kpis.avg_r.unwrap() - 0.5).abs() < 1e-9); // expectancy / r_unit
    }

    #[test]
    fn kpis_apply_fees_when_requested() {
        let trades = [trade("BTC", true, 100.0, 10.0, 1)];
        let refs: Vec<&AggregatedTrade> = trades.iter().collect();

        let gross = journal_kpis(&refs, false);
        assert!((gross.net_pnl - 100.0).abs() < 1e-9);

        let net = journal_kpis(&refs, true);
        assert!((net.net_pnl - 90.0).abs() < 1e-9);
        assert!((net.total_fees - 10.0).abs() < 1e-9);
    }

    #[test]
    fn no_losers_leaves_r_metrics_undefined() {
        let trades = [trade("BTC", true, 100.0, 0.0, 1)];
        let refs: Vec<&AggregatedTrade> = trades.iter().collect();
        let kpis = journal_kpis(&refs, false);

        assert!(kpis.r_unit.is_none());
        assert!(kpis.avg_r.is_none());
        assert!(kpis.profit_factor.is_none());
        assert!(journal_trade_r_multiple(&trades[0], kpis.r_unit, false).is_none());
    }

    #[test]
    fn spot_trades_excluded_from_scoring_but_counted_in_pnl() {
        let trades = [
            trade("@107", true, -50.0, 0.0, 1),
            trade("BTC", true, 100.0, 0.0, 2),
        ];
        let refs: Vec<&AggregatedTrade> = trades.iter().collect();
        let kpis = journal_kpis(&refs, false);

        assert_eq!(kpis.scored, 1); // only BTC perp
        assert_eq!(kpis.wins, 1);
        assert!((kpis.net_pnl - 50.0).abs() < 1e-9); // -50 spot + 100 perp
    }

    #[test]
    fn direction_split_partitions_long_short_spot() {
        let trades = [
            trade("BTC", true, 100.0, 0.0, 1),
            trade("ETH", false, -20.0, 0.0, 2),
            trade("@107", true, 5.0, 0.0, 3),
        ];
        let refs: Vec<&AggregatedTrade> = trades.iter().collect();
        let split = journal_direction_split(&refs, false);

        assert_eq!(split.long.count, 1);
        assert!((split.long.pnl - 100.0).abs() < 1e-9);
        assert_eq!(split.short.count, 1);
        assert!((split.short.pnl + 20.0).abs() < 1e-9);
        assert_eq!(split.spot.count, 1);
        assert!((split.spot.pnl - 5.0).abs() < 1e-9);
    }

    #[test]
    fn asset_pnls_sorted_high_to_low() {
        let trades = [
            trade("ETH", true, 50.0, 0.0, 1),
            trade("BTC", true, 100.0, 0.0, 2),
            trade("SOL", false, -30.0, 0.0, 3),
            trade("BTC", true, 40.0, 0.0, 4),
        ];
        let refs: Vec<&AggregatedTrade> = trades.iter().collect();
        let assets = journal_asset_pnls(&refs, false);

        assert_eq!(assets[0].coin, "BTC");
        assert!((assets[0].pnl - 140.0).abs() < 1e-9);
        assert_eq!(assets.last().unwrap().coin, "SOL");
    }

    #[test]
    fn weekday_index_maps_known_dates() {
        // 2021-01-04 00:00 UTC is a Monday.
        let monday = 1_609_718_400_000;
        assert_eq!(weekday_index(monday), 0);
        // +5 days → Saturday (index 5).
        assert_eq!(weekday_index(monday + 5 * MS_PER_DAY), 5);
    }

    #[test]
    fn time_of_day_buckets_by_weekday_and_hour() {
        let monday_0900 = 1_609_718_400_000 + 9 * MS_PER_HOUR;
        let trades = [trade("BTC", true, 100.0, 0.0, monday_0900)];
        let refs: Vec<&AggregatedTrade> = trades.iter().collect();
        let grid = journal_time_of_day(&refs, false);

        // 09:00 falls in the 08-12 bucket (index 2) on Monday (row 0).
        assert_eq!(grid.cells[0][2].count, 1);
        assert!((grid.cells[0][2].pnl - 100.0).abs() < 1e-9);
        assert!((grid.max_abs_pnl - 100.0).abs() < 1e-9);
    }

    #[test]
    fn journal_analytics_debug_redacts_account_values_without_changing_them() {
        let kpis = JournalKpis {
            net_pnl: 98_765.432_1,
            total_fees: 12_345.678_9,
            win_rate: 66.678_912_3,
            wins: 2,
            losses: 1,
            scored: 3,
            profit_factor: Some(8.123_456_7),
            expectancy: Some(7.234_567_8),
            avg_win: Some(6.345_678_9),
            avg_loss: Some(-5.456_789_1),
            avg_r: Some(4.567_891_2),
            r_unit: Some(3.678_912_3),
        };
        let split = JournalDirectionSplit {
            long: JournalSegmentStats {
                pnl: 98_765.432_1,
                count: 1,
                closed: 1,
                wins: 1,
            },
            short: JournalSegmentStats::default(),
            spot: JournalSegmentStats::default(),
        };
        let asset = JournalAssetPnl {
            coin: "private-journal-asset-sentinel".to_string(),
            pnl: -12_345.678_9,
        };
        let mut time = JournalTimeOfDay::default();
        time.cells[0][0] = JournalHeatCell {
            count: 1,
            pnl: 45_678.912_3,
        };
        time.max_abs_pnl = 45_678.912_3;

        let rendered = format!("{kpis:?} {split:?} {asset:?} {time:?}");

        assert!(rendered.contains("metrics: \"<redacted>\""), "{rendered}");
        assert!(rendered.contains("populated_cells: 1"), "{rendered}");
        assert!(
            !rendered.contains("private-journal-asset-sentinel"),
            "{rendered}"
        );
        for value in [98_765.432_1, 12_345.678_9, 45_678.912_3] {
            assert!(!rendered.contains(&format!("{value:?}")), "{rendered}");
        }
        assert_eq!(kpis.net_pnl.to_bits(), 98_765.432_1_f64.to_bits());
        assert_eq!(asset.coin, "private-journal-asset-sentinel");
        assert_eq!(time.cells[0][0].pnl.to_bits(), 45_678.912_3_f64.to_bits());
    }
}
