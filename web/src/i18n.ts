// 多語言：介面字串集中在這裡。中文（zh）是基準字典，其他語言必須有一樣的 key（型別檢查會擋缺漏）。
// 新增語言（例如泰文 th）：1. 加一份 `const th: Dict = { ... }`；2. 加到 `type Lang`、`DICTS` 與 `LANGS`。
// 靜態 HTML 用 data-i18n（textContent）/ data-i18n-title / data-i18n-placeholder 標記，切換時由 applyStatic 套用；
// 動態產生的字串直接用 `t().xxx`，切換語言時由 main.ts 的 onLangChange 回呼重繪。

const zh = {
  // --- index.html：狀態列 ---
  hudResizeTip: '拖曳調整左側寬度（雙擊還原）',
  verTip: '查看版本紀錄',
  updateTip: '檢查 GitHub 上有沒有新版',
  checkUpdate: '檢查更新',
  langTip: '切換語言',
  light: '光源:',
  lightTip: '固定：光源不隨視角；頭燈：光隨眼睛位置；三燈：頭燈加左右各一',
  lightFixed: '固定',
  lightHead: '頭燈（跟隨眼睛）',
  lightThree: '三燈（眼睛＋左＋右）',
  grid: '格線',
  ortho: '正交投影',
  single: '單面',
  singleTip: '只繪網格的正面（三角形逆時針那一面），背面不畫',
  theme: '主題:',
  themeAuto: '自動（系統）',
  themeLight: '淺色',
  themeDark: '深色',
  themeHcLight: '高對比淺色',
  themeHcDark: '高對比深色',
  assocOpen: '預設程式…',
  assocOpenTip: '把 lobj-view 設成副檔名的開啟程式（雙擊檔案即開啟）',
  status3Tip: '伺服器狀態（約每 5 秒更新）',
  serverLoading: '伺服器：…',
  // --- 選擇檔案 ---
  browser: '選擇檔案',
  pathPh: '貼上路徑後按 Enter（/ 或 \\ 皆可）',
  jawRePh: 'upper 的 regex，; 分隔',
  selectHint: 'Ctrl/⌘ 點擊加選、Shift 點擊範圍選取',
  cancel: '取消',
  details: '細節',
  assocTitle: '預設開啟程式',
  updateTitle: '更新 lobj-view',

  // --- renderer / server ---
  notSecure: ' — 非 secure context，請改用 localhost / ssh -L',
  healthErr: (e: unknown) => `無法連線 (${e})`,

  // --- 上 / 下顎標記 ---
  jawSrcManual: '個別指定',
  jawSrcRegex: '依 regex 判定',
  jawSrcLabel: '依牙位 label',
  jawSrcNoTooth: 'label 無牙，依檔名 regex 判定',
  jawWhyConflict: '；異常：檔名文字與判定結果矛盾',
  jawWhyBoth: '；異常：label 同時有上顎與下顎的牙',
  jawBadgeTip: (j: string, src: string, why: string) => `${j}（${src}）${why}`,
  jawReBad: (segs: string[]) => `語法錯誤已略過：${segs.join('、')}\n`,
  jawReTip: '舊 .lobj 的上 / 下顎（暫時作法）：檔名符合任一個 regex 就是 upper，否則 lower。\n以 ; 分隔、不分大小寫。個別檔案可在右側物件卡片各自指定。',

  // --- 轉 lobj2 ---
  convTitle: (name: string) => `轉成 .lobj2：${name}`,
  convDescLobj: (jaw: string) => `以 ${jaw} 將 label 換成 FDI，存成同資料夾同名的 .lobj2；新檔的建立 / 修改 / 存取時間與舊檔相同。`,
  convDescOther: '只改副檔名存成同資料夾同名的 .lobj2（沒有 label，labels 全是 -1）；新檔的建立 / 修改 / 存取時間與舊檔相同。',
  convDelete: '轉換後刪除舊檔（預設）',
  convKeep: '保留舊檔',
  convOk: '轉換',
  converting: '轉換中…',
  convDone: (p: string, deleted: boolean) => `已轉成 ${p}${deleted ? '，舊檔已刪除' : ''}`,
  failed: (e: string) => `失敗：${e}`,
  convBtn: '轉 lobj2',
  convBtnTip: (jaw: string, legacy: boolean) =>
    `依 ${jaw} 把這個檔案轉存成 .lobj2${legacy ? '（label 改為 FDI）' : '（沒有 label，只改副檔名）'}，新檔沿用舊檔的日期`,

  // --- 物件面板 / 卡片 ---
  objects: (n: number) => `物件 (${n})`,
  noSelection: '尚未選取檔案',
  meshInfo: (v: number, t: number) => `頂點 ${v}、三角形 ${t}`,
  labelKinds: (n: number) => `${n} 種label`,
  cardJawTip: '這個檔案是上顎或下顎；「預設」= 依左側 regex 判定',
  jawDefault: '預設',
  jawUpper: 'upper（上）',
  jawLower: 'lower（下）',
  wireTip: '顯示 mesh 的邊線（黑線）',
  normalTip: '顯示每個頂點的法向量（藍線）',
  curvTip: '以曲率上色（Normal Variation，沿邊計算），會取代 color',
  curvSigned: '凸凹',
  curvSignedTip: '相鄰邊法曲率的平均（帶正負號）：紅 = 凸（牙尖、牙脊），藍 = 凹（窩溝、牙齦線）',
  curvMaxTip: '相鄰邊 |κ| 的最大值：邊緣、稜線最亮',
  curvMinTip: '相鄰邊 |κ| 的最小值：各方向都彎的地方（如牙尖頂）才亮',
  clip: '截斷',
  clipTipSigned: (v: number) => `色階範圍 = ±|κ| 的第 ${v} 百分位；越小對比越強（細微凹凸也上色），越大越不受雜訊極值影響`,
  clipTipSeq: (lo: number, v: number) => `色階範圍 = 第 ${lo}–${v} 百分位；99.5 即 Python clip_percentile 的設定`,
  smooth: '平滑',
  smoothTip: '算曲率前先把法向量與相鄰頂點平均幾次（頂點位置不動），壓掉掃描雜訊、保留較大尺度的凹凸；只影響曲率顏色。0 = 不平滑',
  smoothOff: '關',
  mix: '疊加',
  origShort: '原',
  origTip: '原色（不設定）',
  legendPending: '載入後計算…',
  concave: '凹',
  convex: '凸',
  legendTipSigned: (hi: string, r: string, clip: number) => `κ 單位 1/mm；±${hi} ≈ 曲率半徑 ${r} mm（|κ| ${clip} 百分位截斷）`,
  legendTipSeq: (lo: number, hi: number) => `κ 單位 1/mm；${lo}%–${hi}% 百分位截斷後上色`,
  lobj2JawBoth: '\njaw：上下顎的牙都有（異常）',
  lobj2JawLabel: (j: string) => `\njaw：${j}（依牙位 label）`,
  lobj2JawRegex: (j: string) => `\njaw：${j}（label 沒有牙，依檔名 regex）`,
  loadFailed: (e: string) => `載入失敗：${e}`,
  loading: '載入中…',

  // --- 處理中 / 瀏覽 ---
  busyMore: (n: number) => `等 ${n} 項`,
  busyText: (what: string, secs: number) => `處理中… ${what}（${secs} 秒）`,
  busyTip: (label: string) => `${label}\n第一次進入 .zip 需要解壓縮；按取消就不會進去（原 zip 不會被改動）`,
  emptyDir: '--dir 資料夾內沒有 .lobj / .lobj2 / .stl / .obj / .ply',
  browseCanceled: '已取消（停留在原資料夾）',
  cannotOpenPath: (e: string) => `無法開啟路徑：${e}`,

  // --- 伺服器狀態 ---
  serverPrefix: '伺服器：',
  people: (p: number, t: number) => `${p} 人 / ${t} 分頁`,
  peopleTip: '人＝不同的來源 IP；經 ssh -L 轉發的連線都來自 127.0.0.1，只算一人',
  cpuTip: (cores: number, proc: number) => `伺服器 ${cores} 邏輯核心的整體使用率；lobj-view 本身約 ${proc}%`,
  mem: (u: string, t: string) => `記憶體 ${u}/${t} GB`,
  memTip: (mb: string) => `伺服器整體；lobj-view 本身 ${mb} MB`,
  gpuTip: (name: string, u: number, t: number) => `${name}；顯存 ${u}/${t} MB。這是伺服器的 GPU，3D 繪圖用的是開瀏覽器那台電腦的 GPU`,
  sampling: '資源取樣中…',

  // --- 版本紀錄 / 更新 ---
  versionsErr: (e: string) => `無法取得版本紀錄：${e}`,
  hasUpdate: (v: string) => `⬆ 有新版 v${v}`,
  updCurrent: (c: string, l: string) => `目前版本 v${c}，GitHub 最新版 v${l}`,
  updViewOnGithub: '在 GitHub 查看這個版本',
  updLatest: '已是最新版。',
  updCannot: '目前無法線上更新',
  updApply: (v: string) => `更新到 v${v} 並重新啟動`,
  updApplyHint: '會下載新版取代目前的執行檔，伺服器重新啟動後此頁自動重新整理（已開的模型需重新載入）。',
  updDownloading: (v: string) => `下載 v${v} 中…（依網路速度約數秒到一分鐘）`,
  updFailed: (e: string) => `更新失敗：${e}`,
  updRestarting: (v: string) => `已更新到 v${v}，重新啟動中…`,
  updTimeout: '等候伺服器重新啟動逾時，請手動重新整理此頁（或重新執行 lobj-view）。',
  updChecking: '檢查中…',
  updCheckErr: (e: string) => `無法檢查更新：${e}`,

  // --- 副檔名關聯 ---
  assocDefault: '✓ 預設',
  assocRegisteredCur: (c: string) => `已註冊（目前預設：${c}）`,
  assocRegistered: '已註冊',
  assocUnregCur: (c: string) => `未註冊（目前預設：${c}）`,
  assocUnreg: '未註冊',
  assocRestore: (p: string | null) => `　↩ 還原為：${p ?? '無預設'}`,
  assocUnsupported: '目前只支援 Windows 的副檔名關聯（Linux / macOS 尚未支援）。',
  assocRegister: '註冊所選',
  assocUnregister: '取消註冊並還原所選',
  assocSysSettings: '開啟 Windows 預設應用程式設定',
  assocHint:
    '註冊後，lobj-view 會出現在檔案右鍵「開啟檔案」的清單裡；雙擊檔案會交給執行中的 lobj-view（沒有就啟動並開瀏覽器）。.lobj / .lobj2 沒有其他程式佔用時會直接成為預設；' +
    '.stl / .ply / .obj 通常已有預設程式，Windows 不允許程式自行覆蓋，註冊後請按右邊按鈕，到系統設定自己指定。',
  assocExe: (exe: string) => `執行檔：${exe}`,
  assocErr: (e: string) => `無法取得關聯狀態：${e}`,
  assocNeedOne: '請至少勾選一個副檔名',
  assocDoneReg: '已註冊',
  assocDoneUnreg: '已取消註冊並還原',
  sentenceSep: '。',
};

