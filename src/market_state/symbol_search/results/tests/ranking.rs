use super::*;

fn search(
    symbols: &[ExchangeSymbol],
    query: &str,
    sort_mode: SymbolSearchSortMode,
    favourites: &[&str],
    contexts: &HashMap<String, WatchlistContext>,
) -> (Vec<usize>, usize) {
    filtered_symbol_search_indices(SymbolSearchResultsInput {
        symbols,
        query,
        sort_mode,
        market_filter: SymbolSearchMarketFilter::All,
        hip3_dex_filter: None,
        favourite_symbols: &favourites
            .iter()
            .map(|key| (*key).to_string())
            .collect::<Vec<_>>(),
        contexts,
        is_muted: |_| false,
    })
}

#[test]
fn first_favourite_position_wins_and_equal_favourites_keep_input_order() {
    let symbols = [
        symbol("ETH", "ETH", MarketType::Perp),
        symbol("BTC", "BTC", MarketType::Perp),
        symbol("ETH", "AAA", MarketType::Perp),
        symbol("ZED", "ZED", MarketType::Perp),
        symbol("#missing", "MISSING", MarketType::Outcome),
    ];
    let contexts = HashMap::from([
        ("ETH".to_string(), context(1.0)),
        ("BTC".to_string(), context(2.0)),
        ("ZED".to_string(), context(3.0)),
    ]);
    for mode in SymbolSearchSortMode::ALL {
        assert_eq!(
            search(
                &symbols,
                "",
                mode,
                &["missing", "ETH", "BTC", "ETH", "#missing"],
                &contexts
            ),
            (vec![0, 2, 1, 3], 3),
            "{mode:?}"
        );
    }
}

#[test]
fn relevance_prefers_exact_then_prefix_then_other_matching_fields() {
    let mut symbols = [
        symbol("CATEGORY", "CATEGORY", MarketType::Perp),
        symbol("PREFIX", "FOOBAR", MarketType::Perp),
        symbol("EXACT", "FOO", MarketType::Perp),
        symbol("SUBSTRING", "MIDFOOX", MarketType::Perp),
        symbol("DISPLAY", "DISPLAY", MarketType::Perp),
        symbol("FOO", "KEY", MarketType::Perp),
        symbol("KEYWORD", "KEYWORD", MarketType::Perp),
    ];
    symbols[0].category = "Foo".into();
    symbols[4].display_name = Some("fOo".into());
    symbols[6].keywords = vec!["FOO".into()];
    for mode in SymbolSearchSortMode::ALL {
        let expected = if mode == SymbolSearchSortMode::Alphabetical {
            vec![0, 4, 2, 1, 5, 6, 3]
        } else {
            vec![4, 2, 5, 1, 0, 6, 3]
        };
        assert_eq!(
            search(&symbols, "FoO", mode, &[], &HashMap::new()),
            (expected, 0),
            "{mode:?}"
        );
    }
}

#[test]
fn exchange_groups_use_lexical_dex_names_and_other_modes_keep_primary_dex_ties() {
    let symbols = [
        symbol("flx:AAA", "AAA", MarketType::Perp),
        symbol("xyz:FOO", "FOO", MarketType::Perp),
        symbol("@3", "FOO", MarketType::Spot),
        symbol("FOO", "FOO", MarketType::Perp),
        symbol("AAA", "AAA", MarketType::Perp),
        symbol("aaa:FOO", "FOO", MarketType::Perp),
        symbol("flx:FOO", "FOO", MarketType::Perp),
    ];
    for mode in SymbolSearchSortMode::ALL {
        let expected = if mode == SymbolSearchSortMode::Exchange {
            vec![4, 3, 2, 5, 0, 6, 1]
        } else {
            vec![4, 0, 3, 1, 6, 5, 2]
        };
        assert_eq!(
            search(&symbols, "", mode, &[], &HashMap::new()),
            (expected, 0),
            "{mode:?}"
        );
    }
}

#[test]
fn volume_order_keeps_finite_negatives_zero_ties_and_stable_duplicate_rows() {
    let symbols = [
        "BTC", "ETH", "SOL", "NEG", "ZERO", "POS", "HIGH", "SAME", "NONE", "SAME",
    ]
    .map(|key| symbol(key, key, MarketType::Perp));
    let contexts = [
        ("BTC", f64::NAN),
        ("ETH", f64::INFINITY),
        ("SOL", f64::NEG_INFINITY),
        ("NEG", -10.0),
        ("ZERO", -0.0),
        ("POS", 0.0),
        ("HIGH", 30.0),
        ("SAME", 30.0),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), context(value)))
    .collect();
    assert_eq!(
        search(
            &symbols,
            "",
            SymbolSearchSortMode::Volume24h,
            &[],
            &contexts
        ),
        (vec![6, 7, 9, 5, 4, 3, 0, 1, 8, 2], 0)
    );
}

#[test]
fn relevance_keeps_unicode_lowercasing_and_does_not_trim_the_query() {
    let mut symbols = [
        symbol("EXACT", "Ω", MarketType::Perp),
        symbol("PREFIX", "ΩMEGA", MarketType::Perp),
        symbol("SUBSTRING", "XΩ", MarketType::Perp),
        symbol("CATEGORY", "AAA", MarketType::Perp),
        symbol("KEYWORD", "BBB", MarketType::Perp),
    ];
    symbols[3].category = "Ω_CATEGORY".into();
    symbols[4].keywords = vec!["Ω_KEYWORD".into()];
    assert_eq!(
        search(
            &symbols,
            "Ω",
            SymbolSearchSortMode::Relevance,
            &[],
            &HashMap::new()
        ),
        (vec![0, 1, 3, 4, 2], 0)
    );
    assert_eq!(
        search(
            &symbols,
            " Ω ",
            SymbolSearchSortMode::Relevance,
            &[],
            &HashMap::new()
        ),
        (Vec::new(), 0)
    );
}
