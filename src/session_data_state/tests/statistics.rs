use super::*;

#[test]
fn overall_win_rate_is_count_weighted() {
    let summaries = vec![
        weekday_summary(SessionWeekday::Mon, 2, 0.0, 50.0),
        weekday_summary(SessionWeekday::Tue, 8, 0.0, 75.0),
    ];
    // (50*2 + 75*8) / 10 = 70
    crate::helpers::assert_close(overall_win_rate_pct(&summaries).unwrap(), 70.0);
    assert!(overall_win_rate_pct(&[]).is_none());
    assert!(overall_win_rate_pct(&[weekday_summary(SessionWeekday::Mon, 0, 0.0, 0.0)]).is_none());
}

#[test]
fn average_abs_move_uses_magnitude() {
    let bars = vec![
        sample_bar(SessionWeekday::Mon, 2.0, 1.0, 0),
        sample_bar(SessionWeekday::Tue, -4.0, 1.0, 1),
    ];
    crate::helpers::assert_close(average_abs_move_pct(&bars).unwrap(), 3.0);
    assert!(average_abs_move_pct(&[]).is_none());
}

#[test]
fn total_return_compounds_sessions() {
    let bars = vec![
        sample_bar(SessionWeekday::Mon, 10.0, 1.0, 0),
        sample_bar(SessionWeekday::Tue, -10.0, 1.0, 1),
    ];
    // 1.10 * 0.90 - 1 = -0.01 -> -1%
    crate::helpers::assert_close(total_return_pct(&bars).unwrap(), -1.0);
    assert!(total_return_pct(&[]).is_none());
}

#[test]
fn current_streak_counts_trailing_same_sign() {
    let bars = vec![
        sample_bar(SessionWeekday::Mon, 1.0, 1.0, 0),
        sample_bar(SessionWeekday::Tue, -2.0, 1.0, 1),
        sample_bar(SessionWeekday::Wed, 3.0, 1.0, 2),
        sample_bar(SessionWeekday::Thu, 4.0, 1.0, 3),
    ];
    let streak = current_streak(&bars).expect("trailing up streak");
    assert_eq!(streak.length, 2);
    assert!(streak.positive);
}

#[test]
fn current_streak_none_when_latest_flat() {
    let bars = vec![
        sample_bar(SessionWeekday::Mon, 3.0, 1.0, 0),
        sample_bar(SessionWeekday::Tue, 0.0, 1.0, 1),
    ];
    assert!(current_streak(&bars).is_none());
    assert!(current_streak(&[]).is_none());
}

#[test]
fn most_active_weekday_picks_max_volume() {
    let bars = vec![
        sample_bar(SessionWeekday::Mon, 1.0, 10.0, 0),
        sample_bar(SessionWeekday::Tue, 1.0, 50.0, 1),
        sample_bar(SessionWeekday::Mon, 1.0, 30.0, 2),
    ];
    // Mon totals 40, Tue 50 -> Tue
    assert_eq!(most_active_weekday(&bars), Some(SessionWeekday::Tue));
    assert!(most_active_weekday(&[]).is_none());
}

#[test]
fn weekday_dispersions_use_sample_std_dev() {
    let bars = vec![
        sample_bar(SessionWeekday::Mon, 1.0, 1.0, 0),
        sample_bar(SessionWeekday::Mon, 3.0, 1.0, 1),
        sample_bar(SessionWeekday::Tue, 5.0, 1.0, 2),
    ];
    let disp = weekday_dispersions(&bars);
    // Mon mean 2, sample variance (1 + 1)/(2-1) = 2 -> std sqrt(2)
    crate::helpers::assert_close(disp[SessionWeekday::Mon.index()].unwrap(), 2.0_f64.sqrt());
    // Tue single sample -> None
    assert!(disp[SessionWeekday::Tue.index()].is_none());
}

