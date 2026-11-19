# Versions

### 0.0.11
- 優化: 本機 CA 憑證的主體改為 `O=ccu-cadcam`、`CN=ccu-cadcam lobj-view local CA`；從舊版升級後第一次啟動會自動重建 CA，所有裝置需重新匯入一次，舊的 `lobj-view` CA 可刪除
- 優化: `lobj` 讀寫程式改為 GitHub 上獨立的 crate（https://github.com/snowray712000/lobj ，tag v1.1.1），GitHub Actions 單獨 checkout 本專案即可建置
- 工程師筆記: `tls.rs` 新增 `ca.id` 標記檔，CA 主體（DN）變更時舊 CA 會被重建；不能只改 `ca_params()`，簽發伺服器憑證時 issuer DN 由它重建，與磁碟上舊 CA 不一致會使憑證鏈驗證失敗 #tls #rcgen #ca-id
- 工程師筆記: Cargo.toml 的 lobj 為 `git + tag` 相依，本機開發在上層 `rust-workspace/.cargo/config.toml` 用 `[patch]` 指回 `./lobj`（該檔不入版控）#cargo-patch #git-dependency

### 0.0.10
- 新功能: 狀態列新增「光源」下拉：固定（原本的光，不隨視角）、頭燈（光一直跟著眼睛位置，轉到哪看細節哪裡就亮）、三燈（眼睛＋左＋右各一盞）；選擇會記住
- 新功能: 狀態列新增「格線」checkbox，控制世界座標的地面格線顯示（預設關閉，選擇會記住；x-y-z 座標軸不受影響）
- 優化: 狀態列分成兩行，第二行放「光源」、「格線」與「主題」
- 工程師筆記: 跟隨型的燈為 DirectionalLight，每幀依相機 right/up/back 軸擺在 controls.target 周圍（距離 = 相機到目標點），切換時環境光降為 0.9、固定太陽光隱藏 #lighting #headlight #DirectionalLight

### 0.0.9
- 新功能: 畫面右側新增「物件」面板（預設展開，點標題可收合），每個已選檔案一張卡片，顯示頂點數、三角形數與標記數量
- 新功能: 每個物件可調 alpha 滑桿（0–1，預設 1），多個模型同時顯示時可調成半透明看到重疊處
- 新功能: 每個物件可選 color（Apple 系統色）；預設「原」＝不設定，維持牙齒標記色；選了顏色後以「疊加」滑桿決定與原色的混合比例（預設 0.6，1 = 完全換色）；取消選取再選回來時設定仍保留
- 新功能: 左側分成兩塊：上方狀態列「lobj-view @ccu-cadcam v版本 · server: ok · renderer: WebGPU」，點版本號開啟版本紀錄視窗（幾乎全畫面，Esc 或 ✕ 關閉）；下方「選擇檔案」半透明面板可收合（預設展開）
- 新功能: 狀態列最後新增「主題」下拉：自動（跟隨系統）、淺色、深色、高對比淺色、高對比深色；選擇會記住，介面、3D 背景與物件色調一起切換
- 優化: 左側不再顯示檔案資訊（移到右側物件卡片），只留路徑錯誤訊息；介面顏色改用 Apple 色票，不再有與背景難分辨的綠字
- 優化: 地面格線與 x-y-z 座標軸大小隨已載入模型自動調整，沒有模型時回到預設大小
- 優化: 世界座標改為 z 向上（CAD 慣用）：格線在水平面，預設從前方（-y）看向模型
- 工程師筆記: 新增 `web/src/props.ts`（alpha/color 套用）、`web/src/apple-color-ios.css`（複製自 FHLNUI `index/theme`，不手改）；屬性只存 CSS 變數名，套用時由目前主題解析；一律建立 color attribute、原色存 `orig`；alpha<1 時 `depthWrite=false` #theme #css-variables #alpha #three.js 261119e
- 工程師筆記: 新增 `/api/version`、`/api/versions`（`include_str!` 內嵌 doc/Versions.md，Cargo `include` 已加入）；格線 `niceCeil` 取 1/2/5×10ⁿ；z-up 需在建立 OrbitControls 前設 `camera.up`，GridHelper 旋轉 x 軸 90° #z-up #GridHelper #OrbitControls 261119e

### 0.0.8
- 新功能: 檔案清單可以多選：Ctrl（Mac 用 ⌘）點擊加選／取消，Shift 點擊選取範圍，一般點擊仍是只選一個
- 新功能: 選了多個檔案會同時載入並同時顯示在畫面上，相機自動對準全部模型；取消選取的檔案會從畫面移除
- 新功能: 資訊區會分檔列出各檔案的頂點、三角形與標記數量，單一檔案載入失敗不影響其他檔案
- 優化: 換資料夾時已選的檔案仍保留顯示；改 jaw 選單會重新載入所有已選檔案
- 工程師筆記: 純前端改動（`web/src/main.ts`）：`selected` / `loaded` 兩個 Map/Set 與 `syncSelection()` 做差異同步，後端 API 不變 #multi-select #three.js

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
