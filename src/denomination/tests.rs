use super::*;

mod display;
mod fallback;
mod invalid_inputs;

fn eur_context(rate: f64) -> DisplayDenominationContext {
    DisplayDenominationContext::from_mids(
        DisplayDenominationConfig::eur(),
        &HashMap::from([("xyz:EUR".to_string(), rate)]),
        &HashMap::from([("xyz:EUR".to_string(), 1_000)]),
        1_000,
    )
}

fn hype_context(rate: f64) -> DisplayDenominationContext {
    DisplayDenominationContext::from_mids(
        DisplayDenominationConfig::hype(),
        &HashMap::from([("HYPE".to_string(), rate)]),
        &HashMap::from([("HYPE".to_string(), 1_000)]),
        1_000,
    )
}

fn btc_context(rate: f64) -> DisplayDenominationContext {
    DisplayDenominationContext::from_mids(
        DisplayDenominationConfig::btc(),
        &HashMap::from([("BTC".to_string(), rate)]),
        &HashMap::from([("BTC".to_string(), 1_000)]),
        1_000,
    )
}
