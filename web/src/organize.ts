// 整理頁（/organize）：壓縮、解壓縮、同步到另一台電腦、刪除 zip，以及資料夾的重新命名、拿掉這層、遮蔽 / 還原名稱。
// 這些與「檢視」無關，原本在主畫面的「選擇檔案」區，1.21.1 起獨立成這一頁（獨立的 Vite 入口，共用 picker / i18n / 樣式）。
// 後端 API 都只限本機（POST 要帶 X-Lobj-Requested）。

import './apple-color-ios.css';
import './style.css';
import './to-lobj2.css';
import './organize.css';
import { listLobj, type LobjListing } from './lobj';
import { applyStatic, onLangChange, t } from './i18n';
import { initPicker, recordRecent } from './picker';

const infoEl = document.getElementById('info')!;
const pathEl = document.getElementById('path') as HTMLInputElement;
const filesEl = document.getElementById('files')!;
const allEl = document.getElementById('org-all') as HTMLInputElement;

// 主題沿用主畫面記住的選擇（空字串 = 跟隨系統）
try {
  const th = localStorage.getItem('lobj-view-theme');
  if (th) document.documentElement.dataset.theme = th;
} catch {
  /* 沿用系統主題 */
}
applyStatic();

const isAbort = (e: unknown) => e instanceof DOMException && e.name === 'AbortError';
const join = (dir: string, name: string) => (dir ? `${dir.endsWith('/') ? dir : `${dir}/`}${name}` : name);

