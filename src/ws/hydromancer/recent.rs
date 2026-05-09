use std::collections::{HashSet, VecDeque};

pub(super) struct RecentHydromancerKeys {
    keys: VecDeque<String>,
    set: HashSet<String>,
    capacity: usize,
}

impl RecentHydromancerKeys {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            keys: VecDeque::with_capacity(capacity),
            set: HashSet::with_capacity(capacity),
            capacity,
        }
    }

    pub(super) fn insert_new(&mut self, key: String) -> bool {
        if !self.set.insert(key.clone()) {
            return false;
        }

        self.keys.push_back(key);
        while self.keys.len() > self.capacity {
            if let Some(old) = self.keys.pop_front() {
                self.set.remove(&old);
            }
        }

        true
    }
}