#[test]
fn session_verdict_picks_strongest_and_weakest_eligible() {
    let weekdays = vec![
        // Highest avg but below MIN_N -> must be ignored.
        weekday_summary(SessionWeekday::Mon, 2, 5.0, 100.0),
        weekday_summary(SessionWeekday::Tue, 10, 0.8, 62.0),
        weekday_summary(SessionWeekday::Sun, 10, -0.6, 40.0),
    ];
    match session_verdict(&weekdays, &[], 24) {
        SessionVerdict::Edge { strongest, weakest } => {
            assert_eq!(strongest.label, "Tue");
            crate::helpers::assert_close(strongest.average_return_pct, 0.8);
            let weakest = weakest.expect("distinct weakest");
            assert_eq!(weakest.label, "Sun");
            crate::helpers::assert_close(weakest.average_return_pct, -0.6);
        }
        other => panic!("expected Edge, got {other:?}"),
    }
}

#[test]
fn session_verdict_insufficient_when_no_bucket_clears_min() {
    let weekdays = vec![
        weekday_summary(SessionWeekday::Mon, 3, 5.0, 100.0),
        weekday_summary(SessionWeekday::Tue, 2, -5.0, 0.0),
    ];
    // total_samples 5 -> min_required max(4, 0) = 4; nothing qualifies.
    assert!(matches!(
        session_verdict(&weekdays, &[], 5),
        SessionVerdict::Insufficient {
            min_required: 4,
            ..
        }
    ));
}

#[test]
fn session_verdict_single_eligible_has_no_weakest() {
    let weekdays = vec![weekday_summary(SessionWeekday::Tue, 10, 0.8, 62.0)];
    match session_verdict(&weekdays, &[], 10) {
        SessionVerdict::Edge { strongest, weakest } => {
            assert_eq!(strongest.label, "Tue");
            assert!(weakest.is_none());
        }
        other => panic!("expected Edge, got {other:?}"),
    }
}

#[test]
fn session_verdict_preserves_cross_group_ties_and_total_float_order() {
    let weekdays = [weekday_summary(SessionWeekday::Mon, 4, 1.0, 50.0)];
    let mut sessions = [MarketSessionSummary {
        session: MarketSession::Asia,
        sample_count: 4,
        average_return_pct: 1.0,
        win_rate_pct: 75.0,
    }];
    let SessionVerdict::Edge { strongest, weakest } = session_verdict(&weekdays, &sessions, 8)
    else {
        panic!("eligible buckets");
    };
    assert_eq!(strongest.group, SessionGroup::Session);
    assert_eq!(strongest.label, "Asia");
    assert_eq!(strongest.win_rate_pct, 75.0);
    let weakest = weakest.expect("distinct tied bucket");
    assert_eq!(weakest.group, SessionGroup::Weekday);
    assert_eq!(weakest.label, "Mon");

    sessions[0].average_return_pct = f64::NAN;
    let SessionVerdict::Edge { strongest, weakest } = session_verdict(&weekdays, &sessions, 8)
    else {
        panic!("eligible nonfinite bucket");
    };
    assert!(strongest.average_return_pct.is_nan());
    assert_eq!(strongest.label, "Asia");
    assert_eq!(weakest.expect("finite weakest").label, "Mon");

    let signed_zeros = [
        weekday_summary(SessionWeekday::Mon, 4, 0.0, 0.0),
        weekday_summary(SessionWeekday::Tue, 4, -0.0, 0.0),
    ];
    let SessionVerdict::Edge { strongest, weakest } = session_verdict(&signed_zeros, &[], 8) else {
        panic!("eligible signed zeros");
    };
    assert_eq!(strongest.label, "Mon");
    assert_eq!(weakest.expect("negative zero weakest").label, "Tue");
    assert_eq!(
        session_verdict(&weekdays, &sessions, 200),
        SessionVerdict::Insufficient {
            total_samples: 200,
            min_required: 5,
        }
    );
}
