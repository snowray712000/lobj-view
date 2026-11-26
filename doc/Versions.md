# Versions

版本號規則：第 1 碼為大變動；第 2 碼為小變動，偶數 = 開發中、奇數 = 發佈（例如 1.1.1 發佈，之後小修為 1.1.2；繼續開發為 1.2.x，告一段落發佈 1.3.1）。介面預設只顯示「概要」（x.奇數.1），可切換成顯示全部細節。

### 1.14.3
- 新功能: 「壓縮資料夾」顯示進度列（依已處理位元組 / 資料夾總位元組換算百分比）；「同步到…」顯示進度列、百分比與傳輸速度（取自 rsync `--info=progress2`）
- 改變: 壓縮與同步的完成 / 失敗改在進度串流的最後一行回報；開始前的錯誤（沒權限、同名 zip 已存在、主機格式不對）仍是一般的錯誤訊息
- 已知限制: 同步很快（區網、小檔）時進度列可能一閃而過；壓縮目前沒有取消鈕（關掉頁面會中止並刪掉 `.partial`）
- 工程師筆記: `POST /api/zip-dir`、`/api/zip-sync` 改回傳 NDJSON 串流（`lobj_api.rs::ndjson_response`、`ZipProgress`；`zipsync.rs::rsync_run`、`parse_progress`），前端 `readStream` 解析；前端中斷連線時 `blocking_send` / `send` 失敗即停止，rsync 由 `kill_on_drop` 殺掉 #ui #progress

### 1.14.2
- 新功能: 選取磁碟上的 .zip 後出現「同步到…」與「刪除」：同步視窗輸入 `user@host`，按「連線」瀏覽遠端資料夾（點資料夾進入、`..` 回上層，空白＝家目錄），按「同步到這個資料夾」以 rsync（ssh）傳送；傳送中關閉視窗可中止。主機與最後的遠端資料夾會記在瀏覽器
- 新功能: 密碼欄（金鑰密碼或 ssh 登入密碼）：ssh-agent / 免密碼金鑰可留空；填了只用於該次連線，不儲存、不寫檔。遠端資料夾不可寫入時會提示並停用同步
- 改變: 左側清單的資料夾與 .zip 改為單擊＝選取（再點取消）、雙擊＝進入；`..` 仍是單擊。目前人在 zip 裡面時，裡面的 zip 不提供同步 / 刪除
- 已知限制: 只在伺服器監聽本機位址時可用；連接埠請寫在 `~/.ssh/config`（主機欄不接受 `:`）；遠端需是 Linux / macOS 且 shell 支援 POSIX `{ }`（bash / zsh / sh；fish 不行）；本機需有 `ssh` 與 `rsync`；第一次連線的遠端主機金鑰自動記下（`accept-new`）
- 工程師筆記: 新增 `src/zipsync.rs`（`POST /api/remote-ls`、`/api/zip-sync`、`/api/zip-delete`）；host 嚴格檢查避免被當成選項；密碼經 `SSH_ASKPASS` 指向本執行檔（`main` 開頭 `askpass_mode()`，密碼只放子行程環境變數），沒密碼時 `BatchMode=yes`，有密碼時 `NumberOfPasswordPrompts=1`；子行程 `kill_on_drop` #ui #zip #ssh

### 1.14.1
- 新功能: 選擇檔案區新增「壓縮資料夾」按鈕：把目前資料夾（含子資料夾）壓成與它同層的「資料夾名.zip」，用來傳到另一台電腦；不會觸發瀏覽器下載，完成後在下方資訊區顯示 zip 的完整路徑、檔案數與大小
- 改變: 不分副檔名全部壓縮（略過符號連結與無法開啟的檔案）；同名 zip 已存在時詢問是否覆蓋，先寫 `.partial` 完成才改名，失敗不會留下半套
- 已知限制: 只在伺服器監聽本機位址時可用（會在伺服器上寫檔，需 `X-Lobj-Requested` 標頭）；根目錄與 zip 內的資料夾不能壓縮；同步到別台電腦（rsync / ssh）尚未做
- 工程師筆記: 新增 `POST /api/zip-dir`（`lobj_api.rs::zip_dir`、`zip_add_dir`，用既有 `zip` crate，無新相依）；前端 `#zip-dir` 按鈕與 i18n `zipDir*` / `zip*` 三語 #ui #zip

### 1.13.3
- 改變: 預設 port 依建置模式決定：開發（debug）建置 8079，發行（release）建置 8080，讓開發版能與已安裝的穩定版同時執行；`--port` 仍可覆蓋
- 改變: `web` 的 Vite 開發 proxy 預設指向 8079（可用 `LOBJ_API` 覆蓋）
- 工程師筆記: `DEFAULT_PORT` 以 `cfg!(debug_assertions)` 決定 #dev

### 1.13.2
- 修正: 右側設定面板（alpha、wire、normal、curv、color / mix）原本是一份對所有物件共用的設定，改任何屬性都會讓全部物件一起變；現在只作用在「作用中物件」（點右側清單檔名切換），其他物件不受影響
- 改變: 每個物件各自記住自己的屬性，取消選取再選回來時沿用；曲率圖例也讀該物件自己的設定；solo 對 alpha 的規則不變（非 solo 物件 alpha 最高 0.5）
- 工程師筆記: 移除 `common: ObjProps`；`effProps(f)` 改為 `{ ...getProps(f), alpha }`，`buildSettings` 編輯 `getProps(activeFile())` 且 `updateAll` 只更新作用中物件；開發環境改在 Linux，`web/node_modules` 需在本機重裝（`npm install`）#ui #bugfix

### 1.13.1
統整 1.12.1 ～ 1.12.3 的發佈版，細節請看下方各舊版本。
- 新功能: 右側面板改成「共同設定」（alpha、wire、normal、curv、color 含自訂色，對所有物件一起生效）加物件清單；每列有 h（隱藏）與 s（solo，有 solo 時其他模型 alpha 最高 0.5）；物件資訊、jaw、轉 .lobj2、曲率圖例顯示「作用中」物件，點檔名切換並短暫顯示其 AABB（1.12.1）
- 改變: 左側檔案清單一般點擊＝加選／取消，Shift 範圍選取，雙擊＝只開這一個（1.12.2）
- 新功能: 選擇檔案區新增「瀏覽…」資料夾選擇對話框（常用位置、釘選、最近；可拖曳、改大小、調透明度），按「開啟」切換主畫面資料夾（1.12.3）
- 已知限制: 釘選與最近存在瀏覽器 localStorage，換瀏覽器不會跟著走；非本機模式（`--host 0.0.0.0`）左側只有 --dir
- 工程師筆記: 由 1.12.* 統整而來，僅調整版本號與版本紀錄，功能無變動 #versioning