export type Dict = typeof zh;

const en: Dict = {
  hudResizeTip: 'Drag to resize the left panel (double-click to reset)',
  verTip: 'View version history',
  updateTip: 'Check GitHub for a newer version',
  checkUpdate: 'Check for updates',
  langTip: 'Switch language',
  light: 'Light:',
  lightTip: 'Fixed: light does not follow the view; Headlight: light follows the eye; Three: headlight plus one on each side',
  lightFixed: 'Fixed',
  lightHead: 'Headlight (follows eye)',
  lightThree: 'Three lights (eye + left + right)',
  grid: 'Grid',
  ortho: 'Orthographic',
  single: 'Single-sided',
  singleTip: 'Draw only the front faces (counter-clockwise side); back faces are culled',
  theme: 'Theme:',
  themeAuto: 'Auto (system)',
  themeLight: 'Light',
  themeDark: 'Dark',
  themeHcLight: 'High-contrast light',
  themeHcDark: 'High-contrast dark',
  assocOpen: 'Default app…',
  assocOpenTip: 'Make lobj-view the app that opens these file types (double-click a file to open it)',
  status3Tip: 'Server status (updated about every 5 s)',
  serverLoading: 'Server: …',
  browser: 'Select files',
  pathPh: 'Paste a path and press Enter (/ or \\)',
  jawRePh: 'regex for upper, ; separated',
  selectHint: 'Ctrl/⌘-click to add, Shift-click to select a range',
  cancel: 'Cancel',
  details: 'Details',
  assocTitle: 'Default app',
  updateTitle: 'Update lobj-view',

  notSecure: ' — not a secure context; use localhost / ssh -L',
  healthErr: (e) => `unreachable (${e})`,

  jawSrcManual: 'set per file',
  jawSrcRegex: 'by regex',
  jawSrcLabel: 'by tooth labels',
  jawSrcNoTooth: 'no teeth in labels, by filename regex',
  jawWhyConflict: '; problem: filename contradicts the verdict',
  jawWhyBoth: '; problem: labels contain both upper and lower teeth',
  jawBadgeTip: (j, src, why) => `${j} (${src})${why}`,
  jawReBad: (segs) => `Invalid regex skipped: ${segs.join(', ')}\n`,
  jawReTip: 'Upper / lower jaw for legacy .lobj (temporary): a filename matching any regex is upper, otherwise lower.\nSeparated by ;, case-insensitive. Each file can be overridden on its object card on the right.',

  convTitle: (name) => `Convert to .lobj2: ${name}`,
  convDescLobj: (jaw) => `Convert labels to FDI as ${jaw} and save as a .lobj2 with the same name in the same folder; the new file keeps the old file's created / modified / accessed times.`,
  convDescOther: 'Save as a .lobj2 with the same name in the same folder, changing only the extension (no labels; all labels are -1); the new file keeps the old file\'s created / modified / accessed times.',
  convDelete: 'Delete the old file afterwards (default)',
  convKeep: 'Keep the old file',
  convOk: 'Convert',
  converting: 'Converting…',
  convDone: (p, deleted) => `Converted to ${p}${deleted ? '; old file deleted' : ''}`,
  failed: (e) => `Failed: ${e}`,
  convBtn: 'to lobj2',
  convBtnTip: (jaw, legacy) =>
    `Save this file as .lobj2 as ${jaw}${legacy ? ' (labels converted to FDI)' : ' (no labels, extension only)'}; the new file keeps the old dates`,

  objects: (n) => `Objects (${n})`,
  noSelection: 'No file selected',
  meshInfo: (v, t) => `${v} vertices, ${t} triangles`,
  labelKinds: (n) => `${n} ${n === 1 ? 'label' : 'labels'}`,
  cardJawTip: 'Is this file the upper or lower jaw? "Default" = by the regex on the left',
  jawDefault: 'Default',
  jawUpper: 'upper',
  jawLower: 'lower',
  wireTip: 'Show mesh edges (black lines)',
  normalTip: 'Show the normal of every vertex (blue lines)',
  curvTip: 'Color by curvature (normal variation along edges); replaces color',
  curvSigned: 'convex/concave',
  curvSignedTip: 'Mean signed normal curvature of adjacent edges: red = convex (cusps, ridges), blue = concave (fissures, gum line)',
  curvMaxTip: 'Max |κ| of adjacent edges: edges and ridges are brightest',
  curvMinTip: 'Min |κ| of adjacent edges: bright only where it curves in every direction (e.g. cusp tips)',
  clip: 'clip',
  clipTipSigned: (v) => `Color range = ${v}th percentile of ±|κ|; smaller = more contrast (subtle bumps colored), larger = less affected by noisy extremes`,
  clipTipSeq: (lo, v) => `Color range = ${lo}–${v}th percentile; 99.5 matches Python clip_percentile`,
  smooth: 'smooth',
  smoothTip: 'Average normals with neighbouring vertices this many times before computing curvature (vertices do not move); removes scan noise and keeps larger features. Affects curvature colors only. 0 = off',
  smoothOff: 'off',
  mix: 'mix',
  origShort: '∅',
  origTip: 'Original color (unset)',
  legendPending: 'computed after loading…',
  concave: 'concave',
  convex: 'convex',
  legendTipSigned: (hi, r, clip) => `κ in 1/mm; ±${hi} ≈ radius of curvature ${r} mm (clipped at the ${clip}th percentile of |κ|)`,
  legendTipSeq: (lo, hi) => `κ in 1/mm; colored after clipping to the ${lo}%–${hi}% percentiles`,
  lobj2JawBoth: '\njaw: both upper and lower teeth (problem)',
  lobj2JawLabel: (j) => `\njaw: ${j} (by tooth labels)`,
  lobj2JawRegex: (j) => `\njaw: ${j} (no teeth in labels, by filename regex)`,
  loadFailed: (e) => `Failed to load: ${e}`,
  loading: 'Loading…',

  busyMore: (n) => `${n} items`,
  busyText: (what, secs) => `Working… ${what} (${secs} s)`,
  busyTip: (label) => `${label}\nThe first visit to a .zip extracts it; Cancel stays out (the zip itself is never modified)`,
  emptyDir: 'No .lobj / .lobj2 / .stl / .obj / .ply in the --dir folder',
  browseCanceled: 'Canceled (stayed in the current folder)',
  cannotOpenPath: (e) => `Cannot open path: ${e}`,

  serverPrefix: 'Server: ',
  people: (p, t) => `${p} ${p === 1 ? 'person' : 'people'} / ${t} ${t === 1 ? 'tab' : 'tabs'}`,
  peopleTip: 'People = distinct source IPs; connections forwarded by ssh -L all come from 127.0.0.1 and count as one',
  cpuTip: (cores, proc) => `Overall usage of the server's ${cores} logical cores; lobj-view itself about ${proc}%`,
  mem: (u, t) => `Memory ${u}/${t} GB`,
  memTip: (mb) => `Whole server; lobj-view itself ${mb} MB`,
  gpuTip: (name, u, t) => `${name}; VRAM ${u}/${t} MB. This is the server's GPU; 3D rendering uses the GPU of the computer running the browser`,
  sampling: 'Sampling resources…',

  versionsErr: (e) => `Cannot load version history: ${e}`,
  hasUpdate: (v) => `⬆ New v${v}`,
  updCurrent: (c, l) => `Current v${c}, latest on GitHub v${l}`,
  updViewOnGithub: 'View this release on GitHub',
  updLatest: 'You are up to date.',
  updCannot: 'Online update is not available right now',
  updApply: (v) => `Update to v${v} and restart`,
  updApplyHint: 'Downloads the new version to replace the current executable; after the server restarts this page reloads (open models need to be reloaded).',
  updDownloading: (v) => `Downloading v${v}… (a few seconds to a minute, depending on the network)`,
  updFailed: (e) => `Update failed: ${e}`,
  updRestarting: (v) => `Updated to v${v}, restarting…`,
  updTimeout: 'Timed out waiting for the server to restart; reload this page manually (or run lobj-view again).',
  updChecking: 'Checking…',
  updCheckErr: (e) => `Cannot check for updates: ${e}`,

  assocDefault: '✓ default',
  assocRegisteredCur: (c) => `registered (current default: ${c})`,
  assocRegistered: 'registered',
  assocUnregCur: (c) => `not registered (current default: ${c})`,
  assocUnreg: 'not registered',
  assocRestore: (p) => `  ↩ restores to: ${p ?? 'no default'}`,
  assocUnsupported: 'File associations are only supported on Windows (not yet on Linux / macOS).',
  assocRegister: 'Register selected',
  assocUnregister: 'Unregister and restore selected',
  assocSysSettings: 'Open Windows default apps settings',
  assocHint:
    'Once registered, lobj-view appears in the "Open with" list of the right-click menu; double-clicking a file hands it to the running lobj-view (or starts it and opens the browser). .lobj / .lobj2 become the default if no other app owns them; ' +
    '.stl / .ply / .obj usually already have a default app that Windows does not let programs override, so after registering use the button on the right to choose it in system settings.',
  assocExe: (exe) => `Executable: ${exe}`,
  assocErr: (e) => `Cannot read association status: ${e}`,
  assocNeedOne: 'Select at least one extension',
  assocDoneReg: 'Registered',
  assocDoneUnreg: 'Unregistered and restored',
  sentenceSep: '. ',
};

