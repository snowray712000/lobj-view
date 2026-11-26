# lobj-view

用瀏覽器檢視 `.lobj`（帶 label 的二進位 3D 物件）檔案的小工具。單一執行檔，內含前端，執行後用瀏覽器開網址即可，不需安裝 Node.js。

> 目前版本 1.19.1：新增資料集統計頁 `/stat`：每個子資料夾視為一個病例，統計有無上 / 下顎與各 FDI 的病例數與百分比（可轉置），可輸出 `stat.csv`、資料夾已有 `stat.csv` 時直接載入（可「重新產生」），也可「下載 .csv」。以下為 1.18.* 與 1.17.1 細節。1.18.3：`/stat` 新增「下載 .csv」。1.18.2：`/stat` 顯示統計結果、可載入既有 `stat.csv`。1.18.1：新增 `/stat`（輸出 `stat.csv`）。1.17.1：新增命令列 `lobj-view convert-dir <資料夾>` 與網頁版批次轉檔頁 `/to-lobj2`，把 .lobj 批次轉成 .lobj2（可包含次目錄、可刪除已轉過的舊 .lobj）。以下為 1.16.* 細節。1.16.5：`/to-lobj2` 新增「包含次目錄」。1.16.1：新增命令列 `convert-dir` 批次把資料夾下的 .lobj 轉成 .lobj2。1.15.1：選擇檔案區新增「壓縮資料夾」（把目前資料夾壓成同層的「資料夾名.zip」，有進度列）；選取 .zip 後可「同步到…」另一台電腦（rsync over ssh，可瀏覽遠端資料夾，可選「背景執行（tmux）」、關掉 ssh 也不中斷，背景工作清單可停止 / 清除已結束）或「刪除」。以下為 1.14.* 細節。1.14.4：同步視窗新增「背景執行（tmux）」：rsync 在本機的 detached tmux session 裡跑，關掉 ssh、網頁或重啟 lobj-view 都不會中斷；視窗內的「背景工作」清單顯示進度、完成 / 失敗，可停止，也可用 `tmux attach -t lobj-sync-…` 查看（需要安裝 tmux，Windows 不支援）。1.14.3：壓縮資料夾與同步到另一台電腦都有進度列（壓縮顯示百分比，同步顯示百分比與傳輸速度）。1.14.2：選取磁碟上的 .zip 後可「同步到…」另一台電腦（rsync over ssh：輸入 user@host，可瀏覽並選擇遠端資料夾；有 ssh-agent 或免密碼金鑰不必輸入密碼，否則在密碼欄輸入金鑰密碼或登入密碼，不會儲存）或「刪除」；資料夾 / .zip 改為單擊選取、雙擊進入。1.14.1：選擇檔案區新增「壓縮資料夾」按鈕，把目前資料夾（含子資料夾，不分副檔名，略過符號連結）壓成同層的「資料夾名.zip」，用來傳到另一台電腦（只在本機模式可用，不會觸發下載；同名已存在會先問是否覆蓋）。1.13.3：開發（debug）建置預設 port 8079、release 預設 8080，可與已安裝的穩定版同時執行。1.13.2：修正右側設定面板原本對所有物件一起生效的問題，現在 alpha、color、wire、normal、curv 只改「作用中物件」。1.13.1：選擇檔案區新增「瀏覽…」資料夾選擇對話框（左側常用位置 / 釘選 / 最近，右側列出資料夾與檔案，按「開啟」切換主畫面資料夾；可拖曳右下角改大小、◐ 滑桿調透明度）。1.12.2：左側檔案清單單擊＝加選 / 取消（不清掉已載入的）、Shift 單擊範圍選取、雙擊＝只開這一個。1.12.1：右側面板改為共同設定 + 物件清單（h 隱藏 / s solo）。1.11.1：介面支援多語言（預設中文，狀態列的「lang」按鈕可切換成 English / Tiếng Việt，選擇會記住）。版本號旁的「檢查更新」可一鍵更新到 GitHub 最新版並自動重新啟動（見「更新」）。可用 `--dir` 指定根資料夾，在畫面左側瀏覽子資料夾或貼上路徑來開啟 `.lobj` / `.lobj2` / `.stl` / `.obj` / `.ply`（可多選同時顯示），依 label 上色；右側「物件」面板上方是作用中物件的設定（資訊、jaw、轉 .lobj2 顯示點選的作用中物件；alpha、color 含自訂色、wire / normal / curv 只對作用中物件生效、各物件各自記住），下方物件清單每列有 h（隱藏）/ s（solo：有 solo 時其他模型 alpha 最高 0.5）核取方塊，點檔名切換作用中物件並短暫顯示其 AABB（沒有 label 的模型載入後會自動配色，盡量不重複；可勾選 wire 顯示黑色邊線、normal 顯示頂點法向量、curv 以曲率上色（凸凹 / max / min，可先平滑、可用滑桿調截斷百分位）），計算方式見 [doc/curvature.md](doc/curvature.md)；另有光源（固定 / 頭燈 / 三燈）、正交投影、單面繪製（左上狀態列 checkbox，預設關閉）、主題（淺 / 深 / 高對比）切換與 z 向上的世界座標。

