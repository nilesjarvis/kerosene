use crate::api::{ExchangeSymbol, OutcomeVolume24h};
use crate::message::Message;
use iced::widget::text;
use iced::{Element, Theme};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

pub(super) fn view_outcome_volume(
    volume: Option<f64>,
    loading: bool,
    theme: &Theme,
) -> Option<Element<'static, Message>> {
    let label = match volume {
        Some(volume) => Cow::Owned(format!(
            "24h Vol {}",
            format_outcome_contract_volume(volume)
        )),
        None if loading => Cow::Borrowed("24h Vol ..."),
        None => return None,
    };
    Some(
        text(label)
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(theme.extended_palette().background.weak.text)
            .into(),
    )
}

pub(super) fn outcome_market_set_volume(
    outcomes: &BTreeMap<u32, Vec<&ExchangeSymbol>>,
    volumes: &HashMap<String, OutcomeVolume24h>,
) -> Option<f64> {
    let mut total = 0.0;
    let mut found = false;
    for sides in outcomes.values() {
        if let Some(volume) = outcome_group_volume(sides, volumes) {
            total += volume;
            found = true;
        }
    }
    found.then_some(total)
}

pub(super) fn outcome_group_volume(
    sides: &[&ExchangeSymbol],
    volumes: &HashMap<String, OutcomeVolume24h>,
) -> Option<f64> {
    sides
        .iter()
        .filter_map(|symbol| volumes.get(&symbol.key).map(|volume| volume.contract))
        .filter(|volume| volume.is_finite() && *volume >= 0.0)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

fn format_outcome_contract_volume(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1_000_000_000.0 {
        format!("{:.1}B contracts", value / 1_000_000_000.0)
    } else if abs >= 1_000_000.0 {
        format!("{:.1}M contracts", value / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.1}K contracts", value / 1_000.0)
    } else if abs >= 1.0 {
        format!("{value:.0} contracts")
    } else {
        format!("{value:.2} contracts")
    }
}

#[cfg(test)]
mod tests;
