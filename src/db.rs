/// Lsm5 — Main database engine
///
/// Coordinates MemTable, WAL, SSTables, and compaction.
///
/// Read path:  MemTable → ImmutableMemTables → L0..LN SSTables
/// Write path: WAL append → MemTable insert → (flush when full)
use std::fs;
use std::path::{Path, PathBuf};

pub use crate::config::Config;
pub use crate::stats::DbStats;

use crate::compaction::{compact, level_size_threshold, L0_COMPACTION_TRIGGER, MAX_LEVELS};
use crate::error::Result;
use crate::memtable::{MemTable, Value};
use crate::sstable::{write_sstable, SsTableMeta, SsTableReader};
use crate::wal::{Wal, WalRecord};

/// The main LSM5 database handle.
pub struct Lsm5 {
    config: Config,
    wal: Wal,
    memtable: MemTable,
    /// Immutable MemTables waiting to be flushed (oldest first).
    imm_memtables: Vec<Vec<(Vec<u8>, Value)>>,
    /// SSTables organised per level.  levels[0] is L0.
    levels: Vec<Vec<SsTableMeta>>,
    seq: u64, // monotonic sequence number for SSTable filenames
}

impl Lsm5 {
    /// Open (or create) an LSM5 database at the given directory.
    pub fn open(config: Config) -> Result<Self> {
        fs::create_dir_all(&config.dir)?;

        let wal_path = config.dir.join("wal.log");
        let mut memtable = MemTable::new();

        // Replay WAL for crash recovery
        let records = Wal::replay(&wal_path)?;
        for record in records {
            match record {
                WalRecord::Put { key, value } => memtable.put(key, value),
                WalRecord::Delete { key } => memtable.delete(key),
            }
        }

        let wal = Wal::open(&wal_path)?;

        // Scan existing SSTable files
        let levels = Self::load_existing_sstables(&config.dir)?;
        let seq: u64 = levels
            .iter()
            .flat_map(|l: &Vec<SsTableMeta>| l.iter().map(|m| m.sequence))
            .max()
            .unwrap_or(0);

        Ok(Lsm5 {
            config,
            wal,
            memtable,
            imm_memtables: Vec::new(),
            levels,
            seq,
        })
    }

    // ── Public API ───────────────────────────────────────────────────────────

    /// Insert or update a key.
    pub fn put(&mut self, key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) -> Result<()> {
        let key = key.into();
        let value = value.into();
        self.wal.append_put(&key, &value)?;
        if self.config.sync_writes {
            self.wal.sync()?;
        }
        self.memtable.put(key, value);
        self.maybe_flush()?;
        Ok(())
    }

    /// Delete a key (inserts a tombstone).
    pub fn delete(&mut self, key: impl Into<Vec<u8>>) -> Result<()> {
        let key = key.into();
        self.wal.append_delete(&key)?;
        if self.config.sync_writes {
            self.wal.sync()?;
        }
        self.memtable.delete(key);
        self.maybe_flush()?;
        Ok(())
    }

    /// Get the value for a key.  Returns None if the key does not exist.
    pub fn get(&self, key: impl AsRef<[u8]>) -> Result<Option<Vec<u8>>> {
        let key = key.as_ref();

        // 1. Check active MemTable
        if let Some(v) = self.memtable.get(key) {
            return match v {
                Value::Data(d) => Ok(Some(d.clone())),
                Value::Tombstone => Ok(None),
            };
        }

        // 2. Check immutable MemTables (newest first)
        for imm in self.imm_memtables.iter().rev() {
            if let Some((_, v)) = imm.iter().find(|(k, _)| k.as_slice() == key) {
                return match v {
                    Value::Data(d) => Ok(Some(d.clone())),
                    Value::Tombstone => Ok(None),
                };
            }
        }

        // 3. Search SSTables level by level
        for level in &self.levels {
            // Within a level search from newest to oldest (highest seq first)
            let mut sorted_level = level.clone();
            sorted_level.sort_by(|a, b| b.sequence.cmp(&a.sequence));

            for meta in &sorted_level {
                // Skip if key is out of this SSTable's range
                if !meta.min_key.is_empty() && key < meta.min_key.as_slice() {
                    continue;
                }
                if !meta.max_key.is_empty() && key > meta.max_key.as_slice() {
                    continue;
                }

                let reader = SsTableReader::open(&meta.path)?;
                if let Some(v) = reader.get(key)? {
                    return match v {
                        Value::Data(d) => Ok(Some(d)),
                        Value::Tombstone => Ok(None),
                    };
                }
            }
        }

        Ok(None)
    }

