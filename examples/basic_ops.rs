//! 基本操作範例
//!
//! 展示 lsm5 的基本 CRUD 操作

use lsm5::{db::Config, Lsm5};
use std::fs;

fn main() {
    let dir = "./example_data_basic";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let config = Config::new(dir).memtable_size_limit(1024);
    let mut db = Lsm5::open(config).unwrap();

    println!("=== 基本操作範例 ===\n");

    // PUT - 寫入資料
    db.put("name", "Alice").unwrap();
    db.put("age", "25").unwrap();
    db.put("city", "Taipei").unwrap();
    println!("寫入 3 筆資料: name, age, city");

    // GET - 讀取資料
    let name = db.get("name").unwrap();
    println!("讀取 name: {:?}", name);

    let age = db.get("age").unwrap();
    println!("讀取 age: {:?}", age);

    // UPDATE - 更新資料
    db.put("age", "26").unwrap();
    let age = db.get("age").unwrap();
    println!("更新 age 後: {:?}", age);

    // DELETE - 刪除資料
    db.delete("city").unwrap();
    let city = db.get("city").unwrap();
    println!("刪除 city 後: {:?}", city);

    // SCAN - 範圍查詢
    println!("\n掃描所有 key:");
    let results = db.scan(b"a", b"z").unwrap();
    for (k, v) in &results {
        println!(
            "  {} => {}",
            String::from_utf8_lossy(k),
            String::from_utf8_lossy(v)
        );
    }

    // STATS - 統計資訊
    println!("\n{}", db.stats());

    // 清理
    let _ = fs::remove_dir_all(dir);
    println!("\n範例完成！");
}