## 安裝

預編譯執行檔（Linux x86_64 / aarch64、Windows、macOS Intel / Apple Silicon）由 GitHub Actions 建置，發佈在 repo 的 Releases。

### 方法 A：`cargo binstall`（推薦，免編譯）

```bash
cargo binstall lobj-view --git https://github.com/snowray712000/lobj-view
```

會下載對應平台的 Release 檔，裝到 `~/.cargo/bin`（Windows 為 `%USERPROFILE%\.cargo\bin`）。
尚未安裝 cargo / cargo-binstall 的使用者（皆使用預編譯檔，免編譯）：
- Ubuntu 見 [doc/ubuntu-install-rust-and-binstall.md](doc/ubuntu-install-rust-and-binstall.md)（不用 `apt install cargo`，改用 rustup）。
- Windows 見 [doc/windows-install-rust-and-binstall.md](doc/windows-install-rust-and-binstall.md)（全程 cmd 指令，不需 MSVC、不需管理員權限，ssh 到 Windows 也適用）。

### 方法 B：`cargo install`（從原始碼編譯）

```bash
cargo install --git https://github.com/snowray712000/lobj-view
```

需要 Rust 1.85 以上與 C 編譯器（Ubuntu：`sudo apt install build-essential`）；不需要 Node.js（`web/dist` 已提交）。

### 方法 C：直接下載

到 Releases 下載 `lobj-view-v<版本>-<target>.tar.gz`（Windows 為 `.zip`），解壓縮後把執行檔放進 PATH。

發佈流程（GitHub Actions cross-compile）見 [doc/github-cross-compile.md](doc/github-cross-compile.md)。

### 更新

1.8.1 起可以直接從網頁更新：版本號旁的「檢查更新」在有新版時會變成綠色「⬆ 有新版 vX」，點開後按「更新到 vX 並重新啟動」即可——會下載這個平台的 Release 檔、驗證後替換執行檔，伺服器在同一個 port 重新啟動，網頁自動重新整理。

```bash
lobj-view update --check   # 只檢查
lobj-view update           # 下載並替換（執行中的 lobj-view 要重新啟動才會換成新版）
```

- 網頁更新預設只在伺服器監聽本機（預設 `127.0.0.1`）時可用；`--host 0.0.0.0` 時請在伺服器上用 `lobj-view update`，或加上 `--allow-remote-update` 啟動，讓區網上的網頁也能按更新（開啟後任何能開這個網頁的人都能讓伺服器更新並重新啟動，請只在信任的網路使用）。
- 需要能連上 github.com，且執行檔所在資料夾有寫入權限；剛推 tag 時 Actions 還在建置，會提示稍後再試。
- 1.7.1 以前的版本沒有這個功能，要先用上面任一種方法裝一次 1.8.1。

## 快速開始（本機使用）

## 批次轉 .lobj2

```bash
lobj-view convert-dir <資料夾> [--dry-run] [--delete-old]
```

資料集統計：開 `http://localhost:8080/stat`，選資料集資料夾（每個子資料夾一個病例），統計上 / 下顎與 FDI 並輸出 `stat.csv`（輸出只限本機）。結果以「項目 × 病例數 / %」顯示，預設直式（手機），「⇄ 轉置」切成橫式（電腦）；資料夾已有 `stat.csv` 就直接載入，按「重新產生」才重新讀取 `.lobj2`。「⬇ 下載 .csv」把目前顯示的資料下載成 `stat-{資料集名稱}.csv`（存在瀏覽器那台電腦）。

網頁版：啟動後開 `http://localhost:8080/to-lobj2`，瀏覽資料夾、勾選檔案後轉換，也能一鍵刪除「已有 .lobj2 的 .lobj」（只限本機）。

命令列遞迴轉換資料夾下檔名明確含 `upper` / `lower` 的 `.lobj` 為同名 `.lobj2`；無法判斷上下顎的略過。預設保留原檔。

```bash
lobj-view
```

