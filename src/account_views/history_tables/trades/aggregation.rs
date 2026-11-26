use crate::account::UserFill;
use crate::helpers::{finite_value, parse_finite_number, parse_positive_finite_number};
use std::cmp::Reverse;
use std::collections::HashMap;

const ROW_LIMIT: usize = 50;

pub(super) fn fee_currency(token: Option<&str>) -> &str {
    match token.map(str::trim) {
        Some(token) if !token.is_empty() && !token.eq_ignore_ascii_case("USDC") => token,
        _ => "USDC",
    }
}

pub(super) struct HistoryTrade<'a> {
    pub first: &'a UserFill,
    partials: Vec<&'a UserFill>,
}

pub(super) struct TradeAmounts {
    pub price: Option<f64>,
    pub size: Option<f64>,
    pub pnl: Option<f64>,
    pub fee: Option<f64>,
}

impl<'a> HistoryTrade<'a> {
    fn single(first: &'a UserFill) -> Self {
        Self {
            first,
            partials: Vec::new(),
        }
    }

    pub fn count(&self) -> usize {
        1 + self.partials.len()
    }

    pub fn direction(&self) -> &'a str {
        if self.partials.iter().all(|fill| fill.dir == self.first.dir) {
            &self.first.dir
        } else {
            "Mixed"
        }
    }

    pub fn amounts(&self) -> TradeAmounts {
        let first = self.first;
        if self.partials.is_empty() {
            return TradeAmounts {
                price: parse_finite_number(&first.px),
                size: parse_finite_number(&first.sz),
                pnl: parse_finite_number(&first.closed_pnl),
                fee: parse_finite_number(&first.fee),
            };
        }

        let sum = |field: fn(&UserFill) -> Option<f64>| {
            std::iter::once(first)
                .chain(self.partials.iter().copied())
                .map(field)
                .sum::<Option<f64>>()
                .and_then(finite_value)
        };
        let size = sum(|f| parse_positive_finite_number(&f.sz));
        let notional =
            sum(|f| Some(parse_finite_number(&f.px)? * parse_positive_finite_number(&f.sz)?));
        TradeAmounts {
            price: notional.zip(size).and_then(|(n, s)| finite_value(n / s)),
            size,
            pnl: sum(|f| parse_finite_number(&f.closed_pnl)),
            fee: sum(|f| parse_finite_number(&f.fee)),
        }
    }
}

pub(super) fn trade_history_rows<'a>(
    fills: impl Iterator<Item = &'a UserFill>,
    aggregate: bool,
) -> Vec<HistoryTrade<'a>> {
    if !aggregate {
        return fills.take(ROW_LIMIT).map(HistoryTrade::single).collect();
    }

    let mut fills: Vec<_> = fills.collect();
    fills.sort_by_key(|fill| Reverse(fill.time));
    let mut rows: Vec<HistoryTrade<'a>> = Vec::new();
    // Fill time is the exchange block timestamp. A resting order can execute in many blocks.
    for block in fills.chunk_by(|a, b| a.time == b.time) {
        if rows.len() == ROW_LIMIT {
            break;
        }
        let mut orders = HashMap::<_, usize>::new();
        for &fill in block {
            let key = fill
                .oid
                .filter(|&oid| oid != 0 && fill.time != 0)
                .map(|oid| {
                    (
                        oid,
                        fill.coin.as_str(),
                        fill.side.as_str(),
                        fee_currency(fill.fee_token.as_deref()),
                    )
                });
            if let Some(index) = key.and_then(|key| orders.get(&key).copied()) {
                rows[index].partials.push(fill);
            } else if rows.len() < ROW_LIMIT {
                if let Some(key) = key {
                    orders.insert(key, rows.len());
                }
                rows.push(HistoryTrade::single(fill));
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests;
