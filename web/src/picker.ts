// 資料夾選擇對話框：左側「常用位置 / 已釘選 / 最近」，右側列出該資料夾的子資料夾與檔案。
// 非遮蔽式浮動面板（可拖曳標題列移動）：點檔案＝即時加入選取、雙擊＝只開這一個、Ctrl/⌘ / Shift＝複選，
// 與主畫面的清單一致（共用同一個 selected 集合）；按「開啟」才把主畫面的清單切到這個資料夾。

import { listLobj, type LobjListing } from './lobj';
import { t, onLangChange } from './i18n';

export interface PickerDeps {
  /** 與主畫面共用的已選檔案集合（完整路徑） */
  selected: Set<string>;
  /** selected 被 picker 改動後呼叫：主畫面重畫高亮並載入 / 卸載模型 */
  applySelection(): void;
  /** 雙擊：清掉其他已選的，只開這一個 */
  openOnly(full: string): void;
  /** 主畫面切換資料夾 */
  browse(path: string): Promise<void>;
  /** 主畫面目前的資料夾，開啟對話框時作為起點 */
  getCurDir(): string;
  withBusy<T>(path: string, run: (signal: AbortSignal) => Promise<T>, ctl?: AbortController): Promise<T>;
  isAbort(e: unknown): boolean;
}

interface Place {
  kind: string;
  name: string;
  path: string;
}

const PINS_KEY = 'lobj-view-pins';
const RECENT_KEY = 'lobj-view-recent';
const RECENT_MAX = 12;

function load(key: string): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(key) ?? '[]');
    return Array.isArray(v) ? v.filter((x) => typeof x === 'string') : [];
  } catch {
    return [];
  }
}
function save(key: string, v: string[]): void {
  try {
    localStorage.setItem(key, JSON.stringify(v));
  } catch {
    /* localStorage 不可用就只留在記憶體 */
  }
}

let pins = load(PINS_KEY);
let recent = load(RECENT_KEY);

/** 主畫面成功進入資料夾時呼叫：記進「最近」（根資料夾不記）。 */
export function recordRecent(path: string): void {
  if (!path) return;
  recent = [path, ...recent.filter((p) => p !== path)].slice(0, RECENT_MAX);
  save(RECENT_KEY, recent);
}

const join = (dir: string, name: string) => (dir ? `${dir.endsWith('/') ? dir : `${dir}/`}${name}` : name);
const label = (p: string) => p.replace(/\/+$/, '').split('/').pop() || p;

