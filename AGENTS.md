# AGENTS.md

## Project
Rust LSM-Tree storage engine (library + CLI binary)

## Key Commands
- `cargo test` - run all tests (unit in `src/` + integration in `tests/`)
- `./test.sh` - runs test + clippy + fmt (the full verification)
- `cargo build --release` - build CLI binary to `./target/release/lsm5`

## CLI Usage
```bash
./target/release/lsm5 put   <key> <value>
./target/release/lsm5 get   <key>
./target/release/lsm5 del   <key>
./target/release/lsm5 scan  <start> <end>
./target/release/lsm5 stats
./target/release/lsm5 bench  # 100k-op benchmark
```

## Architecture
- Library entry: `src/lib.rs` → `Lsm5` struct in `src/db.rs`
- CLI entry: `src/main.rs`
- Core modules: `memtable`, `wal`, `sstable`, `bloom`, `compaction`, `db`

## Testing
- Unit tests inline in `src/*.rs`
- Integration tests in `tests/` (concurrent, iterator, performance, system, transaction)
- Tests use `tempfile` crate (dev-dependency)

## Notes
- Release build uses LTO + single codegen-unit for optimization
- No external crates in dependencies (pure std)