### 1.12.3
- 新功能: 選擇檔案區新增「瀏覽…」按鈕，開啟資料夾選擇對話框（非遮蔽式浮動面板，可拖曳標題列移動）：左側有常用位置（--dir、使用者、桌面、文件、下載、磁碟機）、已釘選（📌）與最近開過的資料夾，右側列出該資料夾的子資料夾（含 `..`、zip）與支援的檔案
- 新功能: 對話框內點檔案＝加入選取、Ctrl/⌘ 加選、Shift 範圍選取、雙擊＝只開這一個（與主畫面清單一致、共用同一組選取）；按「開啟」才把主畫面切到這個資料夾
- 新功能: 對話框可從右下角拖曳改變寬高，標題列的 ◐ 滑桿可調背景透明度（預設 0.85），讓點選檔案時看得到後面主畫面的變化；大小與透明度會記住（localStorage）
- 已知限制: 釘選與最近存在瀏覽器的 localStorage，換瀏覽器不會跟著走；路徑相對於 --dir，換了 --dir 後舊項目可能失效（點了會顯示錯誤，可按 ✕ 移除）。非本機模式（`--host 0.0.0.0`）左側只有 --dir
- 工程師筆記: 新增 `GET /api/places`（`lobj_api.rs::places_for`）與前端 `web/src/picker.ts`（`initPicker` 以相依注入共用 main.ts 的 `selected`、`openOnly`、`browse`、`withBusy`）；`browse` 成功時呼叫 `recordRecent` #ui #picker

### 1.12.2
- 改變: 左側檔案清單的點擊行為：一般點擊＝加選／取消（原本需按 Ctrl，現在預設）；Shift+點擊、Ctrl+點擊維持原樣；雙擊＝清掉其他、只開這一個（原本的一般點擊）
- 工程師筆記: `clickFile` 移除「一般點擊＝只選這個」分支，並以 `e.detail > 1` 略過雙擊的第二下；新增 `openOnly`，由 `li` 的 `dblclick` 觸發 #ui

### 1.12.1
- 新功能: 右側面板改成「共同設定」一塊（alpha、wire、normal、curv、color / mix 對所有物件一起生效），下方是物件清單，每列有 h（hide，隱藏）與 s（solo）核取方塊
- 新功能: solo 規則：沒有任何 solo 勾選時依設定顯示；有 solo 勾選時，非 solo 的物件 alpha 若高於 0.5 就以 0.5 顯示（低於 0.5 用設定值），solo 的物件維持設定的 alpha
- 新功能: 物件資訊（頂點 / 三角形 / labels）、jaw、轉 .lobj2、曲率圖例也移到共同區，顯示「作用中」物件；點清單的檔名切換作用中物件，並以黃色外框顯示其 AABB 約 2 秒
- 新功能: wire 與 normal 併成一行；color 多一格彩虹圓「自訂」，開色彩對話框選任意顏色（以 `#rrggbb` 存）
- 已知限制: 沒有 label 的模型，color 為「原色」時仍各自用自動配的顏色；非點檔名造成的作用中物件改變不會觸發 AABB
- 工程師筆記: `common: ObjProps` 為共同設定，`effProps(f)` 合成實際屬性（含 solo 對 alpha 的調整），`hiddenSet` / `soloSet` 存 h / s，hide 用 `mesh.visible`；`buildCard` 拆成 `buildSettings` + `buildRow` #ui #solo

### 1.11.1
統整 1.10.1 ～ 1.10.5 的發佈版，細節請看下方各舊版本。
- 新功能: 伺服器只監聽本機位址（預設 localhost）時，瀏覽 / 開檔 / 轉檔不再限制在 `--dir` 之內，可到整台電腦的任何資料夾；`--host 0.0.0.0` 時維持只限 `--dir` 之內（1.10.1）
- 新功能: 左側（狀態列、選擇檔案）寬度可拖曳調整，雙擊還原預設，寬度會記住；寬度不足時選項自動換行（1.10.2）
- 新功能: 介面多語言：中文（預設）、English、Tiếng Việt，狀態列「lang」按鈕切換，選擇會記住（1.10.3、1.10.4）
- 新功能: 右側物件卡片的 labels 清單預設收合，只顯示「N 種label」，點開才看到各 label 的頂點數（1.10.5）
- 已知限制: 版本紀錄與後端訊息仍為中文；越南文為 AI 翻譯，尚待母語者校對
- 工程師筆記: 由 1.10.* 統整而來，僅調整版本號與版本紀錄，功能無變動 #versioning

### 1.10.5
- 新功能: 右側物件卡片的 labels 清單預設收合，只顯示「N 種label」（English: `N labels`、Tiếng Việt: `N nhãn`），點開才看到各 label 的頂點數；展開狀態在重繪 / 切換語言時保留
- 工程師筆記: `renderInfo` 用 `<details class="card-labels">`，展開的檔案記在 `labelsOpen`；`info.labelKinds` 為不同 label 數（含 -1）；字串 `t().labelKinds(n)` #ui #labels

### 1.10.4
- 新功能: 介面新增越南文（Tiếng Việt）：`lang` 選單多一個選項，與中文 / English 一樣即時切換、選擇會記住
- 已知限制: 同 1.10.3，版本紀錄與後端訊息仍為中文；越南文為 AI 翻譯，牙科術語（hàm trên / hàm dưới、nhãn、độ cong 等）尚待母語者校對
- 工程師筆記: `i18n.ts` 新增 `vi: Dict`，`Lang` 加 `'vi'`，列入 `DICTS` / `LANGS`（`html: 'vi'`）#i18n #vi