開啟印出的網址（release 版預設 `http://localhost:8080`，開發（debug）版預設 `http://localhost:8079`；port 被佔用會自動往上找）。

| 選項 | 說明 |
|---|---|
| `--host <IP>` | 監聽位址，預設 `127.0.0.1` |
| `-p, --port <N>` | 偏好的 port，release 預設 8080、debug 預設 8079 |
| `-d, --dir <資料夾>` | 提供 `.lobj` / `.lobj2` / `.stl` / `.obj` / `.ply` 的資料夾（可瀏覽其子資料夾，但不能超出此資料夾），預設目前資料夾 |
| `[PATH]` | 要開啟的檔案或資料夾；已有執行中的 lobj-view 就交給它，否則啟動並自動開瀏覽器（見「單一實例」） |
| `--new` / `--no-browser` | 另起獨立實例 / 帶 PATH 時不自動開瀏覽器 |
| `--allow-remote-update` | 監聽非本機位址時也允許網頁觸發線上更新（預設關閉，見「更新」） |
| `--http` / `--https` | 強制使用 HTTP / HTTPS（見下） |
| `--cert <pem> --key <pem>` | 自備憑證 |
| `--san <名稱或IP>` | 額外寫入憑證的名稱（可重複） |

## 開啟 .lobj

```bash
lobj-view --dir path/to/models
```

畫面左側有狀態列（版本、server、renderer、主題，以及一列伺服器資源：幾人 / 分頁、CPU、記憶體、有 NVIDIA 顯卡時的 GPU；點版本號可看版本紀錄）與「選擇檔案」面板（可收合）：點資料夾進入下一層、點 `📁 ..` 回上一層，也可貼上路徑（`/` 或 `\` 皆可，相對或 `--dir` 底下的絕對路徑）後按 Enter；路徑不可超出 `--dir`。`.zip` 檔會顯示成 📦 資料夾，點進去就能直接看裡面的模型（原 zip 不會被改動，第一次進入時在背景解壓到快取；太大可按左側「處理中」提示的「取消」中斷）。點檔案即載入；Ctrl（⌘）點擊加選、Shift 點擊範圍選取，可同時顯示多個檔案。

右側「物件」面板（可收合）每個已選檔案一張卡片，顯示頂點數、三角形數與各 label 數量，並可調整：

- **alpha**：0–1，預設 1；多個模型重疊時調成半透明。
- **color**：預設「原」（不設定，維持 label / 頂點顏色）；選 Apple 系統色後，用「疊加」滑桿決定與原色的混合比例（1 = 完全換色）。

- `.lobj2` 的 label 是 FDI；舊 `.lobj` 是 arch_index，需要知道上顎或下顎。上 / 下顎的判定（暫時作法）：左側「選擇檔案」的 regex 輸入框（預設 `upper;_u_;u\d+-;ur`，以 `;` 分隔、不分大小寫）——檔名符合任一個就是 upper，否則 lower；檔案清單每個 `.lobj` / `.stl` / `.ply` / `.obj` 前面的綠色 `up` / `lo` 就是目前的判定結果；若判定與檔名字面矛盾（例如判成 upper 但檔名含 `lower`）改顯示紅色 `er`（滑鼠移上去看實際判定）（regex 語法錯誤的段落會略過並讓輸入框變紅）。`.stl` / `.ply` / `.obj` 的判定用於轉 `.lobj2`（見下）。個別檔案可在右側物件卡片的 jaw 選單覆蓋（卡片 > regex）。每個 `.lobj` / `.stl` / `.ply` / `.obj` 的卡片上有「轉 lobj2」鈕：跳出確認視窗（預設「轉換後刪除舊檔」，也可選保留），在同資料夾存成同主檔名的 `.lobj2`（只改副檔名；`.stl` / `.ply` / `.obj` 沒有 label，轉出的 labels 全是 -1）。`.lobj2` 載入後會看牙位 label 判斷上 / 下顎，沒有牙（例如 labels 全是 -1）就退回檔名 regex 判定，清單標記與卡片的 jaw 一行都會顯示，新檔的建立 / 修改 / 存取時間與舊檔相同；同名 `.lobj2` 已存在時不會覆蓋，zip 內的檔案不能轉，且只在伺服器監聽本機位址時可用。
- 檔案若沒有頂點顏色，後端會依 label 補上。
- API（前後端以 `.lobj` 二進位串流傳遞，labels 一律 FDI）：`GET /api/lobj`（列表）、`GET /api/lobj/<檔名>[?jaw=upper|lower]`。

## 單一實例（開檔交給執行中的 lobj-view）

```bash
lobj-view path/to/model.lobj
```

- 已有 lobj-view 在跑：把這個檔案交給它（根資料夾切到該檔所在的資料夾並選取，網頁自動跳過去），這個程序隨即結束；若沒有任何網頁連著，會自動開瀏覽器。
- 沒有：啟動伺服器，根資料夾為該檔所在的資料夾，並自動開瀏覽器（`--no-browser` 可關）。
- `--new` 另起獨立實例。交接走只聽 `127.0.0.1` 的 IPC 加 token（`<data_local_dir>/lobj-view/instance.txt`），不經 HTTP，區網上的人無法切換你的資料夾。
- 同時開多個檔案（例如檔案總管多選）時，目前只會顯示最後一個。

### 雙擊檔案就開啟（Windows）

```bash
lobj-view register            # 全部五種副檔名
lobj-view register --ext stl,obj   # 只處理指定的
lobj-view status              # 各副檔名目前由誰開啟
lobj-view unregister          # 取消註冊並還原成註冊前的樣子
```

也可以在網頁左側狀態列按「預設程式…」勾選後註冊。只寫目前使用者的登錄（`HKCU`），不需要系統管理員。

- 註冊後 lobj-view 會出現在檔案右鍵「開啟檔案」（與「選擇其他應用程式」）的清單，五種副檔名都有。
- `.lobj` / `.lobj2` 若沒有其他程式佔用，會直接成為預設。
- `.stl` / `.ply` / `.obj` 通常已有預設程式，Windows 10 不允許程式自行覆蓋，只會註冊成「開啟方式」候選；請到「設定 > 預設應用程式」自己指定。
- 註冊前會先記錄原狀（登錄 `HKCU\Software\lobj-view\AssocBackup`），`unregister` 依記錄還原；若你曾在系統設定把 lobj-view 指定成預設，會提示原本的預設程式，需自己改回。
- 網頁的變更功能只在監聽本機位址時可用（`--host 0.0.0.0` 時停用），CLI 不受影響。
- Linux / macOS 尚未支援。
- 檔案圖示：Windows 的 exe 內嵌 `assets/lobj-view.ico`（多尺寸；`assets/lobj-view.png` 為大圖），註冊後 `.lobj` 等檔案會顯示它；若沒立刻更新，重開檔案總管或清除圖示快取。

## 為什麼需要 HTTPS

畫面預設使用 WebGPU，瀏覽器只在 **secure context** 提供它：`localhost` 或 `https`。
所以從區網其他電腦用 `http://192.168.x.x` 連線時只能退回 WebGL2（畫面左上角的狀態列會顯示目前用哪一種）。

