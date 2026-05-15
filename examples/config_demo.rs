//! 配置範例
//!
//! 展示如何自訂 lsm5 的配置參數

use lsm5::{db::Config, Lsm5};
use std::fs;

fn main() {
    let dir = "./example_data_config";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    println!("=== 配置範例 ===\n");

    // 預設配置
    println!("1. 使用預設配置:");
    let config = Config::new(dir);
    println!("   {}", config.dir.display());
    println!(
        "   memtable_size_limit: {} bytes",
        config.memtable_size_limit
    );
    println!("   max_sstable_size: {} bytes", config.max_sstable_size);
    println!("   sync_writes: {}", config.sync_writes);

    // 自訂配置
    println!("\n2. 使用自訂配置:");
    let custom_config = Config::new(dir)
        .memtable_size_limit(1024 * 1024) // 1 MB
        .max_sstable_size(32 * 1024 * 1024) // 32 MB
        .sync_writes(true) // 每次寫入後同步
        .l0_compaction_trigger(4)
        .level_size_multiplier(10);

    println!(
        "   memtable_size_limit: {} bytes",
        custom_config.memtable_size_limit
    );
    println!(
        "   max_sstable_size: {} bytes",
        custom_config.max_sstable_size
    );
    println!("   sync_writes: {}", custom_config.sync_writes);

    // 使用自訂配置開啟資料庫
    let mut db = Lsm5::open(custom_config).unwrap();

    // 寫入大量資料觀察壓縮
    println!("\n3. 寫入 100 筆資料觀察統計:");
    for i in 0..100 {
        db.put(format!("key{}", i), format!("value{}", i)).unwrap();
    }

    println!("{}", db.stats());

    // 再次 flush 觀察
    db.flush().unwrap();
    println!("\nFlush 後:");
    println!("{}", db.stats());

    // 清理
    let _ = fs::remove_dir_all(dir);
    println!("\n範例完成！");
}
