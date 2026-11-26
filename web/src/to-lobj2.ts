// 批次轉 .lobj2 頁（/to-lobj2）：與檢視器主畫面各自獨立的 Vite 入口，共用 picker / i18n / lobj 列舉 / 樣式。
// 轉檔本身走既有的 POST /api/lobj-convert（只限本機，需 X-Lobj-Requested）。

import './apple-color-ios.css';
import './style.css';
import './to-lobj2.css';
import { listLobj, type LobjListing } from './lobj';
import { applyStatic, onLangChange, t } from './i18n';
import { initPicker, recordRecent } from './picker';

type Jaw = 'upper' | 'lower' | '';
interface Row {
  /** 顯示用：遞迴時含相對於目前資料夾的子路徑 */
  name: string;
  full: string;
  size: number;
  jaw: Jaw;
  exists: boolean;
  /** 這個 .lobj 已被刪除（列仍保留顯示結果） */
  removed?: boolean;
  /** 轉換進度的狀態（HTML 已跳脫）；空字串 = 依其他欄位顯示預設狀態 */
  state: () => string;
}

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

// 主題沿用主畫面記住的選擇（空字串 = 跟隨系統）
try {
  const th = localStorage.getItem('lobj-view-theme');
  if (th) document.documentElement.dataset.theme = th;
} catch {
  /* 沿用系統主題 */
}
applyStatic();

