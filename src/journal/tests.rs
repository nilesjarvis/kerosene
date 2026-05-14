use super::*;
use crate::api::UserFill;

fn fill(time: u64, tid: u64, coin: &str) -> UserFill {
    UserFill {
        coin: coin.to_string(),
        px: "100.0".to_string(),
        sz: "1.0".to_string(),
        side: "B".to_string(),
        time,
        start_position: "0.0".to_string(),
        dir: "Open Long".to_string(),
        closed_pnl: "0.0".to_string(),
        hash: format!("0x{time:x}{tid:x}"),
        oid: tid + 100,
        crossed: false,
        fee: "0.01".to_string(),
        tid,
        fee_token: "USDC".to_string(),
    }
}

fn wallet_hype_fill(
    time: u64,
    tid: u64,
    side: &str,
    dir: &str,
    sz: &str,
    start_position: &str,
    closed_pnl: &str,
) -> UserFill {
    UserFill {
        coin: "HYPE".to_string(),
        px: "40.0".to_string(),
        sz: sz.to_string(),
        side: side.to_string(),
        time,
        start_position: start_position.to_string(),
        dir: dir.to_string(),
        closed_pnl: closed_pnl.to_string(),
        hash: format!("0x{tid:x}"),
        oid: 422_000_000_000,
        crossed: false,
        fee: "0.0".to_string(),
        tid,
        fee_token: "USDC".to_string(),
    }
}

fn assert_approx_eq(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-6,
        "expected {actual} to be within 1e-6 of {expected}"
    );
}

fn note(open: &str) -> JournalNote {
    JournalNote {
        open: open.to_string(),
        close: String::new(),
    }
}

#[test]
fn aggregate_trades_chains_same_timestamp_open_fills_by_position() {
    let time = 1_778_497_097_655;
    let fills = vec![
        wallet_hype_fill(
            time,
            1_055_837_673_236_715,
            "B",
            "Open Long",
            "36.14",
            "0.0",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            679_973_859_119_944,
            "B",
            "Open Long",
            "36.14",
            "36.14",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            92_397_404_714_723,
            "B",
            "Open Long",
            "36.14",
            "72.28",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            848_850_867_302_117,
            "B",
            "Open Long",
            "36.14",
            "108.42",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            608_117_862_803_864,
            "B",
            "Open Long",
            "36.14",
            "144.56",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            218_418_431_393_666,
            "B",
            "Open Long",
            "60.3",
            "180.7",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            59_376_519_919_481,
            "B",
            "Open Long",
            "36.14",
            "241.0",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            646_991_547_874_007,
            "B",
            "Open Long",
            "60.3",
            "277.14",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            1_075_325_548_837_610,
            "B",
            "Open Long",
            "60.3",
            "337.44",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            177_456_786_002_999,
            "B",
            "Open Long",
            "24.26",
            "397.74",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            323_525_496_356_845,
            "B",
            "Open Long",
            "36.14",
            "422.0",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            840_459_077_386_888,
            "B",
            "Open Long",
            "41.86",
            "458.14",
            "0.0",
        ),
    ];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.diagnostics.incomplete_trade_count, 0);
    assert_eq!(result.diagnostics.same_timestamp_position_mismatch_count, 0);
    assert_eq!(result.trades.len(), 1);
    let trade = &result.trades[0];
    assert_eq!(trade.status, "OPEN");
    assert!(trade.is_long);
    assert!(trade.basis_complete);
    assert_eq!(trade.fill_count, 12);
    assert_approx_eq(trade.max_position, 500.0);
    assert_approx_eq(trade.total_entry_size, 500.0);
}

#[test]
fn aggregate_trades_keeps_same_timestamp_close_fills_in_the_long_trade() {
    let open_time = 1_778_596_387_586;
    let close_time = 1_778_596_428_000;
    let fills = vec![
        wallet_hype_fill(
            open_time,
            21_400_535_966_404,
            "B",
            "Open Long",
            "1.64",
            "0.0",
            "0.0",
        ),
        wallet_hype_fill(
            open_time + 488,
            232_957_291_404_586,
            "B",
            "Open Long",
            "45.57",
            "1.64",
            "0.0",
        ),
        wallet_hype_fill(
            open_time + 8_540,
            296_712_036_058_506,
            "B",
            "Open Long",
            "452.79",
            "47.21",
            "0.0",
        ),
        wallet_hype_fill(
            close_time - 127,
            420_936_527_112_987,
            "A",
            "Close Long",
            "143.29",
            "500.0",
            "7.30779",
        ),
        wallet_hype_fill(
            close_time,
            364_046_121_164_912,
            "A",
            "Close Long",
            "332.43",
            "356.71",
            "16.95393",
        ),
        wallet_hype_fill(
            close_time,
            276_590_854_497_898,
            "A",
            "Close Long",
            "24.28",
            "24.28",
            "1.23828",
        ),
    ];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.diagnostics.incomplete_trade_count, 0);
    assert_eq!(result.trades.len(), 1);
    let trade = &result.trades[0];
    assert_eq!(trade.status, "CLOSED");
    assert!(trade.is_long);
    assert_eq!(trade.fill_count, 6);
    assert_approx_eq(trade.max_position, 500.0);
    assert_approx_eq(trade.pnl, 25.5);
}

