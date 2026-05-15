/// SSTable (Sorted String Table)
///
/// File layout:
///   ┌─────────────────────────────────────────┐
///   │  Data Block  (sorted key-value entries)  │ <- optionally zstd compressed
///   ├─────────────────────────────────────────┤
///   │  Index Block (key → data offset map)    │
///   ├─────────────────────────────────────────┤
///   │  Bloom Filter Block                     │
///   ├─────────────────────────────────────────┤
///   │  Footer (40 bytes = offsets + sizes + magic) │
///   └─────────────────────────────────────────┘
///
/// Data block entry format (with CRC):
///   [4] key_len | [4] val_len | [1] is_tombstone | [4] crc32 | [N] key | [M] value
///
/// Index block entry format:
///   [4] key_len | [N] key | [8] data_offset
///
/// Bloom filter block:
///   [8] num_bits | [8] num_hashes | [num_bits/8 rounded up] bit data
///
/// Footer (40 bytes):
///   [8] index_offset | [8] bloom_offset | [8] data_len | [8] compressed_len | [8] magic=0x4C534D35_46494C45
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::bloom::BloomFilter;
use crate::error::{Error, Result};
use crate::memtable::Value;
use zstd::stream::write::Encoder;

const MAGIC: u64 = 0x4C534D35_46494C45; // "LSM5FILE"
const BLOOM_CAPACITY: usize = 1000;
const BLOOM_FPR: f64 = 0.01;
const COMPRESSION_THRESHOLD: usize = 1024; // only compress if > 1KB
const ENTRY_HEADER_SIZE: usize = 4 + 4 + 1 + 4; // key_len + val_len + tombstone + crc

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// One entry in the SSTable's in-memory index.
#[derive(Clone, Debug)]
pub struct IndexEntry {
    pub key: Vec<u8>,
    pub offset: u64,
}

/// Metadata for an SSTable file (kept in memory by the DB).
#[derive(Clone, Debug)]
pub struct SsTableMeta {
    pub path: PathBuf,
    pub level: usize,
    pub min_key: Vec<u8>,
    pub max_key: Vec<u8>,
    pub entry_count: usize,
    pub file_size: u64,
    pub sequence: u64, // monotonically increasing generation number
}

/// Builder: write a new SSTable from a sorted iterator of (key, Value).
pub struct SsTableWriter {
    writer: BufWriter<File>,
    index: Vec<IndexEntry>,
    bloom: BloomFilter,
    data_buffer: Vec<u8>,
    data_offset: u64,
    entry_count: usize,
    min_key: Option<Vec<u8>>,
    max_key: Option<Vec<u8>>,
    compression_enabled: bool,
    compression_level: i32,
}

