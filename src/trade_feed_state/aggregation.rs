use super::{TradeFeedInstance, TradePrint};
use crate::denomination::format_compact;
use crate::helpers::{parse_positive_finite_number, trim_decimal_zeros};
use std::borrow::Cow;
use std::collections::HashMap;

const GROUP_WINDOW_MS: u64 = 1_000;

pub(crate) struct TradeFeedRow<'a> {
    pub(crate) latest: &'a TradePrint,
    pub(crate) count: usize,
    pub(crate) value: f64,
    size: f64,
    size_decimals: usize,
    price_decimals: usize,
}

impl<'a> TradeFeedRow<'a> {
    fn new(trade: &'a TradePrint) -> Self {
        Self {
            latest: trade,
            count: 1,
            value: trade.value,
            size: parse_positive_finite_number(&trade.size).unwrap_or_default(),
            size_decimals: decimal_places(&trade.size),
            price_decimals: decimal_places(&trade.price),
        }
    }

    fn try_add(&mut self, trade: &TradePrint) -> bool {
        // Anchor to the newest execution, so adjacent gaps cannot chain forever.
        if self.latest.time_ms.abs_diff(trade.time_ms) > GROUP_WINDOW_MS
            || self.size <= 0.0
            || self.latest.intent != trade.intent
        {
            return false;
        }
        let Some(size) = parse_positive_finite_number(&trade.size) else {
            return false;
        };
        let size = self.size + size;
        let value = self.value + trade.value;
        if !size.is_finite() || !value.is_finite() || !(value / size).is_finite() {
            return false;
        }
        self.size = size;
        self.value = value;
        self.count += 1;
        self.size_decimals = self.size_decimals.max(decimal_places(&trade.size));
        self.price_decimals = self.price_decimals.max(decimal_places(&trade.price));
        true
    }

    pub(crate) fn price_label(&self) -> Cow<'a, str> {
        if self.count == 1 {
            Cow::Borrowed(&self.latest.price)
        } else {
            Cow::Owned(format_number(
                self.value / self.size,
                (self.price_decimals + 2).min(18),
            ))
        }
    }

    pub(crate) fn size_label(&self) -> Cow<'a, str> {
        if self.size >= 1_000.0 {
            Cow::Owned(format_compact(self.size))
        } else if self.count == 1 {
            Cow::Borrowed(&self.latest.size)
        } else {
            Cow::Owned(format_number(self.size, self.size_decimals))
        }
    }
}

impl TradeFeedInstance {
    /// Derive groups from retained prints, so switching modes never discards data.
    pub(crate) fn visible_rows(&self) -> Vec<TradeFeedRow<'_>> {
        let mut rows: Vec<TradeFeedRow<'_>> = Vec::new();
        let mut latest_by_trader = HashMap::new();
        for trade in self.visible_trades() {
            let key = self
                .group_trades
                .then_some(trade.trader.as_ref())
                .flatten()
                .map(|address| (address.as_str(), trade.is_buy));
            if let Some(key) = key {
                if let Some(&index) = latest_by_trader.get(&key)
                    && rows
                        .get_mut(index)
                        .is_some_and(|row: &mut TradeFeedRow<'_>| row.try_add(trade))
                {
                    continue;
                }
                latest_by_trader.insert(key, rows.len());
            }
            rows.push(TradeFeedRow::new(trade));
        }
        // Filter only after grouping, so smaller constituent fills can combine
        // into a qualifying group. Raw prints remain available when cleared.
        if self.group_trades && self.min_group_notional > 0.0 {
            rows.retain(|row| row.value >= self.min_group_notional);
        }
        rows
    }
}

fn decimal_places(value: &str) -> usize {
    let (mantissa, exponent) = value.split_once(['e', 'E']).unwrap_or((value, "0"));
    let fractional = mantissa
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    (fractional as i32)
        .saturating_sub(exponent.parse::<i32>().unwrap_or_default())
        .clamp(0, 18) as usize
}

fn format_number(value: f64, decimals: usize) -> String {
    let formatted = trim_decimal_zeros(format!("{value:.decimals$}"));
    if formatted == "0" && value > 0.0 {
        value.to_string()
    } else {
        formatted
    }
}

#[cfg(test)]
mod tests;