// 越南文（1.10.4）：術語以牙科 / 3D 常用說法為主（hàm trên / hàm dưới = 上 / 下顎，nhãn = label，độ cong = 曲率）
const vi: Dict = {
  hudResizeTip: 'Kéo để đổi độ rộng khung bên trái (nhấp đúp để khôi phục)',
  verTip: 'Xem lịch sử phiên bản',
  updateTip: 'Kiểm tra phiên bản mới trên GitHub',
  checkUpdate: 'Kiểm tra cập nhật',
  langTip: 'Đổi ngôn ngữ',
  light: 'Ánh sáng:',
  lightTip: 'Cố định: ánh sáng không đổi theo góc nhìn; Đèn đầu: ánh sáng theo vị trí mắt; Ba đèn: đèn đầu thêm một đèn mỗi bên',
  lightFixed: 'Cố định',
  lightHead: 'Đèn đầu (theo mắt)',
  lightThree: 'Ba đèn (mắt + trái + phải)',
  grid: 'Lưới',
  ortho: 'Chiếu trực giao',
  single: 'Một mặt',
  singleTip: 'Chỉ vẽ mặt trước của lưới (mặt ngược chiều kim đồng hồ), không vẽ mặt sau',
  theme: 'Giao diện:',
  themeAuto: 'Tự động (hệ thống)',
  themeLight: 'Sáng',
  themeDark: 'Tối',
  themeHcLight: 'Sáng tương phản cao',
  themeHcDark: 'Tối tương phản cao',
  assocOpen: 'Ứng dụng mặc định…',
  assocOpenTip: 'Đặt lobj-view làm ứng dụng mở các loại tệp này (nhấp đúp vào tệp để mở)',
  status3Tip: 'Trạng thái máy chủ (cập nhật khoảng mỗi 5 giây)',
  serverLoading: 'Máy chủ: …',
  browser: 'Chọn tệp',
  pathPh: 'Dán đường dẫn rồi nhấn Enter (/ hoặc \\ đều được)',
  jawRePh: 'regex cho hàm trên, phân cách bằng ;',
  selectHint: 'Ctrl/⌘ + nhấp để chọn thêm, Shift + nhấp để chọn một dải',
  cancel: 'Hủy',
  details: 'Chi tiết',
  assocTitle: 'Ứng dụng mặc định',
  updateTitle: 'Cập nhật lobj-view',

  notSecure: ' — không phải secure context, hãy dùng localhost / ssh -L',
  healthErr: (e) => `không kết nối được (${e})`,

  jawSrcManual: 'chỉ định riêng',
  jawSrcRegex: 'theo regex',
  jawSrcLabel: 'theo nhãn răng',
  jawSrcNoTooth: 'nhãn không có răng, theo regex tên tệp',
  jawWhyConflict: '; bất thường: tên tệp mâu thuẫn với kết quả',
  jawWhyBoth: '; bất thường: nhãn có cả răng hàm trên và hàm dưới',
  jawBadgeTip: (j, src, why) => `${j} (${src})${why}`,
  jawReBad: (segs) => `Đã bỏ qua regex sai cú pháp: ${segs.join(', ')}\n`,
  jawReTip: 'Hàm trên / hàm dưới của .lobj cũ (cách tạm thời): tên tệp khớp bất kỳ regex nào là upper, ngược lại là lower.\nPhân cách bằng ;, không phân biệt hoa thường. Có thể chỉ định riêng từng tệp trên thẻ đối tượng bên phải.',

  convTitle: (name) => `Chuyển sang .lobj2: ${name}`,
  convDescLobj: (jaw) => `Đổi nhãn sang FDI theo ${jaw} và lưu thành .lobj2 cùng tên, cùng thư mục; tệp mới giữ thời gian tạo / sửa / truy cập của tệp cũ.`,
  convDescOther: 'Chỉ đổi phần mở rộng, lưu thành .lobj2 cùng tên, cùng thư mục (không có nhãn, tất cả nhãn là -1); tệp mới giữ thời gian tạo / sửa / truy cập của tệp cũ.',
  convDelete: 'Xóa tệp cũ sau khi chuyển (mặc định)',
  convKeep: 'Giữ tệp cũ',
  convOk: 'Chuyển',
  converting: 'Đang chuyển…',
  convDone: (p, deleted) => `Đã chuyển thành ${p}${deleted ? ', đã xóa tệp cũ' : ''}`,
  failed: (e) => `Thất bại: ${e}`,
  convBtn: 'sang lobj2',
  convBtnTip: (jaw, legacy) =>
    `Lưu tệp này thành .lobj2 theo ${jaw}${legacy ? ' (nhãn đổi sang FDI)' : ' (không có nhãn, chỉ đổi phần mở rộng)'}; tệp mới giữ ngày của tệp cũ`,

  objects: (n) => `Đối tượng (${n})`,
  noSelection: 'Chưa chọn tệp nào',
  meshInfo: (v, t) => `${v} đỉnh, ${t} tam giác`,
  labelKinds: (n) => `${n} nhãn`,
  cardJawTip: 'Tệp này là hàm trên hay hàm dưới; "Mặc định" = theo regex bên trái',
  jawDefault: 'Mặc định',
  jawUpper: 'upper (trên)',
  jawLower: 'lower (dưới)',
  wireTip: 'Hiện cạnh của lưới (đường đen)',
  normalTip: 'Hiện pháp tuyến của mỗi đỉnh (đường xanh)',
  curvTip: 'Tô màu theo độ cong (Normal Variation, tính dọc theo cạnh); thay cho color',
  curvSigned: 'lồi/lõm',
  curvSignedTip: 'Trung bình độ cong pháp tuyến có dấu của các cạnh kề: đỏ = lồi (múi răng, gờ), xanh = lõm (rãnh, đường viền nướu)',
  curvMaxTip: '|κ| lớn nhất của các cạnh kề: mép và gờ sáng nhất',
  curvMinTip: '|κ| nhỏ nhất của các cạnh kề: chỉ sáng ở nơi cong theo mọi hướng (như đỉnh múi răng)',
  clip: 'cắt',
  clipTipSigned: (v) => `Dải màu = phân vị thứ ${v} của ±|κ|; càng nhỏ tương phản càng mạnh (lồi lõm nhỏ cũng có màu), càng lớn càng ít bị nhiễu cực trị ảnh hưởng`,
  clipTipSeq: (lo, v) => `Dải màu = phân vị ${lo}–${v}; 99.5 tương ứng clip_percentile của Python`,
  smooth: 'làm mịn',
  smoothTip: 'Trước khi tính độ cong, lấy trung bình pháp tuyến với các đỉnh kề bấy nhiêu lần (đỉnh không di chuyển) để khử nhiễu quét, giữ lồi lõm ở tỉ lệ lớn; chỉ ảnh hưởng màu độ cong. 0 = tắt',
  smoothOff: 'tắt',
  mix: 'trộn',
  origShort: '∅',
  origTip: 'Màu gốc (không đặt)',
  legendPending: 'tính sau khi tải…',
  concave: 'lõm',
  convex: 'lồi',
  legendTipSigned: (hi, r, clip) => `κ đơn vị 1/mm; ±${hi} ≈ bán kính cong ${r} mm (cắt ở phân vị ${clip} của |κ|)`,
  legendTipSeq: (lo, hi) => `κ đơn vị 1/mm; tô màu sau khi cắt ở phân vị ${lo}%–${hi}%`,
  lobj2JawBoth: '\njaw: có răng của cả hai hàm (bất thường)',
  lobj2JawLabel: (j) => `\njaw: ${j} (theo nhãn răng)`,
  lobj2JawRegex: (j) => `\njaw: ${j} (nhãn không có răng, theo regex tên tệp)`,
  loadFailed: (e) => `Tải thất bại: ${e}`,
  loading: 'Đang tải…',

  busyMore: (n) => `${n} mục`,
  busyText: (what, secs) => `Đang xử lý… ${what} (${secs} giây)`,
  busyTip: (label) => `${label}\nLần đầu mở .zip cần giải nén; nhấn Hủy để không vào (tệp zip gốc không bị thay đổi)`,
  emptyDir: 'Thư mục --dir không có .lobj / .lobj2 / .stl / .obj / .ply',
  browseCanceled: 'Đã hủy (vẫn ở thư mục cũ)',
  cannotOpenPath: (e) => `Không mở được đường dẫn: ${e}`,

  serverPrefix: 'Máy chủ: ',
  people: (p, t) => `${p} người / ${t} tab`,
  peopleTip: 'Người = địa chỉ IP nguồn khác nhau; các kết nối chuyển tiếp qua ssh -L đều đến từ 127.0.0.1 nên chỉ tính là một',
  cpuTip: (cores, proc) => `Mức sử dụng tổng của ${cores} lõi logic trên máy chủ; riêng lobj-view khoảng ${proc}%`,
  mem: (u, t) => `Bộ nhớ ${u}/${t} GB`,
  memTip: (mb) => `Toàn máy chủ; riêng lobj-view ${mb} MB`,
  gpuTip: (name, u, t) => `${name}; VRAM ${u}/${t} MB. Đây là GPU của máy chủ; vẽ 3D dùng GPU của máy đang mở trình duyệt`,
  sampling: 'Đang lấy mẫu tài nguyên…',

  versionsErr: (e) => `Không lấy được lịch sử phiên bản: ${e}`,
  hasUpdate: (v) => `⬆ Có bản mới v${v}`,
  updCurrent: (c, l) => `Phiên bản hiện tại v${c}, mới nhất trên GitHub v${l}`,
  updViewOnGithub: 'Xem phiên bản này trên GitHub',
  updLatest: 'Đã là phiên bản mới nhất.',
  updCannot: 'Hiện không thể cập nhật trực tuyến',
  updApply: (v) => `Cập nhật lên v${v} và khởi động lại`,
  updApplyHint: 'Sẽ tải phiên bản mới thay cho tệp thực thi hiện tại; sau khi máy chủ khởi động lại, trang này tự tải lại (các mô hình đang mở cần tải lại).',
  updDownloading: (v) => `Đang tải v${v}… (vài giây đến một phút tùy tốc độ mạng)`,
  updFailed: (e) => `Cập nhật thất bại: ${e}`,
  updRestarting: (v) => `Đã cập nhật lên v${v}, đang khởi động lại…`,
  updTimeout: 'Hết thời gian chờ máy chủ khởi động lại; hãy tự tải lại trang này (hoặc chạy lại lobj-view).',
  updChecking: 'Đang kiểm tra…',
  updCheckErr: (e) => `Không kiểm tra được cập nhật: ${e}`,

  assocDefault: '✓ mặc định',
  assocRegisteredCur: (c) => `đã đăng ký (mặc định hiện tại: ${c})`,
  assocRegistered: 'đã đăng ký',
  assocUnregCur: (c) => `chưa đăng ký (mặc định hiện tại: ${c})`,
  assocUnreg: 'chưa đăng ký',
  assocRestore: (p) => `  ↩ khôi phục về: ${p ?? 'không có mặc định'}`,
  assocUnsupported: 'Hiện chỉ hỗ trợ liên kết loại tệp trên Windows (chưa hỗ trợ Linux / macOS).',
  assocRegister: 'Đăng ký mục đã chọn',
  assocUnregister: 'Hủy đăng ký và khôi phục mục đã chọn',
  assocSysSettings: 'Mở cài đặt ứng dụng mặc định của Windows',
  assocHint:
    'Sau khi đăng ký, lobj-view sẽ xuất hiện trong danh sách "Mở bằng" khi nhấp chuột phải vào tệp; nhấp đúp vào tệp sẽ giao cho lobj-view đang chạy (nếu chưa chạy thì khởi động và mở trình duyệt). .lobj / .lobj2 sẽ trở thành mặc định nếu chưa có ứng dụng khác chiếm; ' +
    '.stl / .ply / .obj thường đã có ứng dụng mặc định và Windows không cho chương trình tự ghi đè, nên sau khi đăng ký hãy nhấn nút bên phải để tự chọn trong cài đặt hệ thống.',
  assocExe: (exe) => `Tệp thực thi: ${exe}`,
  assocErr: (e) => `Không lấy được trạng thái liên kết: ${e}`,
  assocNeedOne: 'Hãy chọn ít nhất một phần mở rộng',
  assocDoneReg: 'Đã đăng ký',
  assocDoneUnreg: 'Đã hủy đăng ký và khôi phục',
  sentenceSep: '. ',
};

