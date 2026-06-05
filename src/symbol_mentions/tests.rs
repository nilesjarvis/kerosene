use std::collections::HashSet;
use std::time::{Duration, Instant};

use super::*;
use crate::api::{ExchangeSymbol, MarketType};

mod curated_keywords;
mod metadata_aliases;
mod ticker_matching;

fn symbol(key: &str, ticker: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 50,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn pairs(mentions: &[SymbolMention]) -> Vec<(String, String)> {
    mentions
        .iter()
        .map(|m| (m.symbol_key.clone(), m.ticker.clone()))
        .collect()
}

#[test]
fn custom_curated_aliases_are_explainable() {
    let symbols = vec![symbol("xyz:NVDA", "NVDA", MarketType::Perp)];
    let resolver = SymbolMentionResolver::with_alias_rules(
        &symbols,
        vec![SymbolAliasRule::new("nvidia", ["xyz:NVDA"], 82)],
    );

    let mentions = resolver.resolve("nvidia headlines crossed first");

    assert_eq!(mentions.len(), 1);
    assert_eq!(mentions[0].symbol_key, "xyz:NVDA");
    assert_eq!(mentions[0].matched_text, "nvidia");
    assert_eq!(mentions[0].source, SymbolAliasSource::CuratedKeyword);
    assert_eq!(mentions[0].confidence, 82);
}

#[test]
fn ticker_source_beats_curated_alias_for_same_ticker_conflict() {
    let symbols = vec![
        symbol("xyz:NVDA", "NVDA", MarketType::Perp),
        symbol("flx:NVDA", "NVDA", MarketType::Perp),
    ];
    let resolver = SymbolMentionResolver::with_alias_rules(
        &symbols,
        vec![SymbolAliasRule::new("nvidia", ["flx:NVDA"], 82)],
    );

    let mentions = resolver.resolve("NVDA and nvidia both moved");

    assert_eq!(
        pairs(&mentions),
        vec![("xyz:NVDA".to_string(), "NVDA".to_string())]
    );
    assert_eq!(mentions[0].source, SymbolAliasSource::Ticker);
}

#[test]
fn synthetic_generated_corpus_has_high_precision_and_recall() {
    let symbols = synthetic_symbols(500);
    let rules = synthetic_alias_rules(1_000, 500);
    let resolver = SymbolMentionResolver::with_alias_rules(&symbols, rules);
    let corpus = synthetic_labeled_messages(2_000, 500, 1_000);

    let mut true_positive = 0usize;
    let mut false_positive = 0usize;
    let mut false_negative = 0usize;

    for (message, expected) in corpus {
        let expected = expected.into_iter().collect::<HashSet<_>>();
        let actual = resolver
            .resolve(&message)
            .into_iter()
            .map(|mention| mention.symbol_key)
            .collect::<HashSet<_>>();

        true_positive += actual.intersection(&expected).count();
        false_positive += actual.difference(&expected).count();
        false_negative += expected.difference(&actual).count();
    }

    let recall = true_positive as f64 / (true_positive + false_negative).max(1) as f64;
    assert_eq!(false_positive, 0);
    assert!(recall >= 0.99, "synthetic recall was {recall:.3}");
}

#[test]
#[ignore]
fn synthetic_resolver_benchmark() {
    let symbols = synthetic_symbols(5_000);
    let rules = synthetic_alias_rules(10_000, 5_000);

    let build_started = Instant::now();
    let resolver = SymbolMentionResolver::with_alias_rules(&symbols, rules);
    let build_elapsed = build_started.elapsed();

    let messages = synthetic_labeled_messages(10_000, 5_000, 10_000)
        .into_iter()
        .map(|(message, _)| message)
        .collect::<Vec<_>>();

    let mut total_mentions = 0usize;
    let mut durations = Vec::with_capacity(messages.len());
    for message in &messages {
        let started = Instant::now();
        total_mentions += resolver.resolve(message).len();
        durations.push(started.elapsed());
    }

    durations.sort_unstable();
    let p50 = percentile_duration(&durations, 50);
    let p95 = percentile_duration(&durations, 95);
    let p99 = percentile_duration(&durations, 99);

    println!(
        "symbol mention resolver synthetic benchmark: build={:?}, messages={}, aliases={}, mentions={}, p50={:?}, p95={:?}, p99={:?}",
        build_elapsed,
        messages.len(),
        resolver.aliases.len(),
        total_mentions,
        p50,
        p95,
        p99
    );

    assert!(total_mentions > 0);
    assert!(build_elapsed < Duration::from_millis(500));
    assert!(p95 < Duration::from_micros(500));
}

fn synthetic_symbols(count: usize) -> Vec<ExchangeSymbol> {
    (0..count)
        .map(|index| {
            let ticker = format!("S{index:04}");
            symbol(&ticker, &ticker, MarketType::Perp)
        })
        .collect()
}

fn synthetic_alias_rules(count: usize, symbol_count: usize) -> Vec<SymbolAliasRule> {
    (0..count)
        .map(|index| {
            let phrase = synthetic_alias_phrase(index);
            let symbol_index = index % symbol_count;
            SymbolAliasRule::new(phrase, [format!("S{symbol_index:04}")], 78)
        })
        .collect()
}

fn synthetic_labeled_messages(
    count: usize,
    symbol_count: usize,
    alias_count: usize,
) -> Vec<(String, Vec<String>)> {
    (0..count)
        .map(|index| match index % 5 {
            0 => {
                let symbol_index = index % symbol_count;
                (
                    format!("desk saw S{symbol_index:04} flow into the close"),
                    vec![format!("S{symbol_index:04}")],
                )
            }
            1 => {
                let symbol_index = (index * 13) % symbol_count;
                (
                    format!("watching $s{symbol_index:04} after the headline"),
                    vec![format!("S{symbol_index:04}")],
                )
            }
            2 => {
                let alias_index = (index * 17) % alias_count;
                let symbol_index = alias_index % symbol_count;
                (
                    format!(
                        "macro wire flags {} demand",
                        synthetic_alias_phrase(alias_index)
                    ),
                    vec![format!("S{symbol_index:04}")],
                )
            }
            3 => {
                let symbol_index = (index * 19) % symbol_count;
                (
                    format!("substring trap S{symbol_index:04}abc should stay quiet"),
                    Vec::new(),
                )
            }
            _ => {
                let alias_index = (index * 23) % alias_count;
                (
                    format!(
                        "phrase trap {}ish should stay quiet",
                        synthetic_alias_phrase(alias_index)
                    ),
                    Vec::new(),
                )
            }
        })
        .collect()
}

fn synthetic_alias_phrase(index: usize) -> String {
    let first = ((index / 26) % 26) as u8;
    let second = (index % 26) as u8;
    let third = ((index / (26 * 26)) % 26) as u8;
    let fourth = ((index / (26 * 26 * 26)) % 26) as u8;
    format!(
        "{}{}{}{} catalyst",
        char::from(b'a' + first),
        char::from(b'a' + second),
        char::from(b'a' + third),
        char::from(b'a' + fourth)
    )
}

fn percentile_duration(durations: &[Duration], percentile: usize) -> Duration {
    if durations.is_empty() {
        return Duration::ZERO;
    }
    let index = ((durations.len() - 1) * percentile) / 100;
    durations[index]
}
