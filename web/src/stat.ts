// 資料集統計頁（/stat）：Vite 第三個入口，共用 picker / i18n / 列舉 / 樣式。
// 每個子資料夾 = 一個病例；逐一呼叫 GET /api/stat-case，最後組成 CSV，由 POST /api/stat-save 寫成 <資料夾>/stat.csv。

import './apple-color-ios.css';
import './style.css';
import './to-lobj2.css';
import './stat.css';
import { listLobj, type LobjListing } from './lobj';
import { applyStatic, onLangChange, t } from './i18n';
import { initPicker, recordRecent } from './picker';

interface CaseRow {
  name: string;
  upper: boolean;
  lower: boolean;
  fdi: Set<number>;
  remark: string;
}

/** 所有 FDI：恆牙 11–18 / 21–28 / 31–38 / 41–48，再接乳牙 51–55 / 61–65 / 71–75 / 81–85 */
const FDI: number[] = [];
for (const q of [1, 2, 3, 4]) for (let i = 1; i <= 8; i++) FDI.push(q * 10 + i);
for (const q of [5, 6, 7, 8]) for (let i = 1; i <= 5; i++) FDI.push(q * 10 + i);

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);
const join = (dir: string, name: string) => (dir ? `${dir.endsWith('/') ? dir : `${dir}/`}${name}` : name);

try {
  const th = localStorage.getItem('lobj-view-theme');
  if (th) document.documentElement.dataset.theme = th;
} catch {
  /* 沿用系統主題 */
}
applyStatic();

let cur: LobjListing = { path: '', parent: false, up: null, dirs: [], zips: [], files: [], selected: null } as unknown as LobjListing;
let rows: CaseRow[] = [];
/** 目前資料夾已有 stat.csv（顯示的是它的內容，按鈕變成「重新產生」） */
let hasCsv = false;
/** rows 是剛統計出來、尚未輸出的結果（才允許輸出 stat.csv） */
let fresh = false;
let transposed = false;
try {
  transposed = localStorage.getItem('lobj-view-stat-transposed') === '1';
} catch {
  /* 預設直式 */
}
let busy = false;
let msg = '';
let browseGen = 0;

const defaultName = (path: string) => (path.replace(/[\\/]+$/, '').split(/[\\/]/).pop() ?? '').replace(/^ACom_/i, '');

/** 統計項目：upper、lower，再來每個 FDI；n = 有這一項的病例數 */
function items(): { label: string; n: number }[] {
  return [
    { label: 'upper', n: rows.filter((r) => r.upper).length },
    { label: 'lower', n: rows.filter((r) => r.lower).length },
    ...FDI.map((f) => ({ label: String(f), n: rows.filter((r) => r.fdi.has(f)).length })),
  ];
}

function render(): void {
  const tt = t();
  const total = rows.length;
  const pct = (n: number) => (total ? `${((n * 100) / total).toFixed(1)}` : '');
  const its = items();
  const z = (n: number) => (n ? '' : ' z');
  $('st-table').innerHTML = !total
    ? ''
    : transposed
      ? // 橫式（電腦）：項目橫排，兩列 = 病例數、%
        `<tr><th class="hd">${esc(tt.stColItem)}</th>${its.map((i) => `<th>${i.label}</th>`).join('')}</tr>` +
        `<tr><td class="hd">${esc(tt.stColCount)}</td>${its.map((i) => `<td class="n${z(i.n)}">${i.n}</td>`).join('')}</tr>` +
        `<tr><td class="hd">${esc(tt.stColPct)}</td>${its.map((i) => `<td class="n${z(i.n)}">${pct(i.n)}</td>`).join('')}</tr>`
      : // 直式（手機）：每個項目一列
        `<tr><th>${esc(tt.stColItem)}</th><th>${esc(tt.stColCount)}</th><th>${esc(tt.stColPct)}</th></tr>` +
        its.map((i) => `<tr><td class="hd">${i.label}</td><td class="n${z(i.n)}">${i.n}</td><td class="n${z(i.n)}">${pct(i.n)}</td></tr>`).join('');
  const withRemark = rows.filter((r) => r.remark);
  $('st-remarks').innerHTML = withRemark.length
    ? `<b>${esc(tt.stRemarks)}</b><ul>${withRemark.map((r) => `<li>${esc(r.name)}：<span class="warn">${esc(r.remark)}</span></li>`).join('')}</ul>`
    : '';
  const zips = new Set(cur.zips ?? []);
  $('l2-dirs').replaceChildren(
    ...cur.dirs
      .filter((d) => !zips.has(d))
      .map((d) => {
        const b = document.createElement('button');
        b.type = 'button';
        b.textContent = `📁 ${d}`;
        b.addEventListener('click', () => void browse(join(cur.path, d)));
        return b;
      }),
  );
  $('l2-msg').textContent = msg;
  $<HTMLButtonElement>('l2-up').disabled = !cur.parent;
  const run = $<HTMLButtonElement>('st-run');
  run.textContent = hasCsv ? tt.stRerun : tt.stRun;
  run.title = hasCsv ? tt.stRerunTip : '';
  run.disabled = busy;
  $<HTMLButtonElement>('st-save').disabled = busy || !fresh || !rows.length;
  $<HTMLButtonElement>('st-tr').disabled = !rows.length;
  $<HTMLButtonElement>('st-dl').disabled = !rows.length;
}

