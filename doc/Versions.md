# Versions

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
