use super::model::{MarketSessionSummary, SessionReturnBar, SessionWeekday, SessionWeekdaySummary};

// ---------------------------------------------------------------------------
// Summary statistics (verdict line + KPI strip)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionGroup {
    Weekday,
    Session,
}

/// One eligible bucket that can headline the verdict line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VerdictBucket {
    pub(crate) group: SessionGroup,
    pub(crate) label: &'static str,
    pub(crate) average_return_pct: f64,
    pub(crate) win_rate_pct: f64,
    pub(crate) sample_count: usize,
}

/// The plain-language headline for the widget: either the strongest/weakest
/// eligible buckets, or an explicit "not enough data" state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SessionVerdict {
    Insufficient {
        total_samples: usize,
        min_required: usize,
    },
    Edge {
        strongest: VerdictBucket,
        weakest: Option<VerdictBucket>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionStreak {
    pub(crate) length: usize,
    pub(crate) positive: bool,
}

/// Minimum completed-session count for a bucket to be eligible for the verdict.
/// Scales gently with the sample size so a single fluke is never crowned.
pub(crate) fn verdict_min_samples(total_samples: usize) -> usize {
    (total_samples / 40).max(4)
}

/// Count-weighted overall win rate across the weekday buckets — equivalent to
/// total green sessions / total sessions. `None` when there are no samples.
pub(crate) fn overall_win_rate_pct(weekday_summaries: &[SessionWeekdaySummary]) -> Option<f64> {
    let mut total = 0usize;
    let mut weighted = 0.0f64;
    for summary in weekday_summaries {
        total += summary.sample_count;
        weighted += summary.win_rate_pct * summary.sample_count as f64;
    }
    (total > 0).then(|| weighted / total as f64)
}

/// Mean absolute open-to-close move across completed sessions. `None` when empty.
pub(crate) fn average_abs_move_pct(bars: &[SessionReturnBar]) -> Option<f64> {
    if bars.is_empty() {
        return None;
    }
    let sum: f64 = bars.iter().map(|bar| bar.return_pct.abs()).sum();
    let avg = sum / bars.len() as f64;
    avg.is_finite().then_some(avg)
}

/// Compounded total return across all completed sessions, in percent.
pub(crate) fn total_return_pct(bars: &[SessionReturnBar]) -> Option<f64> {
    if bars.is_empty() {
        return None;
    }
    let growth = bars
        .iter()
        .fold(1.0f64, |acc, bar| acc * (1.0 + bar.return_pct / 100.0));
    let total = (growth - 1.0) * 100.0;
    total.is_finite().then_some(total)
}

/// Trailing run of same-signed sessions, counted from the most recent bar.
/// `bars` is assumed sorted ascending by open time. A flat (0%) latest session
/// yields `None`.
pub(crate) fn current_streak(bars: &[SessionReturnBar]) -> Option<SessionStreak> {
    let last = bars.last()?;
    let positive = if last.return_pct > 0.0 {
        true
    } else if last.return_pct < 0.0 {
        false
    } else {
        return None;
    };
    let length = bars
        .iter()
        .rev()
        .take_while(|bar| {
            if positive {
                bar.return_pct > 0.0
            } else {
                bar.return_pct < 0.0
            }
        })
        .count();
    (length > 0).then_some(SessionStreak { length, positive })
}

/// Weekday with the greatest total traded volume over the lookback. `None` when
/// there is no positive volume to compare.
pub(crate) fn most_active_weekday(bars: &[SessionReturnBar]) -> Option<SessionWeekday> {
    let mut totals = [0.0f64; 7];
    for bar in bars {
        if bar.volume.is_finite() && bar.volume > 0.0 {
            totals[bar.weekday.index()] += bar.volume;
        }
    }
    let mut best: Option<(usize, f64)> = None;
    for (idx, &total) in totals.iter().enumerate() {
        if total > 0.0 && best.is_none_or(|(_, current)| total > current) {
            best = Some((idx, total));
        }
    }
    best.map(|(idx, _)| SessionWeekday::ALL[idx])
}

/// Sample standard deviation of open-to-close returns per weekday, indexed by
/// `SessionWeekday::index`. `None` for weekdays with fewer than two samples.
pub(crate) fn weekday_dispersions(bars: &[SessionReturnBar]) -> [Option<f64>; 7] {
    let mut sums = [0.0f64; 7];
    let mut sum_sqs = [0.0f64; 7];
    let mut counts = [0usize; 7];
    for bar in bars {
        let idx = bar.weekday.index();
        sums[idx] += bar.return_pct;
        sum_sqs[idx] += bar.return_pct * bar.return_pct;
        counts[idx] += 1;
    }
    let mut out = [None; 7];
    for (idx, &n) in counts.iter().enumerate() {
        if n >= 2 {
            let n_f = n as f64;
            let sum = sums[idx];
            let variance = (sum_sqs[idx] - sum * sum / n_f) / (n_f - 1.0);
            let std = variance.max(0.0).sqrt();
            if std.is_finite() {
                out[idx] = Some(std);
            }
        }
    }
    out
}

/// The strongest, and when distinct the weakest, eligible bucket across both the
/// weekday and market-session breakdowns. Eligibility is gated by
/// [`verdict_min_samples`] so thin buckets never headline.
pub(crate) fn session_verdict(
    weekday_summaries: &[SessionWeekdaySummary],
    session_summaries: &[MarketSessionSummary],
    total_samples: usize,
) -> SessionVerdict {
    let min_required = verdict_min_samples(total_samples);
    let mut buckets: Vec<VerdictBucket> = Vec::new();
    for summary in weekday_summaries {
        if summary.sample_count >= min_required {
            buckets.push(VerdictBucket {
                group: SessionGroup::Weekday,
                label: summary.weekday.label(),
                average_return_pct: summary.average_return_pct,
                win_rate_pct: summary.win_rate_pct,
                sample_count: summary.sample_count,
            });
        }
    }
    for summary in session_summaries {
        if summary.sample_count >= min_required {
            buckets.push(VerdictBucket {
                group: SessionGroup::Session,
                label: summary.session.short_label(),
                average_return_pct: summary.average_return_pct,
                win_rate_pct: summary.win_rate_pct,
                sample_count: summary.sample_count,
            });
        }
    }

    if buckets.is_empty() {
        return SessionVerdict::Insufficient {
            total_samples,
            min_required,
        };
    }

    let strongest_idx = buckets
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.average_return_pct.total_cmp(&b.average_return_pct))
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    let weakest_idx = buckets
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.average_return_pct.total_cmp(&b.average_return_pct))
        .map(|(idx, _)| idx)
        .unwrap_or(0);

    let strongest = buckets[strongest_idx];
    let weakest = (weakest_idx != strongest_idx).then(|| buckets[weakest_idx]);
    SessionVerdict::Edge { strongest, weakest }
}
