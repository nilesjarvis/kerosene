use super::*;

#[test]
fn mid_candidates_cover_encoded_dex_and_u_prefixed_forms() {
    let mut terminal = TradingTerminal::boot().0;

    // Plain ticker.
    assert!(
        terminal
            .mid_candidates_for_symbol("BTC")
            .contains(&"BTC".to_string())
    );

    // A '+' encoded key also probes the '#' form used by the mids map.
    let plus = terminal.mid_candidates_for_symbol("+660");
    assert!(plus.contains(&"+660".to_string()));
    assert!(plus.contains(&"#660".to_string()));

    // A HIP-3 'dex:Usuffix' key also probes the U-stripped suffix.
    let dex = terminal.mid_candidates_for_symbol("xyz:UFOO");
    assert!(dex.contains(&"xyz:UFOO".to_string()));
    assert!(dex.contains(&"xyz:FOO".to_string()));

    // A leading 'U' on a bare symbol also probes the stripped form.
    let u_prefixed = terminal.mid_candidates_for_symbol("UBTC");
    assert!(u_prefixed.contains(&"UBTC".to_string()));
    assert!(u_prefixed.contains(&"BTC".to_string()));

    // Exchange-symbol-derived forms add key/ticker/'U'+ticker without duplicates.
    terminal.exchange_symbols = vec![symbol("ETH", MarketType::Perp)];
    let derived = terminal.mid_candidates_for_symbol("ETH");
    assert!(derived.contains(&"ETH".to_string()));
    assert!(derived.contains(&"UETH".to_string()));
    let eth_count = derived.iter().filter(|c| c.as_str() == "ETH").count();
    assert_eq!(
        eth_count, 1,
        "candidates must be de-duplicated: {derived:?}"
    );
}

#[test]
fn legacy_indexed_key_probes_the_api_named_spot_pair_mid() {
    let mut terminal = TradingTerminal::boot().0;
    let mut purr = symbol("PURR/USDC", MarketType::Spot);
    purr.ticker = "PURR".to_string();
    purr.asset_index = 10_000;
    terminal.exchange_symbols = vec![purr];

    // "PURR/USDC" is the actual allMids key; the legacy "@0" key must still
    // resolve it through the spot asset index alias.
    let candidates = terminal.mid_candidates_for_symbol("@0");
    assert!(candidates.contains(&"@0".to_string()));
    assert!(candidates.contains(&"PURR/USDC".to_string()));
    assert!(candidates.contains(&"PURR".to_string()));

    terminal.all_mids.insert("PURR/USDC".to_string(), 4.2);
    terminal
        .all_mids_updated_at_ms
        .insert("PURR/USDC".to_string(), TradingTerminal::now_ms());
    assert_eq!(terminal.resolve_mid_for_symbol("@0"), Some(4.2));
    assert_eq!(terminal.resolve_mid_for_symbol("PURR/USDC"), Some(4.2));
}
