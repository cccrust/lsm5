//! Iterator 範例
//!
//! 展示如何使用 Iterator 遍歷資料

use lsm5::{db::Config, Lsm5};
use std::fs;

fn main() {
    let dir = "./example_data_iter";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let config = Config::new(dir).memtable_size_limit(1024);
    let mut db = Lsm5::open(config).unwrap();

    println!("=== Iterator 範例 ===\n");

    // 寫入測試資料
    for i in 0..20u32 {
        db.put(format!("user{:02}", i), format!("User{}", i))
            .unwrap();
    }
    db.flush().unwrap();
    println!("寫入 20 筆 user 資料並 flush\n");

    // 使用 iterator 遍歷部分範圍
    println!("迭代 key 'user05' 到 'user12':");
    let iter = db.iterator(Some(b"user05"), Some(b"user12"));
    for (key, value) in iter {
        println!(
            "  {} => {}",
            String::from_utf8_lossy(&key),
            String::from_utf8_lossy(&value)
        );
    }

    // 使用 reverse iterator
    println!("\n反向迭代 key 'user15' 到 'user10':");
    let mut iter = db.reverse_iterator(Some(b"user10"), Some(b"user15"));
    while let Some((key, value)) = iter.next() {
        println!(
            "  {} => {}",
            String::from_utf8_lossy(&key),
            String::from_utf8_lossy(&value)
        );
    }

    // 使用 LsmIterator
    println!("\n使用 LsmIterator (seek 功能):");
    let mut iter = db.iterator(None, None);
    iter.seek(b"user10");
    if let Some(key) = iter.key() {
        println!("  找到 key: {}", String::from_utf8_lossy(key));
    }
    if let Some(value) = iter.value() {
        println!("  對應 value: {}", String::from_utf8_lossy(value));
    }

    // 統計
    println!("\n{}", db.stats());

    // 清理
    let _ = fs::remove_dir_all(dir);
    println!("\n範例完成！");
}