因此 `--host` 設為非本機位址（例如 `0.0.0.0`）時，程式會**自動啟用 HTTPS**，並用自己產生的「本機 CA」簽發憑證。
每台要連線的電腦只要**匯入一次這個 CA**，之後不會再有警告；區網 IP 改變時憑證會自動重簽，不必重新匯入。

> 注意：`--host 0.0.0.0` 會對區網開放，且**沒有任何驗證**。只在可信任的網路使用。

## 區網使用步驟（半自動）

### 1. 在執行伺服器的機器上

```bash
lobj-view trust --install
lobj-view --host 0.0.0.0
```

- `trust` 會產生 CA 與伺服器憑證（存放位置見下）並印出涵蓋的 IP / 名稱。
- `--install` 同時把 CA 匯入**這台機器**的信任清單（Windows、macOS；Linux 請參考下方手動步驟）。
- 本機不需要下載 `ca.crt`，**但執行完後請完全重新啟動瀏覽器**，否則仍會看到憑證警告。
- 不加 `--install` 只產生憑證並印出說明。
- 啟動後會列出 `https://<區網IP>:<port>` 網址。

### 2. 在其他要連線的電腦／手機上

1. 瀏覽器開 `https://<伺服器IP>:<port>/trust`（第一次會有憑證警告，選擇繼續）。
2. 下載 `ca.crt`（`/ca.crt`）。
3. 依系統匯入為「受信任的根憑證授權單位」：

| 系統 | 步驟 |
|---|---|
| Windows | 雙擊 `ca.crt` → 安裝憑證 → 目前使用者 → 「將所有憑證放入以下存放區」→ 受信任的根憑證授權單位。或：`certutil -user -addstore Root ca.crt` |
| macOS | 雙擊 `ca.crt`，鑰匙圈**選「系統」**（不要用預設的「登入」或「本機項目」，否則會出錯），輸入本機登入密碼 → 開啟該憑證 → 信任 → 「永遠信任」 |
| Linux 桌面 | Chrome/Edge：設定 → 隱私權和安全性 → 安全性 → 管理憑證 → 授權單位 → 匯入。Firefox：設定 → 隱私權與安全性 → 憑證 → 檢視憑證 → 授權單位 → 匯入 |
| Android | 設定 → 安全性 → 加密與憑證 → 安裝憑證 → CA 憑證 |
| iOS | 開啟 `ca.crt` → 安裝描述檔，再到 設定 → 一般 → 關於 → 憑證信任設定 開啟完全信任 |