/** 解析 CSV（支援雙引號欄位、欄內逗號 / 換行、"" 跳脫；去掉 BOM） */
function parseCsv(text: string): string[][] {
  const out: string[][] = [];
  let row: string[] = [];
  let f = '';
  let q = false;
  const src = text.replace(/^\uFEFF/, '');
  for (let i = 0; i < src.length; i++) {
    const c = src[i];
    if (q) {
      if (c === '"') {
        if (src[i + 1] === '"') (f += '"'), i++;
        else q = false;
      } else f += c;
    } else if (c === '"') q = true;
    else if (c === ',') (row.push(f), (f = ''));
    else if (c === '\n' || c === '\r') {
      if (c === '\r' && src[i + 1] === '\n') i++;
      row.push(f);
      f = '';
      if (row.length > 1 || row[0] !== '') out.push(row);
      row = [];
    } else f += c;
  }
  if (f !== '' || row.length) {
    row.push(f);
    out.push(row);
  }
  return out;
}

/** 讀取資料夾裡既有的 stat.csv；沒有就回 null */
async function loadCsv(path: string): Promise<{ name: string; rows: CaseRow[] } | null> {
  const res = await fetch(`/api/stat-csv?path=${encodeURIComponent(path)}`);
  if (!res.ok) return null;
  const tbl = parseCsv(await res.text());
  if (tbl.length < 1) return null;
  const col = new Map(tbl[0].map((h, i) => [h.trim(), i] as const));
  const ci = col.get('case name');
  if (ci === undefined) return null;
  const flag = (r: string[], k: string | number) => {
    const i = col.get(String(k));
    return i !== undefined && r[i]?.trim() === '1';
  };
  const di = col.get('dataset_name');
  const ri = col.get('remark');
  const data = tbl.slice(1).map((r) => ({
    name: r[ci] ?? '',
    upper: flag(r, 'upper'),
    lower: flag(r, 'lower'),
    fdi: new Set(FDI.filter((f) => flag(r, f))),
    remark: ri === undefined ? '' : (r[ri] ?? ''),
  }));
  return { name: (di !== undefined && tbl[1]?.[di]) || '', rows: data };
}

async function browse(path: string): Promise<void> {
  const gen = ++browseGen;
  busy = false;
  $<HTMLProgressElement>('l2-bar').hidden = true;
  try {
    const l = await listLobj(path);
    if (gen !== browseGen) return;
    cur = l;
  } catch (e) {
    msg = e instanceof Error ? e.message : String(e);
    render();
    return;
  }
  recordRecent(cur.path);
  $<HTMLInputElement>('l2-path').value = cur.path;
  $<HTMLInputElement>('st-name').value = defaultName(cur.path);
  rows = [];
  msg = '';
  fresh = false;
  hasCsv = false;
  render();
  // 這個資料夾已有 stat.csv → 直接載入，不必重算
  try {
    const c = await loadCsv(cur.path);
    if (gen !== browseGen) return;
    if (c) {
      hasCsv = true;
      rows = c.rows;
      if (c.name) $<HTMLInputElement>('st-name').value = c.name;
      msg = t().stLoaded(rows.length);
    }
  } catch {
    /* 讀不到就當作沒有 */
  }
  render();
}

