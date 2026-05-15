use lsm5::{db::Config, Lsm5};
use tempfile::tempdir;

fn open_db(dir: &std::path::Path) -> Lsm5 {
    Lsm5::open(Config::new(dir).memtable_size_limit(1024)).unwrap()
}

#[test]
fn test_iterator_basic() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in 0..10u32 {
        db.put(format!("key{:02}", i), format!("value{}", i))
            .unwrap();
    }

    let iter = db.iterator(Some(b"key02"), Some(b"key07"));
    let keys: Vec<_> = iter.map(|(k, _)| k).collect();

    assert_eq!(keys.len(), 5);
    assert!(keys[0].starts_with(b"key02"));
    assert!(keys[4].starts_with(b"key06"));
}

#[test]
fn test_iterator_full_range() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.put("a", "1").unwrap();
    db.put("b", "2").unwrap();
    db.put("c", "3").unwrap();
    db.put("d", "4").unwrap();
    db.put("e", "5").unwrap();

    let iter = db.iterator(None, None);
    let count = iter.count();
    assert_eq!(count, 5);
}

#[test]
fn test_reverse_iterator() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in 0..5u32 {
        db.put(format!("key{}", i), format!("val{}", i)).unwrap();
    }

    let mut iter = db.reverse_iterator(Some(b"key0"), Some(b"key5"));
    let first = iter.next().unwrap();
    assert_eq!(first.0, b"key4");
}

#[test]
fn test_iterator_with_flush() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in 0..200u32 {
        db.put(format!("k{:03}", i), format!("v{}", i)).unwrap();
    }

    db.flush().unwrap();

    let iter = db.iterator(Some(b"k100"), Some(b"k150"));
    let count = iter.count();
    assert!(count > 0);
}

#[test]
fn test_iterator_empty_range() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.put("a", "1").unwrap();

    let iter = db.iterator(Some(b"z"), None);
    let count = iter.count();
    assert_eq!(count, 0);
}

#[test]
fn test_iterator_all_deleted() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.put("a", "1").unwrap();
    db.put("b", "2").unwrap();
    db.delete("a").unwrap();
    db.delete("b").unwrap();

    let iter = db.iterator(None, None);
    let count = iter.count();
    assert_eq!(count, 0);
}
