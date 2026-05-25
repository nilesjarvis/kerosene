use super::*;

mod binary;
mod questions;

fn outcome_meta_from_json(value: serde_json::Value) -> OutcomeMetaResponse {
    match serde_json::from_value(value) {
        Ok(response) => response,
        Err(error) => panic!("valid outcome meta fixture: {error}"),
    }
}

fn symbol_by_key_or_panic<'a>(symbols: &'a [ExchangeSymbol], key: &str) -> &'a ExchangeSymbol {
    for symbol in symbols {
        if symbol.key == key {
            return symbol;
        }
    }

    panic!("missing {key} symbol");
}

fn outcome_by_key_or_panic<'a>(symbols: &'a [ExchangeSymbol], key: &str) -> &'a OutcomeSymbolInfo {
    match symbol_by_key_or_panic(symbols, key).outcome.as_ref() {
        Some(outcome) => outcome,
        None => panic!("missing {key} outcome metadata"),
    }
}
