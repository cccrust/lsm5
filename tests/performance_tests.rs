use lsm5::{db::Config, Lsm5};
use std::time::Instant;
use tempfile::tempdir;

fn open_db(dir: &std::path::Path) -> Lsm5 {
    Lsm5::open(Config::new(dir).memtable_size_limit(1024 * 1024)).unwrap()
}

#[test]
fn perf_write_throughput() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    let n = 10_000;
    let start = Instant::now();

    for i in 0..n {
        db.put(format!("key{}", i), format!("value{}", i)).unwrap();
    }

    let elapsed = start.elapsed().as_secs_f64();
    let ops_per_sec = n as f64 / elapsed;

    println!("Write throughput: {:.0} ops/sec", ops_per_sec);
    assert!(ops_per_sec > 0.0, "Should have positive throughput");
}

#[test]
fn perf_read_latency() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in 0..1000 {
        db.put(format!("key{}", i), format!("value{}", i)).unwrap();
    }

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = db.get("key500").unwrap();
    }
    let elapsed = start.elapsed().as_millis();

    println!("Read latency (1000 reads): {} ms", elapsed);
    assert!(elapsed < 5000, "Reads should complete in reasonable time");
}

#[test]
fn perf_scan_throughput() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in 0..1000 {
        db.put(format!("key{:04}", i), format!("value{}", i))
            .unwrap();
    }

    let start = Instant::now();
    let count = db.scan(b"key0000", b"key1000").unwrap().len();
    let elapsed = start.elapsed().as_millis();

    println!("Scan {} entries in {} ms", count, elapsed);
    assert!(count > 0);
}

#[test]
fn perf_many_sstables() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for _ in 0..10 {
        for i in 0..50 {
            db.put(format!("key{}", i), "value").unwrap();
        }
        db.flush().unwrap();
    }

    let stats = db.stats();
    println!("After 10 flushes: {} SSTables", stats.total_sstables());

    let start = Instant::now();
    for i in 0..50 {
        let _ = db.get(format!("key{}", i)).unwrap();
    }
    let elapsed = start.elapsed().as_millis();

    println!("50 reads across 10 levels: {} ms", elapsed);
    assert!(elapsed < 1000);
}

#[test]
fn perf_reopen_persistence() {
    let dir = tempdir().unwrap();
    {
        let mut db = open_db(dir.path());
        for i in 0..100 {
            db.put(format!("key{}", i), format!("value{}", i)).unwrap();
        }
        db.flush().unwrap();
    }

    let start = Instant::now();
    let db = open_db(dir.path());
    let elapsed = start.elapsed().as_millis();

    for i in 0..100 {
        assert!(db.get(format!("key{}", i)).unwrap().is_some());
    }

    println!("Reopen and verify 100 keys: {} ms", elapsed);
}

#[test]
fn perf_delete_performance() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in 0..500 {
        db.put(format!("key{}", i), "value").unwrap();
    }

    let start = Instant::now();
    for i in (0..500).step_by(2) {
        db.delete(format!("key{}", i)).unwrap();
    }
    let elapsed = start.elapsed().as_millis();

    println!("Delete 250 keys: {} ms", elapsed);
    assert!(elapsed < 2000);
}
