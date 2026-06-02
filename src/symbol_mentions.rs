use crate::api::{ExchangeSymbol, MarketType};
use crate::helpers::compare_symbol_keys_for_same_ticker;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

const STRONG_TICKER_WORDS: &[&str] = &[
    "A", "AI", "AM", "AN", "AND", "ARE", "AS", "AT", "BE", "BY", "CEO", "CFO", "CTO", "DO", "FOR",
    "GO", "HAS", "HE", "IN", "IS", "IT", "ME", "NEW", "NO", "NOT", "OF", "ON", "OR", "S", "SEC",
    "SHE", "THE", "TO", "TRUMP", "UP", "US", "USD", "USDC", "USDT", "WE", "YES",
];
const AMBIGUOUS_BARE_TICKER_WORDS: &[&str] = &["APT", "FLOW", "LINK", "MOVE", "NEAR"];
const DEFAULT_OIL_SYMBOL_KEYS: &[&str] = &["xyz:BRENTOIL", "xyz:WTIOIL"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SymbolAliasSource {
    Ticker,
    Key,
    KeySuffix,
    DisplayName,
    Keyword,
    CuratedKeyword,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SymbolMention {
    pub(crate) symbol_key: String,
    pub(crate) ticker: String,
    pub(crate) matched_text: String,
    pub(crate) source: SymbolAliasSource,
    pub(crate) confidence: u8,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SymbolAliasRule {
    phrase: String,
    symbol_keys: Vec<String>,
    confidence: u8,
}

impl SymbolAliasRule {
    pub(crate) fn new<I, S>(phrase: impl Into<String>, symbol_keys: I, confidence: u8) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            phrase: phrase.into(),
            symbol_keys: symbol_keys.into_iter().map(Into::into).collect(),
            confidence,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SymbolMentionResolver {
    aliases: Vec<SymbolAlias>,
    single_char_aliases: HashMap<u8, Vec<usize>>,
    prefix_aliases: HashMap<[u8; 2], Vec<usize>>,
}

impl SymbolMentionResolver {
    pub(crate) fn empty() -> Self {
        Self::from_aliases(Vec::new())
    }

    pub(crate) fn from_symbols(symbols: &[ExchangeSymbol]) -> Self {
        Self::with_alias_rules(symbols, default_curated_symbol_alias_rules())
    }

    pub(crate) fn with_alias_rules(
        symbols: &[ExchangeSymbol],
        alias_rules: Vec<SymbolAliasRule>,
    ) -> Self {
        let mut aliases = Vec::new();
        let mut seen = HashSet::new();
        let symbol_lookup = symbols
            .iter()
            .filter(|symbol| symbol.is_user_selectable_market())
            .filter(|symbol| symbol.market_type != MarketType::Spot)
            .filter(|symbol| !symbol.ticker.trim().is_empty())
            .map(|symbol| (symbol.key.as_str(), symbol))
            .collect::<HashMap<_, _>>();

        for symbol in symbol_lookup.values() {
            push_symbol_aliases(&mut aliases, &mut seen, symbol);
        }

        for rule in alias_rules {
            let phrase = rule.phrase.trim();
            if phrase.is_empty() {
                continue;
            }
            for symbol_key in &rule.symbol_keys {
                let Some(symbol) = symbol_lookup.get(symbol_key.as_str()) else {
                    continue;
                };
                push_alias(
                    &mut aliases,
                    &mut seen,
                    phrase,
                    symbol,
                    SymbolAliasSource::CuratedKeyword,
                    MatchPolicy::Phrase,
                    rule.confidence,
                );
            }
        }

        Self::from_aliases(aliases)
    }

    pub(crate) fn resolve(&self, text: &str) -> Vec<SymbolMention> {
        if text.trim().is_empty() || self.aliases.is_empty() {
            return Vec::new();
        }

        let uppercase_text = text.to_ascii_uppercase();
        let mut candidates = Vec::new();
        let bytes = uppercase_text.as_bytes();

        for (index, ch) in uppercase_text.char_indices() {
            if !ch.is_ascii() {
                continue;
            }
            let first = bytes[index];
            if let Some(alias_indices) = self.single_char_aliases.get(&first) {
                self.push_alias_matches_at(
                    text,
                    &uppercase_text,
                    index,
                    alias_indices,
                    &mut candidates,
                );
            }
            if let Some(second) = bytes.get(index + 1) {
                if let Some(alias_indices) = self.prefix_aliases.get(&[first, *second]) {
                    self.push_alias_matches_at(
                        text,
                        &uppercase_text,
                        index,
                        alias_indices,
                        &mut candidates,
                    );
                }
            }
        }

        dedupe_and_sort_mentions(candidates)
    }

    fn from_aliases(aliases: Vec<SymbolAlias>) -> Self {
        let mut single_char_aliases: HashMap<u8, Vec<usize>> = HashMap::new();
        let mut prefix_aliases: HashMap<[u8; 2], Vec<usize>> = HashMap::new();
        for (index, alias) in aliases.iter().enumerate() {
            let bytes = alias.phrase_upper.as_bytes();
            match bytes {
                [first] => single_char_aliases.entry(*first).or_default().push(index),
                [first, second, ..] => prefix_aliases
                    .entry([*first, *second])
                    .or_default()
                    .push(index),
                [] => {}
            }
        }

        Self {
            aliases,
            single_char_aliases,
            prefix_aliases,
        }
    }

    fn push_alias_matches_at(
        &self,
        text: &str,
        uppercase_text: &str,
        index: usize,
        alias_indices: &[usize],
        candidates: &mut Vec<CandidateMention>,
    ) {
        for alias_index in alias_indices {
            let alias = &self.aliases[*alias_index];
            if let Some(candidate) = alias_match_at(text, uppercase_text, index, alias) {
                candidates.push(candidate);
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn resolve_symbol_mentions(
    text: &str,
    symbols: &[ExchangeSymbol],
) -> Vec<SymbolMention> {
    SymbolMentionResolver::from_symbols(symbols).resolve(text)
}

fn default_curated_symbol_alias_rules() -> Vec<SymbolAliasRule> {
    vec![
        SymbolAliasRule::new("crude oil", DEFAULT_OIL_SYMBOL_KEYS.iter().copied(), 80),
        SymbolAliasRule::new("brent crude", ["xyz:BRENTOIL"], 80),
        SymbolAliasRule::new("wti crude", ["xyz:WTIOIL"], 80),
        SymbolAliasRule::new("iran", DEFAULT_OIL_SYMBOL_KEYS.iter().copied(), 75),
        SymbolAliasRule::new("iranian", DEFAULT_OIL_SYMBOL_KEYS.iter().copied(), 75),
        SymbolAliasRule::new("hormuz", DEFAULT_OIL_SYMBOL_KEYS.iter().copied(), 75),
    ]
}

#[derive(Debug, Clone)]
struct SymbolAlias {
    phrase_upper: String,
    symbol_key: String,
    ticker: String,
    market_type: MarketType,
    source: SymbolAliasSource,
    match_policy: MatchPolicy,
    confidence: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchPolicy {
    Ticker,
    Phrase,
}

#[derive(Debug, Clone)]
struct CandidateMention {
    mention: SymbolMention,
    market_type: MarketType,
}

fn push_symbol_aliases(
    aliases: &mut Vec<SymbolAlias>,
    seen: &mut HashSet<(String, String)>,
    symbol: &ExchangeSymbol,
) {
    push_alias(
        aliases,
        seen,
        &symbol.ticker,
        symbol,
        SymbolAliasSource::Ticker,
        MatchPolicy::Ticker,
        100,
    );

    for split in ['/', '-'] {
        if let Some((base, _)) = symbol.ticker.split_once(split) {
            push_alias(
                aliases,
                seen,
                base,
                symbol,
                SymbolAliasSource::Ticker,
                MatchPolicy::Ticker,
                100,
            );
        }
    }

    if let Some((_, suffix)) = symbol.key.split_once(':') {
        push_alias(
            aliases,
            seen,
            suffix,
            symbol,
            SymbolAliasSource::KeySuffix,
            MatchPolicy::Ticker,
            90,
        );
        if let Some(stripped) = suffix.strip_prefix('U') {
            push_alias(
                aliases,
                seen,
                stripped,
                symbol,
                SymbolAliasSource::KeySuffix,
                MatchPolicy::Ticker,
                90,
            );
        }
    } else if !symbol.key.starts_with('@') {
        push_alias(
            aliases,
            seen,
            &symbol.key,
            symbol,
            SymbolAliasSource::Key,
            MatchPolicy::Ticker,
            90,
        );
    }

    if let Some(display_name) = symbol.display_name.as_deref()
        && metadata_alias_is_safe(display_name)
    {
        push_alias(
            aliases,
            seen,
            display_name,
            symbol,
            SymbolAliasSource::DisplayName,
            MatchPolicy::Phrase,
            78,
        );
    }

    for keyword in &symbol.keywords {
        if metadata_alias_is_safe(keyword) {
            push_alias(
                aliases,
                seen,
                keyword,
                symbol,
                SymbolAliasSource::Keyword,
                MatchPolicy::Phrase,
                74,
            );
        }
    }
}

fn push_alias(
    aliases: &mut Vec<SymbolAlias>,
    seen: &mut HashSet<(String, String)>,
    phrase: &str,
    symbol: &ExchangeSymbol,
    source: SymbolAliasSource,
    match_policy: MatchPolicy,
    confidence: u8,
) {
    let phrase = phrase.trim();
    if phrase.is_empty() {
        return;
    }

    let phrase_upper = phrase.to_ascii_uppercase();
    if !seen.insert((phrase_upper.clone(), symbol.key.clone())) {
        return;
    }

    aliases.push(SymbolAlias {
        phrase_upper,
        symbol_key: symbol.key.clone(),
        ticker: symbol.ticker.trim().to_string(),
        market_type: symbol.market_type,
        source,
        match_policy,
        confidence,
    });
}

fn alias_match_at(
    text: &str,
    uppercase_text: &str,
    index: usize,
    alias: &SymbolAlias,
) -> Option<CandidateMention> {
    if !uppercase_text[index..].starts_with(&alias.phrase_upper) {
        return None;
    }

    let end = index + alias.phrase_upper.len();
    let before = text[..index].chars().next_back();
    let after = text[end..].chars().next();
    if !symbol_mention_boundary(before) || !symbol_mention_boundary(after) {
        return None;
    }

    let prefixed = before == Some('$') || before == Some('#');
    let original = &text[index..end];
    if alias.match_policy == MatchPolicy::Ticker
        && ticker_requires_strong_match(&alias.phrase_upper)
        && !prefixed
        && !original_match_is_strong(original)
    {
        return None;
    }

    Some(CandidateMention {
        mention: SymbolMention {
            symbol_key: alias.symbol_key.clone(),
            ticker: alias.ticker.clone(),
            matched_text: original.to_string(),
            source: alias.source,
            confidence: alias.confidence,
            start: index,
            end,
        },
        market_type: alias.market_type,
    })
}

fn dedupe_and_sort_mentions(candidates: Vec<CandidateMention>) -> Vec<SymbolMention> {
    let mut by_symbol: HashMap<String, CandidateMention> = HashMap::new();
    for candidate in candidates {
        match by_symbol.get_mut(&candidate.mention.symbol_key) {
            Some(existing) => {
                if compare_same_symbol_candidate(&candidate, existing) == Ordering::Less {
                    *existing = candidate;
                }
            }
            None => {
                by_symbol.insert(candidate.mention.symbol_key.clone(), candidate);
            }
        }
    }

    let mut candidates = by_symbol.into_values().collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.mention
            .ticker
            .to_ascii_uppercase()
            .cmp(&right.mention.ticker.to_ascii_uppercase())
            .then_with(|| compare_same_ticker_candidate(left, right))
    });
    candidates.dedup_by(|left, right| {
        left.mention
            .ticker
            .eq_ignore_ascii_case(&right.mention.ticker)
    });
    candidates.sort_by(|left, right| {
        left.mention
            .start
            .cmp(&right.mention.start)
            .then_with(|| left.mention.ticker.cmp(&right.mention.ticker))
            .then_with(|| left.mention.symbol_key.cmp(&right.mention.symbol_key))
    });

    candidates
        .into_iter()
        .map(|candidate| candidate.mention)
        .collect()
}

fn compare_same_symbol_candidate(left: &CandidateMention, right: &CandidateMention) -> Ordering {
    source_rank(left.mention.source)
        .cmp(&source_rank(right.mention.source))
        .then_with(|| right.mention.confidence.cmp(&left.mention.confidence))
        .then_with(|| left.mention.start.cmp(&right.mention.start))
        .then_with(|| left.mention.end.cmp(&right.mention.end))
}

fn compare_same_ticker_candidate(left: &CandidateMention, right: &CandidateMention) -> Ordering {
    market_type_rank(left.market_type)
        .cmp(&market_type_rank(right.market_type))
        .then_with(|| source_rank(left.mention.source).cmp(&source_rank(right.mention.source)))
        .then_with(|| right.mention.confidence.cmp(&left.mention.confidence))
        .then_with(|| {
            compare_symbol_keys_for_same_ticker(&left.mention.symbol_key, &right.mention.symbol_key)
        })
        .then_with(|| left.mention.start.cmp(&right.mention.start))
}

fn source_rank(source: SymbolAliasSource) -> u8 {
    match source {
        SymbolAliasSource::Ticker => 0,
        SymbolAliasSource::Key | SymbolAliasSource::KeySuffix => 1,
        SymbolAliasSource::DisplayName => 2,
        SymbolAliasSource::Keyword => 3,
        SymbolAliasSource::CuratedKeyword => 4,
    }
}

fn ticker_requires_strong_match(candidate: &str) -> bool {
    let alphanumeric_len = candidate
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .count();
    alphanumeric_len <= 2
        || STRONG_TICKER_WORDS.contains(&candidate)
        || AMBIGUOUS_BARE_TICKER_WORDS.contains(&candidate)
}

fn metadata_alias_is_safe(phrase: &str) -> bool {
    let phrase = phrase.trim();
    if phrase.is_empty() {
        return false;
    }

    let alphanumeric_len = phrase
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .count();
    if alphanumeric_len < 4 {
        return false;
    }

    let upper = phrase.to_ascii_uppercase();
    !STRONG_TICKER_WORDS.contains(&upper.as_str())
        && !AMBIGUOUS_BARE_TICKER_WORDS.contains(&upper.as_str())
}

fn original_match_is_strong(original: &str) -> bool {
    original.chars().any(|ch| ch.is_ascii_alphabetic())
        && original
            .chars()
            .all(|ch| !ch.is_ascii_alphabetic() || ch.is_ascii_uppercase())
}

fn symbol_mention_boundary(ch: Option<char>) -> bool {
    ch.is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
}

fn market_type_rank(market_type: MarketType) -> u8 {
    match market_type {
        MarketType::Perp => 0,
        MarketType::Spot => 1,
        MarketType::Outcome => 2,
    }
}

#[cfg(test)]
mod tests;
