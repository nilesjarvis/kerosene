use super::{MAX_ORDERS, PAGE_SIZE, Page, RestingOrder, SCALE, Selection};
use std::collections::{BTreeMap, HashMap, HashSet};

pub(crate) struct Snapshot {
    pub(crate) height: u64,
    pub(crate) timestamp_ms: u64,
    pub(crate) orders: Vec<RestingOrder>,
}

pub(crate) struct Block {
    pub(crate) height: u64,
    pub(crate) timestamp_ms: u64,
    pub(crate) diffs: Vec<Diff>,
}

pub(crate) enum Diff {
    New(RestingOrder, Option<u64>),
    Update(u64, u64),
    Remove(u64),
}

struct Node {
    order: RestingOrder,
    previous: Option<u64>,
    next: Option<u64>,
}

#[derive(Default)]
struct Level {
    head: Option<u64>,
    tail: Option<u64>,
}

/// Price-time queues with O(1) order deletion/priority insertion. New orders
/// match the opposite side before any remainder rests, as required by L4.
#[derive(Default)]
pub(crate) struct Book {
    orders: HashMap<u64, Node>,
    bids: BTreeMap<u64, Level>,
    asks: BTreeMap<u64, Level>,
    height: u64,
    timestamp_ms: u64,
}

impl Book {
    pub(crate) fn from_snapshot(snapshot: Snapshot) -> Result<Self, String> {
        let mut book = Self {
            height: snapshot.height,
            timestamp_ms: snapshot.timestamp_ms,
            ..Self::default()
        };
        for order in snapshot.orders {
            if book.orders.contains_key(&order.oid) {
                return Err("Duplicate order in L4 snapshot".into());
            }
            // Snapshot rows already describe resting queues. Never match away
            // inconsistent snapshot rows and accidentally hide a bad baseline.
            book.insert(order, None)?;
        }
        if book
            .bids
            .last_key_value()
            .zip(book.asks.first_key_value())
            .is_some_and(|((bid, _), (ask, _))| bid >= ask)
        {
            return Err("Crossed L4 snapshot; resynchronizing".into());
        }
        Ok(book)
    }

    pub(crate) fn height(&self) -> u64 {
        self.height
    }

    pub(crate) fn apply(&mut self, block: Block) -> Result<(), String> {
        if block.height <= self.height {
            return Ok(());
        }
        if block.height != self.height.saturating_add(1) {
            return Err("L4 stream gap; resynchronizing".into());
        }
        if block.timestamp_ms < self.timestamp_ms {
            return Err("L4 timestamp moved backwards; resynchronizing".into());
        }
        for diff in block.diffs {
            match diff {
                Diff::New(mut order, before) => {
                    if self.orders.contains_key(&order.oid) {
                        continue;
                    }
                    self.match_order(&mut order);
                    if order.size > 0 {
                        self.insert(order, before)?;
                    }
                }
                Diff::Update(oid, 0) | Diff::Remove(oid) => {
                    self.remove(oid);
                }
                Diff::Update(oid, size) => {
                    // Matching may already have fully consumed this order.
                    if let Some(node) = self.orders.get_mut(&oid) {
                        node.order.size = size;
                    }
                }
            }
        }
        self.height = block.height;
        self.timestamp_ms = block.timestamp_ms;
        Ok(())
    }

    fn match_order(&mut self, order: &mut RestingOrder) {
        while order.size > 0 {
            let maker = if order.is_bid {
                self.asks
                    .first_key_value()
                    .filter(|(px, _)| **px <= order.price)
            } else {
                self.bids
                    .last_key_value()
                    .filter(|(px, _)| **px >= order.price)
            }
            .and_then(|(_, level)| level.head);
            let Some(oid) = maker else {
                break;
            };
            let Some(node) = self.orders.get_mut(&oid) else {
                break;
            };
            let matched = order.size.min(node.order.size);
            order.size -= matched;
            node.order.size -= matched;
            if node.order.size == 0 {
                self.remove(oid);
            }
        }
    }