4. **完全重新啟動瀏覽器**，再開 `https://<伺服器IP>:<port>`。

> CA 主體為 `O=ccu-cadcam`、`CN=ccu-cadcam lobj-view local CA`。從舊版（主體只有 `lobj-view`）升級後，第一次啟動會自動重建 CA，**所有裝置需重新匯入一次**，舊的 `lobj-view` CA 可在憑證管理員中刪除。

### 常見坑

- 匯入後仍有警告 → 瀏覽器沒有完全重開（關掉所有視窗再開）。
- macOS 匯入時出錯 → 鑰匙圈要選「系統」並輸入登入密碼。
- 連的網址不在憑證涵蓋範圍（例如新的 DNS 名稱）→ 用 `--san 名稱` 重新啟動，憑證會自動重簽（CA 不變，不需重新匯入）。
- 伺服器重新產生 CA（刪過憑證目錄）→ 所有裝置需重新匯入新的 CA。
- Windows 上用 `curl` 驗證私有 CA 要加 `--ssl-no-revoke`。
- **Firefox 開 `/trust` 只有「進階」、沒有「接受風險並繼續」**，錯誤碼 `SEC_ERROR_BAD_SIGNATURE`（Edge 卻正常）：
  - 原因：Firefox 勾選了「允許 Firefox 自動信任您安裝的第三方根憑證」（`security.enterprise_roots.enabled`），因此也會採用 Windows 憑證庫裡的 CA。若那裡留有**同名但金鑰不同的舊 CA**（例如伺服器重新產生過 CA），簽章驗不過，Firefox 視為不可略過的錯誤。
  - 作法：到 設定 → 隱私權與安全性 → 憑證，**取消勾選**「允許 Firefox 自動信任您安裝的第三方根憑證」，重新整理即會出現「接受風險並繼續」。
  - 根治：在 Windows 用 `certmgr.msc`（目前使用者）與 `certlm.msc`（本機電腦）刪掉舊的同名 CA，再依上表匯入新的 `ca.crt`，之後可把該選項勾回來。
  - 若按過「繼續」，Firefox 會記成永久例外（設定 → 憑證 → 檢視憑證 → 伺服器），匯入正確 CA 後建議刪除該例外。

## 憑證存放位置

`trust` 會印出實際路徑。Windows 為 `%LOCALAPPDATA%\lobj-view\tls\`，檔案：

| 檔案 | 說明 |
|---|---|
| `ca.crt` | CA 公開憑證（可散布給其他裝置） |
| `ca.key` | CA 私鑰，**不要外流** |
| `server.crt` / `server.key` | 伺服器憑證與私鑰（有效 1 年，快到期或位址變動時自動重簽） |
| `server.san` | 上次簽發涵蓋的名稱清單 |

## 遠端的另一種作法：ssh 轉發

不想匯入憑證時，可讓伺服器保持預設的本機監聽，再由使用者端轉發：

```bash
ssh -L 8080:localhost:8080 <伺服器>
```

然後在本機開 `http://localhost:8080`（`localhost` 本身就是 secure context，有 WebGPU）。

## 建置

`lobj` 讀寫程式是 GitHub 上的 [lobj](https://github.com/snowray712000/lobj)（`Cargo.toml` 以 git tag 相依，一般建置會自動下載）。若本機同層有 `lobj` 資料夾想直接改它，可在上層資料夾建立 `.cargo/config.toml`：`[patch."https://github.com/snowray712000/lobj"]` 加上 `lobj = { path = "lobj" }`。

```bash
cd web && npm install && npm run build   # 先建前端（輸出 web/dist）
cd .. && cargo build --release           # 前端會嵌入執行檔
```

`cargo install` 的安裝端不需要 Node.js，但發佈前必須先 build 好 `web/dist`。
注意：TLS 使用 `ring`，含 C 程式碼，從 Windows 交叉編譯 Linux musl 需要 C 編譯器，請用 GitHub Actions 建置（見 [doc/github-cross-compile.md](doc/github-cross-compile.md)）。

## 開發時

```bash
# 在 lobj-view 目錄下
cargo run -- --dir --host 0.0.0.0
```
