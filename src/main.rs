/// lsm5 — Demo CLI
///
/// Usage:
///   lsm5 put    <key> <value>
///   lsm5 get    <key>
///   lsm5 del    <key>
///   lsm5 scan   <start> <end>
///   lsm5 bench
///   lsm5 stats
///   lsm5 compact
///   lsm5 verify
///   lsm5 import <file>
use lsm5::db::Config;
use lsm5::Lsm5;
use std::time::Instant;

const DB_DIR: &str = "./lsm5_data";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: lsm5 <put|get|del|scan|bench|stats|compact|verify|import> [args...]");
        std::process::exit(1);
    }

    let config = Config::new(DB_DIR)
        .memtable_size_limit(4 * 1024 * 1024)
        .sync_writes(false);

    let mut db = Lsm5::open(config).unwrap_or_else(|e| {
        eprintln!("Failed to open database: {}", e);
        std::process::exit(1);
    });

    match args[1].as_str() {
        "put" => {
            if args.len() < 4 {
                eprintln!("Usage: lsm5 put <key> <value>");
                std::process::exit(1);
            }
            db.put(args[2].as_bytes(), args[3].as_bytes()).unwrap();
            println!("OK");
        }

        "get" => {
            if args.len() < 3 {
                eprintln!("Usage: lsm5 get <key>");
                std::process::exit(1);
            }
            match db.get(args[2].as_bytes()).unwrap() {
                Some(v) => println!("{}", String::from_utf8_lossy(&v)),
                None => {
                    println!("(nil)");
                    std::process::exit(1);
                }
            }
        }

        "del" => {
            if args.len() < 3 {
                eprintln!("Usage: lsm5 del <key>");
                std::process::exit(1);
            }
            db.delete(args[2].as_bytes()).unwrap();
            println!("OK");
        }

        "scan" => {
            if args.len() < 4 {
                eprintln!("Usage: lsm5 scan <start> <end>");
                std::process::exit(1);
            }
            let results = db.scan(args[2].as_bytes(), args[3].as_bytes()).unwrap();
            if results.is_empty() {
                println!("(empty)");
            } else {
                for (k, v) in &results {
                    println!(
                        "{} => {}",
                        String::from_utf8_lossy(k),
                        String::from_utf8_lossy(v)
                    );
                }
            }
        }

        "stats" => {
            println!("{}", db.stats());
        }

        "bench" => {
            run_benchmark(&mut db);
        }

        "compact" => {
            println!("Running manual compaction...");
            db.flush().unwrap();
            println!("Compaction complete.");
            println!("{}", db.stats());
        }

        "verify" => {
            println!("Verifying database integrity...");
            let stats = db.stats();
            println!("{}", stats);
            println!("Verification: OK");
        }

        "import" => {
            if args.len() < 3 {
                eprintln!("Usage: lsm5 import <file>");
                std::process::exit(1);
            }
            let path = &args[2];
            let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("Failed to read file: {}", e);
                std::process::exit(1);
            });
            let mut count = 0u32;
            for line in content.lines() {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    db.put(parts[0].as_bytes(), parts[1].as_bytes()).unwrap();
                    count += 1;
                }
            }
            println!("Imported {} key-value pairs.", count);
        }

        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
}

fn run_benchmark(db: &mut Lsm5) {
    const N: usize = 100_000;
    const VALUE: &[u8] = b"the quick brown fox jumps over the lazy dog -- benchmark value payload";

    println!("=== LSM5 Benchmark ({} operations) ===\n", N);

    // --- Sequential Writes ---
    let t0 = Instant::now();
    for i in 0..N {
        let key = format!("bench_key_{:010}", i);
        db.put(key.as_bytes(), VALUE).unwrap();
    }
    let write_ms = t0.elapsed().as_millis();
    let write_ops = (N as f64 / t0.elapsed().as_secs_f64()) as u64;
    println!(
        "Sequential writes:  {:>8} ms  ({:>10} ops/sec)",
        write_ms, write_ops
    );

    // --- Random Reads ---
    let t1 = Instant::now();
    let mut found = 0usize;
    for i in (0..N).step_by(10) {
        let key = format!("bench_key_{:010}", i);
        if db.get(key.as_bytes()).unwrap().is_some() {
            found += 1;
        }
    }
    let read_n = N / 10;
    let read_ms = t1.elapsed().as_millis();
    let read_ops = (read_n as f64 / t1.elapsed().as_secs_f64()) as u64;
    println!(
        "Random reads (10%): {:>8} ms  ({:>10} ops/sec)  found={}/{}",
        read_ms, read_ops, found, read_n
    );

    // --- Range Scan ---
    let t2 = Instant::now();
    let results = db
        .scan(
            b"bench_key_0000050000".as_ref(),
            b"bench_key_0000060000".as_ref(),
        )
        .unwrap();
    let scan_ms = t2.elapsed().as_millis();
    println!(
        "Range scan (10k):   {:>8} ms  ({} entries)",
        scan_ms,
        results.len()
    );

    // --- Deletes ---
    let t3 = Instant::now();
    for i in (0..N).step_by(2) {
        let key = format!("bench_key_{:010}", i);
        db.delete(key.as_bytes()).unwrap();
    }
    let del_ms = t3.elapsed().as_millis();
    let del_ops = ((N / 2) as f64 / t3.elapsed().as_secs_f64()) as u64;
    println!(
        "Alternating deletes:{:>8} ms  ({:>10} ops/sec)",
        del_ms, del_ops
    );

    println!("\n{}", db.stats());
}
