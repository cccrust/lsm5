//! 事務範例
//!
//! 展示如何使用 BEGIN/COMMIT/ROLLBACK 事務

use lsm5::{db::Config, Lsm5};
use std::fs;

fn main() {
    let dir = "./example_data_tx";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let config = Config::new(dir).memtable_size_limit(1024);
    let mut db = Lsm5::open(config).unwrap();

    println!("=== 事務範例 ===\n");

    // 初始資料
    db.put("initial", "data").unwrap();
    println!("初始資料: initial => data");

    // 開始事務並寫入
    db.begin();
    db.tx_put("key1", "value1").unwrap();
    db.tx_put("key2", "value2").unwrap();
    db.tx_put("key3", "value3").unwrap();
    println!("\n事務中寫入 3 筆資料");

    // 提交事務
    db.commit().unwrap();
    println!("提交事務");

    // 驗證資料
    println!("\n驗證提交後的資料:");
    println!("  key1 => {:?}", db.get("key1").unwrap());
    println!("  key2 => {:?}", db.get("key2").unwrap());
    println!("  key3 => {:?}", db.get("key3").unwrap());

    // 新事務 - 示範回滾
    println!("\n=== 示範回滾 ===");
    db.begin();
    db.tx_put("will_be_rollback", "this_will_be_removed")
        .unwrap();
    db.tx_put("another", "value").unwrap();
    println!("事務中寫入 2 筆資料 (即將回滾)");

    // 回滾事務
    db.rollback().unwrap();
    println!("回滾事務");

    // 驗證回滾後沒有這些資料
    println!("\n驗證回滾後的資料:");
    println!(
        "  will_be_rollback => {:?}",
        db.get("will_be_rollback").unwrap()
    );
    println!("  another => {:?}", db.get("another").unwrap());
    println!("  initial => {:?}", db.get("initial").unwrap());

    // 統計
    println!("\n{}", db.stats());

    // 清理
    let _ = fs::remove_dir_all(dir);
    println!("\n範例完成！");
}