### 1.10.3
- 新功能: 介面多語言：預設中文，可切換成 English。狀態列「檢查更新」右邊的按鈕永遠寫 `lang`（不論目前是哪種語言都找得到），點開列出各語言（以該語言自己的名稱顯示），選擇會記住；切換時已載入的模型與卡片設定不受影響
- 已知限制: 版本紀錄（本頁）與後端回傳的錯誤訊息仍為中文；切換前已顯示的一次性訊息（例如「已轉成…」）維持原語言
- 工程師筆記: 字串集中在 `web/src/i18n.ts`（`zh` 為基準字典，其他語言型別為 `Dict`，缺 key 會編譯失敗）；靜態 HTML 用 `data-i18n` / `data-i18n-title` / `data-i18n-placeholder`，動態字串用 `t().xxx`，`onLangChange` 重繪；存 `localStorage['lobj-view-lang']`。新增語言（如越南文）只要加一份字典並列入 `DICTS` / `LANGS` #i18n #lang

### 1.10.2
- 新功能: 左側（狀態列、選擇檔案）寬度可拖曳調整：拖右邊緣改變寬度，雙擊還原預設，寬度會記住；寬度不足時，狀態列選項、路徑與 regex 輸入框自動換到下一行，檔案清單可橫向捲動
- 優化: 預設寬度縮短（約 34em）
- 工程師筆記: `#hud` 寬度由 CSS 變數 `--hud-w` 控制，`#hud-resize` 為拖曳把手（pointer events），存 `localStorage['lobj-view.hud-w']`；`#path` / `#jaw-re` 改為 flex 換行 #ui #resize

### 1.10.1
- 新功能: 伺服器只監聽本機位址（預設 localhost）時，瀏覽 / 開檔 / 轉檔不再限制在 `--dir` 之內，可到整台電腦的任何資料夾（「..」可一路往上；`--dir` 之外以 `D:/…` 這類絕對路徑表示）；`--host 0.0.0.0` 時維持原本只限 `--dir` 之內
- 工程師筆記: `resolve_ex` / `list_dir_ex` 加 `unrestricted`（= `Shared::local_only()`）；`Resolved.in_zip` 取代 `base != root` 判斷 zip；`virt_of` / `parent_virt`；`/api/lobj` 回應多 `up` 欄位，前端「..」改用它 #localhost #dir

### 1.9.1
統整 1.8.1 ～ 1.8.11 的發佈版，細節請看下方各舊版本。
- 新功能: 線上更新：版本號旁「檢查更新」，有新版時變綠色「⬆ 有新版 vX」，一鍵下載、驗證、替換並在同 port 重新啟動；命令列 `lobj-view update` / `update --check`；`--allow-remote-update` 可讓區網網頁也能更新（1.8.1）
- 新功能: 左上角狀態列新增「正交投影」與「單面」核取方塊，預設皆關閉（1.8.2、1.8.3）
- 新功能: 舊 `.lobj` 的上 / 下顎改由檔名 regex 判定（預設 `upper;_u_;u\d+-;ur`），卡片可個別覆蓋；檔案清單顯示綠色 `up` / `lo`，與檔名字面矛盾時顯示紅色 `er`（1.8.4、1.8.6 ～ 1.8.8）
- 新功能: 卡片「轉 lobj2」：`.lobj` / `.stl` / `.ply` / `.obj` 轉成 `.lobj2`（label 為 FDI），可選刪舊檔或保留，新檔沿用舊檔時間，同名不覆蓋，寫完讀回比對（1.8.5、1.8.9、1.8.10）
- 新功能: `.lobj2` 的上 / 下顎由牙位 FDI label 判定，沒有牙時退回檔名 regex（1.8.11）
- 已知限制: 線上更新需連 github.com 且執行檔資料夾可寫入；zip 內檔案不能轉檔；轉檔只在伺服器監聽本機位址時可用；建立時間只在 Windows 保留；`.stl` / `.ply` / `.obj` 轉出的 `.lobj2` 沒有 label
- 工程師筆記: 由 1.8.* 統整而來，僅調整版本號與版本紀錄，功能無變動 #versioning

### 1.8.11
- 新功能: `.lobj2` 的上 / 下顎判定：載入後看牙位 FDI label（11–48、51–85）判斷；沒有牙（例如沒有 label 的 stl / ply / obj 轉出、labels 全是 -1）就退回檔名 regex 判定。檔案清單載入後的 `.lobj2` 前面也有綠色 `up` / `lo`，滑鼠移上去說明依據；卡片資訊多一行 `jaw：…（依牙位 label / 依檔名 regex）`
- 優化: 牙位判斷與檔名字面矛盾（例如 label 是上顎牙但檔名含 `lower`），或 label 同時有上下顎的牙，顯示紅色 `er`
- 已知限制: `.lobj2` 要載入後才看得到 label，所以沒選取的 `.lobj2` 清單不顯示標記；卡片沒有 `.lobj2` 的 jaw 覆蓋選單
- 工程師筆記: 純前端：`jawFromLabels`（labels 已由後端轉為 FDI）、`Loaded.labelJaw`、`lobj2JawNote`；`renderInfo` 一併呼叫 `updateJawBadges` #jaw #lobj2 #FDI

### 1.8.10
- 優化: `.stl` / `.ply` / `.obj` 轉 `.lobj2` 不再自動更名（取消 1.8.9 加上 `upper_` / `lower_` 的做法），只改副檔名；檔名含相反字樣也不再拒絕。同名 `.lobj2` 已存在一樣不覆蓋
- 已知限制: `.stl` / `.ply` / `.obj` 沒有 label，轉出的 `.lobj2` labels 全是 -1，檔案本身看不出上 / 下顎（`.lobj2` 的上下顎是由 FDI 牙位 label 判斷）；這幾種格式轉換時的上 / 下顎判定目前不會寫進檔案
- 工程師筆記: 移除 `jaw_stem`；`convert` 一律 `with_extension("lobj2")`，非 `.lobj` 也不再需要 `jaw` #convert