/** 沒帶 X-Lobj-Requested 的 POST 後端一律拒絕（擋其他網站的 CSRF）。 */
async function post(url: string, body?: unknown): Promise<Response> {
  return fetch(url, {
    method: 'POST',
    headers: { 'X-Lobj-Requested': '1', 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
}

// --- 瀏覽：單擊選取資料夾 / zip，雙擊進入 ---
let curDir = '';
let browseCtl: AbortController | null = null;
let selDir: string | null = null;
const zipActionsEl = document.getElementById('zip-actions')!;
const dirActionsEl = document.getElementById('dir-actions')!;
const inZip = () => /\.zip(\/|$)/i.test(curDir);

function selectDir(full: string | null, isZip: boolean): void {
  selDir = full === selDir ? null : full; // 再點一次取消
  for (const li of filesEl.querySelectorAll<HTMLLIElement>('li[data-dir]')) li.classList.toggle('sel', li.dataset.dir === selDir);
  // 目前就在某個 zip 裡面時，裡面的 zip / 資料夾是快取副本，不能操作
  zipActionsEl.hidden = !(selDir && isZip && !inZip());
  document.getElementById('zip-sel')!.textContent = selDir ? selDir.split('/').pop()! : '';
  dirActionsEl.hidden = !(selDir && !isZip && !inZip());
  document.getElementById('dir-sel')!.textContent = selDir ? selDir.split('/').pop()! : '';
}

async function browse(path: string): Promise<void> {
  browseCtl?.abort();
  const mine = (browseCtl = new AbortController());
  try {
    const l: LobjListing = await listLobj(path, mine.signal, allEl.checked);
    curDir = l.path;
    recordRecent(curDir);
    pathEl.value = l.path ? (l.path.endsWith('/') ? l.path : `${l.path}/`) : '';
    filesEl.replaceChildren();
    selectDir(null, false);
    (document.getElementById('zip-dir') as HTMLButtonElement).disabled = inZip() || !curDir;
    (document.getElementById('l2-up') as HTMLButtonElement).disabled = !l.parent;
    upTo = l.up ?? (curDir.includes('/') ? curDir.slice(0, curDir.lastIndexOf('/')) : '');
    const zips = new Set(l.zips ?? []);
    for (const d of l.dirs) {
      const full = join(curDir, d);
      const li = document.createElement('li');
      li.className = 'dir';
      li.textContent = `${zips.has(d) ? '📦' : '📁'} ${d}`;
      li.dataset.dir = full;
      li.addEventListener('click', (e) => {
        if (e.detail <= 1) selectDir(full, zips.has(d));
      });
      li.addEventListener('dblclick', () => void browse(full));
      filesEl.appendChild(li);
    }
    for (const f of l.files) {
      const li = document.createElement('li');
      li.className = 'file';
      li.textContent = `${f.name} (${f.size >= 1048576 ? `${(f.size / 1048576).toFixed(1)} MB` : `${Math.ceil(f.size / 1024)} KB`})`;
      filesEl.appendChild(li);
    }
    if (!l.dirs.length && !l.files.length) infoEl.textContent = t().orgEmpty;
  } catch (e) {
    if (!isAbort(e)) infoEl.textContent = t().cannotOpenPath(e instanceof Error ? e.message : String(e));
  }
}
let upTo = '';
document.getElementById('l2-up')!.addEventListener('click', () => void browse(upTo));
allEl.addEventListener('change', () => void browse(curDir));
document.getElementById('org-go')!.addEventListener('click', () => void browse(pathEl.value));
pathEl.addEventListener('keydown', (e) => e.key === 'Enter' && void browse(pathEl.value));

initPicker({
  selected: new Set<string>(),
  applySelection: () => {},
  openOnly: () => {},
  browse,
  getCurDir: () => curDir,
  withBusy: (_path, run, ctl) => run((ctl ?? new AbortController()).signal),
  isAbort,
});

document.getElementById('dir-rename')!.addEventListener('click', async () => {
  if (!selDir) return;
  const old = selDir.split('/').pop()!;
  const name = prompt(t().dirRenamePrompt(old), old)?.trim();
  if (!name || name === old) return;
  const r = await post('/api/dir-rename', { path: selDir, name });
  if (!r.ok) {
    infoEl.textContent = t().failed(await r.text());
    return;
  }
  await browse(curDir);
});

interface FlatPreview {
  folders: number;
  moved: number;
  targets: { path: string; items: number }[];
  clashes: number;
  clash_list: { from: string; to: string }[];
}

/** 預覽視窗：列出會處理的資料夾與不符合的路徑，回傳使用者按了確定（且沒有不符合的項目）。 */
function confirmFlatten(title: string, pv: FlatPreview): Promise<boolean> {
  const dlg = document.getElementById('flat-dlg') as HTMLDialogElement;
  const body = document.getElementById('flat-body')!;
  const ok = document.getElementById('flat-ok') as HTMLButtonElement;
  document.getElementById('flat-title')!.textContent = title;
  body.replaceChildren();
  const addList = (head: string, rows: string[], bad: boolean, more: boolean) => {
    const h = document.createElement('div');
    h.textContent = head;
    if (bad) h.className = 'flat-bad';
    const ul = document.createElement('ul');
    for (const r of rows) {
      const li = document.createElement('li');
      li.textContent = r;
      if (bad) li.className = 'flat-bad';
      ul.append(li);
    }
    if (more) {
      const li = document.createElement('li');
      li.textContent = t().flatMore;
      ul.append(li);
    }
    body.append(h, ul);
  };
  if (pv.clashes > 0) {
    addList(t().flatClashHead(pv.clashes), pv.clash_list.map((c) => `${c.from}  →  ${c.to}`), true, pv.clashes > pv.clash_list.length);
  }
  addList(t().flatSummary(pv.folders, pv.moved), pv.targets.map((x) => `${x.path}  (${t().flatItems(x.items)})`), false, pv.folders > pv.targets.length);
  ok.disabled = pv.clashes > 0;
  return new Promise((resolve) => {
    const done = (v: boolean) => {
      ok.onclick = null;
      (document.getElementById('flat-cancel') as HTMLButtonElement).onclick = null;
      dlg.onclose = null;
      if (dlg.open) dlg.close();
      resolve(v);
    };
    ok.onclick = () => done(true);
    (document.getElementById('flat-cancel') as HTMLButtonElement).onclick = () => done(false);
    dlg.onclose = () => done(false);
    dlg.showModal();
  });
}

document.getElementById('dir-flatten')!.addEventListener('click', async () => {
  if (!selDir) return;
  const n = selDir.split('/').pop()!;
  const level = Math.max(0, Math.min(8, Math.floor(Number((document.getElementById('dir-level') as HTMLInputElement).value) || 0)));
  const pr = await post('/api/dir-flatten', { path: selDir, level, dry_run: true });
  if (!pr.ok) {
    infoEl.textContent = t().failed(await pr.text());
    return;
  }
  if (!(await confirmFlatten(t().flatTitle(n, level), (await pr.json()) as FlatPreview))) return;
  const r = await post('/api/dir-flatten', { path: selDir, level });
  if (!r.ok) {
    infoEl.textContent = t().failed(await r.text());
    return;
  }
  const res = (await r.json()) as { moved: number; folders: number };
  await browse(curDir);
  infoEl.textContent = level === 0 ? t().dirFlattenDone(res.moved) : t().dirFlattenDoneN(res.moved, res.folders);
});

// 遮蔽 / 還原第一層子資料夾名稱（演算法與 no_user_info.py 相容）。密碼只在這個對話框的輸入欄，送出後立刻清掉。
// 檔名處理：不動 / 檔名內的資料夾名稱一併編碼 / 用規則（regex 取代或保留；規則可命名儲存在瀏覽器的 localStorage）。
const maskDlg = document.getElementById('mask-dlg') as HTMLDialogElement;
const maskPass = document.getElementById('mask-pass') as HTMLInputElement;
const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
let maskMode: 'encode' | 'decode' = 'encode';

interface MaskRule {
  name: string;
  kind: 'replace' | 'keep';
  pattern: string;
  replacement: string;
}
const RULES_KEY = 'lobj-view-mask-rules';
// 範例：`93413335A_姓名` 這種「編號A_姓名」結尾（BCom 資料）整段刪除
const DEFAULT_RULES: MaskRule[] = [{ name: 'BCom: _編號A_姓名', kind: 'replace', pattern: '_\\d+A_.+$', replacement: '' }];
function loadRules(): MaskRule[] {
  try {
    const v = JSON.parse(localStorage.getItem(RULES_KEY) ?? 'null');
    if (Array.isArray(v)) return v.filter((r) => r && typeof r.name === 'string' && typeof r.pattern === 'string');
  } catch {
    /* 用預設 */
  }
  return [...DEFAULT_RULES];
}
function saveRules(v: MaskRule[]): void {
  try {
    localStorage.setItem(RULES_KEY, JSON.stringify(v));
  } catch {
    /* 存不了就只留在這次 */
  }
}
let rules = loadRules();

interface RuleBody {
  kind: 'replace' | 'keep';
  pattern: string;
  replacement: string;
}

/** 規則編輯器（取代 / 保留 + regex + 命名儲存）：遮蔽對話框與「處理檔名」對話框共用，各掛在自己的容器裡。 */
function mountRuleEditor(box: HTMLElement): { rule(): RuleBody; refresh(): void } {
  box.innerHTML = `
    <div class="hint" data-r="hint"></div>
    <div class="sync-row"><select data-r="saved"></select><button type="button" data-r="del"></button></div>
    <div class="sync-row"><select data-r="kind"><option value="replace"></option><option value="keep"></option></select><input data-r="pattern" type="text" spellcheck="false"></div>
    <div class="sync-row"><input data-r="repl" type="text" spellcheck="false"></div>
    <div class="sync-row"><input data-r="name" type="text" spellcheck="false"><button type="button" data-r="save"></button></div>`;
  const q = <T extends HTMLElement>(k: string) => box.querySelector<T>(`[data-r="${k}"]`)!;
  const saved = q<HTMLSelectElement>('saved');
  const kind = q<HTMLSelectElement>('kind');
  const pattern = q<HTMLInputElement>('pattern');
  const repl = q<HTMLInputElement>('repl');
  const name = q<HTMLInputElement>('name');
  const syncKind = () => (repl.hidden = kind.value !== 'replace');
  const render = (sel = '') => {
    saved.replaceChildren(new Option(t().maskRuleNew, ''), ...rules.map((r) => new Option(r.name, r.name)));
    saved.value = sel;
  };
  const refresh = () => {
    q('hint').textContent = t().maskRuleHint;
    q('del').textContent = t().maskRuleDel;
    q('save').textContent = t().maskRuleSave;
    kind.options[0].text = t().maskKindReplace;
    kind.options[1].text = t().maskKindKeep;
    pattern.placeholder = t().maskPattern;
    repl.placeholder = t().maskReplacement;
    name.placeholder = t().maskRuleName;
    saved.title = t().maskRuleSaved;
    render(saved.value);
    syncKind();
  };
  saved.addEventListener('change', () => {
    const r = rules.find((x) => x.name === saved.value);
    if (!r) return;
    kind.value = r.kind;
    pattern.value = r.pattern;
    repl.value = r.replacement;
    name.value = r.name;
    syncKind();
  });
  kind.addEventListener('change', syncKind);
  q('save').addEventListener('click', () => {
    const n = name.value.trim();
    if (!n || !pattern.value) return;
    rules = [...rules.filter((x) => x.name !== n), { name: n, kind: kind.value as RuleBody['kind'], pattern: pattern.value, replacement: repl.value }];
    saveRules(rules);
    render(n);
  });
  q('del').addEventListener('click', () => {
    if (!saved.value) return;
    rules = rules.filter((x) => x.name !== saved.value);
    saveRules(rules);
    render();
  });
  refresh();
  return { rule: () => ({ kind: kind.value as RuleBody['kind'], pattern: pattern.value, replacement: repl.value }), refresh };
}

interface RulePreview {
  folders: number;
  changed: number;
  files: { from: string; to: string; unmatched: boolean; leak: boolean }[];
  unmatched: number;
  leaks: number;
  conflict: string | null;
}
function showPreview(box: HTMLElement, pv: RulePreview): void {
  const lines = [t().maskPreviewSummary(pv.folders, pv.changed, pv.unmatched, pv.leaks)];
  if (pv.conflict) lines.push(t().maskPreviewConflict(pv.conflict));
  box.replaceChildren(...lines.map((l) => Object.assign(document.createElement('div'), { textContent: l })));
  const ul = document.createElement('ul');
  ul.className = 'mask-pv';
  for (const f of pv.files) {
    const li = document.createElement('li');
    li.textContent = f.from === f.to ? f.from : `${f.from}  →  ${f.to}`;
    if (f.unmatched || f.leak) li.className = 'flat-bad';
    ul.append(li);
  }
  if (pv.files.length >= 300) ul.append(Object.assign(document.createElement('li'), { textContent: t().maskPreviewMore }));
  box.append(ul);
}

const maskEditor = mountRuleEditor(el('mask-rule'));
el('mask-files').addEventListener('change', () => {
  el('mask-rule').hidden = el<HTMLSelectElement>('mask-files').value !== 'rule';
  el('mask-preview').textContent = '';
});

function filesOption(): unknown {
  const m = el<HTMLSelectElement>('mask-files').value;
  if (maskMode === 'decode' || m === 'none') return { mode: m === 'encode' ? 'encode' : 'none' };
  if (m === 'encode') return { mode: 'encode' };
  return { mode: 'rule', rule: maskEditor.rule() };
}

function openMask(mode: 'encode' | 'decode'): void {
  if (!selDir) return;
  maskMode = mode;
  const n = selDir.split('/').pop()!;
  el('mask-title').textContent = mode === 'encode' ? t().maskMaskTitle(n) : t().maskUnmaskTitle(n);
  el('mask-hint').textContent = t().maskHint;
  el('mask-ok').textContent = mode === 'encode' ? t().dirMask : t().dirUnmask;
  maskPass.value = '';
  el('mask-preview').textContent = '';
  // 還原時只有「檔名內的資料夾名稱一併編碼」需要選擇（規則不可逆，還原時不作用）
  for (const o of el<HTMLSelectElement>('mask-files').options) o.hidden = mode === 'decode' && o.value === 'rule';
  if (mode === 'decode' && el<HTMLSelectElement>('mask-files').value === 'rule') el<HTMLSelectElement>('mask-files').value = 'none';
  el('mask-rule').hidden = mode === 'decode' || el<HTMLSelectElement>('mask-files').value !== 'rule';
  el('mask-preview-btn').hidden = mode === 'decode';
  maskEditor.refresh();
  maskDlg.showModal();
  maskPass.focus();
}
el('dir-mask').addEventListener('click', () => openMask('encode'));
el('dir-unmask').addEventListener('click', () => openMask('decode'));
el('mask-cancel').addEventListener('click', () => {
  maskPass.value = '';
  maskDlg.close();
});

el('mask-preview-btn').addEventListener('click', async () => {
  const box = el('mask-preview');
  if (!selDir || !maskPass.value) {
    box.textContent = t().maskPass;
    return;
  }
  box.textContent = '…';
  const r = await post('/api/dir-mask', { path: selDir, mode: maskMode, password: maskPass.value, files: filesOption(), dry_run: true });
  if (!r.ok) {
    box.textContent = t().failed(await r.text());
    return;
  }
  showPreview(box, (await r.json()) as RulePreview);
});

// --- 處理檔名（只改檔名：不動資料夾名稱、不需要密碼；可指定層數）---
const ruleDlg = el<HTMLDialogElement>('rule-dlg');
const ruleEditor = mountRuleEditor(el('rule-box'));
const depthOf = () => Math.max(0, Math.min(8, Math.floor(Number(el<HTMLInputElement>('file-depth').value) || 0)));
el('file-rule').addEventListener('click', () => {
  if (!selDir) return;
  el('rule-title').textContent = t().fileRuleTitle(selDir.split('/').pop()!);
  el('rule-preview').textContent = '';
  ruleEditor.refresh();
  ruleDlg.showModal();
});
el('rule-cancel').addEventListener('click', () => ruleDlg.close());
el('rule-preview-btn').addEventListener('click', async () => {
  const box = el('rule-preview');
  if (!selDir) return;
  box.textContent = '…';
  const r = await post('/api/file-rule', { path: selDir, rule: ruleEditor.rule(), depth: depthOf(), dry_run: true });
  if (!r.ok) {
    box.textContent = t().failed(await r.text());
    return;
  }
  showPreview(box, (await r.json()) as RulePreview);
});
el('rule-form').addEventListener('submit', async () => {
  if (!selDir) return;
  infoEl.textContent = '…';
  const r = await post('/api/file-rule', { path: selDir, rule: ruleEditor.rule(), depth: depthOf() });
  if (!r.ok) {
    infoEl.textContent = t().failed(await r.text());
    return;
  }
  const res = (await r.json()) as { files: number };
  await browse(curDir);
  infoEl.textContent = t().fileRuleDone(res.files);
});

el('mask-form').addEventListener('submit', async () => {
  const password = maskPass.value;
  maskPass.value = '';
  if (!selDir || !password) return;
  infoEl.textContent = '…';
  const r = await post('/api/dir-mask', { path: selDir, mode: maskMode, password, files: filesOption() });
  if (!r.ok) {
    infoEl.textContent = t().failed(await r.text());
    return;
  }
  const res = (await r.json()) as { count: number };
  await browse(curDir);
  infoEl.textContent = t().maskDone(res.count);
});

// 解壓縮所選的 .zip（同層；已存在先問是否覆蓋）。readStream / opProgress 在檔案後段宣告，點擊時才用到。
document.getElementById('zip-extract')!.addEventListener('click', async () => {
  if (!selDir) return;
  const btn = document.getElementById('zip-extract') as HTMLButtonElement;
  btn.disabled = true;
  infoEl.textContent = t().unzipping;
  try {
    for (let overwrite = false; ; ) {
      const r = await fetch('/api/zip-extract', {
        method: 'POST',
        headers: { 'X-Lobj-Requested': '1', 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: selDir, overwrite }),
      });
      if (r.status === 409 && !overwrite && confirm(`${await r.text()}\n${t().zipOverwrite}`)) {
        overwrite = true;
        continue;
      }
      if (!r.ok) throw new Error(await r.text());
      const res = await readStream<{ dir: string; size: number; files: number }>(r, (p) => {
        opProgress.hidden = false;
        opBar.value = p.progress;
        opText.textContent = `${p.progress}%`;
      });
      infoEl.textContent = t().unzipDone(res.dir, res.size, res.files);
      await browse(curDir);
      break;
    }
  } catch (e) {
    infoEl.textContent = t().failed(e instanceof Error ? e.message : String(e));
  } finally {
    btn.disabled = false;
    opProgress.hidden = true;
  }
});

// --- 同步到另一台電腦（rsync over ssh）：輸入 user@host（要有 ssh 金鑰，不問密碼），挑遠端資料夾 ---
const syncEl = document.getElementById('sync')!;
const syncHost = document.getElementById('sync-host') as HTMLInputElement;
const syncPass = document.getElementById('sync-pass') as HTMLInputElement; // 只留在這個分頁的記憶體，不存 localStorage、不存伺服器
const syncDir = document.getElementById('sync-dir') as HTMLInputElement;
const syncList = document.getElementById('sync-list')!;
const syncMsg = document.getElementById('sync-msg')!;
const syncBar = document.getElementById('sync-bar') as HTMLProgressElement;
const syncBg = document.getElementById('sync-bg') as HTMLInputElement;
const syncJobsEl = document.getElementById('sync-jobs')!;
const syncJobsTitle = document.getElementById('sync-jobs-title')!;
let jobTimer = 0;

interface SyncJob {
  id: string;
  session: string;
  zip: string;
  dest: string;
  state: 'running' | 'done' | 'failed' | 'stopped';
  progress: number;
  speed: string;
  error: string;
}

/** 背景工作（本機 tmux）：列出狀態；有工作在跑且視窗開著就每 2 秒更新一次。 */
async function loadJobs(): Promise<void> {
  window.clearTimeout(jobTimer);
  if (syncEl.hidden) return;
  try {
    const r = await fetch('/api/sync-jobs');
    if (!r.ok) throw new Error(await r.text());
    const res = (await r.json()) as { tmux: boolean; jobs: SyncJob[] };
    syncBg.disabled = !res.tmux;
    if (!res.tmux) syncBg.checked = false;
    document.getElementById('sync-bg-wrap')!.title = res.tmux ? t().syncBgTip : t().syncBgNoTmux;
    syncJobsTitle.hidden = !res.jobs.length;
    (document.getElementById('sync-jobs-clear') as HTMLButtonElement).disabled = !res.jobs.some((j) => j.state !== 'running');
    syncJobsEl.replaceChildren();
    for (const j of res.jobs) {
      const li = document.createElement('li');
      const text = document.createElement('span');
      text.className = `job-text${j.state === 'failed' ? ' failed' : ''}`;
      const name = j.zip.split(/[\\/]/).pop();
      const st = { running: `${j.progress}% ${j.speed}`, done: t().jobDone, failed: t().jobFailed, stopped: t().jobStopped }[j.state];
      text.textContent = `${name} → ${j.dest} · ${st}${j.error ? ` · ${j.error}` : ''}`;
      text.title = `tmux attach -t ${j.session}\n${j.error}`;
      li.appendChild(text);
      if (j.state !== 'running') {
        const del = document.createElement('button');
        del.type = 'button';
        del.textContent = '✕';
        del.title = t().jobClear;
        del.addEventListener('click', async () => {
          await post('/api/sync-job-clear', { id: j.id });
          void loadJobs();
        });
        li.appendChild(del);
      } else {
        const bar = document.createElement('progress');
        bar.max = 100;
        bar.value = j.progress;
        const stop = document.createElement('button');
        stop.type = 'button';
        stop.textContent = t().jobStop;
        stop.addEventListener('click', async () => {
          await post('/api/sync-job-stop', { id: j.id });
          void loadJobs();
        });
        li.append(bar, stop);
      }
      syncJobsEl.appendChild(li);
    }
    if (res.jobs.some((j) => j.state === 'running')) jobTimer = window.setTimeout(() => void loadJobs(), 2000);
  } catch {
    // 讀不到就不更新（可能只是伺服器重啟中）
    jobTimer = window.setTimeout(() => void loadJobs(), 5000);
  }
}
const syncGo = document.getElementById('sync-go') as HTMLButtonElement;
let syncCtl: AbortController | null = null;
let syncZip = '';

function store(k: string, v?: string): string {
  try {
    if (v !== undefined) localStorage.setItem(k, v);
    return localStorage.getItem(k) ?? '';
  } catch {
    return '';
  }
}

/** 認證失敗時多提醒一句：金鑰有密碼的話在上方輸入金鑰密碼。 */
function syncErr(e: unknown): string {
  const m = e instanceof Error ? e.message : String(e);
  return t().failed(m) + (/permission denied \((publickey|password|keyboard)/i.test(m) ? `\n${t().syncPassHint}` : '');
}

async function remoteLs(path: string): Promise<void> {
  syncCtl?.abort();
  const ctl = (syncCtl = new AbortController());
  syncMsg.textContent = t().syncListing;
  try {
    const r = await fetch('/api/remote-ls', {
      method: 'POST',
      headers: { 'X-Lobj-Requested': '1', 'Content-Type': 'application/json' },
      body: JSON.stringify({ host: syncHost.value, path, passphrase: syncPass.value }),
      signal: ctl.signal,
    });
    if (!r.ok) throw new Error(await r.text());
    const res = (await r.json()) as { path: string; writable: boolean; dirs: string[] };
    syncDir.value = res.path;
    store('lobj-view-sync-host', syncHost.value);
    store('lobj-view-sync-dir', res.path);
    syncList.replaceChildren();
    const add = (label: string, to: string) => {
      const li = document.createElement('li');
      li.textContent = label;
      li.addEventListener('click', () => void remoteLs(to));
      syncList.appendChild(li);
    };
    if (res.path !== '/') add('📁 ..', res.path.replace(/\/[^/]*$/, '') || '/');
    for (const d of res.dirs) add(`📁 ${d}`, `${res.path.replace(/\/$/, '')}/${d}`);
    syncGo.disabled = !res.writable;
    syncMsg.textContent = res.writable ? '' : t().syncReadOnly;
  } catch (e) {
    if (!isAbort(e)) syncMsg.textContent = syncErr(e);
  }
}

document.getElementById('zip-sync')!.addEventListener('click', () => {
  if (!selDir) return;
  syncZip = selDir;
  document.getElementById('sync-file')!.textContent = `📦 ${selDir}`;
  syncHost.value ||= store('lobj-view-sync-host');
  syncDir.value ||= store('lobj-view-sync-dir');
  syncMsg.textContent = '';
  syncEl.hidden = false;
  void loadJobs();
  if (syncHost.value) void remoteLs(syncDir.value);
});
document.getElementById('sync-jobs-clear')!.addEventListener('click', async () => {
  await post('/api/sync-job-clear', {});
  void loadJobs();
});
document.getElementById('sync-close')!.addEventListener('click', () => {
  syncCtl?.abort(); // 中斷連線 → 後端殺掉 ssh / rsync（背景工作在 tmux 裡，不受影響）
  syncEl.hidden = true;
  window.clearTimeout(jobTimer);
});
document.getElementById('sync-connect')!.addEventListener('click', () => void remoteLs(syncDir.value));
for (const el of [syncHost, syncPass, syncDir]) el.addEventListener('keydown', (e) => e.key === 'Enter' && void remoteLs(syncDir.value));
syncGo.addEventListener('click', async () => {
  syncCtl?.abort();
  const ctl = (syncCtl = new AbortController());
  syncGo.disabled = true;
  syncMsg.textContent = t().syncRunning;
  try {
    const bg = syncBg.checked;
    const r = await fetch('/api/zip-sync', {
      method: 'POST',
      headers: { 'X-Lobj-Requested': '1', 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: syncZip, host: syncHost.value, dir: syncDir.value, passphrase: syncPass.value, background: bg }),
      signal: ctl.signal,
    });
    if (!r.ok) throw new Error(await r.text());
    if (bg) {
      const j = (await r.json()) as { session: string };
      store('lobj-view-sync-host', syncHost.value);
      store('lobj-view-sync-dir', syncDir.value);
      syncMsg.textContent = t().syncBgStarted(j.session);
      void loadJobs();
      return;
    }
    const res = await readStream<{ dest: string }>(r, (p) => {
      syncBar.hidden = false;
      syncBar.value = p.progress;
      syncMsg.textContent = `${t().syncRunning} ${p.progress}%${p.speed ? `  ${p.speed}` : ''}`;
    });
    store('lobj-view-sync-host', syncHost.value);
    store('lobj-view-sync-dir', syncDir.value);
    syncMsg.textContent = t().syncDone(res.dest);
  } catch (e) {
    if (!isAbort(e)) syncMsg.textContent = syncErr(e);
  } finally {
    syncGo.disabled = false;
    syncBar.hidden = true;
  }
});

const opProgress = document.getElementById('op-progress')!;
const opBar = document.getElementById('op-bar') as HTMLProgressElement;
const opText = document.getElementById('op-text')!;

/** 讀後端的 NDJSON 進度串流（一行一個 JSON）：`progress` 行交給 onProgress，回傳 `done` 的內容；`error` 行丟出例外。 */
async function readStream<T>(r: Response, onProgress: (p: { progress: number; speed?: string }) => void): Promise<T> {
  const reader = r.body!.getReader();
  const dec = new TextDecoder();
  let buf = '';
  let done: T | undefined;
  for (;;) {
    const { done: end, value } = await reader.read();
    if (value) buf += dec.decode(value, { stream: true });
    for (let i = buf.indexOf('\n'); i >= 0; i = buf.indexOf('\n')) {
      const text = buf.slice(0, i).trim();
      buf = buf.slice(i + 1);
      if (!text) continue;
      const o = JSON.parse(text);
      if (o.error !== undefined) throw new Error(o.error);
      if (o.done !== undefined) done = o.done as T;
      else if (o.progress !== undefined) onProgress(o);
    }
    if (end) break;
  }
  if (done === undefined) throw new Error('連線中斷');
  return done;
}

// 壓縮目前資料夾：後端在該資料夾的同層寫出「資料夾名.zip」（之後再用它同步到別台電腦）
document.getElementById('zip-dir')!.addEventListener('click', async () => {
  const btn = document.getElementById('zip-dir') as HTMLButtonElement;
  btn.disabled = true;
  infoEl.textContent = t().zipping;
  try {
    for (let overwrite = false; ; ) {
      const r = await fetch('/api/zip-dir', {
        method: 'POST',
        headers: { 'X-Lobj-Requested': '1', 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: curDir, overwrite }),
      });
      if (r.status === 409 && !overwrite && confirm(`${await r.text()}\n${t().zipOverwrite}`)) {
        overwrite = true;
        continue;
      }
      if (!r.ok) throw new Error(await r.text());
      const res = await readStream<{ zip_path: string; size: number; files: number }>(r, (p) => {
        opProgress.hidden = false;
        opBar.value = p.progress;
        opText.textContent = `${p.progress}%`;
      });
      infoEl.textContent = t().zipDone(res.zip_path, res.size, res.files);
      break;
    }
  } catch (e) {
    infoEl.textContent = t().failed(e instanceof Error ? e.message : String(e));
  } finally {
    btn.disabled = false;
    opProgress.hidden = true;
  }
});


document.getElementById('zip-del')!.addEventListener('click', async () => {
  if (!selDir || !confirm(t().zipDelConfirm(selDir.split('/').pop()!))) return;
  const r = await post('/api/zip-delete', { path: selDir });
  if (!r.ok) {
    infoEl.textContent = t().failed(await r.text());
    return;
  }
  await browse(curDir);
});

onLangChange(() => applyStatic());
void browse('');