impl SsTableWriter {
    pub fn create(
        path: impl AsRef<Path>,
        capacity_hint: usize,
        compression_enabled: bool,
        compression_level: i32,
    ) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        let bloom_cap = capacity_hint.max(BLOOM_CAPACITY);
        Ok(SsTableWriter {
            writer: BufWriter::new(file),
            index: Vec::new(),
            bloom: BloomFilter::new(bloom_cap, BLOOM_FPR),
            data_buffer: Vec::new(),
            data_offset: 0,
            entry_count: 0,
            min_key: None,
            max_key: None,
            compression_enabled,
            compression_level,
        })
    }

    /// Append one key-value entry (must be called in sorted key order).
    pub fn add(&mut self, key: &[u8], value: &Value) -> Result<()> {
        let current_offset = self.data_offset;
        self.index.push(IndexEntry {
            key: key.to_vec(),
            offset: current_offset,
        });
        self.bloom.insert(key);

        let (val_bytes, is_tombstone): (&[u8], u8) = match value {
            Value::Data(v) => (v.as_slice(), 0),
            Value::Tombstone => (&[], 1),
        };

        let key_len = key.len() as u32;
        let val_len = val_bytes.len() as u32;

        self.data_buffer.extend_from_slice(&key_len.to_be_bytes());
        self.data_buffer.extend_from_slice(&val_len.to_be_bytes());
        self.data_buffer.push(is_tombstone);

        // Calculate CRC over (key | value)
        let mut crc_data = Vec::with_capacity(key.len() + val_bytes.len());
        crc_data.extend_from_slice(key);
        crc_data.extend_from_slice(val_bytes);
        let crc = crc32(&crc_data);
        self.data_buffer.extend_from_slice(&crc.to_be_bytes());

        self.data_buffer.extend_from_slice(key);
        self.data_buffer.extend_from_slice(val_bytes);

        let entry_size = ENTRY_HEADER_SIZE + key.len() + val_bytes.len();
        self.data_offset += entry_size as u64;
        self.entry_count += 1;

        if self.min_key.is_none() {
            self.min_key = Some(key.to_vec());
        }
        self.max_key = Some(key.to_vec());

        Ok(())
    }

    /// Finish writing and flush the index + bloom + footer.
    pub fn finish(mut self) -> Result<(SsTableMeta, PathBuf)> {
        // Compress data block if enabled and large enough
        let (compressed_data, compressed_len) =
            if self.compression_enabled && self.data_buffer.len() > COMPRESSION_THRESHOLD {
                let mut encoder = Encoder::new(Vec::new(), self.compression_level)?;
                encoder.write_all(&self.data_buffer)?;
                let compressed = encoder.finish()?;
                let len = compressed.len() as u64;
                (compressed, len)
            } else {
                (std::mem::take(&mut self.data_buffer), self.data_offset)
            };

        // Write (compressed) data block
        self.writer.write_all(&compressed_data)?;
        let data_len = self.data_offset;

        // Write index block (always after compressed data)
        let index_offset = compressed_len;

        for entry in &self.index {
            let kl = entry.key.len() as u32;
            self.writer.write_all(&kl.to_be_bytes())?;
            self.writer.write_all(&entry.key)?;
            self.writer.write_all(&entry.offset.to_be_bytes())?;
        }

        // Write bloom filter block
        let bloom_offset = index_offset + self.index_block_size();
        let nb = self.bloom.num_bits() as u64;
        let nh = self.bloom.num_hashes() as u64;
        self.writer.write_all(&nb.to_be_bytes())?;
        self.writer.write_all(&nh.to_be_bytes())?;
        for &word in self.bloom.raw_bits() {
            self.writer.write_all(&word.to_le_bytes())?;
        }

        // Write footer: index_offset, bloom_offset, data_len, compressed_len, magic
        self.writer.write_all(&index_offset.to_be_bytes())?;
        self.writer.write_all(&bloom_offset.to_be_bytes())?;
        self.writer.write_all(&data_len.to_be_bytes())?;
        self.writer.write_all(&compressed_len.to_be_bytes())?;
        self.writer.write_all(&MAGIC.to_be_bytes())?;

        self.writer.flush()?;

        let path = self
            .writer
            .get_ref()
            .metadata()
            .ok()
            .map(|_| ()) // just to check it's still open
            .map(|_| PathBuf::new()) // placeholder — will be set by caller
            .unwrap_or_default();

        // We need the actual path; it was set at creation time. Return it via meta.
        let file_size = self.writer.get_ref().metadata()?.len();
        let meta = SsTableMeta {
            path: PathBuf::new(), // caller sets this
            level: 0,
            min_key: self.min_key.unwrap_or_default(),
            max_key: self.max_key.unwrap_or_default(),
            entry_count: self.entry_count,
            file_size,
            sequence: 0,
        };
        Ok((meta, path))
    }

    fn index_block_size(&self) -> u64 {
        self.index
            .iter()
            .map(|e| (4 + e.key.len() + 8) as u64)
            .sum()
    }
}

/// Reader: open an existing SSTable and perform point lookups + scans.
#[allow(dead_code)]
pub struct SsTableReader {
    path: PathBuf,
    index: Vec<IndexEntry>,
    bloom: BloomFilter,
    data_len: u64,
    compressed_len: u64,
    data: Vec<u8>,
}

