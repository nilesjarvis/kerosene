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
