//! Batch Operations Example
//!
//! Demonstrates efficient bulk writes using `write_batch`.
//!
//! # Running
//!
//! ```bash
//! cargo run --example batch_ops
//! ```

use lsm5::{db::Config, Lsm5};
use std::time::Instant;

fn main() {
    let dir = tempfile::tempdir().unwrap();
    let config = Config::new(dir.path());
    let mut db = Lsm5::open(config).unwrap();

    println!("=== Batch Write Example ===\n");

    // Batch write 1000 records
    let start = Instant::now();
    let batch: Vec<_> = (0..1000)
        .map(|i| (format!("key{:04}", i), format!("value{:04}", i)))
        .collect();

    db.write_batch(batch).unwrap();
    let elapsed = start.elapsed();

    println!("Wrote 1000 records in {:?}", elapsed);
    println!("Throughput: {:.0} ops/sec", 1000.0 / elapsed.as_secs_f64());

    // Verify reads
    println!("\nVerifying reads...");
    let start = Instant::now();
    for i in 0..1000 {
        let key = format!("key{:04}", i);
        let val = db.get(&key).unwrap().unwrap();
        assert_eq!(val, format!("value{:04}", i).into_bytes());
    }
    println!("Verified 1000 reads in {:?}", start.elapsed());

    // Compare with individual writes
    println!("\nComparing with individual writes...");
    let mut db2 = Lsm5::open(Config::new(dir.path().join("db2"))).unwrap();

    let start = Instant::now();
    for i in 0..1000 {
        db2.put(format!("key{:04}", i), format!("value{:04}", i))
            .unwrap();
    }
    println!(
        "Individual writes: {:?} ({:.0} ops/sec)",
        start.elapsed(),
        1000.0 / start.elapsed().as_secs_f64()
    );
}
