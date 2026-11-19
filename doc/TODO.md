# TODO

## 重要
- [x] lobj 讀寫：共用 crate `../lobj`（自 tech-rust 移植，含 tests）；前端 `web/src/lobj.ts` 對應解析
- [x] 後端 `GET /api/lobj`、`GET /api/lobj/{name}[?jaw=]`（`--dir` 指定資料夾），以 .lobj binary stream 傳遞；labels 一律 FDI、缺 colors 由後端補
- [x] 瀏覽器內實機確認畫面（使用者已確認可用；修正了下拉選單被 pointer-events 擋住的問題，見 261119a）
- [ ] 0.0.7（含 0.0.6、0.0.5、0.0.4）尚未打 tag / 推送發佈（專案未 git init；CI 需先解決 lobj 路徑相依）
- [x] 檔案選擇改為資料夾瀏覽（`..`、點資料夾、貼路徑，限制在 `--dir` 內，見 261119b）
- [ ] 資料夾瀏覽尚未在瀏覽器實機操作驗證（只有單元測試與 build）
- [ ] 後端依 CAD/CAM 需求處理 mesh 後，結果以 .lobj 串流回前端（POST 路徑尚未做）
- [ ] CI（.github/workflows/release.yml）目前單獨 checkout lobj-view，`lobj` 為 `../lobj` 路徑相依，需改成同 repo 或 git 相依才能建置
- [ ] tech-rust 的 src/lobj 與 `../lobj` 目前是兩份拷貝，之後讓 tech-rust 改用 path 相依
- [ ] 建立 git 版本控制（目前專案尚未 git init）
- [x] 驗證 release build（Windows、Linux musl 單一執行檔皆可獨立執行，見 261118b）
- [ ] 驗證 `cargo install --git`（從原始碼編譯）與 `cargo package` 的 web/dist 是否正確帶入；`cargo binstall` 已驗證可用

- [x] 實機驗證 HTTPS：Windows 本機與區網 Mac mini 皆成功（見 261118d）
- [x] 反向測試：Ubuntu（無畫面）當 server、Windows / Mac 當 client，HTTPS 皆可使用（見 261118e）
- [x] Linux 單一執行檔：改走 GitHub Actions + cross，已可建出 Linux / Windows / macOS（見 261118e）

## 次要
- [ ] 0.0.9 的右側物件面板、左側分塊、主題切換、z-up 與格線自動大小，只有 build / cargo check 通過，尚未在瀏覽器實機操作驗證（261119e）
- [ ] 主題預設值待使用者試用後決定（目前為「自動：跟隨系統」）
- [ ] 資料不是 z-up 時（例如 y-up 的 .obj / .stl）可考慮加「上方向」切換
- [ ] 版本紀錄視窗只做簡易渲染（標題與項目），未支援完整 markdown
- [ ] 物件面板可再加：顯示 / 隱藏、自訂顏色（color picker）
- [ ] 用真實 .stl / .obj / .ply 檔在瀏覽器實機確認（目前只有單元測試，見 261119c、261119d）
- [ ] .ply 改善：ASCII 內容非 UTF-8 時的錯誤訊息不準確；list 數量異常大時未防護（261119d）
- [ ] 視需要支援寫出 .obj / .ply，或讀取 .mtl 材質（目前只能讀幾何）
- [ ] 視需要支援寫出 .stl（目前只能讀）
- [ ] 格式確定後，考慮把 lobj 讀寫拆成獨立 crate 以便共用
- [ ] 命令列直接指定檔案：`lobj-view model.lobj`
- [ ] 監看檔案變更並自動重新載入畫面
- [x] 實測遠端 `ssh -L` 使用流程與 `--host 0.0.0.0` 模式（ssh -L 可用；0.0.0.0 會失去 WebGPU，符合預期）
- [ ] 實測 WebGL2 退回路徑與 `npm run dev` 開發流程
- [x] GitHub Actions 發佈流程（多平台 build、tag 自動 release）已完成，說明見 doc/github-cross-compile.md
- [ ] 視需要加入 release profile（strip / lto）縮小執行檔
- [ ] 尚未實測 macOS / Windows 的 release 檔（只驗證了 Linux x86_64 用 binstall 安裝）；aarch64 Linux 也未實測
- [ ] release 檔未簽章；macOS 下載的執行檔可能被 Gatekeeper 擋，需要時再處理
