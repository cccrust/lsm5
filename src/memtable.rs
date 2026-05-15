use std::collections::BTreeMap;

/// A value in the MemTable — either a live value or a tombstone (deletion marker).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Data(Vec<u8>),
    Tombstone,
}

/// The in-memory write buffer backed by a BTreeMap for sorted iteration.
/// When `size_bytes` exceeds the configured threshold it is flushed to an SSTable.
pub struct MemTable {
    map: BTreeMap<Vec<u8>, Value>,
    size_bytes: usize,
}

impl MemTable {
    pub fn new() -> Self {
        MemTable {
            map: BTreeMap::new(),
            size_bytes: 0,
        }
    }

    /// Insert or update a key.
    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let delta = key.len() + value.len();
        if let Some(old) = self.map.insert(key, Value::Data(value)) {
            // Subtract the old value size so size stays accurate
            match old {
                Value::Data(v) => self.size_bytes = self.size_bytes.saturating_sub(v.len()),
                Value::Tombstone => {}
            }
        }
        self.size_bytes += delta;
    }

    /// Mark a key as deleted (tombstone).
    pub fn delete(&mut self, key: Vec<u8>) {
        let key_len = key.len();
        if let Some(old) = self.map.insert(key, Value::Tombstone) {
            match old {
                Value::Data(v) => self.size_bytes = self.size_bytes.saturating_sub(v.len()),
                Value::Tombstone => return,
            }
        }
        self.size_bytes += key_len + 1; // tombstone marker overhead
    }

    /// Get the value for a key. Returns None if not present in this MemTable.
    pub fn get(&self, key: &[u8]) -> Option<&Value> {
        self.map.get(key)
    }

    pub fn size_bytes(&self) -> usize {
        self.size_bytes
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Iterate entries in sorted key order.
    pub fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &Value)> {
        self.map.iter()
    }

    /// Drain all entries for flushing. Resets the MemTable.
    pub fn drain_sorted(&mut self) -> Vec<(Vec<u8>, Value)> {
        let entries: Vec<_> = self
            .map
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.map.clear();
        self.size_bytes = 0;
        entries
    }
}

impl Default for MemTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_get() {
        let mut m = MemTable::new();
        m.put(b"hello".to_vec(), b"world".to_vec());
        assert_eq!(m.get(b"hello"), Some(&Value::Data(b"world".to_vec())));
    }

    #[test]
    fn test_delete_tombstone() {
        let mut m = MemTable::new();
        m.put(b"k".to_vec(), b"v".to_vec());
        m.delete(b"k".to_vec());
        assert_eq!(m.get(b"k"), Some(&Value::Tombstone));
    }

    #[test]
    fn test_sorted_iteration() {
        let mut m = MemTable::new();
        m.put(b"c".to_vec(), b"3".to_vec());
        m.put(b"a".to_vec(), b"1".to_vec());
        m.put(b"b".to_vec(), b"2".to_vec());
        let keys: Vec<_> = m.iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(keys, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn test_overwrite() {
        let mut m = MemTable::new();
        m.put(b"key".to_vec(), b"value1".to_vec());
        m.put(b"key".to_vec(), b"value2".to_vec());
        assert_eq!(m.get(b"key"), Some(&Value::Data(b"value2".to_vec())));
    }

    #[test]
    fn test_delete_nonexistent() {
        let mut m = MemTable::new();
        m.delete(b"missing".to_vec());
        assert_eq!(m.get(b"missing"), Some(&Value::Tombstone));
    }

    #[test]
    fn test_empty_memtable() {
        let m = MemTable::new();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
        assert_eq!(m.get(b"any"), None);
    }

    #[test]
    fn test_drain_sorted() {
        let mut m = MemTable::new();
        m.put(b"b".to_vec(), b"2".to_vec());
        m.put(b"a".to_vec(), b"1".to_vec());
        let entries = m.drain_sorted();
        assert!(m.is_empty());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, b"a".to_vec());
    }

    #[test]
    fn test_size_tracking() {
        let mut m = MemTable::new();
        m.put(b"k".to_vec(), b"v".to_vec());
        let size_with_value = m.size_bytes();
        m.put(b"k".to_vec(), b"vvvv".to_vec());
        assert!(m.size_bytes() > size_with_value);
    }

    #[test]
    fn test_binary_keys() {
        let mut m = MemTable::new();
        let key = vec![0x00, 0x01, 0xFF];
        m.put(key.clone(), vec![0xAA, 0xBB]);
        assert_eq!(m.get(&key), Some(&Value::Data(vec![0xAA, 0xBB])));
    }
}
