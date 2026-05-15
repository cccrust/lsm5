/// Write-Ahead Log (WAL)
///
/// Binary record format per entry:
///   [1 byte]  op_type: 0x01 = Put, 0x02 = Delete
///   [4 bytes] key_len  (big-endian u32)
///   [4 bytes] val_len  (big-endian u32, 0 for Delete)
///   [N bytes] key
///   [M bytes] value
///   [4 bytes] CRC32 of (op_type | key_len | val_len | key | value)

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use crate::error::{Error, Result};

const OP_PUT: u8 = 0x01;
const OP_DELETE: u8 = 0x02;

pub enum WalRecord {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

pub struct Wal {
    path: PathBuf,
    writer: BufWriter<File>,
}

impl Wal {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(Wal {
            path,
            writer: BufWriter::new(file),
        })
    }

    pub fn append_put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.write_record(OP_PUT, key, value)
    }

    pub fn append_delete(&mut self, key: &[u8]) -> Result<()> {
        self.write_record(OP_DELETE, key, &[])
    }

    pub fn sync(&mut self) -> Result<()> {
        self.writer.flush()?;
        self.writer.get_ref().sync_data()?;
        Ok(())
    }

    /// Replay all records from the WAL file for crash recovery.
    pub fn replay(path: impl AsRef<Path>) -> Result<Vec<WalRecord>> {
        let file = match File::open(path.as_ref()) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };
        let mut reader = BufReader::new(file);
        let mut records = Vec::new();

        loop {
            // Read op_type
            let mut op_buf = [0u8; 1];
            match reader.read_exact(&mut op_buf) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }
            let op_type = op_buf[0];

            // Read key_len and val_len
            let key_len = read_u32(&mut reader)? as usize;
            let val_len = read_u32(&mut reader)? as usize;

            // Read key and value
            let mut key = vec![0u8; key_len];
            let mut value = vec![0u8; val_len];
            reader.read_exact(&mut key)?;
            reader.read_exact(&mut value)?;

            // Read and verify CRC
            let stored_crc = read_u32(&mut reader)?;
            let computed_crc = crc32(&op_buf, key_len as u32, val_len as u32, &key, &value);
            if stored_crc != computed_crc {
                return Err(Error::WalReplayError(
                    format!("CRC mismatch at record (key={:?})", String::from_utf8_lossy(&key))
                ));
            }

            match op_type {
                OP_PUT => records.push(WalRecord::Put { key, value }),
                OP_DELETE => records.push(WalRecord::Delete { key }),
                _ => return Err(Error::WalReplayError(
                    format!("Unknown op_type: 0x{:02x}", op_type)
                )),
            }
        }

        Ok(records)
    }

    /// Truncate the WAL (called after a successful MemTable flush).
    pub fn truncate(&self) -> Result<()> {
        let file = OpenOptions::new().write(true).open(&self.path)?;
        file.set_len(0)?;
        Ok(())
    }

    fn write_record(&mut self, op: u8, key: &[u8], value: &[u8]) -> Result<()> {
        let key_len = key.len() as u32;
        let val_len = value.len() as u32;
        let checksum = crc32(&[op], key_len, val_len, key, value);

        self.writer.write_all(&[op])?;
        self.writer.write_all(&key_len.to_be_bytes())?;
        self.writer.write_all(&val_len.to_be_bytes())?;
        self.writer.write_all(key)?;
        self.writer.write_all(value)?;
        self.writer.write_all(&checksum.to_be_bytes())?;
        Ok(())
    }
}

fn read_u32(r: &mut impl Read) -> Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wal_write_and_replay() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wal");
        {
            let mut wal = Wal::open(&path).unwrap();
            wal.append_put(b"key1", b"value1").unwrap();
            wal.append_put(b"key2", b"value2").unwrap();
            wal.append_delete(b"key1").unwrap();
        }
        let records = Wal::replay(&path).unwrap();
        assert_eq!(records.len(), 3);
        match &records[0] {
            WalRecord::Put { key, value } => {
                assert_eq!(key, b"key1");
                assert_eq!(value, b"value1");
            }
            _ => panic!("Expected Put"),
        }
        match &records[2] {
            WalRecord::Delete { key } => {
                assert_eq!(key, b"key1");
            }
            _ => panic!("Expected Delete"),
        }
    }

    #[test]
    fn test_wal_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.wal");
        let records = Wal::replay(&path).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn test_wal_missing_file() {
        let path = std::path::Path::new("/nonexistent/path/wal.log");
        let records = Wal::replay(path).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn test_wal_binary_key_value() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("binary.wal");
        {
            let mut wal = Wal::open(&path).unwrap();
            let binary_key: Vec<u8> = vec![0x00, 0x01, 0x02, 0xFF];
            let binary_val: Vec<u8> = vec![0xFE, 0xFD, 0xFC];
            wal.append_put(&binary_key, &binary_val).unwrap();
        }
        let records = Wal::replay(&path).unwrap();
        assert_eq!(records.len(), 1);
        match &records[0] {
            WalRecord::Put { key, value } => {
                assert_eq!(key, &vec![0x00, 0x01, 0x02, 0xFF]);
                assert_eq!(value, &vec![0xFE, 0xFD, 0xFC]);
            }
            _ => panic!("Expected Put"),
        }
    }
}

/// CRC32 (Castagnoli) — fast enough for WAL checksums.
fn crc32(op: &[u8], key_len: u32, val_len: u32, key: &[u8], value: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    let feed = |crc: &mut u32, data: &[u8]| {
        for &b in data {
            *crc ^= b as u32;
            for _ in 0..8 {
                let mask = ((*crc & 1).wrapping_neg()) as u32;
                *crc = (*crc >> 1) ^ (0xEDB88320 & mask);
            }
        }
    };
    feed(&mut crc, op);
    feed(&mut crc, &key_len.to_be_bytes());
    feed(&mut crc, &val_len.to_be_bytes());
    feed(&mut crc, key);
    feed(&mut crc, value);
    !crc
}
