use lsm5::{db::Config, Lsm5};
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::tempdir;

fn open_db(dir: &std::path::Path) -> Lsm5 {
    Lsm5::open(Config::new(dir).memtable_size_limit(1024)).unwrap()
}

#[test]
fn test_sequential_writes_with_mutex() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.put("initial", "value").unwrap();

    let db = Arc::new(Mutex::new(db));

    let db1 = Arc::clone(&db);
    let h1 = thread::spawn(move || {
        let mut db = db1.lock().unwrap();
        for i in 0..50 {
            db.put(format!("key-a-{}", i), format!("val-{}", i))
                .unwrap();
        }
    });

    let db2 = Arc::clone(&db);
    let h2 = thread::spawn(move || {
        let mut db = db2.lock().unwrap();
        for i in 50..100 {
            db.put(format!("key-b-{}", i), format!("val-{}", i))
                .unwrap();
        }
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let db = db.lock().unwrap();
    assert!(db.get("key-a-0").unwrap().is_some());
    assert!(db.get("key-b-99").unwrap().is_some());
}

#[test]
fn test_concurrent_reads_same_key() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.put("shared-key", "shared-value").unwrap();

    let db = Arc::new(Mutex::new(db));

    let handles: Vec<_> = (0..5)
        .map(|_| {
            let db = Arc::clone(&db);
            thread::spawn(move || {
                let db = db.lock().unwrap();
                db.get("shared-key").unwrap()
            })
        })
        .collect();

    for handle in handles {
        let result = handle.join().unwrap();
        assert_eq!(result, Some(b"shared-value".to_vec()));
    }
}

#[test]
fn test_mixed_read_write_with_mutex() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in 0..20 {
        db.put(format!("key-{}", i), format!("value-{}", i))
            .unwrap();
    }

    let db = Arc::new(Mutex::new(db));

    let reader = {
        let db = Arc::clone(&db);
        thread::spawn(move || {
            let db = db.lock().unwrap();
            for i in 0..20 {
                let _ = db.get(format!("key-{}", i));
            }
        })
    };

    let writer = {
        let db = Arc::clone(&db);
        thread::spawn(move || {
            let mut db = db.lock().unwrap();
            for i in 20..40 {
                db.put(format!("key-{}", i), format!("value-{}", i))
                    .unwrap();
            }
        })
    };

    reader.join().unwrap();
    writer.join().unwrap();

    let db = db.lock().unwrap();
    for i in 0..40 {
        assert!(db.get(format!("key-{}", i)).unwrap().is_some());
    }
}
