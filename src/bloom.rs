/// A simple Bloom filter using double-hashing to generate k hash functions.
/// Used in SSTables to quickly rule out non-existent keys.
pub struct BloomFilter {
    bits: Vec<u64>,
    num_bits: usize,
    num_hashes: usize,
}

impl BloomFilter {
    /// Create a new Bloom filter.
    /// `capacity` is the expected number of items.
    /// `false_positive_rate` is the desired FPR (e.g. 0.01 for 1%).
    pub fn new(capacity: usize, false_positive_rate: f64) -> Self {
        let num_bits = Self::optimal_bits(capacity, false_positive_rate).max(64);
        let num_hashes = Self::optimal_hashes(num_bits, capacity).max(1);
        let words = num_bits.div_ceil(64);
        BloomFilter {
            bits: vec![0u64; words],
            num_bits,
            num_hashes,
        }
    }

    /// Reconstruct a BloomFilter from raw bytes (for SSTable deserialization).
    pub fn from_raw(bits: Vec<u64>, num_bits: usize, num_hashes: usize) -> Self {
        BloomFilter {
            bits,
            num_bits,
            num_hashes,
        }
    }

    pub fn num_bits(&self) -> usize {
        self.num_bits
    }
    pub fn num_hashes(&self) -> usize {
        self.num_hashes
    }
    pub fn raw_bits(&self) -> &[u64] {
        &self.bits
    }

    pub fn insert(&mut self, key: &[u8]) {
        let (h1, h2) = self.hashes(key);
        for i in 0..self.num_hashes {
            let pos = (h1.wrapping_add((i as u64).wrapping_mul(h2))) % self.num_bits as u64;
            self.bits[(pos / 64) as usize] |= 1u64 << (pos % 64);
        }
    }

    /// Returns `false` if the key is definitely NOT in the set.
    /// Returns `true` if the key MAY be in the set.
    pub fn may_contain(&self, key: &[u8]) -> bool {
        let (h1, h2) = self.hashes(key);
        for i in 0..self.num_hashes {
            let pos = (h1.wrapping_add((i as u64).wrapping_mul(h2))) % self.num_bits as u64;
            if self.bits[(pos / 64) as usize] & (1u64 << (pos % 64)) == 0 {
                return false;
            }
        }
        true
    }

    fn hashes(&self, key: &[u8]) -> (u64, u64) {
        let h1 = fnv1a_64(key);
        let h2 = murmur_mix(h1 ^ 0xdeadbeefcafe1234);
        (h1, h2 | 1) // h2 must be odd for full coverage
    }

    fn optimal_bits(n: usize, p: f64) -> usize {
        let ln2 = std::f64::consts::LN_2;
        (-(n as f64 * p.ln()) / (ln2 * ln2)) as usize
    }

    fn optimal_hashes(m: usize, n: usize) -> usize {
        let ln2 = std::f64::consts::LN_2;
        ((m as f64 / n as f64) * ln2).round() as usize
    }
}

fn fnv1a_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn murmur_mix(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51afd7ed558ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
    k ^= k >> 33;
    k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_no_false_negatives() {
        let mut bf = BloomFilter::new(1000, 0.01);
        let keys: Vec<Vec<u8>> = (0..500)
            .map(|i| format!("key-{}", i).into_bytes())
            .collect();
        for k in &keys {
            bf.insert(k);
        }
        for k in &keys {
            assert!(bf.may_contain(k), "False negative for {:?}", k);
        }
    }

    #[test]
    fn test_bloom_fpr_reasonable() {
        let mut bf = BloomFilter::new(1000, 0.01);
        for i in 0..1000u32 {
            bf.insert(&i.to_le_bytes());
        }
        let mut fp = 0;
        for i in 1000..11000u32 {
            if bf.may_contain(&i.to_le_bytes()) {
                fp += 1;
            }
        }
        // Allow up to 5% FPR in practice
        assert!(fp < 500, "Too many false positives: {}", fp);
    }

    #[test]
    fn test_bloom_empty() {
        let bf = BloomFilter::new(100, 0.01);
        assert!(!bf.may_contain(b"any_key"));
    }

    #[test]
    fn test_bloom_single_item() {
        let mut bf = BloomFilter::new(10, 0.01);
        bf.insert(b"only_key");
        assert!(bf.may_contain(b"only_key"));
        assert!(!bf.may_contain(b"other_key"));
    }

    #[test]
    fn test_bloom_from_raw() {
        let mut bf = BloomFilter::new(100, 0.01);
        bf.insert(b"testkey");
        let bits = bf.raw_bits().to_vec();
        let num_bits = bf.num_bits();
        let num_hashes = bf.num_hashes();
        let bf2 = BloomFilter::from_raw(bits, num_bits, num_hashes);
        assert!(bf2.may_contain(b"testkey"));
    }
}
