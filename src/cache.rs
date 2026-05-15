use std::collections::HashMap;

pub struct LruCache<K: std::hash::Hash + Eq + Clone, V: Clone> {
    capacity: usize,
    map: HashMap<K, (V, usize)>,
    order: Vec<K>,
    total_size: usize,
    hits: u64,
    misses: u64,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        LruCache {
            capacity,
            map: HashMap::new(),
            order: Vec::new(),
            total_size: 0,
            hits: 0,
            misses: 0,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        match self.map.get(key) {
            Some((value, _size)) => {
                self.hits += 1;
                let value = value.clone();
                self.move_to_front(key);
                Some(value)
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    pub fn put(&mut self, key: K, value: V, size: usize) {
        if size > self.capacity {
            return;
        }

        if let Some((_old_value, old_size)) = self.map.get(&key) {
            self.total_size -= old_size;
            self.map.remove(&key);
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
        }

        while self.total_size + size > self.capacity && !self.order.is_empty() {
            if let Some(lru_key) = self.order.pop() {
                if let Some((_, old_size)) = self.map.remove(&lru_key) {
                    self.total_size -= old_size;
                }
            }
        }

        self.total_size += size;
        self.map.insert(key.clone(), (value, size));
        self.order.insert(0, key);
    }

    fn move_to_front(&mut self, key: &K) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
            self.order.insert(0, key.clone());
        }
    }

    pub fn stats(&self) -> (u64, u64, f64) {
        let total = self.hits + self.misses;
        let hit_rate = if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        };
        (self.hits, self.misses, hit_rate)
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.total_size = 0;
    }

    pub fn invalidate(&mut self, key: &K) {
        if let Some((_, size)) = self.map.remove(key) {
            self.total_size -= size;
        }
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_basic() {
        let mut cache = LruCache::new(100);
        cache.put("a".to_string(), vec![1, 2, 3], 3);
        assert_eq!(cache.get(&"a".to_string()), Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = LruCache::new(10);
        cache.put("a".to_string(), vec![1, 2, 3, 4], 4);
        cache.put("b".to_string(), vec![5, 6, 7, 8], 4);
        cache.put("c".to_string(), vec![9, 10, 11, 12], 4);
        assert_eq!(cache.get(&"a".to_string()), None);
        assert_eq!(cache.get(&"b".to_string()), Some(vec![5, 6, 7, 8]));
    }

    #[test]
    fn test_lru_stats() {
        let mut cache = LruCache::new(100);
        cache.put("a".to_string(), vec![1], 1);
        cache.get(&"a".to_string());
        cache.get(&"b".to_string());
        let (hits, misses, rate) = cache.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
        assert!((rate - 0.5).abs() < 0.01);
    }
}