    /// Scan all keys in [start, end) range, returning them in sorted order.
    pub fn scan(
        &self,
        start: impl AsRef<[u8]>,
        end: impl AsRef<[u8]>,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let start = start.as_ref();
        let end = end.as_ref();
        let mut map: std::collections::BTreeMap<Vec<u8>, Value> = std::collections::BTreeMap::new();

        // Collect from all SSTables (older levels first so newer ones overwrite)
        for level in self.levels.iter().rev() {
            for meta in level {
                if meta.max_key.as_slice() < start || meta.min_key.as_slice() >= end {
                    continue;
                }
                let reader = SsTableReader::open(&meta.path)?;
                for (k, v) in reader.scan_all()? {
                    if k.as_slice() >= start && k.as_slice() < end {
                        map.insert(k, v);
                    }
                }
            }
        }

        // Immutable MemTables
        for imm in &self.imm_memtables {
            for (k, v) in imm {
                if k.as_slice() >= start && k.as_slice() < end {
                    map.insert(k.clone(), v.clone());
                }
            }
        }

        // Active MemTable
        for (k, v) in self.memtable.iter() {
            if k.as_slice() >= start && k.as_slice() < end {
                map.insert(k.clone(), v.clone());
            }
        }

        let result = map
            .into_iter()
            .filter_map(|(k, v)| match v {
                Value::Data(d) => Some((k, d)),
                Value::Tombstone => None,
            })
            .collect();

        Ok(result)
    }

    /// Force a MemTable flush and compaction cycle.
    pub fn flush(&mut self) -> Result<()> {
        self.flush_memtable()?;
        self.maybe_compact()?;
        Ok(())
    }

    /// Sync WAL to disk.
    pub fn sync(&mut self) -> Result<()> {
        self.wal.sync()
    }