#[test]
fn aggregate_trades_handles_reported_wallet_thirty_three_fill_close_bucket() {
    let close_time = 1_778_503_051_463;
    let close_rows = [
        (908_300_817_329_865, "227.3", "2471.93", "28.93529"),
        (616_127_449_699_225, "5.0", "2244.63", "0.6365"),
        (1_093_329_152_899_926, "36.16", "2239.63", "4.603168"),
        (1_080_945_918_225_223, "19.28", "2203.47", "2.454344"),
        (380_264_142_713_787, "24.08", "2184.19", "3.065384"),
        (45_823_892_378_610, "234.1", "2160.11", "28.86453"),
        (919_339_708_192_467, "17.36", "1926.01", "2.140488"),
        (656_953_175_720_042, "234.1", "1908.65", "27.92813"),
        (397_410_151_264_712, "96.45", "1674.55", "11.506485"),
        (783_647_603_834_063, "100.0", "1578.1", "11.83"),
        (505_762_492_994_420, "17.84", "1478.1", "2.092632"),
        (287_014_690_204_610, "100.0", "1460.26", "11.63"),
        (494_825_769_601_317, "86.79", "1360.26", "10.093677"),
        (727_625_033_095_630, "9.45", "1273.47", "1.089585"),
        (1_008_630_988_473_460, "10.38", "1264.02", "1.196814"),
        (581_254_541_952_460, "85.7", "1253.64", "9.79551"),
        (239_539_886_137_101, "100.0", "1167.94", "11.33"),
        (657_198_484_783_125, "100.0", "1067.94", "11.23"),
        (53_929_361_443_268, "69.64", "967.94", "7.820572"),
        (1_102_174_838_215_259, "12.66", "898.3", "1.409058"),
        (996_477_617_255_075, "48.15", "885.64", "5.310945"),
        (1_059_289_184_432_831, "12.89", "837.49", "1.408877"),
        (1_079_340_279_676_958, "249.95", "824.6", "27.319535"),
        (760_307_975_650_069, "41.77", "574.65", "4.565461"),
        (365_144_445_817_364, "46.81", "532.88", "5.116333"),
        (897_077_113_268_940, "17.1", "486.07", "1.85193"),
        (355_207_080_438_879, "45.55", "468.97", "4.887515"),
        (508_457_872_493_395, "9.66", "423.42", "1.036518"),
        (35_043_472_071_941, "18.64", "413.76", "2.000072"),
        (1_070_499_754_425_229, "62.5", "395.12", "6.64375"),
        (751_696_379_692_816, "18.9", "332.62", "2.00907"),
        (1_015_027_868_078_023, "12.05", "313.72", "1.280915"),
        (1_077_680_846_967_968, "301.67", "301.67", "31.765851"),
    ];
    let mut fills = vec![wallet_hype_fill(
        close_time - 1,
        1,
        "B",
        "Open Long",
        "2471.93",
        "0.0",
        "0.0",
    )];
    fills.extend(
        close_rows
            .into_iter()
            .map(|(tid, sz, start_position, pnl)| {
                wallet_hype_fill(close_time, tid, "A", "Close Long", sz, start_position, pnl)
            }),
    );

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.diagnostics.incomplete_trade_count, 0);
    assert_eq!(result.diagnostics.same_timestamp_position_mismatch_count, 0);
    assert_eq!(result.trades.len(), 1);
    let trade = &result.trades[0];
    assert_eq!(trade.status, "CLOSED");
    assert!(trade.is_long);
    assert_eq!(trade.fill_count, 34);
    assert_approx_eq(trade.max_position, 2471.93);
}

#[test]
fn normalize_fills_sorts_and_deduplicates_by_composite_identity() {
    let duplicate = fill(3, 30, "ETH");
    let mut fills = vec![
        duplicate.clone(),
        fill(1, 10, "BTC"),
        duplicate,
        fill(2, 20, "SOL"),
    ];

    normalize_fills(&mut fills);

    assert_eq!(fills.len(), 3);
    assert_eq!(fills[0].time, 1);
    assert_eq!(fills[1].time, 2);
    assert_eq!(fills[2].time, 3);
}

