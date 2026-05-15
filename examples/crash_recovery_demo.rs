//! 崩潰復原範例
//!
//! 展示 lsm5 的 WAL 機制如何保護資料

use lsm5::{db::Config, Lsm5};
use std::fs;

fn main() {
    let dir = "./example_data_recovery";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    println!("=== 崩潰復原範例 ===\n");

    // 模擬場景 1: 資料只寫入 WAL，還沒 flush 到 SSTable
    println!("1. 模擬程式崩潰 (資料在 WAL 中):");
    {
        let config = Config::new(dir).memtable_size_limit(1024 * 1024);
        let mut db = Lsm5::open(config).unwrap();

        db.put("important_data", "這筆資料很重要").unwrap();
        db.put("another_key", "another_value").unwrap();
        println!("   寫入 2 筆資料 (未 flush)");

        // 這裡模擬崩潰 - db 變數被 drop 但資料在 WAL 中
    }

    println!("   程式結束，模擬崩潰...\n");

    // 模擬場景 2: 重新開啟資料庫，WAL 會自動回放
    println!("2. 重新開啟資料庫 (WAL 會自動回放):");
    {
        let config = Config::new(dir).memtable_size_limit(1024 * 1024);
        let db = Lsm5::open(config).unwrap();

        let data = db.get("important_data").unwrap();
        println!("   讀取 important_data: {:?}", data);

        let another = db.get("another_key").unwrap();
        println!("   讀取 another_key: {:?}", another);
    }

    // 模擬場景 3: 部分資料已經 flush 到 SSTable
    println!("\n3. 模擬部分資料已 flush:");
    {
        let config = Config::new(dir).memtable_size_limit(100);
        let mut db = Lsm5::open(config).unwrap();

        // 寫入足夠觸發 flush 的資料
        for i in 0..50 {
            db.put(format!("key{}", i), format!("value{}", i)).unwrap();
        }
        println!("   寫入 50 筆資料 (已觸發 flush)");

        // 關閉前再寫入一些在 WAL 中
        db.put("just_before_close", "last_value").unwrap();
    }

    println!("   程式結束，模擬崩潰...\n");

    // 驗證資料完整性
    println!("4. 驗證資料完整性:");
    {
        let config = Config::new(dir).memtable_size_limit(100);
        let db = Lsm5::open(config).unwrap();

        // 檢查之前寫入的資料
        for i in 0..10 {
            let key = format!("key{}", i);
            let value = db.get(&key).unwrap();
            println!("   {} => {:?}", key, value);
        }

        let last = db.get("just_before_close").unwrap();
        println!("   just_before_close => {:?}", last);
    }

    println!("\n5. 檢查 SSTable 數量:");
    {
        let config = Config::new(dir).memtable_size_limit(100);
        let db = Lsm5::open(config).unwrap();
        let stats = db.stats();
        println!("   SSTables: {}", stats.total_sstables());
    }

    // 清理
    let _ = fs::remove_dir_all(dir);
    println!("\n範例完成！");
    println!("\n結論: lsm5 使用 WAL 確保資料在崩潰後可以復原");
}