const join = (dir: string, name: string) => (dir ? `${dir.endsWith('/') ? dir : `${dir}/`}${name}` : name);
const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);
const fmtSize = (n: number) => (n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.ceil(n / 1024)} KB`);
const guessJaw = (n: string): Jaw => {
  const s = n.toLowerCase();
  const u = s.includes('upper');
  const l = s.includes('lower');
  return u && !l ? 'upper' : l && !u ? 'lower' : '';
};

let cur: LobjListing = { path: '', parent: false, up: null, dirs: [], zips: [], files: [], selected: null } as unknown as LobjListing;
let rows: Row[] = [];
/** 勾選要轉的檔案（完整路徑）；與 picker 共用，所以在對話框裡點檔案也會勾選 */
const selected = new Set<string>();
let busy = false;
let lastSummary = '';
const done = new Set<string>();

const todo = () => rows.filter((r) => selected.has(r.full) && r.jaw && !r.exists && !done.has(r.full));

function render(): void {
  const tt = t();
  const dirsEl = $('l2-dirs');
  dirsEl.replaceChildren(
    ...cur.dirs
      .filter((d) => !(cur.zips ?? []).includes(d))
      .map((d) => {
        const b = document.createElement('button');
        b.type = 'button';
        b.textContent = `📁 ${d}`;
        b.addEventListener('click', () => void browse(join(cur.path, d)));
        return b;
      }),
  );
  $<HTMLButtonElement>('l2-up').disabled = !cur.parent;
  const tb = $('l2-files');
  tb.replaceChildren();
  for (const r of rows) {
    const tr = document.createElement('tr');
    const st =
      r.state() ||
      (r.exists
        ? `<span class="warn">${esc(tt.l2Exists)}</span>`
        : !r.jaw
          ? `<span class="warn">${esc(tt.l2NeedJaw)}</span>`
          : `<span class="hint">${esc(tt.l2Pending)}</span>`);
    tr.innerHTML =
      `<td><input type="checkbox" ${selected.has(r.full) ? 'checked' : ''} ${r.exists || busy ? 'disabled' : ''}></td>` +
      `<td>${esc(r.name)}</td>` +
      `<td><select ${busy ? 'disabled' : ''}><option value="">?</option><option value="upper">${esc(tt.l2Upper)}</option><option value="lower">${esc(tt.l2Lower)}</option></select></td>` +
      `<td class="num">${fmtSize(r.size)}</td><td>${st}</td>`;
    const sel = tr.querySelector('select')!;
    sel.value = r.jaw;
    sel.addEventListener('change', () => {
      r.jaw = sel.value as Jaw;
      if (!r.jaw) selected.delete(r.full);
      render();
    });
    tr.querySelector('input')!.addEventListener('change', (e) => {
      if ((e.target as HTMLInputElement).checked) selected.add(r.full);
      else selected.delete(r.full);
      renderCount();
    });
    tb.append(tr);
  }
  if (!rows.length) tb.innerHTML = `<tr><td colspan="5" class="hint">${esc(tt.l2Empty)}</td></tr>`;
  $('l2-msg').textContent = lastSummary;
  renderCount();
}

function renderCount(): void {
  $('l2-count').textContent = t().l2Count(todo().length, rows.length);
  $<HTMLButtonElement>('l2-run').disabled = busy || !todo().length;
  $<HTMLButtonElement>('l2-delold').disabled = busy || !rows.some((r) => r.exists && !r.removed);
  $<HTMLInputElement>('l2-all').checked = rows.length > 0 && rows.every((r) => r.exists || selected.has(r.full));
}

/** 一個資料夾的列舉結果轉成列；rel 是相對於目前資料夾的子路徑（空字串 = 目前資料夾本身）。 */
function rowsOf(l: LobjListing, rel: string): Row[] {
  const names = new Set(l.files.map((f) => f.name.toLowerCase()));
  return l.files
    .filter((f) => /\.lobj$/i.test(f.name))
    .map((f) => ({
      name: rel ? `${rel}/${f.name}` : f.name,
      full: join(l.path, f.name),
      size: f.size,
      jaw: guessJaw(f.name),
      exists: names.has(`${f.name}2`.toLowerCase()),
      state: () => '',
    }));
}

/** 連子資料夾一起列舉（不進入 .zip）；同時最多 8 個請求，找到的數量即時顯示。 */
async function scanAll(top: LobjListing, gen: number): Promise<Row[]> {
  const out: Row[] = rowsOf(top, '');
  const queue: { path: string; rel: string }[] = [];
  const push = (l: LobjListing, rel: string) => {
    const zips = new Set(l.zips ?? []);
    for (const d of l.dirs) if (!zips.has(d)) queue.push({ path: join(l.path, d), rel: rel ? `${rel}/${d}` : d });
  };
  push(top, '');
  const worker = async () => {
    for (let q = queue.shift(); q && gen === browseGen; q = queue.shift()) {
      try {
        const l = await listLobj(q.path);
        out.push(...rowsOf(l, q.rel));
        push(l, q.rel);
        $('l2-msg').textContent = t().l2Scanning(out.length);
      } catch {
        /* 讀不了的子資料夾略過 */
      }
    }
  };
  // queue 會在掃描中增長：反覆啟動 worker 直到清空
  while (queue.length && gen === browseGen) await Promise.all(Array.from({ length: 8 }, worker));
  out.sort((a, b) => (a.full < b.full ? -1 : 1));
  return out;
}

let browseGen = 0;

async function browse(path: string): Promise<void> {
  lastSummary = '';
  const gen = ++browseGen;
  try {
    const l = await listLobj(path);
    if (gen !== browseGen) return;
    cur = l;
    recordRecent(cur.path);
    $<HTMLInputElement>('l2-path').value = cur.path;
    selected.clear();
    done.clear();
    rows = $<HTMLInputElement>('l2-rec').checked ? await scanAll(cur, gen) : rowsOf(cur, '');
    if (gen !== browseGen) return;
  } catch (e) {
    $('l2-msg').innerHTML = `<span class="bad">${esc(e instanceof Error ? e.message : String(e))}</span>`;
    return;
  }
  for (const r of rows) if (!r.exists && r.jaw) selected.add(r.full);
  render();
}

// 共用的資料夾選擇對話框：對話框裡點檔案＝勾選（只有 .lobj 且可轉的列才算），「開啟」＝切換到那個資料夾
initPicker({
  selected,
  applySelection: () => {
    for (const p of [...selected]) {
      const r = rows.find((x) => x.full === p);
      if (!r || r.exists || !r.jaw) selected.delete(p); // 非本資料夾的 .lobj、已轉過或不明上下顎的不算
    }
    render();
  },
  openOnly: (full) => {
    selected.clear();
    selected.add(full);
    render();
  },
  browse,
  getCurDir: () => cur.path,
  withBusy: (_path, run, ctl) => run((ctl ?? new AbortController()).signal),
  isAbort: (e) => e instanceof DOMException && e.name === 'AbortError',
});

$('l2-rec').addEventListener('change', () => void browse(cur.path));
$('l2-all').addEventListener('change', (e) => {
  const on = (e.target as HTMLInputElement).checked;
  for (const r of rows) if (!r.exists && r.jaw) on ? selected.add(r.full) : selected.delete(r.full);
  render();
});
$('l2-go').addEventListener('click', () => void browse($<HTMLInputElement>('l2-path').value.trim()));
$('l2-path').addEventListener('keydown', (e) => {
  if (e.key === 'Enter') void browse($<HTMLInputElement>('l2-path').value.trim());
});
$('l2-up').addEventListener('click', () => cur.parent && void browse(cur.up ?? ''));

$('l2-run').addEventListener('click', async () => {
  const list = todo();
  if (!list.length) return;
  const del = $<HTMLInputElement>('l2-del').checked;
  if (del && !confirm(t().l2ConfirmDel(list.length))) return;
  busy = true;
  const bar = $<HTMLProgressElement>('l2-bar');
  bar.hidden = false;
  bar.max = list.length;
  bar.value = 0;
  let ok = 0;
  let fail = 0;
  for (const r of list) {
    r.state = () => `<span class="hint">${esc(t().l2Converting)}</span>`;
    render();
    try {
      const res = await fetch('/api/lobj-convert', {
        method: 'POST',
        headers: { 'X-Lobj-Requested': '1', 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: r.full, jaw: r.jaw, delete_old: del }),
      });
      if (!res.ok) throw new Error(await res.text());
      r.state = () => `<span class="ok">${esc(t().l2Converted(del))}</span>`;
      r.exists = true;
      selected.delete(r.full);
      done.add(r.full);
      ok++;
    } catch (e) {
      const m = e instanceof Error ? e.message : String(e);
      r.state = () => `<span class="bad">${esc(t().l2Failed(m))}</span>`;
      fail++;
    }
    bar.value++;
    render();
  }
  busy = false;
  lastSummary = t().l2Summary(ok, fail);
  render();
});

// 已經轉過了、現在只想清掉舊 .lobj：刪除所有「同名 .lobj2 已存在」的 .lobj（後端刪前會讀回比對）
$('l2-delold').addEventListener('click', async () => {
  const list = rows.filter((r) => r.exists && !r.removed);
  if (!list.length || !confirm(t().l2DelOldConfirm(list.length))) return;
  busy = true;
  const bar = $<HTMLProgressElement>('l2-bar');
  bar.hidden = false;
  bar.max = list.length;
  bar.value = 0;
  let ok = 0;
  let fail = 0;
  for (const r of list) {
    try {
      const res = await fetch('/api/lobj-delete-old', {
        method: 'POST',
        headers: { 'X-Lobj-Requested': '1', 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: r.full }),
      });
      if (!res.ok) throw new Error(await res.text());
      r.removed = true;
      r.state = () => `<span class="ok">${esc(t().l2Deleted)}</span>`;
      ok++;
    } catch (e) {
      const m = e instanceof Error ? e.message : String(e);
      r.state = () => `<span class="bad">${esc(t().l2Failed(m))}</span>`;
      fail++;
    }
    bar.value++;
    render();
  }
  busy = false;
  lastSummary = t().l2DelSummary(ok, fail);
  render();
});

onLangChange(render);
void browse(new URLSearchParams(location.search).get('path') ?? '');
