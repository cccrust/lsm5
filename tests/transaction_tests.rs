use lsm5::{db::Config, Lsm5};
use tempfile::tempdir;

fn open_db(dir: &std::path::Path) -> Lsm5 {
    Lsm5::open(Config::new(dir).memtable_size_limit(1024)).unwrap()
}

#[test]
fn test_transaction_basic() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.begin();
    db.tx_put("key1", "value1").unwrap();
    db.tx_put("key2", "value2").unwrap();
    db.commit().unwrap();

    assert_eq!(db.get("key1").unwrap(), Some(b"value1".to_vec()));
    assert_eq!(db.get("key2").unwrap(), Some(b"value2".to_vec()));
}

#[test]
fn test_transaction_rollback() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.begin();
    db.tx_put("key1", "value1").unwrap();
    db.tx_put("key2", "value2").unwrap();
    db.rollback().unwrap();

    assert_eq!(db.get("key1").unwrap(), None);
    assert_eq!(db.get("key2").unwrap(), None);
}

#[test]
fn test_transaction_delete() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.put("key1", "value1").unwrap();

    db.begin();
    db.tx_delete("key1").unwrap();
    db.commit().unwrap();

    assert_eq!(db.get("key1").unwrap(), None);
}

#[test]
fn test_transaction_rollback_middle() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.put("initial", "data").unwrap();

    db.begin();
    db.tx_put("new1", "v1").unwrap();
    db.tx_put("new2", "v2").unwrap();

    assert!(db.has_transaction());

    db.rollback().unwrap();

    assert!(!db.has_transaction());
    assert_eq!(db.get("initial").unwrap(), Some(b"data".to_vec()));
    assert_eq!(db.get("new1").unwrap(), None);
}

#[test]
fn test_multiple_transactions() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.begin();
    db.tx_put("a", "1").unwrap();
    db.commit().unwrap();

    db.begin();
    db.tx_put("b", "2").unwrap();
    db.commit().unwrap();

    assert_eq!(db.get("a").unwrap(), Some(b"1".to_vec()));
    assert_eq!(db.get("b").unwrap(), Some(b"2".to_vec()));
}

#[test]
fn test_transaction_empty() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.begin();
    db.commit().unwrap();

    assert!(!db.has_transaction());
}

#[test]
fn test_transaction_no_begin() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    let result = db.tx_put("key", "value");
    assert!(result.is_err());

    let result = db.commit();
    assert!(result.is_err());
}

#[test]
fn test_transaction_mixed() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.put("outside", "tx").unwrap();

    db.begin();
    db.tx_put("inside1", "v1").unwrap();
    db.tx_put("inside2", "v2").unwrap();
    db.commit().unwrap();

    assert_eq!(db.get("outside").unwrap(), Some(b"tx".to_vec()));
    assert_eq!(db.get("inside1").unwrap(), Some(b"v1".to_vec()));
    assert_eq!(db.get("inside2").unwrap(), Some(b"v2".to_vec()));
}