    fn insert(&mut self, order: RestingOrder, before: Option<u64>) -> Result<(), String> {
        if self.orders.len() >= MAX_ORDERS {
            return Err("L4 book exceeds the supported order limit".into());
        }
        let before = before.filter(|oid| {
            self.orders.get(oid).is_some_and(|node| {
                node.order.is_bid == order.is_bid && node.order.price == order.price
            })
        });
        let levels = if order.is_bid {
            &mut self.bids
        } else {
            &mut self.asks
        };
        let level = levels.entry(order.price).or_default();
        let previous = before
            .and_then(|oid| self.orders.get(&oid).and_then(|n| n.previous))
            .or_else(|| if before.is_none() { level.tail } else { None });
        let oid = order.oid;
        if let Some(node) = previous.and_then(|id| self.orders.get_mut(&id)) {
            node.next = Some(oid);
        } else {
            level.head = Some(oid);
        }
        if let Some(node) = before.and_then(|id| self.orders.get_mut(&id)) {
            node.previous = Some(oid);
        } else {
            level.tail = Some(oid);
        }
        self.orders.insert(
            oid,
            Node {
                order,
                previous,
                next: before,
            },
        );
        Ok(())
    }

    fn remove(&mut self, oid: u64) {
        let Some(node) = self.orders.remove(&oid) else {
            return;
        };
        if let Some(previous) = node.previous.and_then(|id| self.orders.get_mut(&id)) {
            previous.next = node.next;
        }
        if let Some(next) = node.next.and_then(|id| self.orders.get_mut(&id)) {
            next.previous = node.previous;
        }
        let levels = if node.order.is_bid {
            &mut self.bids
        } else {
            &mut self.asks
        };
        if let Some(level) = levels.get_mut(&node.order.price) {
            if level.head == Some(oid) {
                level.head = node.next;
            }
            if level.tail == Some(oid) {
                level.tail = node.previous;
            }
            if level.head.is_none() {
                levels.remove(&node.order.price);
            }
        }
    }

    pub(crate) fn page(&self, selection: Selection) -> Page {
        // Restrict work to nearby price levels; a deep market may have hundreds
        // of thousands of orders while the selected block contains only a few.
        let lower = ((selection.price - selection.tick).max(0.0) * SCALE as f64).floor() as u64;
        let upper = ((selection.price + selection.tick) * SCALE as f64).ceil() as u64;
        let mut orders = Vec::new();
        for levels in [&self.bids, &self.asks] {
            for (_, level) in levels.range(lower..=upper) {
                let mut current = level.head;
                while let Some(node) = current.and_then(|oid| self.orders.get(&oid)) {
                    if selection.contains(&node.order) {
                        orders.push(&node.order);
                    }
                    current = node.next;
                }
            }
        }
        // Wallet grouping and stable order IDs keep rows readable as sizes change.
        orders.sort_unstable_by(|a, b| {
            a.wallet
                .as_str()
                .cmp(b.wallet.as_str())
                .then(a.price.cmp(&b.price))
                .then(a.oid.cmp(&b.oid))
        });
        let total_orders = orders.len();
        let wallets = orders
            .iter()
            .map(|order| order.wallet.as_str())
            .collect::<HashSet<_>>()
            .len();
        let total_size = orders
            .iter()
            .map(|order| order.size as f64 / SCALE as f64)
            .sum();
        let notional = orders
            .iter()
            .map(|order| (order.size as f64 / SCALE as f64) * (order.price as f64 / SCALE as f64))
            .sum();
        let page = selection
            .page
            .min(total_orders.saturating_sub(1) / PAGE_SIZE);
        let orders = orders
            .into_iter()
            .skip(page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .cloned()
            .collect();
        Page {
            revision: selection.revision,
            height: self.height,
            timestamp_ms: self.timestamp_ms,
            orders,
            total_orders,
            wallets,
            total_size,
            notional,
            page,
        }
    }
}
