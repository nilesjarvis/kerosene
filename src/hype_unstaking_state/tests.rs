use super::*;

fn event(unlock_time_ms: u64, user: &str, hype: u64) -> HypeUnstakingEvent {
    HypeUnstakingEvent {
        unlock_time_ms,
        user: user.to_string(),
        amount_wei: hype * HYPE_CORE_WEI_PER_TOKEN as u64,
    }
}

#[test]
fn data_sorts_events_by_unlock_time() {
    let data = HypeUnstakingQueueData::new(vec![
        event(3_000, "0x3", 1),
        event(1_000, "0x1", 1),
        event(2_000, "0x2", 1),
    ]);

    assert_eq!(
        data.events
            .iter()
            .map(|event| event.unlock_time_ms)
            .collect::<Vec<_>>(),
        vec![1_000, 2_000, 3_000]
    );
}

#[test]
fn hype_unstaking_event_debug_redacts_wallet_and_amount() {
    let secret_address = "0xf764939b589138dd1c75601b10a408c66ee68cbe";
    let event = HypeUnstakingEvent {
        unlock_time_ms: 1_779_301_327_387,
        user: secret_address.to_string(),
        amount_wei: 987_654_321,
    };

    let rendered = format!("{event:?}");

    assert!(rendered.contains("unlock_time_ms: 1779301327387"));
    assert!(rendered.contains("user: \"<redacted>\""));
    assert!(rendered.contains("amount_wei: \"<redacted>\""));
    assert!(!rendered.contains(secret_address));
    assert!(!rendered.contains("987654321"));
}

#[test]
fn hype_unstaking_data_debug_summarizes_events() {
    let secret_address = "0xf764939b589138dd1c75601b10a408c66ee68cbe";
    let data = HypeUnstakingQueueData::new(vec![
        HypeUnstakingEvent {
            unlock_time_ms: 2_000,
            user: secret_address.to_string(),
            amount_wei: 987_654_321,
        },
        HypeUnstakingEvent {
            unlock_time_ms: 4_000,
            user: "0x2c64a1d5d602e7fb6d21da6211dcecc6e17a0649".to_string(),
            amount_wei: 123_456_789,
        },
    ]);

    let rendered = format!("{data:?}");

    assert!(rendered.contains("events_len: 2"));
    assert!(rendered.contains("first_unlock_time_ms: Some(2000)"));
    assert!(rendered.contains("last_unlock_time_ms: Some(4000)"));
    assert!(!rendered.contains(secret_address));
    assert!(!rendered.contains("987654321"));
}

#[test]
fn hype_unstaking_queue_state_debug_redacts_data_and_error() {
    let secret_address = "0xf764939b589138dd1c75601b10a408c66ee68cbe";
    let state = HypeUnstakingQueueState {
        data: Some(HypeUnstakingQueueData::new(vec![HypeUnstakingEvent {
            unlock_time_ms: 2_000,
            user: secret_address.to_string(),
            amount_wei: 987_654_321,
        }])),
        error: Some("unstaking state secret".to_string()),
        ..HypeUnstakingQueueState::default()
    };

    let rendered = format!("{state:?}");

    assert!(rendered.contains("data_events_len: Some(1)"));
    assert!(rendered.contains("error: Some(\"<redacted>\")"));
    assert!(!rendered.contains(secret_address));
    assert!(!rendered.contains("987654321"));
    assert!(!rendered.contains("unstaking state secret"));
}

#[test]
fn filtering_excludes_past_events() {
    let data = HypeUnstakingQueueData::new(vec![
        event(900, "0xpast", 100),
        event(2_000, "0xfuture", 100),
    ]);

    let filtered = data.filtered_events(HypeUnstakingFilter {
        now_ms: 1_000,
        window: HypeUnstakingWindowFilter::All,
        amount: HypeUnstakingAmountFilter::All,
        mine_address: None,
    });

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].user, "0xfuture");
}

#[test]
fn retain_upcoming_events_drops_unlocked_rows() {
    let mut data = HypeUnstakingQueueData::new(vec![
        event(900, "0xpast", 100),
        event(1_000, "0xnow", 100),
        event(2_000, "0xfuture", 100),
    ]);

    data.retain_upcoming_events(1_000);

    assert_eq!(data.events.len(), 1);
    assert_eq!(data.events[0].user, "0xfuture");
}

#[test]
fn filtering_excludes_events_past_window_end() {
    let data = HypeUnstakingQueueData::new(vec![
        event(2_000, "0xinside", 100),
        event(3_700_000, "0xlate", 100),
    ]);

    let filtered = data.filtered_events(HypeUnstakingFilter {
        now_ms: 1_000,
        window: HypeUnstakingWindowFilter::OneHour,
        amount: HypeUnstakingAmountFilter::All,
        mine_address: None,
    });

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].user, "0xinside");
}