    /// Return database statistics.
    pub fn stats(&self) -> DbStats {
        DbStats::new(
            self.memtable.len(),
            self.memtable.size_bytes(),
            self.levels.iter().map(|l| l.len()).collect(),
            self.levels
                .iter()
                .map(|l| l.iter().map(|m| m.file_size).sum())
                .collect(),
        )
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn maybe_flush(&mut self) -> Result<()> {
        if self.memtable.size_bytes() >= self.config.memtable_size_limit {
            self.flush_memtable()?;
            self.maybe_compact()?;
        }
        Ok(())
    }

    fn flush_memtable(&mut self) -> Result<()> {
        if self.memtable.is_empty() {
            return Ok(());
        }

        let entries = self.memtable.drain_sorted();
        self.seq += 1;
        let path = self.config.dir.join(format!("L0-{:016x}.sst", self.seq));
        let meta = write_sstable(&path, &entries, 0, self.seq)?;

        while self.levels.is_empty() {
            self.levels.push(Vec::new());
        }
        self.levels[0].push(meta);

        // Truncate WAL — data is now safely on disk as an SSTable.
        self.wal.truncate()?;

        Ok(())
    }

    fn maybe_compact(&mut self) -> Result<()> {
        // L0: trigger compaction when there are too many files
        if self.levels.first().map_or(0, |l| l.len()) >= L0_COMPACTION_TRIGGER {
            self.compact_level(0)?;
        }

        // L1..LN: trigger when total size exceeds threshold
        for level in 1..self.levels.len() {
            let total_size: u64 = self.levels[level].iter().map(|m| m.file_size).sum();
            if total_size > level_size_threshold(level) {
                self.compact_level(level)?;
            }
        }

        Ok(())
    }

    fn compact_level(&mut self, level: usize) -> Result<()> {
        let target_level = level + 1;
        if target_level >= MAX_LEVELS {
            return Ok(()); // Already at the bottom
        }

        // Ensure levels vec is large enough
        while self.levels.len() <= target_level {
            self.levels.push(Vec::new());
        }

        // Gather inputs: all files from `level` + overlapping files from `target_level`
        let level_files: Vec<&SsTableMeta> = self.levels[level].iter().collect();

        if level_files.is_empty() {
            return Ok(());
        }

        // Find key range of the compaction inputs
        let min_key = level_files
            .iter()
            .map(|m| m.min_key.as_slice())
            .min()
            .unwrap_or(&[])
            .to_vec();
        let max_key = level_files
            .iter()
            .map(|m| m.max_key.as_slice())
            .max()
            .unwrap_or(&[])
            .to_vec();

        // Find overlapping files at target level
        let overlapping: Vec<&SsTableMeta> = self.levels[target_level]
            .iter()
            .filter(|m| {
                m.min_key.as_slice() <= max_key.as_slice()
                    && m.max_key.as_slice() >= min_key.as_slice()
            })
            .collect();

        let mut all_inputs: Vec<&SsTableMeta> = level_files;
        all_inputs.extend_from_slice(&overlapping);

        let is_bottommost = target_level + 1 >= MAX_LEVELS || target_level + 1 >= self.levels.len();

        let new_metas = compact(
            &all_inputs,
            &self.config.dir,
            target_level,
            &mut self.seq,
            is_bottommost,
            self.config.max_sstable_size,
        )?;

        // Remove old SSTable files
        let paths_to_remove: Vec<PathBuf> = all_inputs.iter().map(|m| m.path.clone()).collect();

        // Remove from in-memory metadata
        let overlapping_paths: std::collections::HashSet<PathBuf> =
            overlapping.iter().map(|m| m.path.clone()).collect();
        self.levels[level].clear();
        self.levels[target_level].retain(|m| !overlapping_paths.contains(&m.path));
        self.levels[target_level].extend(new_metas);

        // Delete old files from disk
        for path in paths_to_remove {
            let _ = fs::remove_file(path);
        }

        Ok(())
    }

    fn load_existing_sstables(dir: &Path) -> Result<Vec<Vec<SsTableMeta>>> {
        let mut levels: Vec<Vec<SsTableMeta>> = vec![Vec::new(); MAX_LEVELS];

        let read_dir = match fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(levels),
            Err(e) => return Err(e.into()),
        };

        for entry in read_dir {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("sst") {
                continue;
            }

            let fname = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            // Expected format: L<level>-<hex_seq>
            let parts: Vec<&str> = fname.splitn(2, '-').collect();
            if parts.len() != 2 || !parts[0].starts_with('L') {
                continue;
            }

            let level: usize = parts[0][1..].parse().unwrap_or(0);
            let seq: u64 = u64::from_str_radix(parts[1], 16).unwrap_or(0);

            if level >= MAX_LEVELS {
                continue;
            }

            let reader: SsTableReader = match SsTableReader::open(&path) {
                Ok(r) => r,
                Err(_) => continue, // skip corrupted files
            };

            let min_key = reader.min_key().unwrap_or(&[]).to_vec();
            let max_key = reader.max_key().unwrap_or(&[]).to_vec();
            let entry_count = reader.entry_count();
            let file_size = path.metadata()?.len();

            levels[level].push(SsTableMeta {
                path,
                level,
                min_key,
                max_key,
                entry_count,
                file_size,
                sequence: seq,
            });
        }

        // Sort each level by sequence number (newest last)
        for level in &mut levels {
            level.sort_by_key(|m| m.sequence);
        }

        Ok(levels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn open_db(dir: &Path) -> Lsm5 {
        Lsm5::open(Config::new(dir).memtable_size_limit(1024)).unwrap()
    }

    #[test]
    fn test_basic_put_get() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());
        db.put("hello", "world").unwrap();
        assert_eq!(db.get("hello").unwrap(), Some(b"world".to_vec()));
        assert_eq!(db.get("missing").unwrap(), None);
    }

