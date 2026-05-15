# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [v0.18.0] - 2026-05-15

### Added
- API documentation for all public functions
- `examples/batch_ops.rs` - batch write example
- CHANGELOG.md established

### Changed
- README.md updated with complete feature list

## [v0.17.0] - 2026-05-15

### Added
- `perf` module with `Timer` and `PerfStats` for performance tracking
- Performance summary output

### Changed
- Improved test coverage

## [v0.16.0] - 2026-05-15

### Added
- New error types: `DatabaseFull`, `MemoryExceeded`, `SnapshotNotFound`, `TaskCancelled`
- `catch_panic()` utility function for panic recovery

## [v0.15.0] - 2026-05-15

### Added
- CRC32 checksum for SSTable records
- Data integrity verification on read
- `Error::Corruption` for detected data corruption

### Changed
- SSTable record format now includes 4-byte CRC32

## [v0.14.0] - 2026-05-15

### Added
- `write_batch()` for efficient bulk writes
- `analyze_scan()` for query planning
- `write_buffer_size` and `write_buffer_flush_threshold` configuration
- `query_plan` module with `ScanPlan` struct

## [v0.13.0] - 2026-05-15

### Added
- `background` module with `BackgroundWorker` for async operations
- `background_threads` configuration

## [v0.12.0] - 2026-05-15

### Added
- `scan_cache` module for range scan optimization
- `ScanCache` with LRU eviction
- `scan_cache_size` configuration

## [v0.11.0] - 2026-05-15

### Added
- HTTP monitoring server
- `/stats`, `/cache`, `/health` endpoints
- `monitoring_enabled` and `monitoring_port` configuration

## [v0.10.0] - 2026-05-15

### Added
- Zstd compression for SSTable data blocks
- `compression_enabled` and `compression_level` configuration
- Compression threshold (1KB minimum)

## [v0.9.0] - 2026-05-15

### Added
- LRU cache for SSTable data
- Cache hit/miss statistics
- `cache_size` configuration

## [v0.8.0] - 2026-05-15

### Added
- `Config::new()` builder pattern
- `stats()` method with DbStats struct
- Database statistics tracking

## [v0.7.0] - 2026-05-15

### Added
- `db.flush()` method for manual memtable flush
- `flush_memtable()` internal method

## [v0.6.0] - 2026-05-15

### Added
- `db.stats()` method
- Configurable compaction parameters

## [v0.5.0] - 2026-05-15

### Added
- Iterator support (forward and reverse)
- `db.iterator()` and `db.reverse_iterator()`
- Iterator tests

## [v0.4.0] - 2026-05-15

### Added
- Transaction support (BEGIN/COMMIT/ROLLBACK)
- `db.begin()`, `db.commit()`, `db.rollback()`
- `transaction` module

## [v0.3.0] - 2026-05-15

### Added
- Leveled compaction
- `compact()` function
- Tombstone propagation

## [v0.2.0] - 2026-05-15

### Added
- SSTable support
- Bloom filter
- WAL replay

## [v0.1.0] - 2026-05-15

### Added
- Initial release
- Basic CRUD (put/get/delete/scan)
- MemTable (BTreeMap in-memory buffer)
- WAL (Write-Ahead Log) with CRC32
- SSTable format with data, index, and bloom filter blocks
- Leveled compaction strategy
- Configuration via `Config::new()`

<!-- Genera
 -->
[v0.18.0]: https://github.com/ccc/lsm5/compare/v0.17.0...v0.18.0
[v0.17.0]: https://github.com/ccc/lsm5/compare/v0.16.0...v0.17.0
[v0.16.0]: https://github.com/ccc/lsm5/compare/v0.15.0...v0.16.0
[v0.15.0]: https://github.com/ccc/lsm5/compare/v0.14.0...v0.15.0
[v0.14.0]: https://github.com/ccc/lsm5/compare/v0.13.0...v0.14.0
[v0.13.0]: https://github.com/ccc/lsm5/compare/v0.12.0...v0.13.0
[v0.12.0]: https://github.com/ccc/lsm5/compare/v0.11.0...v0.12.0
[v0.11.0]: https://github.com/ccc/lsm5/compare/v0.10.0...v0.11.0
[v0.10.0]: https://github.com/ccc/lsm5/compare/v0.9.0...v0.10.0
[v0.9.0]: https://github.com/ccc/lsm5/compare/v0.8.0...v0.9.0
[v0.8.0]: https://github.com/ccc/lsm5/compare/v0.7.0...v0.8.0
[v0.7.0]: https://github.com/ccc/lsm5/compare/v0.6.0...v0.7.0
[v0.6.0]: https://github.com/ccc/lsm5/compare/v0.5.0...v0.6.0
[v0.5.0]: https://github.com/ccc/lsm5/compare/v0.4.0...v0.5.0
[v0.4.0]: https://github.com/ccc/lsm5/compare/v0.3.0...v0.4.0
[v0.3.0]: https://github.com/ccc/lsm5/compare/v0.2.0...v0.3.0
[v0.2.0]: https://github.com/ccc/lsm5/compare/v0.1.0...v0.2.0