### 1.8.9
- 新功能: `.stl` / `.ply` / `.obj` 的卡片也有「轉 lobj2」鈕，依判定的上 / 下顎轉存成 `.lobj2`（沒有 label，頂點 / 三角形 / 顏色照舊），同樣可選刪舊檔（預設）或保留，新檔沿用舊檔的建立 / 修改 / 存取時間
- 優化: 沒有 label 的 `.lobj2` 只能靠檔名記上下顎，所以新檔名若沒有 `upper` / `lower` 會在前面加 `upper_` / `lower_`（例如 `a.stl` → `upper_a.lobj2`）；已含判定的字就維持原名；含相反的字或兩個都含（紅色 `er` 的情形）會被拒絕並說明，需先改檔名；同名檔已存在一樣不覆蓋
- 工程師筆記: 後端 `convert` 依副檔名分流，新增 `jaw_stem`；`.stl` / `.ply` / `.obj` 一定要帶 `jaw`（缺少回 400）；前端的轉換鈕不再限舊 `.lobj` #convert #lobj2 #jaw_stem

### 1.8.8
- 優化: 檔案清單的上 / 下顎標記改為只顯示一個字：判定為 upper 顯示綠色 `up`、lower 顯示綠色 `lo`；判定與檔名字面矛盾（判成 upper 但檔名含 `lower`，或相反）時顯示紅色 `er`，滑鼠移上去看實際判定與說明（取代 1.8.7 的 `up lo` 兩字並列）
- 工程師筆記: 純前端，`updateJawBadges` 只產生一個 `<span>`（`.on` / `.bad`）#jaw #badge

### 1.8.7
- 新功能: 上 / 下顎判定（regex）也套用到 `.stl` / `.ply` / `.obj`（它們沒有 label，之後轉 `.lobj2` 要知道上下顎）；`.lobj2` 與 `.zip` 不判定。這些檔案的卡片也有 jaw 選單可個別覆蓋。目前只做判定，「轉 lobj2」仍只有舊 `.lobj`
- 優化: 檔案清單的標記改為 `up lo` 兩個字都寫，判定的那個用 iOS 綠色（`--systemGreen`），另一個淡灰；判定與檔名字面矛盾（判成 upper 但檔名含 `lower`，或相反）時判定的那個改紅色（`--systemRed`），滑鼠移上去說明
- 工程師筆記: 純前端：`isJawFile`、`jawConflict`、`updateJawBadges` 改為產生兩個 `<span>`（`.on` / `.bad`）；只有舊 `.lobj` 的載入受 jaw 影響，所以 jaw 變動時只重載 `.lobj`，後端不用改 #jaw #badge

### 1.8.6
- 新功能: 舊 `.lobj` 的上 / 下顎判定改用 regex（暫時作法）：左側「選擇檔案」原本的「jaw: 自動」下拉選單換成輸入框，預設 `upper;_u_;u\d+-;ur`；以 `;` 分隔（檔名不會有分號）、不分大小寫，檔名符合任一個就是 upper，否則一律 lower。輸入時即時生效，已載入且判定改變的檔案會自動重載
- 新功能: 檔案清單中每個 `.lobj` 前面多了小字 `up` / `lo`，表示目前此資料夾內該檔的判定結果（含卡片個別指定的覆蓋）；`.lobj2` / `.stl` / `.obj` / `.ply` 不需要所以留空
- 優化: regex 有語法錯誤的段落會略過並讓輸入框變紅（滑鼠移上去看哪一段）；右側卡片的 jaw 選單仍可個別覆蓋；因為一定判得出上 / 下顎，「轉 lobj2」鈕現在每個 `.lobj` 都有
- 已知限制: 不再使用檔名含 `upper` / `lower` 的內建判斷，只含 `lower` 的檔名是靠「不符合 upper」才判成 lower；regex 設定重新整理後會回到預設值
- 工程師筆記: 純前端：`main.ts` 的 `jawRegexes` / `parseJawRegexes` / `regexJaw` / `effectiveJaw` / `updateJawBadges`；`Loaded.jaw` 記錄載入時用的判定，regex 變動時只重載判定結果改變的檔案；後端不用改（對 `.lobj` 一律帶 `?jaw=`）#jaw #regex

### 1.8.5
- 新功能: 右側物件卡片上，已知上 / 下顎（卡片指定、左側預設、或檔名含 upper / lower）的 `.lobj` 出現「轉 lobj2」鈕：跳出視窗確認，預設「轉換後刪除舊檔」，也可選「保留舊檔」；轉出的 `.lobj2`（label 為 FDI）放在同資料夾、同主檔名，建立 / 修改 / 存取時間與舊檔相同，成功後選取自動換成新檔並更新檔案清單
- 優化: 同名 `.lobj2` 已存在時拒絕（不覆蓋）；寫完會讀回比對頂點、三角形與 label，不一致就刪掉新檔並放棄，確認無誤才刪舊檔
- 已知限制: zip 內的檔案不能轉；只在伺服器監聽本機位址時可用（會寫入 / 刪除檔案）；建立時間只在 Windows 保留
- 工程師筆記: 新增 `POST /api/lobj-convert`（`lobj_api.rs` 的 `convert`，沿用 `assoc::guard_with`：本機 + `X-Lobj-Requested`）；用 `lobj::load(jaw)` + `lobj::save`，時間以 `File::set_times`（Windows 加 `FileTimesExt::set_created`）；前端 `convertDialog` 使用原生 `<dialog>` #convert #lobj2 #FileTimes

### 1.8.4
- 新功能: 舊 `.lobj` 的上 / 下顎改為「每個檔案各自指定」：右側物件卡片（僅 `.lobj`）新增 jaw 選單（預設 / upper / lower），兩個檔名沒規律的檔案可以一個上顎、一個下顎同時載入；改選只會重新載入該檔案
- 優化: 左側原本的 jaw 選單變成「預設值」，只影響沒有自己指定的舊 `.lobj`；優先順序為 卡片指定 > 左側預設 > 檔名含 upper / lower 判斷
- 工程師筆記: 後端不用改（`?jaw=` 本來就是每個請求獨立）。前端 `main.ts` 新增 `jawOf: Map<檔案, 'upper'|'lower'>`、`effectiveJaw(f)`、`reloadFiles()`；載入途中 jaw 被改時以 `jawUsed` 比對丟棄過期結果。jaw 設定與 props 一樣取消選取後保留，不記入 localStorage #jaw #per-file

