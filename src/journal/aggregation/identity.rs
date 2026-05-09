use crate::api::UserFill;

use std::cmp::Ordering;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FillIdentity {
    pub time: u64,
    pub tid: u64,
    pub oid: u64,
    pub hash: String,
    pub coin: String,
    pub side: String,
    pub px: String,
    pub sz: String,
}

impl From<&UserFill> for FillIdentity {
    fn from(fill: &UserFill) -> Self {
        Self {
            time: fill.time,
            tid: fill.tid,
            oid: fill.oid,
            hash: fill.hash.clone(),
            coin: fill.coin.clone(),
            side: fill.side.clone(),
            px: fill.px.clone(),
            sz: fill.sz.clone(),
        }
    }
}

pub fn compare_fills(a: &UserFill, b: &UserFill) -> Ordering {
    a.time
        .cmp(&b.time)
        .then(a.tid.cmp(&b.tid))
        .then(a.oid.cmp(&b.oid))
        .then(a.hash.cmp(&b.hash))
        .then(a.coin.cmp(&b.coin))
        .then(a.side.cmp(&b.side))
        .then(a.px.cmp(&b.px))
        .then(a.sz.cmp(&b.sz))
}

pub fn normalize_fills(fills: &mut Vec<UserFill>) {
    fills.sort_by(compare_fills);

    let mut seen = HashSet::with_capacity(fills.len());
    fills.retain(|fill| seen.insert(FillIdentity::from(fill)));
}

pub fn merge_fills(existing: &mut Vec<UserFill>, new_fills: Vec<UserFill>) -> usize {
    let mut seen: HashSet<FillIdentity> = existing.iter().map(FillIdentity::from).collect();
    let mut added = 0;

    for fill in new_fills {
        if seen.insert(FillIdentity::from(&fill)) {
            existing.push(fill);
            added += 1;
        }
    }

    normalize_fills(existing);
    added
}

pub fn newest_fill_time(fills: &[UserFill]) -> Option<u64> {
    fills.iter().map(|fill| fill.time).max()
}