#[test]
fn filtering_excludes_events_below_amount_floor() {
    let data = HypeUnstakingQueueData::new(vec![
        event(2_000, "0xsmall", 99),
        event(3_000, "0xbig", 100),
    ]);

    let filtered = data.filtered_events(HypeUnstakingFilter {
        now_ms: 1_000,
        window: HypeUnstakingWindowFilter::All,
        amount: HypeUnstakingAmountFilter::AtLeast100,
        mine_address: None,
    });

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].user, "0xbig");
}

#[test]
fn filtering_supports_mine_only() {
    let data = HypeUnstakingQueueData::new(vec![
        event(2_000, "0xAAA111", 1),
        event(2_000, "0xBBB222", 1),
        event(2_000, "0xAAA333", 1),
    ]);

    let mine = data.filtered_events(HypeUnstakingFilter {
        now_ms: 1_000,
        window: HypeUnstakingWindowFilter::All,
        amount: HypeUnstakingAmountFilter::All,
        mine_address: Some("0xbbb222"),
    });
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].user, "0xBBB222");
}

#[test]
fn mine_filter_ignores_ascii_case_without_trimming_or_folding_unicode() {
    let data = HypeUnstakingQueueData::new(
        ["0xAaB", "0xaab", "0xÄA", "0xäa", " 0xAaB", ""]
            .into_iter()
            .map(|user| event(2_000, user, 1))
            .collect(),
    );

    for (address, expected) in [
        ("0xAAB", vec!["0xAaB", "0xaab"]),
        ("0xÄa", vec!["0xÄA"]),
        ("0xäA", vec!["0xäa"]),
        (" 0xaAb", vec![" 0xAaB"]),
        ("0xaab ", vec![]),
        ("", vec![""]),
    ] {
        let filtered = data.filtered_events(HypeUnstakingFilter {
            now_ms: 1_000,
            window: HypeUnstakingWindowFilter::All,
            amount: HypeUnstakingAmountFilter::All,
            mine_address: Some(address),
        });
        assert_eq!(
            filtered
                .iter()
                .map(|event| event.user.as_str())
                .collect::<Vec<_>>(),
            expected,
        );
    }
}

#[test]
fn summary_aggregates_filtered_events() {
    let first = event(2_000, "0xAAA", 10);
    let second = event(3_000, "0xaaa", 25);
    let events = vec![&first, &second];

    assert_eq!(
        summarize_unstaking_events(&events),
        HypeUnstakingSummary {
            event_count: 2,
            unique_wallet_count: 1,
            total_wei: 35 * HYPE_CORE_WEI_PER_TOKEN,
            next_unlock_time_ms: Some(2_000),
            largest_amount_wei: Some(25 * HYPE_CORE_WEI_PER_TOKEN as u64),
        }
    );
}

#[test]
fn sort_change_amount_defaults_descending_and_toggles() {
    let mut state = HypeUnstakingQueueState::default();

    state.apply_sort_change(HypeUnstakingSortField::Amount);
    assert_eq!(state.sort_field, HypeUnstakingSortField::Amount);
    assert_eq!(state.sort_direction, SortDirection::Descending);

    state.apply_sort_change(HypeUnstakingSortField::Amount);
    assert_eq!(state.sort_direction, SortDirection::Ascending);
}

#[test]
fn amount_sort_orders_full_filtered_set() {
    let small = event(2_000, "0xsmall", 10);
    let large = event(4_000, "0xlarge", 1_000);
    let mid = event(3_000, "0xmid", 100);
    let mut events = vec![&small, &mid, &large];

    sort_unstaking_events(
        events.as_mut_slice(),
        HypeUnstakingSortField::Amount,
        SortDirection::Descending,
    );

    assert_eq!(
        events
            .iter()
            .map(|event| event.user.as_str())
            .collect::<Vec<_>>(),
        vec!["0xlarge", "0xmid", "0xsmall"]
    );
}

#[test]
fn formats_hype_wei_amounts() {
    assert_eq!(format_hype_wei(123_450_000_000), "1,234 HYPE");
    assert_eq!(format_hype_wei(150_000_000), "1.5 HYPE");
    assert_eq!(format_hype_wei(12_345), "0.0001 HYPE");
    assert_eq!(format_hype_wei(1), "<0.0001 HYPE");
}

#[test]
fn formats_countdowns() {
    assert_eq!(format_countdown(1_000, 1_000), "Unlocked");
    assert_eq!(format_countdown(91_000, 1_000), "1m 30s");
    assert_eq!(format_countdown(3_661_000, 1_000), "1h 1m");
}
