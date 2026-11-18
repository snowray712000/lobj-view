# TODO

## 重要
- [ ] 取得 lobj 二進位格式規格（檔頭、各區段排列、label 型別），實作解析器並讓檢視器能開啟 .lobj 檔
- [ ] 後端提供讀取 lobj 檔的 API，前端載入後依 label 上色
- [ ] 建立 git 版本控制（目前專案尚未 git init）
- [ ] 驗證 release build 與 `cargo install`（確認 web/dist 有正確嵌入）

## 次要
- [ ] 格式確定後，考慮把 lobj 讀寫拆成獨立 crate 以便共用
- [ ] 命令列直接指定檔案：`lobj-view model.lobj`
- [ ] 監看檔案變更並自動重新載入畫面
- [ ] 實測遠端 `ssh -L` 使用流程與 `--host 0.0.0.0` 模式
- [ ] 實測 WebGL2 退回路徑與 `npm run dev` 開發流程