#[test]
fn merge_fills_uses_composite_identity_not_tid_only() {
    let mut existing = vec![fill(1, 10, "BTC")];
    let mut same_tid_different_fill = fill(2, 10, "ETH");
    same_tid_different_fill.hash = "0xdifferent".to_string();

    let added = merge_fills(
        &mut existing,
        vec![fill(1, 10, "BTC"), same_tid_different_fill],
    );

    assert_eq!(added, 1);
    assert_eq!(existing.len(), 2);
    assert_eq!(newest_fill_time(&existing), Some(2));
}

#[test]
fn aggregate_trades_skips_malformed_numeric_fills() {
    let mut malformed = fill(1, 10, "BTC");
    malformed.sz = "not-a-number".to_string();

    let result = aggregate_trades_with_diagnostics(vec![malformed]);

    assert!(result.trades.is_empty());
    assert_eq!(result.diagnostics.skipped_fill_count, 1);
}

#[test]
fn aggregate_trades_marks_missing_opening_basis_as_partial() {
    let mut close = fill(1, 10, "BTC");
    close.side = "A".to_string();
    close.start_position = "1.0".to_string();
    close.dir = "Close Long".to_string();
    close.closed_pnl = "10.0".to_string();

    let result = aggregate_trades_with_diagnostics(vec![close]);

    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.diagnostics.incomplete_trade_count, 1);
    assert!(!result.trades[0].basis_complete);
    assert_eq!(result.trades[0].pnl, 10.0);
}

#[test]
fn note_lookup_keeps_legacy_time_based_keys_working() {
    let result = aggregate_trades_with_diagnostics(vec![fill(1, 10, "BTC")]);
    let trade = &result.trades[0];
    let legacy_key = "BTC_1".to_string();
    let mut entries = HashMap::new();
    entries.insert(
        legacy_key.clone(),
        JournalNote {
            open: "legacy note".to_string(),
            close: String::new(),
        },
    );

    assert_ne!(trade.id, legacy_key);
    assert_eq!(note_key_for_trade(&entries, trade), Some(legacy_key));
    assert_eq!(
        note_for_trade(&entries, trade).map(|note| note.open.as_str()),
        Some("legacy note")
    );
}

#[test]
fn journal_state_migrates_legacy_notes_to_active_account() {
    let mut legacy_entries = HashMap::new();
    legacy_entries.insert("BTC_1".to_string(), note("legacy"));

    let state = JournalState::new_for_account(
        Some("account-a".to_string()),
        HashMap::new(),
        legacy_entries,
    );

    assert_eq!(
        state.entries.get("BTC_1").map(|entry| entry.open.as_str()),
        Some("legacy")
    );
    assert_eq!(
        state
            .account_states
            .get("account-a")
            .and_then(|account| account.entries.get("BTC_1"))
            .map(|entry| entry.open.as_str()),
        Some("legacy")
    );
}

#[test]
fn journal_state_switches_entries_by_account() {
    let mut account_entries = HashMap::new();
    account_entries.insert(
        "account-a".to_string(),
        HashMap::from([("a".to_string(), note("a"))]),
    );
    account_entries.insert(
        "account-b".to_string(),
        HashMap::from([("b".to_string(), note("b"))]),
    );

    let mut state = JournalState::new_for_account(
        Some("account-a".to_string()),
        account_entries,
        HashMap::new(),
    );
    state.entries.insert("a2".to_string(), note("a2"));

    state.switch_active_account(Some("account-b".to_string()));
    assert!(state.entries.contains_key("b"));
    assert!(!state.entries.contains_key("a"));
    state.entries.insert("b2".to_string(), note("b2"));

    state.switch_active_account(Some("account-a".to_string()));
    assert!(state.entries.contains_key("a"));
    assert!(state.entries.contains_key("a2"));
    assert!(!state.entries.contains_key("b2"));

    state.switch_active_account(Some("account-b".to_string()));
    assert!(state.entries.contains_key("b2"));
}

#[test]
fn journal_entries_snapshot_includes_current_active_entries() {
    let mut state = JournalState::new_for_account(
        Some("account-a".to_string()),
        HashMap::new(),
        HashMap::new(),
    );
    state.entries.insert("active".to_string(), note("active"));

    let snapshot = state.entries_by_account_snapshot();

    assert_eq!(
        snapshot
            .get("account-a")
            .and_then(|entries| entries.get("active"))
            .map(|entry| entry.open.as_str()),
        Some("active")
    );
}