initPicker({
  selected: new Set<string>(),
  applySelection: () => {},
  openOnly: () => {},
  browse,
  getCurDir: () => cur.path,
  withBusy: (_path, run, ctl) => run((ctl ?? new AbortController()).signal),
  isAbort: (e) => e instanceof DOMException && e.name === 'AbortError',
});

/** CSV 欄位：含逗號 / 引號 / 換行才加引號 */
const csvCell = (s: string) => (/[",\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s);

function buildCsv(): string {
  const ds = $<HTMLInputElement>('st-name').value.trim() || defaultName(cur.path);
  const lines = [['dataset_name', 'case name', 'upper', 'lower', ...FDI.map(String), 'remark'].join(',')];
  for (const r of rows) {
    lines.push(
      [csvCell(ds), csvCell(r.name), r.upper ? 1 : 0, r.lower ? 1 : 0, ...FDI.map((f) => (r.fdi.has(f) ? 1 : 0)), csvCell(r.remark)].join(','),
    );
  }
  return lines.join('\n') + '\n';
}

$('st-run').addEventListener('click', async () => {
  const zips = new Set(cur.zips ?? []);
  const cases = cur.dirs.filter((d) => !zips.has(d));
  rows = [];
  if (!cases.length) {
    msg = t().stEmpty;
    render();
    return;
  }
  const gen = ++browseGen; // 統計中切換資料夾就丟棄
  busy = true;
  const bar = $<HTMLProgressElement>('l2-bar');
  bar.hidden = false;
  bar.max = cases.length;
  bar.value = 0;
  const dir = cur.path;
  const out: CaseRow[] = [];
  for (const [i, c] of cases.entries()) {
    if (gen !== browseGen) return;
    msg = t().stRunning(i + 1, cases.length);
    $('l2-msg').textContent = msg;
    let row: CaseRow;
    try {
      const res = await fetch(`/api/stat-case?path=${encodeURIComponent(join(dir, c))}`);
      if (!res.ok) throw new Error(await res.text());
      const j = (await res.json()) as { upper: boolean; lower: boolean; fdi: number[]; remark: string };
      row = { name: c, upper: j.upper, lower: j.lower, fdi: new Set(j.fdi), remark: j.remark };
    } catch (e) {
      row = { name: c, upper: false, lower: false, fdi: new Set(), remark: e instanceof Error ? e.message : String(e) };
    }
    out.push(row);
    bar.value = i + 1;
  }
  rows = out;
  fresh = true;
  busy = false;
  bar.hidden = true;
  msg = t().stSummary(rows.length, rows.filter((r) => r.upper).length, rows.filter((r) => r.lower).length, rows.filter((r) => r.remark).length);
  render();
});

// 下載：在瀏覽器端用目前顯示的資料組 CSV（含 BOM，Excel 才不會亂碼），檔名 stat-{dataset_name}.csv；不經伺服器、遠端也能用
$('st-dl').addEventListener('click', () => {
  const ds = $<HTMLInputElement>('st-name').value.trim() || defaultName(cur.path) || 'dataset';
  const blob = new Blob(['\uFEFF', buildCsv()], { type: 'text/csv;charset=utf-8' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = `stat-${ds.replace(/[\\/:*?"<>|]/g, '_')}.csv`;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 10000);
});

$('st-tr').addEventListener('click', () => {
  transposed = !transposed;
  try {
    localStorage.setItem('lobj-view-stat-transposed', transposed ? '1' : '0');
  } catch {
    /* 只是記住偏好 */
  }
  render();
});

$('st-save').addEventListener('click', async () => {
  try {
    const res = await fetch('/api/stat-save', {
      method: 'POST',
      headers: { 'X-Lobj-Requested': '1', 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: cur.path, csv: buildCsv() }),
    });
    if (!res.ok) throw new Error(await res.text());
    msg = t().stSaved(((await res.json()) as { path: string }).path);
    hasCsv = true;
    fresh = false;
  } catch (e) {
    msg = e instanceof Error ? e.message : String(e);
  }
  render();
});

$('l2-go').addEventListener('click', () => void browse($<HTMLInputElement>('l2-path').value.trim()));
$('l2-path').addEventListener('keydown', (e) => {
  if (e.key === 'Enter') void browse($<HTMLInputElement>('l2-path').value.trim());
});
$('l2-up').addEventListener('click', () => cur.parent && void browse(cur.up ?? ''));

onLangChange(render);
void browse(new URLSearchParams(location.search).get('path') ?? '');
