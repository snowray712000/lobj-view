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
let busy = false;
let msg = '';
let browseGen = 0;

const defaultName = (path: string) => (path.replace(/[\\/]+$/, '').split(/[\\/]/).pop() ?? '').replace(/^ACom_/i, '');

function render(): void {
  const tt = t();
  const head = `<tr><th>${esc(tt.stColCase)}</th><th>upper</th><th>lower</th>${FDI.map((f) => `<th>${f}</th>`).join('')}<th>${esc(tt.stColRemark)}</th></tr>`;
  const body = rows
    .map((r) => {
      const cell = (v: boolean) => `<td class="c ${v ? '' : 'z'}">${v ? 1 : 0}</td>`;
      return `<tr><td>${esc(r.name)}</td>${cell(r.upper)}${cell(r.lower)}${FDI.map((f) => cell(r.fdi.has(f))).join('')}<td class="${r.remark ? 'warn' : ''}">${esc(r.remark)}</td></tr>`;
    })
    .join('');
  $('st-table').innerHTML = rows.length ? head + body : '';
  $('l2-msg').textContent = msg;
  $<HTMLButtonElement>('l2-up').disabled = !cur.parent;
  $<HTMLButtonElement>('st-run').disabled = busy;
  $<HTMLButtonElement>('st-save').disabled = busy || !rows.length;
}

async function browse(path: string): Promise<void> {
  const gen = ++browseGen;
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
  busy = false;
  bar.hidden = true;
  msg = t().stSummary(rows.length, rows.filter((r) => r.upper).length, rows.filter((r) => r.lower).length, rows.filter((r) => r.remark).length);
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
