# TODO

## 重要
- [ ] 取得 lobj 二進位格式規格（檔頭、各區段排列、label 型別），實作解析器並讓檢視器能開啟 .lobj 檔
- [ ] 後端提供讀取 lobj 檔的 API，前端載入後依 label 上色
- [ ] 建立 git 版本控制（目前專案尚未 git init）
- [x] 驗證 release build（Windows、Linux musl 單一執行檔皆可獨立執行，見 261118b）
- [ ] 驗證 `cargo install --git`（從原始碼編譯）與 `cargo package` 的 web/dist 是否正確帶入；`cargo binstall` 已驗證可用

- [x] 實機驗證 HTTPS：Windows 本機與區網 Mac mini 皆成功（見 261118d）
- [x] 反向測試：Ubuntu（無畫面）當 server、Windows / Mac 當 client，HTTPS 皆可使用（見 261118e）
- [x] Linux 單一執行檔：改走 GitHub Actions + cross，已可建出 Linux / Windows / macOS（見 261118e）

## 次要
- [ ] 格式確定後，考慮把 lobj 讀寫拆成獨立 crate 以便共用
- [ ] 命令列直接指定檔案：`lobj-view model.lobj`
- [ ] 監看檔案變更並自動重新載入畫面
- [x] 實測遠端 `ssh -L` 使用流程與 `--host 0.0.0.0` 模式（ssh -L 可用；0.0.0.0 會失去 WebGPU，符合預期）
- [ ] 實測 WebGL2 退回路徑與 `npm run dev` 開發流程
- [x] GitHub Actions 發佈流程（多平台 build、tag 自動 release）已完成，說明見 doc/github-cross-compile.md
- [ ] 視需要加入 release profile（strip / lto）縮小執行檔
- [ ] 尚未實測 macOS / Windows 的 release 檔（只驗證了 Linux x86_64 用 binstall 安裝）；aarch64 Linux 也未實測
- [ ] release 檔未簽章；macOS 下載的執行檔可能被 Gatekeeper 擋，需要時再處理