export function initPicker(d: PickerDeps): void {
  const el = document.getElementById('picker')!;
  const head = el.querySelector<HTMLElement>('.dlg-head')!;
  const sideEl = document.getElementById('picker-side')!;
  const pathEl = document.getElementById('picker-path') as HTMLInputElement;
  const pinBtn = document.getElementById('picker-pin') as HTMLButtonElement;
  const listEl = document.getElementById('picker-list') as HTMLUListElement;
  const infoEl = document.getElementById('picker-info')!;
  const openBtn = document.getElementById('picker-do-open') as HTMLButtonElement;

  let places: Place[] = [];
  let dir = ''; // 對話框目前的資料夾（和主畫面的 curDir 各自獨立）
  let ok = false; // 目前資料夾是否列舉成功
  let files: string[] = [];
  let lastClicked = '';
  let ctl: AbortController | null = null;

  const placeName = (p: Place): string => {
    const tt = t() as unknown as Record<string, string>;
    return p.kind === 'drive' ? p.name : (tt[`place_${p.kind}`] ?? p.kind);
  };

  function markSel(): void {
    for (const li of listEl.querySelectorAll<HTMLLIElement>('li[data-path]')) {
      li.classList.toggle('sel', d.selected.has(li.dataset.path!));
    }
  }

  function sideItem(text: string, path: string, tip: string, btns: [string, string, () => void][] = []): HTMLElement {
    const row = document.createElement('div');
    row.className = 'pk-item' + (path === dir ? ' cur' : '');
    const name = document.createElement('span');
    name.className = 'pk-name';
    name.textContent = text;
    name.title = tip;
    name.addEventListener('click', () => void go(path));
    row.appendChild(name);
    for (const [txt, title, fn] of btns) {
      const b = document.createElement('button');
      b.type = 'button';
      b.textContent = txt;
      b.title = title;
      b.addEventListener('click', (e) => {
        e.stopPropagation();
        fn();
      });
      row.appendChild(b);
    }
    return row;
  }

  function renderSide(): void {
    const tt = t();
    const sec = (title: string) => {
      const h = document.createElement('div');
      h.className = 'pk-sec';
      h.textContent = title;
      return h;
    };
    const frag = document.createDocumentFragment();
    frag.append(sec(tt.pickerPlaces));
    for (const p of places) frag.append(sideItem(placeName(p), p.path, p.path || '--dir'));
    if (pins.length) {
      frag.append(sec(tt.pickerPinned));
      for (const p of pins) frag.append(sideItem(`📌 ${label(p)}`, p, p, [['✕', tt.pickerUnpin, () => togglePin(p)]]));
    }
    const rec = recent.filter((p) => !pins.includes(p));
    if (rec.length) {
      frag.append(sec(tt.pickerRecent));
      for (const p of rec) {
        frag.append(
          sideItem(label(p), p, p, [
            ['📌', tt.pickerPin, () => togglePin(p)],
            ['✕', tt.pickerRemove, () => removeRecent(p)],
          ]),
        );
      }
    }
    sideEl.replaceChildren(frag);
    pinBtn.textContent = pins.includes(dir) ? '📌' : '📍';
    pinBtn.title = pins.includes(dir) ? tt.pickerUnpin : tt.pickerPin;
    pinBtn.disabled = !dir;
  }

  function togglePin(p: string): void {
    pins = pins.includes(p) ? pins.filter((x) => x !== p) : [p, ...pins];
    save(PINS_KEY, pins);
    renderSide();
  }
  function removeRecent(p: string): void {
    recent = recent.filter((x) => x !== p);
    save(RECENT_KEY, recent);
    renderSide();
  }
  pinBtn.addEventListener('click', () => dir && togglePin(dir));

  function addRow(text: string, cls: string, onClick: (e: MouseEvent) => void): HTMLLIElement {
    const li = document.createElement('li');
    li.textContent = text;
    li.className = cls;
    li.addEventListener('click', onClick);
    listEl.appendChild(li);
    return li;
  }

  /** 與主畫面 clickFile 相同的選取語意（範圍以對話框目前的清單為準）。 */
  function clickFile(full: string, e: MouseEvent): void {
    if (e.detail > 1) return; // 雙擊的第二下交給 dblclick
    const additive = e.ctrlKey || e.metaKey;
    if (e.shiftKey && files.includes(lastClicked)) {
      const a = files.indexOf(lastClicked);
      const b = files.indexOf(full);
      if (!additive) d.selected.clear();
      for (const f of files.slice(Math.min(a, b), Math.max(a, b) + 1)) d.selected.add(f);
    } else {
      if (!d.selected.delete(full)) d.selected.add(full);
      lastClicked = full;
    }
    d.applySelection();
    markSel();
  }

  async function go(path: string): Promise<void> {
    ctl?.abort();
    const mine = (ctl = new AbortController());
    infoEl.textContent = t().pickerLoading;
    try {
      const l: LobjListing = await d.withBusy(path, (signal) => listLobj(path, signal), mine);
      dir = l.path;
      ok = true;
      openBtn.disabled = false;
      pathEl.value = dir ? (dir.endsWith('/') ? dir : `${dir}/`) : '';
      listEl.replaceChildren();
      if (l.parent) {
        const up = l.up ?? (dir.includes('/') ? dir.slice(0, dir.lastIndexOf('/')) : '');
        addRow('📁 ..', 'dir', () => void go(up));
      }
      const zips = new Set(l.zips ?? []);
      for (const n of l.dirs) addRow(`${zips.has(n) ? '📦' : '📁'} ${n}`, 'dir', () => void go(join(dir, n)));
      files = l.files.map((f) => join(dir, f.name));
      for (const f of l.files) {
        const full = join(dir, f.name);
        const li = addRow(`${f.name} (${(f.size / 1048576).toFixed(1)} MB)`, '', (e) => clickFile(full, e));
        li.dataset.path = full;
        li.addEventListener('dblclick', () => {
          d.openOnly(full);
          markSel();
        });
      }
      markSel();
      infoEl.textContent = !l.dirs.length && !l.files.length ? t().emptyDir : '';
    } catch (e) {
      if (d.isAbort(e)) {
        if (ctl === mine) infoEl.textContent = t().browseCanceled;
        return;
      }
      ok = false;
      openBtn.disabled = true;
      infoEl.textContent = t().cannotOpenPath(e instanceof Error ? e.message : String(e));
    }
    renderSide();
  }

  async function loadPlaces(): Promise<void> {
    try {
      const r = await fetch('/api/places');
      if (r.ok) places = await r.json();
    } catch {
      /* 沒有常用位置也能用 */
    }
    if (!places.length) places = [{ kind: 'root', name: '', path: '' }];
  }

  async function open(): Promise<void> {
    el.hidden = false;
    if (!places.length) await loadPlaces();
    await go(d.getCurDir());
  }

  document.getElementById('picker-open')!.addEventListener('click', () => (el.hidden ? void open() : (el.hidden = true)));
  document.getElementById('picker-close')!.addEventListener('click', () => (el.hidden = true));
  openBtn.addEventListener('click', () => {
    if (!ok) return;
    void d.browse(dir);
    el.hidden = true;
  });
  pathEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') void go(pathEl.value);
  });
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !el.hidden) el.hidden = true;
  });

  // 背景透明度（滑桿）與大小（CSS resize）都記住
  const alphaEl = document.getElementById('picker-alpha') as HTMLInputElement;
  const SIZE_KEY = 'lobj-view-picker-size';
  const ALPHA_KEY = 'lobj-view-picker-alpha';
  const setAlpha = (v: string) => el.style.setProperty('--pk-alpha', v);
  try {
    const a = localStorage.getItem(ALPHA_KEY);
    if (a && +a >= 0.3 && +a <= 1) alphaEl.value = a;
    const sz = JSON.parse(localStorage.getItem(SIZE_KEY) ?? 'null');
    if (sz && sz.w > 0 && sz.h > 0) {
      el.style.width = `${sz.w}px`;
      el.style.height = `${sz.h}px`;
    }
  } catch {
    /* 用預設 */
  }
  setAlpha(alphaEl.value);
  alphaEl.addEventListener('input', () => {
    setAlpha(alphaEl.value);
    try {
      localStorage.setItem(ALPHA_KEY, alphaEl.value);
    } catch {
      /* 忽略 */
    }
  });
  new ResizeObserver(() => {
    if (el.hidden) return;
    try {
      localStorage.setItem(SIZE_KEY, JSON.stringify({ w: el.offsetWidth, h: el.offsetHeight }));
    } catch {
      /* 忽略 */
    }
  }).observe(el);

  // 拖曳標題列移動
  head.addEventListener('pointerdown', (e) => {
    if ((e.target as HTMLElement).closest('button, label')) return;
    const r = el.getBoundingClientRect();
    const dx = e.clientX - r.left;
    const dy = e.clientY - r.top;
    el.style.transform = 'none';
    el.style.left = `${r.left}px`;
    el.style.top = `${r.top}px`;
    head.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => {
      el.style.left = `${Math.max(0, Math.min(innerWidth - 80, ev.clientX - dx))}px`;
      el.style.top = `${Math.max(0, Math.min(innerHeight - 40, ev.clientY - dy))}px`;
    };
    const up = () => {
      head.removeEventListener('pointermove', move);
      head.removeEventListener('pointerup', up);
    };
    head.addEventListener('pointermove', move);
    head.addEventListener('pointerup', up);
  });

  // 切換語言時重繪動態字串
  onLangChange(() => {
    if (!el.hidden) renderSide();
  });
}
