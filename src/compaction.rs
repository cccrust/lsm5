/// Compaction
///
/// Strategy: Leveled compaction (similar to LevelDB/RocksDB).
///
/// - Level-0: SSTables flushed directly from MemTable. May overlap.
///   Trigger: when L0 count >= L0_COMPACTION_TRIGGER.
/// - Level-1..N: SSTables are non-overlapping within each level.
///   Trigger: when a level's total size exceeds its threshold.
///   Target sizes: L1 = 10 MB, L2 = 100 MB, L3 = 1 GB, ...
///
/// Compaction merges sorted runs via k-way merge, dropping
/// tombstones at the bottommost level.

use std::collections::BinaryHeap;
use std::cmp::Reverse;
use std::path::Path;

use crate::error::Result;
use crate::memtable::Value;
use crate::sstable::{SsTableMeta, SsTableReader, write_sstable};

pub const L0_COMPACTION_TRIGGER: usize = 4;
pub const MAX_LEVELS: usize = 7;

/// Size threshold for each level (bytes).
pub fn level_size_threshold(level: usize) -> u64 {
    match level {
        0 => u64::MAX, // L0 is count-based, not size-based
        1 => 10 * 1024 * 1024,        // 10 MB
        l => 10u64.pow(l as u32) * 1024 * 1024, // 10^l MB
    }
}

/// Entry used in the k-way merge heap.
#[derive(Eq, PartialEq)]
struct HeapEntry {
    key: Vec<u8>,
    value: Value,
    /// SSTable sequence number (higher = newer).
    seq: u64,
    /// Iterator source index (for tie-breaking).
    src: usize,
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Min-heap by key; break ties by sequence (newer = higher seq wins)
        self.key.cmp(&other.key)
            .then(other.seq.cmp(&self.seq)) // larger seq = newer
            .then(other.src.cmp(&self.src))
    }
}

/// Merge multiple sorted SSTable entry lists into a single sorted list,
/// keeping only the newest version of each key, and dropping tombstones
/// at the bottom level.
pub fn k_way_merge(
    sources: Vec<Vec<(Vec<u8>, Value)>>,
    seqs: &[u64],
    drop_tombstones: bool,
) -> Vec<(Vec<u8>, Value)> {
    // Build iterators
    let mut iters: Vec<std::vec::IntoIter<(Vec<u8>, Value)>> =
        sources.into_iter().map(|v| v.into_iter()).collect();

    let mut heap: BinaryHeap<Reverse<HeapEntry>> = BinaryHeap::new();

    // Seed the heap with the first entry from each iterator
    for (i, iter) in iters.iter_mut().enumerate() {
        if let Some((key, value)) = iter.next() {
            heap.push(Reverse(HeapEntry { key, value, seq: seqs[i], src: i }));
        }
    }

    let mut result: Vec<(Vec<u8>, Value)> = Vec::new();
    let mut last_key: Option<Vec<u8>> = None;

    while let Some(Reverse(entry)) = heap.pop() {
        // Advance the corresponding iterator
        if let Some((key, value)) = iters[entry.src].next() {
            heap.push(Reverse(HeapEntry { key, value, seq: seqs[entry.src], src: entry.src }));
        }

        // Skip older versions of the same key
        if last_key.as_deref() == Some(entry.key.as_slice()) {
            continue;
        }

        last_key = Some(entry.key.clone());

        // At the bottom level, drop tombstones (they've propagated far enough)
        if drop_tombstones && entry.value == Value::Tombstone {
            continue;
        }

        result.push((entry.key, entry.value));
    }

    result
}

/// Read all entries from a list of SSTables (used for compaction input).
pub fn read_sstables(metas: &[&SsTableMeta]) -> Result<Vec<Vec<(Vec<u8>, Value)>>> {
    metas.iter().map(|m| {
        let reader = SsTableReader::open(&m.path)?;
        reader.scan_all()
    }).collect()
}

/// Compact a set of SSTables into one or more output SSTables at `target_level`.
/// Returns metadata of the newly written SSTables.
pub fn compact(
    inputs: &[&SsTableMeta],
    output_dir: &Path,
    target_level: usize,
    next_seq: &mut u64,
    is_bottommost: bool,
    max_sstable_size: u64,
) -> Result<Vec<SsTableMeta>> {
    if inputs.is_empty() {
        return Ok(vec![]);
    }

    // Read all input data
    let all_data = read_sstables(inputs)?;
    let seqs: Vec<u64> = inputs.iter().map(|m| m.sequence).collect();

    // Merge
    let merged = k_way_merge(all_data, &seqs, is_bottommost);

    if merged.is_empty() {
        return Ok(vec![]);
    }

    // Split into multiple SSTables if needed (respect max_sstable_size)
    let mut output_metas = Vec::new();
    let mut chunk: Vec<(Vec<u8>, Value)> = Vec::new();
    let mut chunk_size: u64 = 0;

    let flush_chunk = |chunk: &mut Vec<(Vec<u8>, Value)>,
                       seq: &mut u64,
                       metas: &mut Vec<SsTableMeta>| -> Result<()> {
        if chunk.is_empty() { return Ok(()); }
        *seq += 1;
        let path = output_dir.join(format!("L{}-{:016x}.sst", target_level, *seq));
        let meta = write_sstable(&path, chunk, target_level, *seq)?;
        metas.push(meta);
        chunk.clear();
        Ok(())
    };

    for (k, v) in merged {
        let entry_size = (k.len() + match &v {
            Value::Data(d) => d.len(),
            Value::Tombstone => 0,
        }) as u64;

        if chunk_size + entry_size > max_sstable_size && !chunk.is_empty() {
            flush_chunk(&mut chunk, next_seq, &mut output_metas)?;
            chunk_size = 0;
        }
        chunk_size += entry_size;
        chunk.push((k, v));
    }
    flush_chunk(&mut chunk, next_seq, &mut output_metas)?;

    Ok(output_metas)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(key: &str, val: &str, seq: u64, src: usize) -> Reverse<HeapEntry> {
        Reverse(HeapEntry {
            key: key.as_bytes().to_vec(),
            value: Value::Data(val.as_bytes().to_vec()),
            seq,
            src,
        })
    }

    #[test]
    fn test_k_way_merge_dedup() {
        let s1 = vec![
            (b"a".to_vec(), Value::Data(b"old".to_vec())),
            (b"b".to_vec(), Value::Data(b"b_val".to_vec())),
        ];
        let s2 = vec![
            (b"a".to_vec(), Value::Data(b"new".to_vec())), // newer
            (b"c".to_vec(), Value::Data(b"c_val".to_vec())),
        ];
        let seqs = vec![1, 2]; // s2 is newer
        let merged = k_way_merge(vec![s1, s2], &seqs, false);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0], (b"a".to_vec(), Value::Data(b"new".to_vec())));
    }

    #[test]
    fn test_k_way_merge_drop_tombstones() {
        let s1 = vec![
            (b"a".to_vec(), Value::Tombstone),
            (b"b".to_vec(), Value::Data(b"live".to_vec())),
        ];
        let merged = k_way_merge(vec![s1], &[1], true);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].0, b"b".to_vec());
    }

    #[test]
    fn test_heap_entry_ordering() {
        let e1 = mk("key", "val1", 1, 0);
        let e2 = mk("key", "val2", 2, 1);
        assert!(e1 < e2);
    }
}
