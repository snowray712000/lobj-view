# Versions

### 0.0.7
- 新功能: 現在可以開啟 .obj 檔（支援多邊形面與頂點顏色），檔案清單也會列出 .obj
- 新功能: 現在可以開啟 .ply 檔（ASCII 與 binary 兩種格式皆可，含頂點顏色），檔案清單也會列出 .ply
- 優化: .obj / .ply 沒有牙齒標記，所以以「不確定」的顏色顯示；檔案本身若有頂點顏色則顯示該顏色
- 工程師筆記: 新增 `lobj/src/obj.rs`、`lobj/src/ply.rs`，`lobj::load` 依副檔名分派，僅能讀不能存；PLY 以 header 逐屬性消耗資料所以能跳過未知 element #obj #ply #BadObj #BadPly #binary_big_endian 詳見 261119d

### 0.0.6
- 新功能: 現在可以開啟 .stl 檔（支援 binary 與 ASCII 兩種格式），檔案清單也會列出 .stl
- 優化: .stl 沒有牙齒標記，所以整個模型以「不確定」的顏色顯示
- 工程師筆記: 新增 `lobj/src/stl.rs`，以長度 `84+50n` 判斷 binary（binary header 可能以 solid 開頭）、相同座標頂點合併、labels 全為 -1；`lobj::load` 對 .stl 直接走此模組，僅能讀不能存 #stl #binary-stl #ascii-stl 詳見 261119c

### 0.0.5
- 新功能: 檔案選擇改成資料夾瀏覽：清單最上方有 `..` 可回上一層，點資料夾可進入下一層
- 新功能: 路徑欄可直接貼上路徑（`/` 或 `\` 皆可，相對路徑或完整路徑都行），按 Enter 即跳到該資料夾；貼的是檔案就直接開啟
- 優化: 只能瀏覽 `--dir` 指定的資料夾以內，超出的路徑會被拒絕
- 工程師筆記: 後端 `resolve()` 以 canonicalize + starts_with 限制在 `--dir` 內，`/api/lobj?path=` 列出資料夾、`/api/lobj/{*path}` 讀子資料夾檔案 #path-traversal #axum 詳見 261119b

### 0.0.4
- 新功能: 現在可以開啟 .lobj / .lobj2 檔：啟動時用 `--dir` 指定資料夾，畫面左上角的 file 下拉選單選檔即可顯示，並依牙齒標記自動上色
- 新功能: 顯示頂點數、三角形數與各標記的數量；相機會自動對準模型
- 新功能: 舊格式 .lobj 若檔名沒有 upper / lower，可用畫面上的 jaw 選單指定上顎或下顎
- 優化: 原本的示範模型已移除
- 優化: README 更新目前版本說明，新增 `--dir` 選項與開啟 .lobj 的使用說明
- 工程師筆記: 新增共用 crate `lobj/`（自 tech-rust 移植）；後端 `/api/lobj`、`/api/lobj/{name}?jaw=` 以 .lobj binary stream 傳遞，labels 一律 FDI、缺 colors 由後端補；前端 `web/src/lobj.ts` 解析 #lobj #axum #binary-stream 詳見 261119a
- 工程師筆記: `#hud` 的 `pointer-events: none` 會讓內部下拉選單無法點擊，需對 select 另設 auto #css #pointer-events 261119a
- 工程師筆記: lobj 以 `../lobj` 路徑相依，現有 GitHub Actions 單獨 checkout 會建置失敗，發佈前需處理 #github-actions #path-dependency 261119a

### 0.0.3
- 新功能: 現在有 Linux（x86_64、ARM）、Windows、macOS（Intel、Apple Silicon）的預編譯執行檔，可直接到 GitHub Releases 下載
- 新功能: 可用 `cargo binstall lobj-view --git https://github.com/snowray712000/lobj-view` 直接安裝，不需要在本機編譯
- 優化: README 新增安裝說明；新增「GitHub 發佈流程」與「Ubuntu 用 rustup 安裝最新 cargo 與 cargo-binstall」兩份教學（doc 資料夾）
- 優化: 已實測在 Ubuntu 執行伺服器，Windows、Mac 的瀏覽器以 HTTPS 連線可正常使用
- 工程師筆記: GitHub Actions（cross 建 Linux musl、runner 原生建 Windows/macOS）+ Cargo.toml 的 `[package.metadata.binstall]` 對應 Release 檔名；tag 必須是 `v<版本>` #github-actions #cross #cargo-binstall 詳見 261118e

### 0.0.2
- 新功能: 使用 `--host 0.0.0.0` 時自動改用 HTTPS，區網內其他電腦的瀏覽器也能使用 WebGPU
- 新功能: 新增 `lobj-view trust` 指令，自動產生憑證並顯示其他裝置的信任步驟；加上 `--install` 可直接匯入本機信任清單（Windows、macOS）
- 新功能: 伺服器提供 `/trust` 說明頁與 `/ca.crt` 下載，其他電腦、手機可直接取得憑證
- 新功能: 可用 `--http` / `--https` 強制選擇連線方式，`--cert` / `--key` 使用自備憑證，`--san` 加入額外名稱
- 優化: 新增 README.md，說明區網使用、各系統匯入憑證步驟與常見問題（macOS 要選「系統」鑰匙圈、匯入後要完全重開瀏覽器）
- 工程師筆記: rcgen 本機 CA + 伺服器憑證，rustls/ring + axum-server；憑證存 data_local_dir/lobj-view/tls。#tls #rcgen #ring #secure-context 詳見 261118c 261118d
- 工程師筆記: 加入 ring 後無法從 Windows 交叉編譯 Linux musl（缺 C 編譯器） #musl #ring #cc-rs 詳見 261118c

### 0.0.1
- 新功能: 建立 lobj-view 檢視器雛形，執行後用瀏覽器開啟網址即可看到 3D 畫面（目前是示範模型，尚無法開啟 .lobj 檔）
- 新功能: 預設使用 WebGPU 繪圖，瀏覽器不支援時自動改用 WebGL2，畫面左上角會顯示目前使用哪一種
- 新功能: 預設的 port 被佔用時會自動改用其他 port，並在啟動時印出網址與遠端連線（ssh -L）的提示
- 新功能: 可用 `--host`、`--port` 指定監聽位址與 port
- 工程師筆記: axum + rust-embed 單一執行檔；web/dist 需先 build 並隨 crate 發佈（Cargo.toml include）。#rust-embed #WebGPURenderer #secure-context 詳見 261118a
- 工程師筆記: 畫布尺寸為 0 時 WebGPU 會報 texture size empty，已用 ResizeObserver 跳過繪製 #WebGPU #ResizeObserver 261118a
- 優化: 可打包成單一執行檔，複製到其他電腦（含 Ubuntu）直接執行，不需另外安裝 Node.js 或前端檔案
- 工程師筆記: Linux 版以 `x86_64-unknown-linux-musl` + rust-lld 從 Windows 交叉編譯，靜態連結不依賴 glibc #musl #cross-compile #rust-lld 詳見 261118b