    #[test]
    fn test_delete() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());
        db.put("k", "v").unwrap();
        db.delete("k").unwrap();
        assert_eq!(db.get("k").unwrap(), None);
    }

    #[test]
    fn test_overwrite() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());
        db.put("k", "v1").unwrap();
        db.put("k", "v2").unwrap();
        assert_eq!(db.get("k").unwrap(), Some(b"v2".to_vec()));
    }

    #[test]
    fn test_flush_and_read_from_sstable() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());

        // Write enough to trigger a flush
        for i in 0..200u32 {
            db.put(format!("key-{:05}", i), format!("value-{}", i))
                .unwrap();
        }

        // Verify all keys are still accessible
        for i in 0..200u32 {
            let val = db.get(format!("key-{:05}", i)).unwrap();
            assert_eq!(
                val,
                Some(format!("value-{}", i).into_bytes()),
                "key-{:05} mismatch",
                i
            );
        }
    }

    #[test]
    fn test_crash_recovery() {
        let dir = tempdir().unwrap();
        {
            let mut db = open_db(dir.path());
            db.put("persistent_key", "persistent_value").unwrap();
            // Don't flush — data is only in WAL + MemTable
        }
        // Re-open — should replay WAL
        let db = open_db(dir.path());
        assert_eq!(
            db.get("persistent_key").unwrap(),
            Some(b"persistent_value".to_vec())
        );
    }

    #[test]
    fn test_scan_range() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());
        for i in 0..10u32 {
            db.put(format!("k{:02}", i), format!("v{}", i)).unwrap();
        }
        let results = db.scan("k03", "k07").unwrap();
        let keys: Vec<_> = results.iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(
            keys,
            vec![
                b"k03".to_vec(),
                b"k04".to_vec(),
                b"k05".to_vec(),
                b"k06".to_vec()
            ]
        );
    }

    #[test]
    fn test_stats() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());
        for i in 0..50u32 {
            db.put(format!("k{}", i), format!("v{}", i)).unwrap();
        }
        let stats = db.stats();
        println!("{}", stats);
    }

    #[test]
    fn test_binary_keys() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());
        let key = vec![0x00, 0x01, 0xFF, 0x00];
        let value = vec![0xAA, 0xBB, 0xCC];
        db.put(key.clone(), value.clone()).unwrap();
        assert_eq!(db.get(key).unwrap(), Some(value));
    }

    #[test]
    fn test_empty_scan() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());
        db.put("a", "1").unwrap();
        let results = db.scan("z", "zzz").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_delete_then_put() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());
        db.put("key", "value1").unwrap();
        db.delete("key").unwrap();
        db.put("key", "value2").unwrap();
        assert_eq!(db.get("key").unwrap(), Some(b"value2".to_vec()));
    }

    #[test]
    fn test_scan_empty_to_empty() {
        let dir = tempdir().unwrap();
        let db = open_db(dir.path());
        let results = db.scan("a", "z").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_many_small_values() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());
        for i in 0..100u32 {
            db.put(format!("key{}", i), "v").unwrap();
        }
        for i in 0..100u32 {
            assert!(db.get(format!("key{}", i)).unwrap().is_some());
        }
    }

    #[test]
    fn test_sync_writes() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path())
            .memtable_size_limit(1024)
            .sync_writes(true);
        let mut db = Lsm5::open(config).unwrap();
        db.put("key", "value").unwrap();
        assert_eq!(db.get("key").unwrap(), Some(b"value".to_vec()));
    }

    #[test]
    fn test_explicit_flush() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());
        db.put("key", "value").unwrap();
        db.flush().unwrap();
        let stats = db.stats();
        assert!(stats.memtable_entries == 0);
    }

    #[test]
    fn test_range_scan_with_deleted_keys() {
        let dir = tempdir().unwrap();
        let mut db = open_db(dir.path());
        db.put("a", "1").unwrap();
        db.put("b", "2").unwrap();
        db.put("c", "3").unwrap();
        db.delete("b").unwrap();
        let results = db.scan("a", "d").unwrap();
        let keys: Vec<_> = results.iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(keys, vec![b"a".to_vec(), b"c".to_vec()]);
    }
}
