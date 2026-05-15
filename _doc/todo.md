不足	說明
CRC 校驗	資料無 integrity check，無法偵測損壞
壓力測試	無 fuzz testing / chaos testing
錯誤復原	panic 時可能資料遺失
效能優化	缺少效能 profiling / tuning
文件	無 API 文件 (doc comments)
實用功能缺口
不足	說明
壓縮格式	只有 zstd，可加 lz4 (更快)
範圍壓縮	可指定 key 範圍壓縮
Snapshots	時間點快照
TTL	自動過期 keys
穩定性缺口
不足	說明
Bounds checking	記憶體可能爆衝
優雅關機	Ctrl+C 時 flush
監控儀表板	只有 HTTP JSON，無圖形
建議優先順序:
1. 
CRC 校驗 - 資料完整性最重要
2. 
錯誤處理 - 讓 library 更安全
3. 
效能優化 - 實際 benchmark
要實作哪個？