### 1.8.3
- 新功能: 左上角狀態列新增「單面」核取方塊，預設關閉（維持原本的雙面繪製）；勾選後網格只繪正面（三角形頂點逆時針的那一面），背面不畫，可從背面看穿模型，也可用來檢查三角形方向是否一致
- 工程師筆記: 原本材質固定 `THREE.DoubleSide`；改為 `meshSide()` 依 `singleSided` 回傳 `FrontSide` / `DoubleSide`，切換時逐一更新已載入模型的 `material.side` 並設 `needsUpdate`，之後新載入的模型沿用目前設定。wire / normal 線段不受影響。此選項不記入 localStorage #cull #FrontSide

### 1.8.2
- 新功能: 左上角狀態列（格線旁）新增「正交投影」核取方塊，預設關閉；勾選後改用正交相機（平行投影、沒有近大遠小），切換時視角與畫面大小維持不變，取消勾選回到透視
- 工程師筆記: `main.ts` 新增 `orthoCamera`，`camera` 改為可切換的 `let`，`setOrthographic` 換相機並同步 `controls.object`；正交可視範圍依目標點距離與 fov 計算（`updateOrthoFrustum`），滾輪縮放走 `zoom`，換回透視時把 zoom 換算成距離。此選項不記入 localStorage #ortho #camera

### 1.8.1
- 新功能: 線上更新：版本號旁新增「檢查更新」鈕，開頁時會安靜地查一次 GitHub，有新版時變成綠色「⬆ 有新版 vX」；點開後按「更新到 vX 並重新啟動」，會下載新版取代執行檔、伺服器在同一個 port 自動重新啟動，網頁等到新版上線就自動重新整理（已開的模型需重新載入）
- 新功能: 命令列 `lobj-view update`（下載並替換）與 `lobj-view update --check`（只檢查）；伺服器以 `--host 0.0.0.0` 對外開放時網頁預設不開放更新，請在伺服器上用這個指令
- 新功能: 啟動參數 `--allow-remote-update`：搭配 `--host 0.0.0.0` 時明確開啟，區網上的網頁也能按更新（預設關閉；開啟後任何能開網頁的人都能讓伺服器更新並重新啟動）
- 優化: 下載後先驗證大小與 GitHub 提供的 sha256，並試跑 `新檔 --version` 確認版本正確才替換；任何一步失敗都不動到原本的執行檔
- 已知限制: 需要能連上 github.com；執行檔所在資料夾要有寫入權限（例如裝在 Program Files 就不行）；GitHub 發佈後 Actions 還在建置時會顯示「還沒有這個平台的檔案，請過幾分鐘再試」；用 `cargo install` 從原始碼裝的也會被換成 Release 的預編譯檔
- 工程師筆記: 新增 `src/update.rs`：`/api/update`（GET 查詢，結果快取 10 分鐘，`?force=1` 略過；POST 更新，與副檔名關聯共用 `assoc::guard_with` 只允許本機 + `X-Lobj-Requested`；`--allow-remote-update` 時 `update::guard` 略過本機與 Host 檢查，但仍要 `X-Lobj-Requested`，重新啟動沿用原參數所以設定會保留）。ureq 3（rustls + ring）查 `releases/latest`，依 build.rs 傳入的 `LOBJ_VIEW_TARGET` 挑 asset（linux-gnu 對應 musl 版），zip / tar.gz 取出 `lobj-view(.exe)`，`self_replace` 換掉執行中的檔案。重新啟動：主迴圈的 `select!` 多等 `restart_requested()` → 清掉 `instance.txt` → 以相同參數加環境變數 `LOBJ_VIEW_RESTART=<pid>:<port>` 重新執行（Unix 用 `exec` 保留 pid；Windows 開子程序後結束，新程序以 sysinfo 等舊 pid 結束）→ 新程序沿用 port（`bind_retry`）、不開瀏覽器 #update #self_replace #ureq #restart

### 1.7.1
統整 1.6.1 ～ 1.6.2 的發佈版，細節請看下方各舊版本。
- 新功能: .zip 當資料夾用：檔案瀏覽裡的 .zip 顯示成 📦 資料夾，可點進去看裡面的模型檔，也可貼 `資料夾/a.zip/子資料夾/檔案.stl` 路徑（zip 裡的 zip 也可以）；原 .zip 不會被改動，第一次進入才在背景解壓到快取（1.6.1）
- 新功能: 太大的 zip 解壓或載入模型時，左側顯示「處理中… 名稱（N 秒）」與「取消」鈕，取消後不留半套檔案（1.6.1）
- 優化: 解壓只取模型檔與內層 zip；zip 被換掉會自動重新解壓；快取放系統 `lobj-view/zipcache`，超過 7 天沒用於下次啟動時清掉（1.6.1）
- 新功能: 左上角狀態列新增「伺服器」一列：人 / 分頁數、CPU、記憶體，有 NVIDIA 顯卡時另顯示 GPU；達 70% 變黃、90% 變紅（1.6.2）
- 已知限制: zip 內非 UTF-8 檔名（舊版 Windows 壓縮軟體的 Big5 / GBK）可能亂碼；加密 zip 無法開啟；「人」以來源 IP 計算，ssh -L 轉發只算一人
- 工程師筆記: 由 1.6.* 統整而來，僅調整版本號與版本紀錄，功能無變動；zip 實作在 `src/zipfs.rs`，伺服器狀態在 `src/stats.rs` 與 `instance.rs` 的 `tab_guard` #versioning #zip #stats

