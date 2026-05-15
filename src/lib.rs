// lsm5 - A Log-Structured Merge Tree implementation in Rust
//
// Architecture:
//   MemTable  (in-memory BTreeMap, write buffer)
//     ↓  flush when full
//   Level-0 SSTables  (sorted, may have overlaps)
//     ↓  compaction
//   Level-1..N SSTables  (sorted, no overlaps within a level)

pub mod background;
pub mod bloom;
pub mod cache;
pub mod compaction;
pub mod config;
pub mod db;
pub mod error;
pub mod iterator;
pub mod memtable;
pub mod monitoring;
pub mod sstable;
pub mod stats;
pub mod transaction;
pub mod wal;

pub use db::{Config, DbStats, Lsm5};
pub use error::{Error, Result};
pub use iterator::LsmIterator;
pub use transaction::{Operation, Transaction};
