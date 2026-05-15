use lsm5::{db::Config, Lsm5};
use std::path::Path;
use tempfile::tempdir;

fn open_db(dir: &Path) -> Lsm5 {
    Lsm5::open(Config::new(dir).memtable_size_limit(512)).unwrap()
}

#[test]
fn test_workflow_sequential_operations() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in 0..100u32 {
        db.put(format!("key{}", i), format!("value{}", i)).unwrap();
    }

    for i in 0..100u32 {
        assert_eq!(
            db.get(format!("key{}", i)).unwrap(),
            Some(format!("value{}", i).into_bytes())
        );
    }
}

#[test]
fn test_workflow_mixed_operations() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    db.put("a", "1").unwrap();
    db.put("b", "2").unwrap();
    db.put("c", "3").unwrap();
    db.delete("b").unwrap();
    db.put("b", "new_b").unwrap();
    db.put("d", "4").unwrap();

    assert_eq!(db.get("a").unwrap(), Some(b"1".to_vec()));
    assert_eq!(db.get("b").unwrap(), Some(b"new_b".to_vec()));
    assert_eq!(db.get("c").unwrap(), Some(b"3".to_vec()));
    assert_eq!(db.get("d").unwrap(), Some(b"4".to_vec()));
}

#[test]
fn test_workflow_reopen_with_data() {
    let dir = tempdir().unwrap();
    {
        let mut db = open_db(dir.path());
        db.put("key1", "val1").unwrap();
        db.put("key2", "val2").unwrap();
        db.flush().unwrap();
    }

    {
        let db = open_db(dir.path());
        assert_eq!(db.get("key1").unwrap(), Some(b"val1".to_vec()));
        assert_eq!(db.get("key2").unwrap(), Some(b"val2".to_vec()));
    }
}

#[test]
fn test_workflow_compaction_trigger() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in 0..500u32 {
        db.put(format!("key{:04}", i), format!("value{:04}", i))
            .unwrap();
    }

    let stats = db.stats();
    assert!(stats.level_counts[0] > 0 || stats.memtable_entries > 0);

    for i in 0..500u32 {
        assert!(db.get(format!("key{:04}", i)).unwrap().is_some());
    }
}

#[test]
fn test_workflow_range_scan_across_levels() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in (0..300).step_by(3) {
        db.put(format!("key{:03}", i), format!("val{:03}", i))
            .unwrap();
    }
    db.flush().unwrap();
    for i in (1..300).step_by(3) {
        db.put(format!("key{:03}", i), format!("val{:03}", i))
            .unwrap();
    }
    db.flush().unwrap();
    for i in (2..300).step_by(3) {
        db.put(format!("key{:03}", i), format!("val{:03}", i))
            .unwrap();
    }

    let results = db.scan("key000", "key150").unwrap();
    assert!(results.len() > 0);

    for (k, _v) in &results {
        let key_str = String::from_utf8_lossy(k);
        let idx: usize = key_str[3..].parse().unwrap();
        assert!(idx < 150);
    }
}

#[test]
fn test_workflow_delete_all_keys() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in 0..50u32 {
        db.put(format!("k{}", i), "v").unwrap();
    }

    for i in 0..50u32 {
        db.delete(format!("k{}", i)).unwrap();
    }

    for i in 0..50u32 {
        assert_eq!(db.get(format!("k{}", i)).unwrap(), None);
    }

    let results = db.scan("k0", "k99").unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_workflow_bulk_insert_and_verify() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    let mut expected = std::collections::BTreeMap::new();
    for i in 0..100u32 {
        let key = format!("key{:04}", i);
        let value = format!("value{:04}", i);
        expected.insert(key.clone(), value.clone());
        db.put(key, value).unwrap();
    }

    for (k, v) in &expected {
        let result = db.get(k).unwrap();
        assert_eq!(result, Some(v.as_bytes().to_vec()), "Key {}", k);
    }
}

#[test]
fn test_workflow_interleaved_read_write() {
    let dir = tempdir().unwrap();
    let mut db = open_db(dir.path());

    for i in 0..20u32 {
        db.put(format!("key{}", i), format!("val{}", i)).unwrap();
        assert_eq!(
            db.get(format!("key{}", i)).unwrap(),
            Some(format!("val{}", i).into_bytes())
        );
    }

    db.flush().unwrap();

    for i in 0..20u32 {
        assert_eq!(
            db.get(format!("key{}", i)).unwrap(),
            Some(format!("val{}", i).into_bytes())
        );
    }
}