### 1.6.2
- 新功能: 左上角狀態列新增一列「伺服器」：目前有幾「人」/ 幾個分頁在使用、CPU 使用率、記憶體用量，伺服器有 NVIDIA 顯卡（`nvidia-smi`）時另顯示 GPU；達 70% 變黃、90% 變紅，滑鼠移上去看細節（lobj-view 本身的用量、顯卡型號與顯存）。覺得變慢時可以先看是不是伺服器忙
- 優化: 「人」以不同的來源 IP 計算；用 ssh -L 轉發的連線都來自 127.0.0.1，只會算成一人。GPU 是「伺服器」的，3D 繪圖用的是開瀏覽器那台電腦的 GPU，兩者不一定相同
- 工程師筆記: 新增 `src/stats.rs`（sysinfo）：背景執行緒每 3 秒取樣並快取，`/api/stats` 只讀快取，所以輪詢的人再多成本也固定；超過 20 秒沒人讀就暫停取樣；`nvidia-smi` 約每 6 秒一次、Windows 以 CREATE_NO_WINDOW 避免閃黑視窗，找不到就不再嘗試。`instance.rs` 的 `tab_guard(ip)` 以 `HashMap<IpAddr, 分頁數>` 記錄，伺服器改用 `into_make_service_with_connect_info` 取得來源位址。前端約每 5 秒輪詢（分頁隱藏時暫停）#stats #sysinfo #nvidia-smi #ConnectInfo
### 1.6.1
- 新功能: .zip 當資料夾用：檔案瀏覽裡的 .zip 會顯示成 📦 資料夾，點進去就能看到裡面的模型檔，也可以貼 `資料夾/a.zip/子資料夾/檔案.stl` 這種路徑，zip 裡的 zip 也可以；原 .zip 完全不會被改動（只讀取），第一次進入時才在背景解壓到快取，之後再進入是即時的
- 新功能: 太大的 zip 解壓要等時，左側會跳出「處理中… 名稱（N 秒）」提示與「取消」鈕，按下去就不會進去、也不會留下半套檔案，原本的畫面不變；載入模型時同樣可取消
- 優化: 解壓時只取模型檔（.lobj / .lobj2 / .stl / .obj / .ply）與內層 zip，其他檔案不解出來，更快也更省空間；zip 被換掉（大小或修改時間不同）會自動重新解壓
- 優化: 快取放在系統的 `lobj-view/zipcache` 資料夾，超過 7 天沒用會在下次啟動時自動清掉
- 工程師筆記: 新增 `src/zipfs.rs`（zip crate、deflate）：解壓到 `<key>.partial` 再改名、`AtomicBool` 取消旗標每個 chunk 檢查、同一 zip 以 per-key Mutex 避免重複解壓（等待者也可取消）、`enclosed_name` 擋路徑穿越、略過符號連結、總量 8 GB / 檔數 50000 上限。`lobj_api.rs` 的 `resolve` 回傳 `Resolved{real, virt, base}`，路徑穿過 zip 時以 lexical normalize 找出 zip 祖先再 `descend`（可巢狀）；列表 JSON 新增 `zips`。取消機制：前端 `AbortController` 中斷連線 → axum 丟棄 handler → `CancelOnDrop` 設旗標。前端 `withBusy`（400 ms 內完成不顯示提示）#zip #cancel #AbortController #spawn_blocking 詳見 261119m
- 已知限制: zip 內檔名若不是 UTF-8 旗標（舊版 Windows 壓縮軟體的 Big5 / GBK 檔名）可能顯示亂碼；加密 zip 無法開啟

### 1.5.1
統整 1.4.1 ～ 1.4.6 的發佈版，細節請看下方各舊版本。
- 新功能: 沒有 label 的模型（.stl / .obj / .ply，或 labels 全為 -1）載入後自動配色，優先挑畫面上還沒被用過的顏色；檔案自帶頂點色的預設疊加 0.2、保留原色（1.4.1）
- 新功能: 右側物件卡片新增 wire 核取方塊，顯示模型 mesh 的黑色邊線，隨 alpha 變化（1.4.2）
- 新功能: 新增 normal 核取方塊，在每個頂點畫出法向量；沒有法向量的檔案以計算出的頂點法向量顯示（1.4.3）
- 新功能: 新增 curv 核取方塊，以曲率上色（凸凹 / max |κ| / min |κ|），附色階與 κ 範圍；計算方式見 doc/curvature.md（1.4.4）
- 新功能: 曲率「截斷」百分位滑桿（p50–p100），即時調整色階範圍，雙擊回預設（1.4.5）
- 新功能: 曲率「平滑」滑桿（關、×1–×20），先平滑法向量再算曲率，壓掉口掃雜訊、保留牙齦線 / 牙尖（1.4.6）
- 工程師筆記: 由 1.4.* 統整而來，僅調整版本號與版本紀錄，功能無變動；曲率實作在 `web/src/curvature.ts`，物件屬性在 `props.ts`（`ObjProps.wire` / `normal` / `curvClip` / `curvSmooth`）#versioning #curvature #auto-color

### 1.4.6
- 新功能: curv 勾選後多一列「平滑」滑桿（關、×1–×20），算曲率前先把法向量與相鄰頂點平均幾次，壓掉口掃雜訊、保留牙齦線 / 牙尖這類較大尺度的凹凸；只影響曲率顏色，模型本身與 normal 線不變。放開滑桿才重算（18 萬頂點 ×5 約 0.8 秒），算過的次數會記住，切回去不必再算
- 工程師筆記: `curvature.ts` 的 `smoothNormals`（位置不動，只平滑法向量）；曾試先 Laplacian / Taubin 平滑頂點位置，但頂點沿切向漂移讓三角形變細長，|e| 變小反而讓 p99 從 6.1 升到 6.9，故改平滑法向量（×10 後 p99 6.1 → 1.4、p90 1.68 → 0.67；球面 ×10 仍為 1/R）。`ObjProps.curvSmooth`，曲率依次數快取於 `mesh.userData.curvBySmooth` #curvature #smoothing #normals

### 1.4.5
- 新功能: curv 勾選後多一列「截斷」滑桿（p50–p100），即時調整曲率色階範圍：凸凹模式 = ±|κ| 的該百分位，越小對比越強；max / min 模式 = 第 min(1, 100−p)–p 百分位，p99.5 即 Python 的 0.5%–99.5%。各模式分開記（預設凸凹 p90、max / min p95），雙擊滑桿回預設
- 工程師筆記: `curvature.ts` 快取各模式排序後的陣列（`Curvature.sorted`），拖滑桿只查表 + 重算顏色；`ObjProps.curvClip`、`DEFAULT_CLIP`；`curvatureColoring` 改以 `mode:clip` 為快取鍵 #curvature #percentile #slider

