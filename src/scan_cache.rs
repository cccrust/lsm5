use std::collections::VecDeque;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ScanKey {
    pub start: Vec<u8>,
    pub end: Vec<u8>,
}

pub struct ScanCache {
    capacity: usize,
    cache: VecDeque<(ScanKey, Vec<u64>)>,
}

impl ScanCache {
    pub fn new(capacity: usize) -> Self {
        ScanCache {
            capacity,
            cache: VecDeque::new(),
        }
    }

    pub fn get(&self, key: &ScanKey) -> Option<Vec<u64>> {
        self.cache
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, sstables)| sstables.clone())
    }

    pub fn put(&mut self, key: ScanKey, sstables: Vec<u64>) {
        self.cache.retain(|(k, _)| k != &key);

        while self.cache.len() >= self.capacity {
            self.cache.pop_back();
        }

        self.cache.push_front((key, sstables));
    }

    pub fn invalidate_for_sstable(&mut self, seq: u64) {
        self.cache.retain(|(_, sstables)| !sstables.contains(&seq));
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_cache_basic() {
        let mut cache = ScanCache::new(3);
        let key = ScanKey {
            start: b"a".to_vec(),
            end: b"z".to_vec(),
        };
        cache.put(key.clone(), vec![1, 2, 3]);
        assert_eq!(cache.get(&key), Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_scan_cache_eviction() {
        let mut cache = ScanCache::new(2);
        cache.put(
            ScanKey {
                start: b"a".to_vec(),
                end: b"b".to_vec(),
            },
            vec![1],
        );
        cache.put(
            ScanKey {
                start: b"c".to_vec(),
                end: b"d".to_vec(),
            },
            vec![2],
        );
        cache.put(
            ScanKey {
                start: b"e".to_vec(),
                end: b"f".to_vec(),
            },
            vec![3],
        );

        assert_eq!(
            cache.get(&ScanKey {
                start: b"a".to_vec(),
                end: b"b".to_vec()
            }),
            None
        );
    }

    #[test]
    fn test_scan_cache_invalidation() {
        let mut cache = ScanCache::new(3);
        cache.put(
            ScanKey {
                start: b"a".to_vec(),
                end: b"z".to_vec(),
            },
            vec![1, 2],
        );
        cache.invalidate_for_sstable(1);
        assert_eq!(
            cache.get(&ScanKey {
                start: b"a".to_vec(),
                end: b"z".to_vec()
            }),
            None
        );
    }
}