export type Lang = 'zh' | 'en' | 'vi';

/** 可選語言：value 與該語言自己的名稱（選單上用原文顯示，不隨介面語言翻譯）。 */
export const LANGS: readonly { code: Lang; name: string; html: string }[] = [
  { code: 'zh', name: '中文', html: 'zh-Hant' },
  { code: 'en', name: 'English', html: 'en' },
  { code: 'vi', name: 'Tiếng Việt', html: 'vi' },
];

const DICTS: Record<Lang, Dict> = { zh, en, vi };
const DEFAULT_LANG: Lang = 'zh';
const KEY = 'lobj-view-lang';

let cur: Lang = DEFAULT_LANG;
try {
  const saved = localStorage.getItem(KEY);
  if (saved && saved in DICTS) cur = saved as Lang;
} catch {
  /* localStorage 不可用就用預設 */
}

export const getLang = (): Lang => cur;
/** 目前語言的字典。 */
export const t = (): Dict => DICTS[cur];

const listeners: (() => void)[] = [];
export const onLangChange = (fn: () => void): void => void listeners.push(fn);

export function setLang(l: Lang): void {
  if (l === cur) return;
  cur = l;
  try {
    localStorage.setItem(KEY, l);
  } catch {
    /* 忽略 */
  }
  applyStatic();
  for (const fn of listeners) fn();
}

const str = (k: string): string | null => {
  const v = (t() as Record<string, unknown>)[k];
  return typeof v === 'string' ? v : null;
};

/** 套用靜態 HTML 上的 data-i18n / data-i18n-title / data-i18n-placeholder。 */
export function applyStatic(root: ParentNode = document): void {
  document.documentElement.lang = LANGS.find((x) => x.code === cur)!.html;
  for (const el of root.querySelectorAll<HTMLElement>('[data-i18n]')) {
    const s = str(el.dataset.i18n!);
    if (s !== null) el.textContent = s;
  }
  for (const el of root.querySelectorAll<HTMLElement>('[data-i18n-title]')) {
    const s = str(el.dataset.i18nTitle!);
    if (s !== null) el.title = s;
  }
  for (const el of root.querySelectorAll<HTMLInputElement>('[data-i18n-placeholder]')) {
    const s = str(el.dataset.i18nPlaceholder!);
    if (s !== null) el.placeholder = s;
  }
}