### 1.4.4
- 新功能: 右側物件卡片新增 curv 核取方塊，勾選後以曲率上色（取代 color），旁邊可選模式：凸凹（預設，紅 = 凸如牙尖 / 牙齒邊緣，藍 = 凹如牙齦線 / 窩溝，灰 = 平）、max |κ|、min |κ|；下方顯示色階與 κ 範圍（單位 1/mm，滑鼠移上去看對應曲率半徑）
- 工程師筆記: 新增 `web/src/curvature.ts`，移植 AppLabelTooth `compute_vertex_curvature_normal_variation_edge.py`（κ_edge = Δn·e / |e|²，每頂點取相鄰邊 |κ| 的 max / min），另加帶正負號的平均（signed）；球面測試得 κ = 1/R。不做 Python demo 的除標準差尺度正規化（保留物理單位）；口掃 mesh 的 κ 分佈尾巴很長，截斷改為 signed 取 |κ| 90 百分位、max / min 取 1%–95%（Python 為 0.5%–99.5%）。第一次勾選才計算並快取於 `mesh.userData.curv` #curvature #normal-variation #viridis

### 1.4.3
- 新功能: 右側物件卡片新增 normal 核取方塊，勾選後在每個頂點畫出法向量（藍色短線，長度為模型外接球半徑的 3%）；沒有法向量的檔案（.stl / .obj / .ply）以計算出的頂點法向量顯示
- 工程師筆記: `props.ts` 的 `ObjProps.normal` 與 `buildNormalLines`（每頂點一段 `LineSegments`，存於 `mesh.userData.normals`，第一次勾選才建立，`disposeMesh` 一併釋放）#normals #LineSegments

### 1.4.2
- 新功能: 右側物件卡片新增 wire 核取方塊，勾選後顯示該模型 mesh 的邊線（黑線）；邊線隨 alpha 透明度變化，取消選取再選回來設定仍保留
- 工程師筆記: `props.ts` 的 `ObjProps.wire`；邊線為 `WireframeGeometry` + `LineBasicMaterial` 的 `LineSegments`，第一次勾選才建立並掛在 mesh 底下（存於 `mesh.userData.wire`），`disposeMesh` 一併釋放；勾選時面材質加 polygonOffset 避免 z-fighting #wireframe #LineSegments #polygonOffset

### 1.4.1
- 新功能: 載入後若模型沒有 label（.stl / .obj / .ply，或 labels 全為 -1），自動配一個 color，並優先挑目前畫面上還沒被其他模型用過的顏色；同時載入的模型比色票多時才重複（挑用得最少的）。疊加比例：檔案自帶顏色（.ply / .obj 頂點色）預設 0.2，保留原色；沒有顏色預設 0.6；有 label 的維持 0.6。只配一次，之後你在物件卡片改的選擇不會被覆蓋
- 工程師筆記: `props.ts` 新增 `pickAutoColor`（色票排除 Gray，因為跟未標記的灰色太像）；後端 `/api/lobj/{name}` 回應加 `X-Lobj-File-Colors` 標頭（檔案本身有沒有顏色，後端依 labels 補的不算）；`main.ts` 的 `Loaded.unlabeled` 與 `autoColored` 集合，於載入完成、`applyProps` 之前決定顏色並重建物件面板 #auto-color #palette

### 1.3.1
統整 1.2.1 ～ 1.2.6 的發佈版，細節請看下方各舊版本。
- 新功能: 單一實例：`lobj-view <檔案或資料夾>` 會把路徑交給執行中的 lobj-view（網頁自動跳到該檔），沒有才自己啟動並開瀏覽器；`--new` 另起獨立實例、`--no-browser` 不自動開瀏覽器（1.2.2）
- 新功能: Windows 副檔名關聯：`lobj-view register | unregister | status` 與網頁「預設程式…」設定頁，只寫 HKCU、不需系統管理員，不會搶走其他程式的預設（1.2.3）
- 新功能: 註冊前記錄原狀，取消註冊時依記錄還原；曾被指定為預設時提示原本的預設程式（1.2.5）
- 新功能: 註冊後 lobj-view 出現在檔案右鍵「開啟檔案」清單，顯示為「lobj-view 3D 檢視器」並帶圖示；`register` 預設處理全部五種副檔名（1.2.6）
- 優化: Windows 執行檔與網頁分頁內嵌圖示（1.2.4）
- 優化: 新增 Windows 安裝文件（全程 cmd、預編譯），Ubuntu 安裝文件改為只用預編譯檔（1.2.1）
- 工程師筆記: 由 1.2.* 統整而來，僅調整版本號與版本紀錄，功能無變動；轉交走僅聽 127.0.0.1 的 IPC，網頁寫入端點檢查 Host 與 `X-Lobj-Requested` #versioning #single-instance #assoc

### 1.2.6
- 新功能: 註冊後，lobj-view 會出現在檔案右鍵「開啟檔案」清單（含「選擇其他應用程式」），顯示為「lobj-view 3D 檢視器」並帶圖示；.stl / .ply / .obj 即使已有其他預設程式也看得到
- 優化: `register` 不指定 `--ext` 時改為處理全部五種副檔名（只有沒有其他預設程式的才會成為預設，不會搶走別人的）；網頁設定頁預設全勾；移除 `--all`（不指定即全部）
- 工程師筆記: 新增 `HKCU\Software\Classes\Applications\lobj-view.exe`（`FriendlyAppName`、`shell\open\command`、`DefaultIcon`、`SupportedTypes`）與各副檔名的 `OpenWithList\lobj-view.exe`；備份多記 `HadOpenWithList`，取消註冊依序還原，最後一個副檔名取消時連同 Applications 與 ProgId 一併移除。用 `SHAssocEnumHandlers`（「開啟檔案」選單背後的 API）驗證清單 #assoc #open-with #Applications

### 1.2.5
- 新功能: 註冊前先記錄原狀（該副檔名的機碼原本是否存在、類別預設值、當時生效的預設程式），「取消註冊」改為依記錄**還原**：原本沒有的機碼刪除、原本有的值寫回。已註冊的副檔名會顯示「還原為：…」
- 優化: 若你曾在 Windows 設定把 lobj-view 指定為預設，還原時會提示原本的預設程式，請自己改回（UserChoice 有雜湊保護，程式無法代改）
- 工程師筆記: 備份存在 `HKCU\Software\lobj-view\AssocBackup\.ext`，已有備份時不覆蓋（避免把註冊後的狀態當成原狀）；1.2.4 以前註冊的沒有備份，取消註冊時只移除我們加的部分；最後一筆備份刪除時連同 `Software\lobj-view` 一併清掉 #assoc #backup #restore