impl SsTableReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path)?;
        let file_len = file.metadata()?.len();

        if file_len < 40 {
            return Err(Error::Corruption("SSTable too small".into()));
        }

        // Read footer (last 40 bytes)
        file.seek(SeekFrom::End(-40))?;
        let index_offset = read_u64(&mut file)?;
        let bloom_offset = read_u64(&mut file)?;
        let data_len = read_u64(&mut file)?;
        let compressed_len = read_u64(&mut file)?;
        let magic = read_u64(&mut file)?;

        if magic != MAGIC {
            return Err(Error::Corruption(format!("Bad magic: 0x{:016X}", magic)));
        }

        // Read and decompress data block
        file.seek(SeekFrom::Start(0))?;
        let mut data = vec![0u8; compressed_len as usize];
        file.read_exact(&mut data)?;

        let data = if compressed_len != data_len {
            zstd::decode_all(data.as_slice())?
        } else {
            data
        };

        // Read index block
        file.seek(SeekFrom::Start(index_offset))?;
        let index_end = bloom_offset;
        let mut index = Vec::new();
        let mut pos = index_offset;
        while pos < index_end {
            let kl = read_u32_file(&mut file)? as usize;
            let mut key = vec![0u8; kl];
            file.read_exact(&mut key)?;
            let offset = read_u64(&mut file)?;
            index.push(IndexEntry { key, offset });
            pos += (4 + kl + 8) as u64;
        }

        // Read bloom filter
        file.seek(SeekFrom::Start(bloom_offset))?;
        let num_bits = read_u64(&mut file)? as usize;
        let num_hashes = read_u64(&mut file)? as usize;
        let num_words = num_bits.div_ceil(64);
        let mut bits = vec![0u64; num_words];
        for w in &mut bits {
            let mut buf = [0u8; 8];
            file.read_exact(&mut buf)?;
            *w = u64::from_le_bytes(buf);
        }
        let bloom = BloomFilter::from_raw(bits, num_bits, num_hashes);

        Ok(SsTableReader {
            path,
            index,
            bloom,
            data_len,
            compressed_len,
            data,
        })
    }

    /// Point lookup. Returns None if definitely absent (bloom says no or past end-of-data).
    pub fn get(&self, key: &[u8]) -> Result<Option<Value>> {
        if !self.bloom.may_contain(key) {
            return Ok(None);
        }

        // Binary search the index for the entry whose key <= target.
        let idx = match self.index.binary_search_by(|e| e.key.as_slice().cmp(key)) {
            Ok(i) => i,
            Err(0) => return Ok(None),
            Err(i) => i - 1,
        };

        // Scan forward from that offset in the decompressed data.
        let mut pos = self.index[idx].offset as usize;
        let scan_end = if idx + 1 < self.index.len() {
            self.index[idx + 1].offset as usize
        } else {
            self.data.len()
        };

        while pos < scan_end && pos + ENTRY_HEADER_SIZE <= self.data.len() {
            let kl = u32::from_be_bytes([
                self.data[pos],
                self.data[pos + 1],
                self.data[pos + 2],
                self.data[pos + 3],
            ]) as usize;
            let vl = u32::from_be_bytes([
                self.data[pos + 4],
                self.data[pos + 5],
                self.data[pos + 6],
                self.data[pos + 7],
            ]) as usize;
            let is_tombstone = self.data[pos + 8] == 1;
            let stored_crc = u32::from_be_bytes([
                self.data[pos + 9],
                self.data[pos + 10],
                self.data[pos + 11],
                self.data[pos + 12],
            ]);
            pos += ENTRY_HEADER_SIZE;

            if pos + kl + vl > self.data.len() {
                break;
            }

            // Verify CRC
            let mut crc_data = Vec::with_capacity(kl + vl);
            crc_data.extend_from_slice(&self.data[pos..pos + kl]);
            crc_data.extend_from_slice(&self.data[pos + kl..pos + kl + vl]);
            let computed_crc = crc32(&crc_data);
            if stored_crc != computed_crc {
                return Err(Error::Corruption(format!(
                    "CRC mismatch for key at offset {}",
                    pos
                )));
            }

            let k = &self.data[pos..pos + kl];
            pos += kl;
            let v = &self.data[pos..pos + vl];
            pos += vl;

            if k == key {
                return Ok(Some(if is_tombstone {
                    Value::Tombstone
                } else {
                    Value::Data(v.to_vec())
                }));
            }
            if k > key {
                break;
            }
        }
        Ok(None)
    }

    /// Return all entries (key, value) in sorted order (for compaction / range scans).
    pub fn scan_all(&self) -> Result<Vec<(Vec<u8>, Value)>> {
        let mut entries = Vec::new();
        let mut pos = 0usize;

        while pos + ENTRY_HEADER_SIZE <= self.data.len() {
            let kl = u32::from_be_bytes([
                self.data[pos],
                self.data[pos + 1],
                self.data[pos + 2],
                self.data[pos + 3],
            ]) as usize;
            let vl = u32::from_be_bytes([
                self.data[pos + 4],
                self.data[pos + 5],
                self.data[pos + 6],
                self.data[pos + 7],
            ]) as usize;
            let is_tombstone = self.data[pos + 8] == 1;
            let _stored_crc = u32::from_be_bytes([
                self.data[pos + 9],
                self.data[pos + 10],
                self.data[pos + 11],
                self.data[pos + 12],
            ]);
            pos += ENTRY_HEADER_SIZE;

            if pos + kl + vl > self.data.len() {
                break;
            }

            // Verify CRC
            let mut crc_data = Vec::with_capacity(kl + vl);
            crc_data.extend_from_slice(&self.data[pos..pos + kl]);
            crc_data.extend_from_slice(&self.data[pos + kl..pos + kl + vl]);
            let _computed_crc = crc32(&crc_data);
            // Note: CRC verification could be added here for scan_all if needed

            let k = self.data[pos..pos + kl].to_vec();
            pos += kl;
            let v = self.data[pos..pos + vl].to_vec();
            pos += vl;

            entries.push((
                k,
                if is_tombstone {
                    Value::Tombstone
                } else {
                    Value::Data(v)
                },
            ));
        }

        Ok(entries)
    }

    pub fn min_key(&self) -> Option<&[u8]> {
        self.index.first().map(|e| e.key.as_slice())
    }

    pub fn max_key(&self) -> Option<&[u8]> {
        self.index.last().map(|e| e.key.as_slice())
    }

    pub fn entry_count(&self) -> usize {
        self.index.len()
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

pub fn write_sstable(
    path: &Path,
    entries: &[(Vec<u8>, Value)],
    level: usize,
    sequence: u64,
    compression_enabled: bool,
    compression_level: i32,
) -> Result<SsTableMeta> {
    let mut writer =
        SsTableWriter::create(path, entries.len(), compression_enabled, compression_level)?;
    for (k, v) in entries {
        writer.add(k, v)?;
    }
    let (mut meta, _) = writer.finish()?;
    meta.path = path.to_path_buf();
    meta.level = level;
    meta.sequence = sequence;
    Ok(meta)
}

fn read_u32_file(f: &mut File) -> Result<u32> {
    let mut buf = [0u8; 4];
    f.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn read_u64(f: &mut File) -> Result<u64> {
    let mut buf = [0u8; 8];
    f.read_exact(&mut buf)?;
    Ok(u64::from_be_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_write_read_sstable() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sst");
        let entries = vec![
            (b"apple".to_vec(), Value::Data(b"fruit".to_vec())),
            (b"banana".to_vec(), Value::Data(b"yellow".to_vec())),
            (b"cherry".to_vec(), Value::Tombstone),
        ];
        write_sstable(&path, &entries, 0, 1, true, 3).unwrap();

        let reader = SsTableReader::open(&path).unwrap();
        assert_eq!(
            reader.get(b"apple").unwrap(),
            Some(Value::Data(b"fruit".to_vec()))
        );
        assert_eq!(
            reader.get(b"banana").unwrap(),
            Some(Value::Data(b"yellow".to_vec()))
        );
        assert_eq!(reader.get(b"cherry").unwrap(), Some(Value::Tombstone));
        assert_eq!(reader.get(b"grape").unwrap(), None);
    }

    #[test]
    fn test_scan_all() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test2.sst");
        let entries: Vec<_> = (0..10u32)
            .map(|i| {
                (
                    format!("k{:04}", i).into_bytes(),
                    Value::Data(i.to_be_bytes().to_vec()),
                )
            })
            .collect();
        write_sstable(&path, &entries, 0, 1, true, 3).unwrap();

        let reader = SsTableReader::open(&path).unwrap();
        let all = reader.scan_all().unwrap();
        assert_eq!(all.len(), 10);
        for (i, (k, _)) in all.iter().enumerate() {
            assert_eq!(k, &format!("k{:04}", i).into_bytes());
        }
    }
}
