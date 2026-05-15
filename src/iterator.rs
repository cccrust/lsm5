use crate::memtable::Value;

pub struct LsmIterator {
    items: Vec<(Vec<u8>, Vec<u8>)>,
    pos: usize,
}

impl LsmIterator {
    pub fn new(items: Vec<(Vec<u8>, Vec<u8>)>) -> Self {
        Self { items, pos: 0 }
    }

    pub fn valid(&self) -> bool {
        self.pos < self.items.len()
    }

    pub fn next(&mut self) {
        if self.pos < self.items.len() {
            self.pos += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    pub fn key(&self) -> Option<&[u8]> {
        self.items.get(self.pos).map(|(k, _)| k.as_slice())
    }

    pub fn value(&self) -> Option<&[u8]> {
        self.items.get(self.pos).map(|(_, v)| v.as_slice())
    }

    pub fn seek_to_first(&mut self) {
        self.pos = 0;
    }

    pub fn seek_to_last(&mut self) {
        self.pos = self.items.len().saturating_sub(1);
    }

    pub fn seek(&mut self, target: &[u8]) {
        self.pos = self
            .items
            .binary_search_by(|(k, _)| k.as_slice().cmp(target))
            .unwrap_or_else(|p| p);
    }

    pub fn seek_for_prev(&mut self, target: &[u8]) {
        self.pos = self
            .items
            .binary_search_by(|(k, _)| k.as_slice().cmp(target))
            .map_or(0, |p| p.saturating_sub(1));
    }
}

impl Iterator for LsmIterator {
    type Item = (Vec<u8>, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.items.len() {
            let item = self.items[self.pos].clone();
            self.pos += 1;
            Some(item)
        } else {
            None
        }
    }
}

impl DoubleEndedIterator for LsmIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.pos < self.items.len() {
            let item = self.items.pop()?;
            Some(item)
        } else {
            None
        }
    }
}

pub fn filter_tombstones(items: Vec<(Vec<u8>, Value)>) -> Vec<(Vec<u8>, Vec<u8>)> {
    items
        .into_iter()
        .filter_map(|(k, v)| match v {
            Value::Data(data) => Some((k, data)),
            Value::Tombstone => None,
        })
        .collect()
}