### 1.2.4
- 優化: Windows 的 lobj-view.exe 內嵌圖示（`assets/lobj-view.ico`）：藍底圓角方塊上一顆貼著三角網格的牙齒，網格中幾個三角形塗上 label 色（呼應依牙齒標記上色）；16–256 共七種尺寸，16 / 24 px 改用沒有網格的簡化版以保持清晰，`assets/lobj-view.png` 為大圖；網頁分頁也使用同一個圖示（`web/public/favicon.ico`），檔案總管、工作列，以及註冊為預設程式後的 .lobj 等檔案都會顯示它
- 工程師筆記: 新增 `build.rs`（只在 Windows 目標以 `winresource` 嵌入圖示，需要 Windows SDK 的 rc.exe；找不到時只警告、不中斷建置）；`Cargo.toml` 的 `include` 加入 `build.rs` 與 `.ico`，`cargo install` 才編得起來 #icon #winresource #build-rs

### 1.2.3
- 新功能: Windows 副檔名關聯。`lobj-view register [--ext lobj,lobj2,stl,ply,obj | --all]`、`unregister`、`status`；不需要系統管理員（只寫 HKCU）。預設只處理 .lobj / .lobj2
- 新功能: 左側狀態列新增「預設程式…」設定頁，可勾選副檔名後註冊／取消註冊，顯示每個副檔名目前的預設程式，並可一鍵開啟 Windows「預設應用程式」設定
- 優化: .stl / .ply / .obj 已有其他預設程式時不會被搶走：只註冊為「開啟方式」候選，並提示到系統設定自己指定（Windows 10 的 UserChoice 有雜湊保護，程式無法靜默覆蓋）
- 工程師筆記: 網頁寫入端點只在 `--host` 為本機位址時啟用，並檢查 `Host`（擋 DNS rebinding）與自訂標頭 `X-Lobj-Requested`（觸發 CORS 預檢、擋 CSRF）；邏輯集中在 `assoc.rs`，CLI 與網頁共用。Linux / macOS 尚未支援 #assoc #registry #winreg

### 1.2.2
- 新功能: 單一實例。`lobj-view <檔案或資料夾>` 若發現已有執行中的 lobj-view，就把路徑交給它（根資料夾切到該檔所在資料夾並選取該檔、網頁自動跳過去）後結束；沒有才自己啟動，並自動開瀏覽器。為「雙擊 .lobj 開啟」鋪路
- 新功能: 新增 `--new`（另起獨立實例，不轉交也不登記）與 `--no-browser`（帶路徑啟動時不自動開瀏覽器）
- 優化: 已有網頁連著時不會再開新分頁；網頁以 `/api/events`（SSE）接收切換通知
- 工程師筆記: 根資料夾改為執行中可切換（`Shared.root`）。轉交走只聽 127.0.0.1 的獨立 IPC（`OPEN	<token>	<路徑>`），不放在 HTTP API，避免區網、CSRF、DNS rebinding 改到根資料夾；port 與 token 寫在 `<data_local_dir>/lobj-view/instance.txt`，以 `instance.lock` 處理同時啟動的競爭，殘留的登記檔以連線失敗判定作廢 #single-instance #ipc #sse

### 1.2.1
- 優化: 新增 Windows 安裝文件 `doc/windows-install-rust-and-binstall.md`，全程使用 cmd 與預編譯的 rustup / cargo-binstall（不需 MSVC、不需管理員權限），對應 Ubuntu 版
- 優化: Ubuntu 安裝文件改為只用預編譯檔：移除 `build-essential` 與 `cargo install cargo-binstall`，`cargo binstall` 加上 `--disable-strategies compile`
- 工程師筆記: cmd 貼上多行指令可能把下一行黏在前一行參數後（例如 `-ydel`），文件中每個指令獨立成區塊；已有 rustup 的機器執行 `rustup-init -y` 會順帶更新曾加過的 target（如 `x86_64-unknown-linux-musl`）#docs #windows #cmd

### 1.1.1
統整 0.0.1 ～ 0.0.11 的第一個正式發佈版，細節請看下方各舊版本。
- 新功能: 用瀏覽器開啟網址即可檢視 3D 牙模：預設 WebGPU，不支援時自動改用 WebGL2；單一執行檔，免裝 Node.js（0.0.1）
- 新功能: 可開啟 .lobj / .lobj2（依牙齒標記上色）、.stl、.obj、.ply 檔，含頂點顏色與數量統計（0.0.4、0.0.6、0.0.7）
- 新功能: 資料夾瀏覽（限制在 `--dir` 內）、路徑欄貼上路徑、檔案多選並同時顯示（0.0.5、0.0.8）
- 新功能: 右側「物件」面板，每個物件可調 alpha 透明度與 color 疊加（0.0.9）
- 新功能: 光源（固定／頭燈／三燈）、地面格線開關、五種主題（自動／淺／深／高對比淺／高對比深），選擇會被記住（0.0.9、0.0.10）
- 新功能: 左側狀態列顯示版本，點擊開啟版本紀錄視窗（0.0.9）
- 新功能: `--host 0.0.0.0` 自動改用 HTTPS，`lobj-view trust` 與 `/trust` 頁協助其他裝置信任本機 CA（0.0.2、0.0.11）
- 新功能: 預編譯執行檔（Linux x86_64/ARM、Windows、macOS Intel/Apple Silicon），可 `cargo binstall` 安裝（0.0.3）
- 優化: 世界座標改為 z 向上（CAD 慣用），格線與座標軸隨模型自動縮放（0.0.9）
- 優化: `lobj` 讀寫程式改為獨立 GitHub crate，單獨 checkout 本專案即可建置（0.0.11）
- 新功能: 版本紀錄視窗可切換「概要／細節」，預設只列發佈版（x.奇數.1）
- 工程師筆記: 由 0.0.* 統整而來，僅調整版本號與版本紀錄顯示，功能無變動 #versioning

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
