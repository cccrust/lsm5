# lsm5

A **Log-Structured Merge Tree** (LSM-Tree) storage engine written in Rust, from scratch.

---

## Architecture

```
Write path:
  put(key, value)
      │
      ├─► WAL append (crash-safe)
      │
      └─► MemTable (BTreeMap, in-memory)
               │  (when size >= threshold)
               ▼
           L0 SSTable (flushed to disk)
               │  (when L0 count >= 4)
               ▼
           L1 SSTable (compacted, non-overlapping)
               │  (when L1 size > 10 MB)
               ▼
           L2 ... LN

Read path:
  get(key)
      │
      ├─► MemTable          (newest)
      ├─► Immutable MemTables
      ├─► L0 SSTables       (Bloom filter → binary search index → disk read)
      ├─► L1 SSTables
      └─► ... LN            (oldest)
```

## Components

| Module | Description |
|--------|-------------|
| `memtable` | In-memory `BTreeMap` write buffer; supports `put`, `delete` (tombstone), `get`, sorted iteration |
| `wal` | Write-Ahead Log with CRC32 checksums per record; replayed on startup for crash recovery |
| `sstable` | On-disk Sorted String Table: data block + sparse index block + Bloom filter block + footer |
| `bloom` | Bloom filter using double-hashing (FNV-1a + Murmur mix); configurable FPR |
| `compaction` | Leveled compaction strategy; k-way merge via `BinaryHeap`; tombstone GC at bottommost level |
| `db` | Top-level `Lsm5` engine: coordinates all components, manages levels, triggers flush/compaction |

## SSTable File Format

```
┌────────────────────────────────────────────────────┐
│ Data Block                                         │
│   [4] key_len │ [4] val_len │ [1] tombstone flag  │
│   [N] key     │ [M] value   (repeated, sorted)     │
├────────────────────────────────────────────────────┤
│ Index Block (sparse: one entry per key written)    │
│   [4] key_len │ [N] key │ [8] data_offset          │
├────────────────────────────────────────────────────┤
│ Bloom Filter Block                                 │
│   [8] num_bits │ [8] num_hashes │ [...] bit array  │
├────────────────────────────────────────────────────┤
│ Footer (32 bytes)                                  │
│   [8] index_offset │ [8] bloom_offset              │
│   [8] data_len     │ [8] magic = 0x4C534D3546494C45│
└────────────────────────────────────────────────────┘
```

## WAL Record Format

```
[1]  op_type    (0x01=Put, 0x02=Delete)
[4]  key_len    (big-endian u32)
[4]  val_len    (big-endian u32, 0 for Delete)
[N]  key
[M]  value
[4]  CRC32      (Castagnoli, over all preceding fields)
```

## Usage

### Library

```rust
use lsm5::{Lsm5, db::Config};

let config = Config::new("./mydb")
    .memtable_size_limit(4 * 1024 * 1024)  // 4 MB
    .sync_writes(false);

let mut db = Lsm5::open(config)?;

// Write
db.put("hello", "world")?;
db.put(b"binary_key", b"binary_value")?;

// Read
let val = db.get("hello")?;  // Some(b"world")

// Delete
db.delete("hello")?;

// Range scan [start, end)
let entries = db.scan("a", "z")?;

// Stats
println!("{}", db.stats());
```

### CLI

```bash
cargo build --release

./target/release/lsm5 put   mykey   myvalue
./target/release/lsm5 get   mykey
./target/release/lsm5 del   mykey
./target/release/lsm5 scan  aaa zzz
./target/release/lsm5 stats
./target/release/lsm5 bench     # 100k-op benchmark
```

## Compaction Strategy

**Leveled** (inspired by LevelDB/RocksDB):

- **L0** — SSTables flushed directly from MemTable, may overlap. Compaction triggered when count ≥ 4.
- **L1..LN** — SSTables are non-overlapping within a level. Compaction triggered when total level size exceeds threshold: `10^level MB` (L1=10MB, L2=100MB, L3=1GB …).
- Compaction reads all input SSTables, merges them via a k-way min-heap, discards older versions of duplicate keys, and drops tombstones at the bottommost level.

## Running Tests

```bash
cargo test
```

16 unit tests covering: Bloom filter, MemTable, WAL CRC, SSTable read/write, compaction merge logic, end-to-end DB operations (put/get/delete/scan/flush/crash-recovery).

## License

MIT
