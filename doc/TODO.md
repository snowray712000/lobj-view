# TODO

## 重要
- [ ] 取得 lobj 二進位格式規格（檔頭、各區段排列、label 型別），實作解析器並讓檢視器能開啟 .lobj 檔
- [ ] 後端提供讀取 lobj 檔的 API，前端載入後依 label 上色
- [ ] 建立 git 版本控制（目前專案尚未 git init）
- [x] 驗證 release build（Windows、Linux musl 單一執行檔皆可獨立執行，見 261118b）
- [ ] 驗證 `cargo install`（確認 web/dist 有正確嵌入 crate 包內）

## 次要
- [ ] 格式確定後，考慮把 lobj 讀寫拆成獨立 crate 以便共用
- [ ] 命令列直接指定檔案：`lobj-view model.lobj`
- [ ] 監看檔案變更並自動重新載入畫面
- [x] 實測遠端 `ssh -L` 使用流程與 `--host 0.0.0.0` 模式（ssh -L 可用；0.0.0.0 會失去 WebGPU，符合預期）
- [ ] 實測 WebGL2 退回路徑與 `npm run dev` 開發流程
- [ ] 視需要加入 GitHub Actions 發佈流程（多平台 build、tag 自動 release；需先建立 remote）
- [ ] 視需要加入 release profile（strip / lto）縮小執行檔
