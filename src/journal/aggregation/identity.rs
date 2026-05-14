use crate::api::UserFill;

use std::cmp::Ordering;
use std::collections::HashSet;

const POSITION_CHAIN_EPSILON: f64 = 1e-6;

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

#[derive(Debug, Clone, Copy)]
struct FillPositionEdge {
    start: f64,
    end: f64,
}

pub fn compare_fills(a: &UserFill, b: &UserFill) -> Ordering {
    a.time
        .cmp(&b.time)
        .then(a.coin.cmp(&b.coin))
        .then(a.tid.cmp(&b.tid))
        .then(a.oid.cmp(&b.oid))
        .then(a.hash.cmp(&b.hash))
        .then(a.side.cmp(&b.side))
        .then(a.px.cmp(&b.px))
        .then(a.sz.cmp(&b.sz))
}

pub fn normalize_fills(fills: &mut Vec<UserFill>) {
    fills.sort_by(compare_fills);

    let mut seen = HashSet::with_capacity(fills.len());
    fills.retain(|fill| seen.insert(FillIdentity::from(fill)));
    order_same_timestamp_position_chains(fills);
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

fn order_same_timestamp_position_chains(fills: &mut Vec<UserFill>) {
    if fills.len() < 2 {
        return;
    }

    let mut ordered = Vec::with_capacity(fills.len());
    let mut start = 0;

    while start < fills.len() {
        let time = fills[start].time;
        let coin = fills[start].coin.clone();
        let mut end = start + 1;

        while end < fills.len() && fills[end].time == time && fills[end].coin == coin {
            end += 1;
        }

        if end - start > 1 {
            ordered.extend(position_chain_order(&fills[start..end]));
        } else {
            ordered.push(fills[start].clone());
        }

        start = end;
    }

    *fills = ordered;
}

fn position_chain_order(group: &[UserFill]) -> Vec<UserFill> {
    let Some(edges): Option<Vec<_>> = group.iter().map(fill_position_edge).collect() else {
        return group.to_vec();
    };

    let mut used = vec![false; group.len()];
    let mut ordered_indices = Vec::with_capacity(group.len());
    let heads: Vec<usize> = edges
        .iter()
        .enumerate()
        .filter_map(|(index, edge)| {
            let has_predecessor = edges.iter().enumerate().any(|(other_index, other)| {
                other_index != index && positions_match(other.end, edge.start)
            });
            (!has_predecessor).then_some(index)
        })
        .collect();

    if heads.is_empty() {
        return group.to_vec();
    }

    for head in heads {
        follow_position_chain(head, &edges, &mut used, &mut ordered_indices);
    }

    for index in 0..group.len() {
        if !used[index] {
            follow_position_chain(index, &edges, &mut used, &mut ordered_indices);
        }
    }

    ordered_indices
        .into_iter()
        .map(|index| group[index].clone())
        .collect()
}

fn follow_position_chain(
    start: usize,
    edges: &[FillPositionEdge],
    used: &mut [bool],
    ordered_indices: &mut Vec<usize>,
) {
    let mut current = Some(start);

    while let Some(index) = current {
        if used[index] {
            break;
        }

        used[index] = true;
        ordered_indices.push(index);
        let target = edges[index].end;
        current = edges.iter().enumerate().find_map(|(next_index, edge)| {
            (!used[next_index] && positions_match(edge.start, target)).then_some(next_index)
        });
    }
}

fn fill_position_edge(fill: &UserFill) -> Option<FillPositionEdge> {
    let start = fill.start_position.parse::<f64>().ok()?;
    let size = fill.sz.parse::<f64>().ok()?;
    if !start.is_finite() || !size.is_finite() {
        return None;
    }

    let signed_size = if fill.side == "A" { -size } else { size };
    let end = if fill.dir == "Settlement" {
        start
    } else {
        start + signed_size
    };
    if !end.is_finite() {
        return None;
    }

    Some(FillPositionEdge { start, end })
}

fn positions_match(a: f64, b: f64) -> bool {
    (a - b).abs() <= POSITION_CHAIN_EPSILON
}
