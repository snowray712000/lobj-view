import { initPicker, recordRecent } from './picker';
import './apple-color-ios.css';
import './style.css';
import * as THREE from 'three/webgpu';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { fetchLobj, labelCounts, listLobj, type LobjData } from './lobj';
import { applyProps, curvatureColoring, defaultProps, PALETTE, pickAutoColor, resolveCssColor, type ObjProps } from './props';
import { DEFAULT_CLIP, gradientCss, type CurvMode } from './curvature';
import { applyStatic, getLang, LANGS, onLangChange, setLang, t } from './i18n';

applyStatic(); // 靜態 HTML 先換成目前語言（預設中文）

const container = document.getElementById('viewport')!;
const backendEl = document.getElementById('backend')!;
const healthEl = document.getElementById('health')!;

// --- renderer：WebGPU 優先，不可用時自動退回 WebGL2 ---
// 注意：WebGPURenderer 不支援 GLSL ShaderMaterial，自訂 shader 請用 TSL（three/tsl）。
const renderer = new THREE.WebGPURenderer({ antialias: true });
renderer.setPixelRatio(window.devicePixelRatio);
container.appendChild(renderer.domElement);
await renderer.init();

const isWebGPU = (renderer.backend as { isWebGPUBackend?: boolean }).isWebGPUBackend === true;
backendEl.className = isWebGPU ? 'ok' : 'warn';
const renderBackend = () => {
  backendEl.textContent = (isWebGPU ? 'WebGPU' : 'WebGL2 (fallback)') + (!isWebGPU && !window.isSecureContext ? t().notSecure : '');
};
renderBackend();

// --- scene ---
const scene = new THREE.Scene();

const perspCamera = new THREE.PerspectiveCamera(45, 1, 0.01, 1000);
perspCamera.up.set(0, 0, 1); // CAD 資料多半 z 向上；必須在建立 OrbitControls 之前設定
perspCamera.position.set(4, -3, 2.5);
const orthoCamera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.01, 1000);
orthoCamera.up.set(0, 0, 1);
let camera: THREE.PerspectiveCamera | THREE.OrthographicCamera = perspCamera;
let viewAspect = 1;

const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;

const sky = new THREE.HemisphereLight(0xffffff, 0x444444, 1.5);
sky.position.set(0, 0, 1);
scene.add(sky);
const sun = new THREE.DirectionalLight(0xffffff, 2);
sun.position.set(5, -7, 10);
scene.add(sun);

// --- 光源模式：fixed = 固定（原本）；head = 頭燈，光隨眼睛；three = 頭燈 + 左 + 右 ---
// 跟隨型的燈每幀依相機的 右 / 上 / 前 軸擺在「看向目標點」的方向上。
type LightMode = 'fixed' | 'head' | 'three';
const headLight = new THREE.DirectionalLight(0xffffff, 2.2);
const leftLight = new THREE.DirectionalLight(0xffffff, 1.0);
const rightLight = new THREE.DirectionalLight(0xffffff, 1.0);
for (const l of [headLight, leftLight, rightLight]) {
  scene.add(l);
  scene.add(l.target);
}
let lightMode: LightMode = 'fixed';

function setLightMode(m: LightMode): void {
  lightMode = m;
  sun.visible = m === 'fixed';
  sky.intensity = m === 'fixed' ? 1.5 : 0.9; // 跟隨型時環境光降低，頭燈才有明暗層次
  headLight.visible = m !== 'fixed';
  leftLight.visible = rightLight.visible = m === 'three';
  headLight.intensity = m === 'three' ? 1.6 : 2.2;
}

const _right = new THREE.Vector3();
const _up = new THREE.Vector3();
const _fwd = new THREE.Vector3();
function updateLights(): void {
  if (lightMode === 'fixed') return;
  camera.updateMatrixWorld();
  camera.matrixWorld.extractBasis(_right, _up, _fwd); // _fwd 為相機 +z（指向眼睛後方）
  const d = Math.max(camera.position.distanceTo(controls.target), 1);
  const place = (l: THREE.DirectionalLight, x: number, y: number, z: number): void => {
    l.position.copy(controls.target)
      .addScaledVector(_right, x * d)
      .addScaledVector(_up, y * d)
      .addScaledVector(_fwd, z * d);
    l.target.position.copy(controls.target);
  };
  place(headLight, 0, 0.15, 1); // 略偏上，像頭燈
  place(leftLight, -1, 0.3, 0.6);
  place(rightLight, 1, 0.3, 0.6);
}
// 背景與格線跟著主題（Apple 色票）；系統切換淺 / 深色時重建。
// 格線與座標軸的大小隨已載入模型的 bbox 調整（見 updateWorld）。
let grid: THREE.GridHelper | null = null;
let gridSize = 10;
let gridVisible = false; // 預設關閉，由狀態列「格線」checkbox 控制
const axes = new THREE.AxesHelper(1);
axes.scale.setScalar(1.5);
scene.add(axes);

function buildGrid(): void {
  if (grid) {
    scene.remove(grid);
    grid.geometry.dispose();
    (grid.material as THREE.Material).dispose();
  }
  grid = new THREE.GridHelper(gridSize, 10, resolveCssColor('--systemGray'), resolveCssColor('--systemGray3'));
  grid.rotation.x = Math.PI / 2; // GridHelper 預設在 x-z 平面，轉成 x-y（z 向上）
  grid.visible = gridVisible;
  scene.add(grid);
}

function applySceneTheme(): void {
  scene.background = resolveCssColor('--systemGray6');
  buildGrid();
}

/** 取 1、2、5 × 10^n 中不小於 v 的值，格線間距才會是整齊的數字。 */
function niceCeil(v: number): number {
  const e = Math.pow(10, Math.floor(Math.log10(v)));
  const m = v / e;
  return (m <= 1 ? 1 : m <= 2 ? 2 : m <= 5 ? 5 : 10) * e;
}

/** 依 bbox（含原點）調整格線與座標軸大小；沒有模型時回到預設的 10 / 1.5。 */
function updateWorld(box: THREE.Box3 | null): void {
  let size = 10;
  if (box && !box.isEmpty()) {
    // 格線以原點為中心，要涵蓋 bbox 離原點最遠的位置（x、y 在地面上；z 為高度）
    const r = Math.max(
      Math.abs(box.min.x), Math.abs(box.max.x), Math.abs(box.min.y), Math.abs(box.max.y),
      (box.max.z - box.min.z) / 2,
    );
    if (r > 0) size = niceCeil(r * 2.2);
  }
  axes.scale.setScalar(size * 0.15);
  if (size !== gridSize) {
    gridSize = size;
    buildGrid();
  }
}
applySceneTheme();

// --- .lobj：後端以 binary stream 送來，前端解析後建立 three.js mesh ---
const filesEl = document.getElementById('files') as HTMLUListElement;
const pathEl = document.getElementById('path') as HTMLInputElement;
const jawReEl = document.getElementById('jaw-re') as HTMLInputElement;
const infoEl = document.getElementById('info')!;

// 已載入的模型：key 為相對 --dir 的完整路徑。可同時顯示多個檔案。
interface Loaded {
  mesh: THREE.Mesh;
  /** 依牙位 label 判斷的上 / 下顎（FDI；沒有牙 = undefined，此時要退回檔名判定） */
  labelJaw?: Jaw | 'both';
  /** 載入時用的上 / 下顎（舊 .lobj 才有）；判定改變時據此決定要不要重載 */
  jaw?: 'upper' | 'lower';
  /** 頂點數、三角形數與 label 統計（顯示時才依目前語言組字串） */
  info: { verts: number; tris: number; counts: string; labelKinds: number };
  /** 檔案原本的頂點顏色，color 屬性設回「不設定」時用來還原 */
  orig: Float32Array;
  /** 沒有任何 label（.stl / .obj / .ply，或 labels 全為 -1）；載入時會自動配一個顏色 */
  unlabeled: boolean;
  /** 檔案本身帶顏色（例如有頂點色的 .ply / .obj） */
  fileHasColors: boolean;
}
const loaded = new Map<string, Loaded>();
// 各檔案的屬性（alpha / color）。取消選取後仍保留，再選回來時沿用。
const propsOf = new Map<string, ObjProps>();
const getProps = (f: string): ObjProps => {
  let p = propsOf.get(f);
  if (!p) propsOf.set(f, (p = defaultProps()));
  return p;
};
// 各檔案自己指定的上 / 下顎（只對舊 .lobj 有意義）。沒設 = 依左側 regex 判定（符合任一個 = upper，否則 lower）。
const jawOf = new Map<string, 'upper' | 'lower'>();
const isLegacyLobj = (f: string) => f.toLowerCase().endsWith('.lobj');
/** 需要判定上 / 下顎的檔案：舊 .lobj 與沒有 label 的 .stl / .ply / .obj（之後轉 .lobj2 要用）；.lobj2 本身已是 FDI，不判。 */
const isJawFile = (f: string) => /\.(lobj|stl|ply|obj)$/i.test(f);
/** 異常：判定結果與檔名字面矛盾（判成 lower 但檔名有 upper，或判成 upper 但檔名有 lower）。 */
const jawConflict = (f: string, j: Jaw): boolean => {
  const n = f.slice(f.lastIndexOf('/') + 1).toLowerCase();
  return j === 'upper' ? n.includes('lower') : n.includes('upper');
};
type Jaw = 'upper' | 'lower';
let jawRegexes: RegExp[] = [];
/** 解析左側輸入：以 `;` 分隔（檔名不會有分號）、不分大小寫；空段略過，語法錯誤的段落略過並讓輸入框變紅。 */
function parseJawRegexes(text: string): { res: RegExp[]; bad: string[] } {
  const res: RegExp[] = [], bad: string[] = [];
  for (const seg of text.split(';')) {
    if (!seg) continue;
    try {
      res.push(new RegExp(seg, 'i'));
    } catch {
      bad.push(seg);
    }
  }
  return { res, bad };
}
/** 暫時的判定：檔名（不含資料夾）符合任一個 regex = upper，否則 lower。 */
const regexJaw = (f: string): Jaw => {
  const n = f.slice(f.lastIndexOf('/') + 1);
  return jawRegexes.some((r) => r.test(n)) ? 'upper' : 'lower';
};
const effectiveJaw = (f: string): Jaw => jawOf.get(f) ?? regexJaw(f);
/** FDI labels 裡有哪一顎的牙：只有上 / 只有下 / 兩邊都有（'both'）/ 沒有牙（undefined）。1、2、5、6 象限為上顎，3、4、7、8 為下顎。 */
function jawFromLabels(labels: Int8Array | null): Jaw | 'both' | undefined {
  if (!labels) return undefined;
  let up = false, lo = false;
  for (const [l] of labelCounts(labels)) {
    const q = Math.floor(l / 10), t = l % 10;
    if (l < 11 || l > 85 || t < 1 || t > 8 || q < 1 || q > 8 || (q >= 5 && t > 5)) continue; // 不是牙（牙齦 0、-1、bracket 100 …）
    if (q === 1 || q === 2 || q === 5 || q === 6) up = true;
    else lo = true;
  }
  return up && lo ? 'both' : up ? 'upper' : lo ? 'lower' : undefined;
}
/** 檔案清單每個舊 .lobj 前面的小標記：up / lo；其他格式（label 已是 FDI）留空。 */
function updateJawBadges(): void {
  for (const li of filesEl.querySelectorAll<HTMLLIElement>('li[data-path]')) {
    const f = li.dataset.path!;
    const b = li.querySelector<HTMLElement>('.jaw-badge');
    if (!b) continue;
    let j: Jaw, src: string, bad: boolean, why = '';
    if (isJawFile(f)) {
      j = effectiveJaw(f);
      src = jawOf.has(f) ? t().jawSrcManual : t().jawSrcRegex;
      bad = jawConflict(f, j);
      if (bad) why = t().jawWhyConflict;
    } else if (f.toLowerCase().endsWith('.lobj2') && loaded.has(f)) {
      // .lobj2 要載入後才看得到 label：牙位能判定就用牙位，否則（沒有牙，例如 -1 全滿）退回檔名 regex
      const lj = loaded.get(f)!.labelJaw;
      j = lj && lj !== 'both' ? lj : effectiveJaw(f);
      src = lj && lj !== 'both' ? t().jawSrcLabel : jawOf.has(f) ? t().jawSrcManual : t().jawSrcNoTooth;
      bad = lj === 'both' || jawConflict(f, j);
      if (lj === 'both') why = t().jawWhyBoth;
      else if (bad) why = t().jawWhyConflict;
    } else {
      b.replaceChildren();
      b.title = '';
      continue;
    }
    // 只顯示一個：up / lo（綠）；異常顯示 er（紅）
    const s = document.createElement('span');
    s.textContent = bad ? 'er' : j === 'upper' ? 'up' : 'lo';
    s.className = bad ? 'bad' : 'on';
    b.replaceChildren(s);
    b.title = t().jawBadgeTip(j, src, why);
  }
}

/** 轉 .lobj2 的確認視窗：預設刪舊檔；成功後把選取換成新檔並重新整理檔案清單。 */
function convertDialog(f: string, jaw: 'upper' | 'lower'): void {
  const name = f.slice(f.lastIndexOf('/') + 1);
  const dlg = document.createElement('dialog');
  dlg.className = 'convert-dlg';
  const h = document.createElement('div');
  h.className = 'convert-title';
  h.textContent = t().convTitle(name);
  const p = document.createElement('div');
  p.className = 'hint';
  p.textContent = isLegacyLobj(f) ? t().convDescLobj(jaw) : t().convDescOther;
  const opts = document.createElement('div');
  opts.className = 'convert-opts';
  const radio = (value: string, text: string, checked: boolean) => {
    const l = document.createElement('label');
    const r = document.createElement('input');
    r.type = 'radio';
    r.name = 'del';
    r.value = value;
    r.checked = checked;
    l.append(r, ' ', text);
    return l;
  };
  opts.append(radio('1', t().convDelete, true), radio('0', t().convKeep, false));
  const msg = document.createElement('div');
  msg.className = 'convert-msg';
  const btns = document.createElement('div');
  btns.className = 'convert-btns';
  const cancel = document.createElement('button');
  cancel.type = 'button';
  cancel.textContent = t().cancel;
  const ok = document.createElement('button');
  ok.type = 'button';
  ok.textContent = t().convOk;
  btns.append(cancel, ok);
  dlg.append(h, p, opts, msg, btns);
  document.body.appendChild(dlg);
  const close = () => {
    dlg.close();
    dlg.remove();
  };
  cancel.addEventListener('click', close);
  dlg.addEventListener('cancel', () => dlg.remove());
  ok.addEventListener('click', async () => {
    const del = (dlg.querySelector('input[name=del]:checked') as HTMLInputElement).value === '1';
    ok.disabled = cancel.disabled = true;
    msg.textContent = t().converting;
    try {
      const r = await fetch('/api/lobj-convert', {
        method: 'POST',
        headers: { 'X-Lobj-Requested': '1', 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: f, jaw, delete_old: del }),
      });
      if (!r.ok) throw new Error(await r.text());
      const res = (await r.json()) as { new_path: string; deleted: boolean };
      if (res.deleted) {
        selected.delete(f);
        jawOf.delete(f);
        propsOf.delete(f);
      }
      selected.add(res.new_path);
      lastClicked = res.new_path;
      close();
      await browse(curDir); // 重新整理清單（舊檔消失、新檔出現），並維持選取
      await syncSelection();
      infoEl.textContent = t().convDone(res.new_path, res.deleted);
    } catch (e) {
      msg.textContent = t().failed(e instanceof Error ? e.message : String(e));
      ok.disabled = cancel.disabled = false;
    }
  });
  dlg.showModal();
}

/** 丟掉已載入的檔案，讓下一次 syncSelection 重新載入（jaw 變了才需要）。 */
function reloadFiles(files: Iterable<string>): void {
  for (const f of files) {
    const l = loaded.get(f);
    if (!l) continue;
    disposeMesh(l.mesh);
    loaded.delete(f);
    failed.delete(f);
  }
  if (selected.size) void syncSelection();
}
// 已做過自動配色的檔案：只配一次，之後（含取消選取再選回來）尊重使用者的選擇
const autoColored = new Set<string>();
const failed = new Map<string, string>();
// 目前選取的檔案（可多選，保留選取順序）
const selected = new Set<string>();
let syncToken = 0;

let singleSided = false; // 預設雙面；由狀態列「單面」checkbox 控制
const meshSide = () => (singleSided ? THREE.FrontSide : THREE.DoubleSide);

function buildMesh(data: LobjData): Loaded {
  const geom = new THREE.BufferGeometry();
  geom.setAttribute('position', new THREE.BufferAttribute(data.vertices, 3));
  geom.setIndex(new THREE.BufferAttribute(data.triangles, 1));
  if (data.normals) geom.setAttribute('normal', new THREE.BufferAttribute(data.normals, 3));
  else geom.computeVertexNormals();
  // 一律建立 color attribute（沒有頂點色就用淺灰），這樣 color 屬性才有東西可以混合
  const orig = data.colors ?? new Float32Array(data.vertices.length).fill(0.8);
  geom.setAttribute('color', new THREE.BufferAttribute(orig.slice(), 3));
  geom.computeBoundingSphere();
  const mesh = new THREE.Mesh(
    geom,
    new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.6, side: meshSide() }),
  );
  const lc = labelCounts(data.labels);
  const counts = [...lc].map(([l, n]) => `${l}:${n}`).join(' ');
  const info = { verts: data.vertices.length / 3, tris: data.triangles.length / 3, counts, labelKinds: lc.size };
  const unlabeled = !data.labels || data.labels.every((l) => l < 0);
  return { mesh, info, orig, unlabeled, fileHasColors: data.fileHasColors !== false, labelJaw: jawFromLabels(data.labels) };
}

// --- 右側物件面板：每個已選檔案一張卡片（alpha、color） ---
const panelEl = document.getElementById('panel')!;
const panelBodyEl = document.getElementById('panel-body')!;
const panelToggleEl = document.getElementById('panel-toggle')!;

panelToggleEl.addEventListener('click', () => {
  const collapsed = panelEl.classList.toggle('collapsed');
  panelToggleEl.setAttribute('aria-expanded', String(!collapsed));
});

/** 共同設定：alpha / wire / normal / curv / color 對所有物件一起生效（沒有 label 的模型，color 為「原色」時各自用自動配的顏色）。 */
// 右側設定面板只改「作用中物件」的屬性（getProps(activeFile())），不影響其他物件。
/** 隱藏（h）與單獨（s）的檔案；取消選取後仍保留。 */
const hiddenSet = new Set<string>();
const soloSet = new Set<string>();
const hasSolo = () => [...selected].some((f) => soloSet.has(f));

/** 某檔案實際套用的屬性：共同設定 + 各檔自己的自動色 + solo 對 alpha 的調整（有 solo 時，非 solo 的 alpha 最高 0.5）。 */
function effProps(f: string): ObjProps {
  const own = getProps(f);
  const alpha = hasSolo() && !soloSet.has(f) ? Math.min(own.alpha, 0.5) : own.alpha;
  return { ...own, alpha };
}

function updateProps(f: string): void {
  const l = loaded.get(f);
  if (!l) return;
  applyProps(l.mesh, l.orig, effProps(f));
  l.mesh.visible = !hiddenSet.has(f); // 邊線 / 法向量是 mesh 的子物件，跟著隱藏
}
const updateAll = (): void => {
  for (const f of loaded.keys()) updateProps(f);
};
const renderAllLegends = (): void => {
  for (const el of panelBodyEl.querySelectorAll<HTMLElement>('.curv-legend')) renderLegend(el);
};

function renderPanel(): void {
  panelToggleEl.textContent = t().objects(selected.size);
  panelBodyEl.replaceChildren();
  panelBodyEl.appendChild(buildSettings());
  if (!selected.size) {
    const empty = document.createElement('div');
    empty.className = 'hint';
    empty.textContent = t().noSelection;
    panelBodyEl.appendChild(empty);
    return;
  }
  const head = document.createElement('div');
  head.className = 'obj-head';
  const h = document.createElement('span');
  h.textContent = 'h';
  h.title = t().hideTip;
  const sl = document.createElement('span');
  sl.textContent = 's';
  sl.title = t().soloTip;
  head.append(h, sl);
  panelBodyEl.appendChild(head);
  for (const f of selected) panelBodyEl.appendChild(buildRow(f));
  renderInfo();
}

/** 物件清單的一列：h（hide）、s（solo）核取方塊 + 檔名（點檔名＝設為作用中），。 */
function buildRow(f: string): HTMLElement {
  const card = document.createElement('div');
  card.className = 'card obj-row' + (f === activeFile() ? ' active' : '');

  const top = document.createElement('div');
  top.className = 'obj-top';
  const check = (set: Set<string>, tip: string) => {
    const box = document.createElement('input');
    box.type = 'checkbox';
    box.checked = set.has(f);
    box.title = tip;
    box.addEventListener('change', () => {
      if (box.checked) set.add(f);
      else set.delete(f);
      updateAll(); // solo 會影響其他檔案的 alpha
    });
    return box;
  };
  const title = document.createElement('span');
  title.className = 'card-title';
  title.textContent = f.slice(f.lastIndexOf('/') + 1);
  title.title = f;
  title.addEventListener('click', () => {
    active = f;
    renderPanel();
    flashAabb(f);
  });
  top.append(check(hiddenSet, t().hideTip), check(soloSet, t().soloTip), title);
  card.appendChild(top);

  return card;
}

let aabb: THREE.Box3Helper | null = null;
let aabbTimer = 0;
function clearAabb(): void {
  clearTimeout(aabbTimer);
  if (aabb) {
    scene.remove(aabb);
    aabb.geometry.dispose();
    (aabb.material as THREE.Material).dispose();
    aabb = null;
  }
}
/** 點選物件時，把它的 AABB 外框顯示約 2 秒（再點別的物件或重新點選會先清掉舊的）。 */
function flashAabb(f: string): void {
  clearAabb();
  const l = loaded.get(f);
  if (!l) return;
  aabb = new THREE.Box3Helper(new THREE.Box3().setFromObject(l.mesh), 0xffcc00);
  scene.add(aabb);
  aabbTimer = window.setTimeout(clearAabb, 2000);
}

/** 目前「作用中」的物件（點清單的檔名切換）；沒指定或已取消選取時用第一個。 */
let active = '';
const activeFile = (): string => (selected.has(active) ? active : ([...selected][0] ?? ''));

/** 共同處的物件資訊：作用中物件的檔名、頂點 / 三角形 / labels、jaw、轉 lobj2。 */
function buildDetail(card: HTMLElement): void {
  const f = activeFile();
  if (!f) return;
  const title = document.createElement('div');
  title.className = 'card-title';
  title.textContent = f.slice(f.lastIndexOf('/') + 1);
  title.title = f;
  card.appendChild(title);
  const info = document.createElement('div');
  info.className = 'card-info';
  info.dataset.path = f;
  card.appendChild(info);

  // 要知道上 / 下顎：舊 .lobj 的 label 是 arch_index 需要它才能換成 FDI，.stl / .ply / .obj 則是之後轉 .lobj2 要用；檔名沒規律時在這裡各檔自己指定
  if (isJawFile(f)) {
    const jawRow = document.createElement('label');
    jawRow.className = 'row';
    const jawName = document.createElement('span');
    jawName.textContent = 'jaw';
    const jawSel = document.createElement('select');
    jawSel.className = 'card-jaw';
    jawSel.title = t().cardJawTip;
    for (const [v, text] of [['', t().jawDefault], ['upper', t().jawUpper], ['lower', t().jawLower]] as const) {
      const o = document.createElement('option');
      o.value = v;
      o.textContent = text;
      jawSel.appendChild(o);
    }
    jawSel.value = jawOf.get(f) ?? '';
    jawSel.addEventListener('change', () => {
      if (jawSel.value) jawOf.set(f, jawSel.value as 'upper' | 'lower');
      else jawOf.delete(f);
      if (isLegacyLobj(f)) reloadFiles([f]); // 只有舊 .lobj 的解讀受 jaw 影響；只重載這一個檔案
      renderPanel();
      updateJawBadges();
    });
    jawRow.append(jawName, jawSel);
    const known = effectiveJaw(f);
    {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'card-convert';
      btn.textContent = t().convBtn;
      btn.title = t().convBtnTip(known, isLegacyLobj(f));
      btn.addEventListener('click', () => void convertDialog(f, known));
      jawRow.appendChild(btn);
    }
    card.appendChild(jawRow);
  }

}

function buildSettings(): HTMLElement {
  const af = activeFile();
  const p = af ? getProps(af) : defaultProps(); // 沒有選取時只是個不連動任何物件的空設定
  const updateAll = (): void => (af ? updateProps(af) : undefined); // 只更新作用中物件
  const card = document.createElement('div');
  card.className = 'card settings';
  buildDetail(card);

  const slider = (label: string, value: number, onInput: (v: number) => void) => {
    const row = document.createElement('label');
    row.className = 'row';
    const name = document.createElement('span');
    name.textContent = label;
    const input = document.createElement('input');
    input.type = 'range';
    input.min = '0';
    input.max = '1';
    input.step = '0.01';
    input.value = String(value);
    const num = document.createElement('span');
    num.className = 'num';
    num.textContent = value.toFixed(2);
    input.addEventListener('input', () => {
      const v = Number(input.value);
      num.textContent = v.toFixed(2);
      onInput(v);
    });
    row.append(name, input, num);
    return row;
  };

  card.appendChild(
    slider('alpha', p.alpha, (v) => {
      p.alpha = v;
      updateAll();
    }),
  );

  const wireRow = document.createElement('label');
  wireRow.className = 'row';
  const wireName = document.createElement('span');
  wireName.textContent = 'wire';
  const wireBox = document.createElement('input');
  wireBox.type = 'checkbox';
  wireBox.checked = p.wire;
  wireBox.title = t().wireTip;
  wireBox.addEventListener('change', () => {
    p.wire = wireBox.checked;
    updateAll();
  });
  wireRow.append(wireName, wireBox);

  const normRow = document.createElement('label');
  normRow.className = 'row';
  const normName = document.createElement('span');
  normName.textContent = 'normal';
  const normBox = document.createElement('input');
  normBox.type = 'checkbox';
  normBox.checked = p.normal;
  normBox.title = t().normalTip;
  normBox.addEventListener('change', () => {
    p.normal = normBox.checked;
    updateAll();
  });
  normRow.append(normName, normBox);
  const wnRow = document.createElement('div');
  wnRow.className = 'row wn-row';
  wnRow.append(wireRow, normRow);
  card.appendChild(wnRow);

  // 曲率：勾選後以曲率上色（取代 color），旁邊選模式；下方顯示色階圖例
  const curvRow = document.createElement('label');
  curvRow.className = 'row';
  const curvName = document.createElement('span');
  curvName.textContent = 'curv';
  const curvBox = document.createElement('input');
  curvBox.type = 'checkbox';
  curvBox.checked = p.curv;
  curvBox.title = t().curvTip;
  const curvSel = document.createElement('select');
  curvSel.className = 'curv-mode';
  for (const [v, text, tip] of [
    ['signed', t().curvSigned, t().curvSignedTip],
    ['max', 'max |κ|', t().curvMaxTip],
    ['min', 'min |κ|', t().curvMinTip],
  ] as const) {
    const o = document.createElement('option');
    o.value = v;
    o.textContent = text;
    o.title = tip;
    curvSel.appendChild(o);
  }
  curvSel.value = p.curvMode;
  // 截斷：上界百分位（50–100），各模式分開記；拖動時即時重新上色（排序已快取，只重算顏色）
  const clipRow = document.createElement('label');
  clipRow.className = 'row';
  const clipName = document.createElement('span');
  clipName.textContent = t().clip;
  const clipInput = document.createElement('input');
  clipInput.type = 'range';
  clipInput.min = '50';
  clipInput.max = '100';
  clipInput.step = '0.5';
  const clipNum = document.createElement('span');
  clipNum.className = 'num';
  const showClip = () => {
    const v = p.curvClip[p.curvMode];
    clipInput.value = String(v);
    clipNum.textContent = `p${v}`;
    clipRow.title =
      p.curvMode === 'signed' ? t().clipTipSigned(v) : t().clipTipSeq(Math.min(1, 100 - v), v);
  };
  clipInput.addEventListener('input', () => {
    p.curvClip[p.curvMode] = Number(clipInput.value);
    showClip();
    updateAll();
    renderAllLegends();
  });
  clipInput.addEventListener('dblclick', () => {
    p.curvClip[p.curvMode] = DEFAULT_CLIP[p.curvMode];
    showClip();
    updateAll();
    renderAllLegends();
  });
  clipRow.append(clipName, clipInput, clipNum);

  // 平滑：算曲率前先把法向量平滑幾次，壓掉掃描雜訊；放開滑鼠（change）才重算，拖動中只更新數字
  const smoothRow = document.createElement('label');
  smoothRow.className = 'row';
  smoothRow.title = t().smoothTip;
  const smoothName = document.createElement('span');
  smoothName.textContent = t().smooth;
  const smoothInput = document.createElement('input');
  smoothInput.type = 'range';
  smoothInput.min = '0';
  smoothInput.max = '20';
  smoothInput.step = '1';
  smoothInput.value = String(p.curvSmooth);
  const smoothNum = document.createElement('span');
  smoothNum.className = 'num';
  const showSmooth = (v: number) => (smoothNum.textContent = v ? `×${v}` : t().smoothOff);
  showSmooth(p.curvSmooth);
  smoothInput.addEventListener('input', () => showSmooth(Number(smoothInput.value)));
  smoothInput.addEventListener('change', () => {
    p.curvSmooth = Number(smoothInput.value);
    updateAll();
    renderAllLegends();
  });
  smoothRow.append(smoothName, smoothInput, smoothNum);

  const refreshCurv = () => {
    curvSel.hidden = !p.curv;
    clipRow.hidden = !p.curv;
    for (const el of panelBodyEl.querySelectorAll<HTMLElement>('.curv-legend')) el.hidden = !p.curv;
    smoothRow.hidden = !p.curv;
    showClip();
    curvSel.title = curvSel.selectedOptions[0]?.title ?? '';
    renderAllLegends();
  };
  curvBox.addEventListener('change', () => {
    p.curv = curvBox.checked;
    updateAll();
    refreshCurv();
  });
  curvSel.addEventListener('change', () => {
    p.curvMode = curvSel.value as CurvMode;
    updateAll();
    refreshCurv();
  });
  curvRow.append(curvName, curvBox, curvSel);
  const legend = document.createElement('div');
  legend.className = 'curv-legend';
  legend.dataset.path = activeFile();
  card.append(curvRow, smoothRow, clipRow, legend);
  refreshCurv();

  // color：第一格「原色」= 不設定
  const swatches = document.createElement('div');
  swatches.className = 'swatches';
  const mixRow = slider(t().mix, p.mix, (v) => {
    p.mix = v;
    updateAll();
  });
  const customBtn = document.createElement('button');
  const customInput = document.createElement('input');
  const refresh = () => {
    for (const b of swatches.querySelectorAll<HTMLButtonElement>('button')) {
      if (b === customBtn) continue;
      b.classList.toggle('on', (b.dataset.color || null) === p.color);
    }
    const isCustom = !!p.color && p.color.startsWith('#');
    customBtn.classList.toggle('on', isCustom);
    // 已選自訂色時，圓心顯示該顏色（外圈維持彩虹漸層）
    customBtn.style.setProperty('--custom-dot', isCustom ? p.color! : 'transparent');
    mixRow.hidden = p.color === null;
  };
  const addSwatch = (color: string | null, label: string) => {
    const b = document.createElement('button');
    b.type = 'button';
    b.title = label;
    b.dataset.color = color ?? '';
    if (color) b.style.background = `var(${color})`;
    else b.textContent = t().origShort;
    b.addEventListener('click', () => {
      p.color = color;
      updateAll();
      refresh();
    });
    swatches.appendChild(b);
  };
  addSwatch(null, t().origTip);
  for (const n of PALETTE) addSwatch(`--system${n}`, n);
  // 自訂：開系統的色彩對話框；選到的顏色以 #rrggbb 存
  customBtn.type = 'button';
  customBtn.className = 'custom-color';
  customBtn.title = t().customColorTip;
  customInput.type = 'color';
  customInput.value = p.color?.startsWith('#') ? p.color : '#ff8800';
  customInput.addEventListener('input', () => {
    p.color = customInput.value;
    updateAll();
    refresh();
  });
  customBtn.addEventListener('click', () => customInput.click());
  customBtn.appendChild(customInput);
  swatches.appendChild(customBtn);
  const colorRow = document.createElement('div');
  colorRow.className = 'row';
  const colorName = document.createElement('span');
  colorName.textContent = 'color';
  colorRow.append(colorName, swatches);
  card.append(colorRow, mixRow);
  refresh();
  return card;
}

// 切換淺色 / 深色主題時，色票解析出來的顏色會變，重新套用
matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
  applySceneTheme();
  for (const f of loaded.keys()) updateProps(f);
});

function disposeMesh(mesh: THREE.Mesh): void {
  scene.remove(mesh);
  delete mesh.userData.curvBySmooth; // 曲率快取（純 JS 陣列，交給 GC）
  delete mesh.userData.curvColoring;
  delete mesh.userData.curvKey;
  for (const key of ['wire', 'normals']) {
    const ls = mesh.userData[key] as THREE.LineSegments | undefined;
    if (ls) {
      ls.geometry.dispose();
      (ls.material as THREE.Material).dispose();
    }
  }
  mesh.geometry.dispose();
  (mesh.material as THREE.Material).dispose();
}

/** 相機對準所有已顯示的模型。 */
function fitCamera(): void {
  const box = new THREE.Box3();
  for (const l of loaded.values()) box.expandByObject(l.mesh);
  updateWorld(box);
  if (box.isEmpty()) return;
  const sphere = box.getBoundingSphere(new THREE.Sphere());
  const radius = Math.max(sphere.radius, 1e-6);
  controls.target.copy(sphere.center);
  for (const c of [perspCamera, orthoCamera]) {
    c.near = radius / 100;
    c.far = radius * 100;
  }
  camera.position.copy(sphere.center).add(new THREE.Vector3(0, -radius * 2.6, radius * 0.6));
  camera.zoom = 1;
  updateOrthoFrustum();
  camera.updateProjectionMatrix();
  controls.update();
}

/** 正交相機的可視範圍：與透視相機在目標點距離處的視野等大，切換時畫面大小不跳動。 */
function updateOrthoFrustum(): void {
  const d = Math.max(camera.position.distanceTo(controls.target), 1e-6);
  const halfH = d * Math.tan(THREE.MathUtils.degToRad(perspCamera.fov / 2));
  orthoCamera.left = -halfH * viewAspect;
  orthoCamera.right = halfH * viewAspect;
  orthoCamera.top = halfH;
  orthoCamera.bottom = -halfH;
}

function setOrthographic(on: boolean): void {
  const from = camera;
  const to = on ? orthoCamera : perspCamera;
  if (from === to) return;
  const dir = new THREE.Vector3().subVectors(from.position, controls.target);
  // 正交以 zoom 縮放；換回透視時把 zoom 換算成距離，畫面大小不變
  const dist = on ? dir.length() : dir.length() / Math.max(from.zoom, 1e-6);
  to.position.copy(controls.target).addScaledVector(dir.normalize(), dist);
  to.quaternion.copy(from.quaternion);
  to.near = from.near;
  to.far = from.far;
  to.zoom = 1;
  camera = to;
  controls.object = to as THREE.PerspectiveCamera; // OrbitControls 型別只標 Perspective，實際兩者皆可
  if (on) updateOrthoFrustum();
  to.updateProjectionMatrix();
  controls.update();
}

const fmtK = (v: number) => (Math.abs(v) >= 100 ? v.toFixed(0) : Math.abs(v) >= 1 ? v.toFixed(2) : v.toPrecision(2));

/** 曲率色階圖例：色條 + 兩端（signed 時含中間 0）的 κ 值；κ 單位為 1/模型單位，括號內為對應曲率半徑。 */
function renderLegend(el: HTMLElement): void {
  const f = el.dataset.path!;
  const p = getProps(f);
  const l = loaded.get(f);
  el.replaceChildren();
  if (!p.curv) return;
  const bar = document.createElement('div');
  bar.className = 'bar';
  bar.style.background = gradientCss(p.curvMode === 'signed' ? 'div' : 'seq');
  const ticks = document.createElement('div');
  ticks.className = 'ticks';
  const tick = (t: string) => {
    const s = document.createElement('span');
    s.textContent = t;
    ticks.appendChild(s);
  };
  if (!l) {
    tick(t().legendPending);
  } else {
    const { lo, hi } = curvatureColoring(l.mesh, p.curvMode, p.curvClip[p.curvMode], p.curvSmooth);
    if (p.curvMode === 'signed') {
      tick(`${t().concave} ${fmtK(lo)}`);
      tick('0');
      tick(`${t().convex} +${fmtK(hi)}`);
      el.title = t().legendTipSigned(fmtK(hi), fmtK(1 / hi), p.curvClip.signed);
    } else {
      tick(fmtK(lo));
      tick(fmtK(hi));
      el.title = t().legendTipSeq(Math.min(1, 100 - p.curvClip[p.curvMode]), p.curvClip[p.curvMode]);
    }
  }
  el.append(bar, ticks);
}

/** .lobj2 的卡片資訊多一行 jaw：牙位 label 能判斷就用牙位，沒有牙就依檔名 regex。 */
function lobj2JawNote(f: string, l: Loaded): string {
  if (!f.toLowerCase().endsWith('.lobj2')) return '';
  if (l.labelJaw === 'both') return t().lobj2JawBoth;
  if (l.labelJaw) return t().lobj2JawLabel(l.labelJaw);
  return t().lobj2JawRegex(effectiveJaw(f));
}

/** 更新右側各卡片的資訊（頂點、三角形、labels；載入中 / 失敗）。 */
function renderInfo(): void {
  updateJawBadges(); // .lobj2 的標記要等載入後（看得到 label）才有
  for (const el of panelBodyEl.querySelectorAll<HTMLElement>('.curv-legend')) renderLegend(el);
  for (const el of panelBodyEl.querySelectorAll<HTMLElement>('.card-info')) {
    const f = el.dataset.path!;
    const l = loaded.get(f);
    if (!l) {
      el.textContent = failed.has(f) ? t().loadFailed(failed.get(f)!) : t().loading;
      continue;
    }
    el.textContent = t().meshInfo(l.info.verts, l.info.tris) + lobj2JawNote(f, l);
    if (l.info.counts) {
      // labels 清單很長：預設收合，摘要只顯示有幾種 label（展開狀態在重繪時保留）
      const det = document.createElement('details');
      det.className = 'card-labels';
      det.open = labelsOpen.has(f);
      det.addEventListener('toggle', () => {
        if (det.open) labelsOpen.add(f);
        else labelsOpen.delete(f);
      });
      const sum = document.createElement('summary');
      sum.textContent = t().labelKinds(l.info.labelKinds);
      const body = document.createElement('div');
      body.textContent = l.info.counts;
      det.append(sum, body);
      el.appendChild(det);
    }
  }
}
/** 哪些檔案的卡片 labels 是展開的（預設全部收合）。 */
const labelsOpen = new Set<string>();

/** 讓畫面上的模型與 `selected` 一致：移除取消選取的、同時載入新選取的。 */
async function syncSelection(): Promise<void> {
  const token = ++syncToken;
  for (const [f, l] of [...loaded]) {
    if (!selected.has(f)) {
      disposeMesh(l.mesh);
      loaded.delete(f);
    }
  }
  for (const f of [...failed.keys()]) if (!selected.has(f)) failed.delete(f);
  updateAll(); // solo 只看已選取的檔案，選取變了其他檔的 alpha 要重算
  const todo = [...selected].filter((f) => !loaded.has(f));
  renderPanel();
  if (!todo.length) {
    fitCamera();
    return;
  }
  await Promise.all(
    todo.map(async (f) => {
      try {
        const jawUsed = isLegacyLobj(f) ? effectiveJaw(f) : undefined;
        const built = buildMesh(await withBusy(f, (signal) => fetchLobj(f, jawUsed, signal)));
        built.jaw = jawUsed;
        // 期間選取已變（或已被取消）、或 jaw 已改（由改 jaw 時啟動的 sync 重載）就丟棄，避免殘留
        if (!selected.has(f) || loaded.has(f) || (jawUsed && effectiveJaw(f) !== jawUsed)) {
          built.mesh.geometry.dispose();
          (built.mesh.material as THREE.Material).dispose();
          return;
        }
        loaded.set(f, built);
        // 沒有 label 的模型：自動配一個還沒被其他已載入模型用過的顏色（模型比色票多時才重複）
        if (built.unlabeled && !autoColored.has(f)) {
          autoColored.add(f);
          const p = getProps(f);
          if (p.color === null) {
            p.color = pickAutoColor([...loaded].filter(([k]) => k !== f).map(([k]) => getProps(k).color));
            // 檔案自帶顏色（.ply / .obj 頂點色）→ 只輕輕疊上 0.2 保留原色；沒有顏色 → 0.6（同預設）
            p.mix = built.fileHasColors ? 0.2 : 0.6;
            renderPanel(); // 卡片已先建好，重建才會顯示選中的色票
          }
        }
        updateProps(f);
        failed.delete(f);
        scene.add(built.mesh);
      } catch (e) {
        if (isAbort(e)) {
          // 使用者按了取消：不載入，也取消選取
          selected.delete(f);
          markSelected();
          renderPanel();
          return;
        }
        if (selected.has(f)) failed.set(f, e instanceof Error ? e.message : String(e));
      }
      renderInfo();
    }),
  );
  if (token !== syncToken) return; // 期間選取又變了，由新的 sync 收尾
  fitCamera();
}

// 目前資料夾（相對 --dir，以 / 分隔）
let curDir = '';
let lastClicked = ''; // shift 範圍選取的起點
let curFiles: string[] = []; // 目前資料夾內的檔案完整路徑（依清單順序）

const join = (dir: string, name: string) => (dir ? `${dir.endsWith('/') ? dir : `${dir}/`}${name}` : name);

function addItem(text: string, cls: string, onClick: (e: MouseEvent) => void): HTMLLIElement {
  const li = document.createElement('li');
  li.textContent = text;
  li.className = cls;
  li.addEventListener('click', onClick);
  filesEl.appendChild(li);
  return li;
}

function markSelected(): void {
  for (const li of filesEl.querySelectorAll<HTMLLIElement>('li[data-path]')) {
    li.classList.toggle('sel', selected.has(li.dataset.path!));
  }
}

/** 點檔案：一般點擊／Ctrl/⌘＝加選或取消（保留原本已載入的）；Shift＝從上次點的到這個。雙擊＝只開這個（見 openOnly）。 */
function clickFile(full: string, e: MouseEvent): void {
  if (e.detail > 1) return; // 雙擊的第二下交給 dblclick（openOnly）處理，不要再切換一次
  const additive = e.ctrlKey || e.metaKey;
  if (e.shiftKey && curFiles.includes(lastClicked)) {
    const a = curFiles.indexOf(lastClicked);
    const b = curFiles.indexOf(full);
    if (!additive) selected.clear();
    for (const f of curFiles.slice(Math.min(a, b), Math.max(a, b) + 1)) selected.add(f);
  } else {
    if (!selected.delete(full)) selected.add(full);
    lastClicked = full;
  }
  markSelected();
  void syncSelection();
}

/** 雙擊檔案：清掉其他已選的，只開這一個（舊版的一般點擊行為）。 */
function openOnly(full: string): void {
  selected.clear();
  selected.add(full);
  lastClicked = full;
  markSelected();
  void syncSelection();
}

// ---- 「處理中」提示：慢的請求（例如第一次進入大 zip 需解壓）顯示名稱、秒數與取消鈕 ----
const busyEl = document.getElementById('busy')!;
const busyTextEl = document.getElementById('busy-text')!;
const busyJobs = new Map<number, { label: string; start: number; ctl: AbortController }>();
let busySeq = 0;
let busyTimer = 0;
const BUSY_DELAY_MS = 400; // 很快完成的請求不要閃一下提示

const isAbort = (e: unknown) => e instanceof DOMException && e.name === 'AbortError';

function renderBusy(): void {
  const now = Date.now();
  const shown = [...busyJobs.values()].filter((j) => now - j.start >= BUSY_DELAY_MS);
  busyEl.hidden = shown.length === 0;
  if (!shown.length) return;
  const first = shown[0];
  const secs = Math.floor((now - first.start) / 1000);
  const more = shown.length > 1 ? t().busyMore(shown.length) : first.label;
  busyTextEl.textContent = t().busyText(more, secs);
  busyTextEl.title = t().busyTip(first.label);
}

/** 執行一個可取消的請求；超過一小段時間才顯示「處理中」提示。 */
async function withBusy<T>(path: string, run: (signal: AbortSignal) => Promise<T>, ctl = new AbortController()): Promise<T> {
  const id = ++busySeq;
  busyJobs.set(id, { label: path.split('/').pop() || path || '/', start: Date.now(), ctl });
  if (!busyTimer) busyTimer = window.setInterval(renderBusy, 250);
  try {
    return await run(ctl.signal);
  } finally {
    busyJobs.delete(id);
    if (!busyJobs.size) {
      clearInterval(busyTimer);
      busyTimer = 0;
    }
    renderBusy();
  }
}

initPicker({
  selected,
  applySelection: () => {
    markSelected();
    void syncSelection();
  },
  openOnly,
  browse,
  getCurDir: () => curDir,
  withBusy,
  isAbort,
});

document.getElementById('busy-cancel')!.addEventListener('click', () => {
  for (const j of busyJobs.values()) j.ctl.abort();
});

let browseCtl: AbortController | null = null;

/** 進入資料夾（或貼上的路徑；若是檔案則進入它所在的資料夾並開啟它）。 */
async function browse(path: string): Promise<void> {
  browseCtl?.abort(); // 新的瀏覽取代還沒完成的舊瀏覽
  const mine = (browseCtl = new AbortController());
  try {
    const l = await withBusy(path, (signal) => listLobj(path, signal), mine);
    curDir = l.path;
    recordRecent(curDir);
    pathEl.value = l.path ? (l.path.endsWith('/') ? l.path : `${l.path}/`) : '';
    filesEl.replaceChildren();
    if (l.parent) {
      const up = l.up ?? (curDir.includes('/') ? curDir.slice(0, curDir.lastIndexOf('/')) : '');
      addItem('📁 ..', 'dir', () => void browse(up));
    }
    const zips = new Set(l.zips ?? []);
    for (const d of l.dirs) {
      // 資料夾 / zip：單擊進入（壓縮、同步、重新命名等整理操作在 /organize）
      const full = join(curDir, d);
      addItem(`${zips.has(d) ? '📦' : '📁'} ${d}`, 'dir', () => void browse(full));
    }
    curFiles = l.files.map((f) => join(curDir, f.name));
    for (const f of l.files) {
      const full = join(curDir, f.name);
      const li = addItem(`${f.name} (${(f.size / 1048576).toFixed(1)} MB)`, '', (e) => clickFile(full, e));
      li.dataset.path = full;
      li.addEventListener('dblclick', () => openOnly(full));
      const badge = document.createElement('span');
      badge.className = 'jaw-badge';
      li.prepend(badge);
    }
    updateJawBadges();
    if (l.selected) {
      // 貼上的是檔案路徑：只選它
      const full = join(curDir, l.selected);
      selected.clear();
      selected.add(full);
      lastClicked = full;
      markSelected();
      await syncSelection();
    } else {
      markSelected(); // 其他資料夾已選的檔案仍保留顯示
      if (!l.dirs.length && !l.files.length && !l.parent) {
        infoEl.textContent = t().emptyDir;
      }
    }
  } catch (e) {
    if (isAbort(e)) {
      if (browseCtl === mine) infoEl.textContent = t().browseCanceled;
      return;
    }
    infoEl.textContent = t().cannotOpenPath(e instanceof Error ? e.message : String(e));
  }
}

pathEl.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') void browse(pathEl.value);
});
function onJawRegexChange(): void {
  const { res, bad } = parseJawRegexes(jawReEl.value);
  jawRegexes = res;
  jawReEl.classList.toggle('bad', bad.length > 0);
  jawReBad = bad;
  setJawReTitle();
  updateJawBadges();
  // 已各自指定的檔案不受影響；其餘舊 .lobj 只重載判定結果真的變了的
  reloadFiles([...loaded].filter(([f, l]) => isLegacyLobj(f) && l.jaw !== effectiveJaw(f)).map(([f]) => f));
  renderPanel(); // 卡片上「轉 lobj2」的 jaw 與提示要跟著更新
}
let jawReBad: string[] = [];
function setJawReTitle(): void {
  jawReEl.title = (jawReBad.length ? t().jawReBad(jawReBad) : '') + t().jawReTip;
}
jawReEl.addEventListener('input', onJawRegexChange);
onJawRegexChange();

/** 後端通知「有人用 `lobj-view <檔案>` 開檔」：根資料夾已換，舊的選取與模型都作廢，改顯示新的。 */
async function openFromServer(path: string): Promise<void> {
  selected.clear();
  lastClicked = '';
  for (const l of loaded.values()) disposeMesh(l.mesh);
  loaded.clear();
  failed.clear();
  await browse(path);
  await syncSelection(); // 開的是資料夾時 browse 不會觸發，確保畫面清空
}

// 連上時後端會先送目前狀態：seq=0 表示沒人指定過，照一般流程瀏覽根目錄
let initialized = false;
let lastOpenSeq = -1;
const events = new EventSource('/api/events');
events.onmessage = (ev) => {
  const { seq, path } = JSON.parse(ev.data) as { seq: number; path: string };
  if (seq === lastOpenSeq) return; // 斷線重連後同一個事件不重複處理
  lastOpenSeq = seq;
  initialized = true;
  if (seq === 0) void browse('');
  else void openFromServer(path);
};
// 事件通道連不上（例如被代理擋掉）時，仍照舊瀏覽根目錄
setTimeout(() => {
  if (!initialized) {
    initialized = true;
    void browse('');
  }
}, 2000);

// 依容器實際大小調整；分頁隱藏或尺寸為 0 時不繪製（WebGPU 不允許 0 大小的 texture）
let hasSize = false;
new ResizeObserver(() => {
  const w = container.clientWidth;
  const h = container.clientHeight;
  hasSize = w > 0 && h > 0;
  if (!hasSize) return;
  viewAspect = w / h;
  perspCamera.aspect = viewAspect;
  perspCamera.updateProjectionMatrix();
  updateOrthoFrustum();
  orthoCamera.updateProjectionMatrix();
  renderer.setSize(w, h);
}).observe(container);

renderer.setAnimationLoop(() => {
  if (!hasSize) return;
  controls.update();
  updateLights();
  renderer.render(scene, camera);
});

// --- 格線 checkbox（預設關閉，記住選擇）---
const gridEl = document.getElementById('grid-toggle') as HTMLInputElement;
const GRID_KEY = 'lobj-view-grid';
try {
  gridEl.checked = localStorage.getItem(GRID_KEY) === '1';
} catch {
  /* localStorage 不可用就用預設 */
}
gridVisible = gridEl.checked;
buildGrid();
gridEl.addEventListener('change', () => {
  gridVisible = gridEl.checked;
  buildGrid();
  try {
    localStorage.setItem(GRID_KEY, gridEl.checked ? '1' : '0');
  } catch {
    /* 忽略 */
  }
});

// --- 正交投影 checkbox（預設關閉，不記住）---
const orthoEl = document.getElementById('ortho-toggle') as HTMLInputElement;
orthoEl.checked = false;
orthoEl.addEventListener('change', () => setOrthographic(orthoEl.checked));

// --- 單面 checkbox（預設關閉＝雙面；只繪正面，背面不畫）---
const singleEl = document.getElementById('single-toggle') as HTMLInputElement;
singleEl.checked = false;
singleEl.addEventListener('change', () => {
  singleSided = singleEl.checked;
  for (const l of loaded.values()) {
    const m = l.mesh.material as THREE.Material;
    m.side = meshSide();
    m.needsUpdate = true;
  }
});

// --- 光源選單（記住選擇）---
const lightEl = document.getElementById('light') as HTMLSelectElement;
const LIGHT_KEY = 'lobj-view-light';
try {
  const saved = localStorage.getItem(LIGHT_KEY) ?? '';
  if ([...lightEl.options].some((o) => o.value === saved)) lightEl.value = saved;
} catch {
  /* localStorage 不可用就用預設 */
}
setLightMode(lightEl.value as LightMode);
lightEl.addEventListener('change', () => {
  setLightMode(lightEl.value as LightMode);
  try {
    localStorage.setItem(LIGHT_KEY, lightEl.value);
  } catch {
    /* 忽略 */
  }
});

// --- 後端連線檢查 ---
let healthErr: unknown = null; // 連不上時記住原因，切換語言時重組訊息
fetch('/api/health')
  .then((r) => (r.ok ? r.text() : Promise.reject(r.status)))
  .then((text) => {
    healthEl.textContent = text;
    healthEl.className = 'ok';
  })
  .catch((e) => {
    healthErr = e;
    healthEl.textContent = t().healthErr(e);
    healthEl.className = 'err';
  });

// --- 伺服器狀態列：幾人幾個分頁、CPU、記憶體、GPU（後端每 3 秒取樣並快取，這裡約每 5 秒讀一次）---
interface ServerStats {
  people: number;
  tabs: number;
  ready: boolean;
  cpu: number;
  cores: number;
  mem_used: number;
  mem_total: number;
  proc_cpu: number;
  proc_mem: number;
  gpu: { name: string; util: number; mem_used_mb: number; mem_total_mb: number } | null;
}

const status3El = document.getElementById('status3')!;
const gb = (b: number) => (b / 1073741824).toFixed(1);

function statStat(text: string, level: 'ok' | 'warn' | 'err', title: string): HTMLSpanElement {
  const s = document.createElement('span');
  s.textContent = text;
  s.className = level;
  s.title = title;
  return s;
}

const level = (pct: number): 'ok' | 'warn' | 'err' => (pct >= 90 ? 'err' : pct >= 70 ? 'warn' : 'ok');

let lastStats: ServerStats | null = null;
function renderStats(s: ServerStats): void {
  lastStats = s;
  const parts: HTMLElement[] = [
    statStat(t().people(s.people, s.tabs), 'ok', t().peopleTip),
  ];
  if (s.ready) {
    const memPct = s.mem_total ? (s.mem_used / s.mem_total) * 100 : 0;
    parts.push(
      statStat(`CPU ${s.cpu}%`, level(s.cpu), t().cpuTip(s.cores, s.proc_cpu)),
      statStat(t().mem(gb(s.mem_used), gb(s.mem_total)), level(memPct), t().memTip((s.proc_mem / 1048576).toFixed(0))),
    );
    if (s.gpu) {
      parts.push(
        statStat(
          `GPU ${s.gpu.util}%`,
          level(s.gpu.util),
          t().gpuTip(s.gpu.name, s.gpu.mem_used_mb, s.gpu.mem_total_mb),
        ),
      );
    }
  } else {
    parts.push(statStat(t().sampling, 'ok', ''));
  }
  const nodes: (Node | string)[] = [t().serverPrefix];
  parts.forEach((p, i) => nodes.push(...(i ? [' · ', p] : [p])));
  status3El.replaceChildren(...nodes);
}

async function pollStats(): Promise<void> {
  let next = 5000;
  if (!document.hidden) {
    try {
      const r = await fetch('/api/stats');
      if (r.ok) {
        const s = (await r.json()) as ServerStats;
        renderStats(s);
        if (!s.ready) next = 1500; // 後端剛開始取樣，很快就有資料
      }
    } catch {
      /* 伺服器暫時連不上：下次再試 */
    }
  }
  setTimeout(() => void pollStats(), next);
}
void pollStats();

// --- 左側：版本號、版本紀錄 dialog、選擇檔案面板收合 ---
const verEl = document.getElementById('ver')!;
const versionsEl = document.getElementById('versions')!;
const versionsBodyEl = document.getElementById('versions-body')!;
const browserEl = document.getElementById('browser')!;
const browserToggleEl = document.getElementById('browser-toggle')!;

browserToggleEl.addEventListener('click', () => {
  const collapsed = browserEl.classList.toggle('collapsed');
  browserToggleEl.setAttribute('aria-expanded', String(!collapsed));
});

fetch('/api/version')
  .then((r) => (r.ok ? r.text() : Promise.reject(r.status)))
  .then((t) => (verEl.textContent = `v${t}`))
  .catch(() => (verEl.textContent = 'versions'));

/** Versions.md 的簡易呈現：`### 版本` 為標題，`- ` 為項目，其餘為段落。 */
const detailEl = document.getElementById('versions-detail') as HTMLInputElement;
const DETAIL_KEY = 'lobj-view-versions-detail';
let versionsMd = '';
try {
  detailEl.checked = localStorage.getItem(DETAIL_KEY) === '1';
} catch {
  /* 忽略 */
}

/** 概要版本：第 2 碼為奇數且第 3 碼為 1（1.1.1、1.3.1、1.5.1…）。 */
function isReleaseVersion(v: string): boolean {
  const m = /^(\d+)\.(\d+)\.(\d+)/.exec(v);
  return !!m && Number(m[2]) % 2 === 1 && Number(m[3]) === 1;
}

function renderVersions(md: string): void {
  versionsBodyEl.replaceChildren();
  let ul: HTMLUListElement | null = null;
  let show = true; // 第一個版本標題之前的說明一律顯示
  for (const line of md.split(/\r?\n/)) {
    const hd = /^#{2,}\s*(.*)$/.exec(line);
    if (hd) show = detailEl.checked || isReleaseVersion(hd[1]);
    if (!show) continue;
    if (line.startsWith('- ')) {
      if (!ul) versionsBodyEl.appendChild((ul = document.createElement('ul')));
      const li = document.createElement('li');
      li.textContent = line.slice(2);
      ul.appendChild(li);
      continue;
    }
    ul = null;
    const h = /^#+\s*(.*)$/.exec(line);
    if (h) {
      const el = document.createElement('h3');
      el.textContent = h[1];
      versionsBodyEl.appendChild(el);
    } else if (line.trim()) {
      const p = document.createElement('p');
      p.textContent = line;
      versionsBodyEl.appendChild(p);
    }
  }
}

verEl.addEventListener('click', async () => {
  versionsEl.hidden = false;
  try {
    const r = await fetch('/api/versions');
    if (!r.ok) throw new Error(String(r.status));
    versionsMd = await r.text();
    renderVersions(versionsMd);
  } catch (e) {
    versionsBodyEl.textContent = t().versionsErr(e instanceof Error ? e.message : String(e));
  }
});
detailEl.addEventListener('change', () => {
  try {
    localStorage.setItem(DETAIL_KEY, detailEl.checked ? '1' : '0');
  } catch {
    /* 忽略 */
  }
  if (versionsMd) renderVersions(versionsMd);
});
document.getElementById('versions-close')!.addEventListener('click', () => (versionsEl.hidden = true));
window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') versionsEl.hidden = true;
});

// --- 線上更新（GitHub Release）---
interface UpdateInfo {
  current: string;
  latest: string;
  newer: boolean;
  url: string;
  canApply: boolean;
  reason: string | null;
}
const updateBtn = document.getElementById('update-btn') as HTMLButtonElement;
const updateEl = document.getElementById('update')!;
const updateBodyEl = document.getElementById('update-body')!;
let updating = false;

async function checkUpdate(force: boolean): Promise<UpdateInfo> {
  const r = await fetch(`/api/update${force ? '?force=1' : ''}`);
  if (!r.ok) throw new Error((await r.text()) || String(r.status));
  return (await r.json()) as UpdateInfo;
}

let lastUpdate: UpdateInfo | null = null;
function showUpdateBadge(u: UpdateInfo): void {
  lastUpdate = u;
  updateBtn.classList.toggle('has-update', u.newer);
  updateBtn.textContent = u.newer ? t().hasUpdate(u.latest) : t().checkUpdate;
}

function renderUpdate(u: UpdateInfo | null, msg = ''): void {
  const p = (text: string, cls?: string) => {
    const e = document.createElement('p');
    e.textContent = text;
    if (cls) e.className = cls;
    return e;
  };
  updateBodyEl.replaceChildren();
  if (u) {
    updateBodyEl.append(p(t().updCurrent(u.current, u.latest)));
    if (u.url) {
      const a = document.createElement('a');
      a.href = u.url;
      a.target = '_blank';
      a.rel = 'noopener';
      a.textContent = t().updViewOnGithub;
      const wrap = p('');
      wrap.append(a);
      updateBodyEl.append(wrap);
    }
    if (!u.newer) updateBodyEl.append(p(t().updLatest));
    else if (!u.canApply) updateBodyEl.append(p(u.reason ?? t().updCannot, 'msg'));
    else {
      const b = document.createElement('button');
      b.type = 'button';
      b.textContent = t().updApply(u.latest);
      b.disabled = updating;
      b.addEventListener('click', () => void applyUpdate(u.latest));
      updateBodyEl.append(
        b,
        p(t().updApplyHint, 'hint'),
      );
    }
  }
  if (msg) updateBodyEl.append(p(msg, 'msg'));
}

async function applyUpdate(version: string): Promise<void> {
  updating = true;
  renderUpdate(null, t().updDownloading(version));
  try {
    const r = await fetch('/api/update', { method: 'POST', headers: { 'X-Lobj-Requested': '1' } });
    if (!r.ok) throw new Error((await r.text()) || String(r.status));
  } catch (e) {
    updating = false;
    renderUpdate(null, t().updFailed(e instanceof Error ? e.message : String(e)));
    return;
  }
  renderUpdate(null, t().updRestarting(version));
  // 伺服器重新啟動期間連線會失敗；等到回報新版本就重新整理
  const start = Date.now();
  const poll = async (): Promise<void> => {
    try {
      const r = await fetch('/api/version', { cache: 'no-store' });
      if (r.ok && (await r.text()).trim() === version) {
        location.reload();
        return;
      }
    } catch {
      /* 重新啟動中 */
    }
    if (Date.now() - start > 60_000) {
      renderUpdate(null, t().updTimeout);
      return;
    }
    setTimeout(() => void poll(), 1000);
  };
  setTimeout(() => void poll(), 1000);
}

updateBtn.addEventListener('click', async () => {
  updateEl.hidden = false;
  if (updating) return;
  renderUpdate(null, t().updChecking);
  try {
    const u = await checkUpdate(true);
    showUpdateBadge(u);
    renderUpdate(u);
  } catch (e) {
    renderUpdate(null, t().updCheckErr(e instanceof Error ? e.message : String(e)));
  }
});
document.getElementById('update-close')!.addEventListener('click', () => (updateEl.hidden = true));
window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && !updating) updateEl.hidden = true;
});
// 開頁時安靜地查一次（後端快取 10 分鐘），有新版才讓按鈕變色
checkUpdate(false)
  .then(showUpdateBadge)
  .catch(() => {
    /* 離線或 GitHub 限流就維持「檢查更新」 */
  });

// --- 預設開啟程式（副檔名關聯）---
interface AssocExt {
  ext: string;
  registered: boolean;
  isDefault: boolean;
  current: string | null;
  hasBackup: boolean;
  previous: string | null;
}
interface AssocStatus {
  supported: boolean;
  exe: string;
  exts: AssocExt[];
  notes?: string[];
}
const assocEl = document.getElementById('assoc')!;
const assocBodyEl = document.getElementById('assoc-body')!;
/** 預設全勾：註冊只會讓 lobj-view 出現在「開啟檔案」清單，已有預設程式的副檔名不會被搶走。 */
const assocChecked = new Set(['lobj', 'lobj2', 'stl', 'ply', 'obj']);
let assocMsg = '';

/** 沒帶 X-Lobj-Requested 的 POST 後端一律拒絕（擋其他網站的 CSRF）。 */
async function assocCall(url: string, body?: unknown): Promise<Response> {
  return fetch(url, {
    method: 'POST',
    headers: { 'X-Lobj-Requested': '1', 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
}

function assocState(e: AssocExt): string {
  let s: string;
  if (e.isDefault) s = t().assocDefault;
  else if (e.registered) s = e.current ? t().assocRegisteredCur(e.current) : t().assocRegistered;
  else s = e.current ? t().assocUnregCur(e.current) : t().assocUnreg;
  // 註冊前已記錄原狀：取消註冊時會還原
  if (e.hasBackup) s += t().assocRestore(e.previous);
  return s;
}

let lastAssoc: AssocStatus | null = null;
function renderAssoc(st: AssocStatus): void {
  lastAssoc = st;
  assocBodyEl.replaceChildren();
  const el = <K extends keyof HTMLElementTagNameMap>(tag: K, text = '', cls = ''): HTMLElementTagNameMap[K] => {
    const n = document.createElement(tag);
    n.textContent = text;
    if (cls) n.className = cls;
    return n;
  };
  if (!st.supported) {
    assocBodyEl.appendChild(el('p', t().assocUnsupported));
    return;
  }
  const table = el('table');
  for (const e of st.exts) {
    const tr = el('tr');
    const td1 = el('td');
    const label = el('label');
    const cb = el('input');
    cb.type = 'checkbox';
    cb.checked = assocChecked.has(e.ext);
    cb.addEventListener('change', () => (cb.checked ? assocChecked.add(e.ext) : assocChecked.delete(e.ext)));
    label.append(cb, ` .${e.ext}`);
    td1.appendChild(label);
    tr.append(td1, el('td', assocState(e)));
    table.appendChild(tr);
  }
  const reg = el('button', t().assocRegister);
  reg.addEventListener('click', () => void assocAction('register'));
  const unreg = el('button', t().assocUnregister);
  unreg.addEventListener('click', () => void assocAction('unregister'));
  const sys = el('button', t().assocSysSettings);
  sys.addEventListener('click', () => void assocCall('/api/assoc/settings'));
  assocBodyEl.append(
    table,
    reg,
    unreg,
    sys,
    el('p', assocMsg, 'msg'),
    el('p', t().assocHint, 'hint'),
    el('p', t().assocExe(st.exe), 'hint'),
  );
}

async function loadAssoc(): Promise<void> {
  try {
    const r = await fetch('/api/assoc');
    if (!r.ok) throw new Error(await r.text());
    renderAssoc((await r.json()) as AssocStatus);
  } catch (e) {
    assocBodyEl.textContent = t().assocErr(e instanceof Error ? e.message : String(e));
  }
}

async function assocAction(action: 'register' | 'unregister'): Promise<void> {
  const exts = [...assocChecked];
  if (!exts.length) {
    assocMsg = t().assocNeedOne;
  } else {
    const r = await assocCall('/api/assoc', { action, exts });
    if (r.ok) {
      const st = (await r.json()) as AssocStatus;
      assocMsg = [action === 'register' ? t().assocDoneReg : t().assocDoneUnreg, ...(st.notes ?? [])].join(t().sentenceSep);
      renderAssoc(st);
      return;
    }
    assocMsg = t().failed(await r.text());
  }
  void loadAssoc();
}

document.getElementById('assoc-open')!.addEventListener('click', () => {
  assocMsg = '';
  assocEl.hidden = false;
  void loadAssoc();
});
document.getElementById('assoc-close')!.addEventListener('click', () => (assocEl.hidden = true));
window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') assocEl.hidden = true;
});

// --- 主題：空字串 = 跟隨系統（不設 data-theme），其餘為 apple-color-ios.css 的四種 ---
const themeEl = document.getElementById('theme') as HTMLSelectElement;
const THEME_KEY = 'lobj-view-theme';

function setTheme(t: string): void {
  if (t) document.documentElement.dataset.theme = t;
  else delete document.documentElement.dataset.theme;
  applySceneTheme();
  for (const f of loaded.keys()) updateProps(f);
}

try {
  const saved = localStorage.getItem(THEME_KEY) ?? '';
  if ([...themeEl.options].some((o) => o.value === saved)) themeEl.value = saved;
} catch {
  /* localStorage 不可用就用預設 */
}
setTheme(themeEl.value);
themeEl.addEventListener('change', () => {
  setTheme(themeEl.value);
  try {
    localStorage.setItem(THEME_KEY, themeEl.value);
  } catch {
    /* 忽略 */
  }
});

// 左側 HUD 寬度：拖曳右緣調整（存在 localStorage），雙擊還原預設；內容會自動換行
(() => {
  const hud = document.getElementById('hud')!;
  const handle = document.getElementById('hud-resize')!;
  const KEY = 'lobj-view.hud-w';
  const apply = (w: number | null) => {
    if (w === null) hud.style.removeProperty('--hud-w');
    else hud.style.setProperty('--hud-w', `${Math.round(w)}px`);
  };
  try {
    const saved = Number(localStorage.getItem(KEY));
    if (saved >= 200) apply(saved);
  } catch {
    /* 無法使用 localStorage 就用預設 */
  }
  handle.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    handle.setPointerCapture(e.pointerId);
    handle.classList.add('dragging');
    const left = hud.getBoundingClientRect().left;
    const move = (ev: PointerEvent) => apply(Math.min(Math.max(ev.clientX - left, 200), window.innerWidth - 16));
    const up = () => {
      handle.classList.remove('dragging');
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', up);
      handle.removeEventListener('pointercancel', up);
      try {
        localStorage.setItem(KEY, String(Math.round(hud.getBoundingClientRect().width)));
      } catch {
        /* 忽略 */
      }
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', up);
    handle.addEventListener('pointercancel', up);
  });
  handle.addEventListener('dblclick', () => {
    apply(null);
    try {
      localStorage.removeItem(KEY);
    } catch {
      /* 忽略 */
    }
  });
})();

// --- 語言：按鈕永遠寫「lang」，點開列出各語言（以該語言自己的名稱顯示），選擇會記住 ---
(() => {
  const btn = document.getElementById('lang-btn')!;
  const menu = document.getElementById('lang-menu')!;
  const close = () => {
    menu.hidden = true;
    btn.setAttribute('aria-expanded', 'false');
  };
  const build = () => {
    menu.replaceChildren(
      ...LANGS.map(({ code, name }) => {
        const b = document.createElement('button');
        b.type = 'button';
        b.setAttribute('role', 'menuitemradio');
        b.setAttribute('aria-checked', String(code === getLang()));
        b.textContent = name;
        b.addEventListener('click', () => {
          close();
          setLang(code);
        });
        return b;
      }),
    );
  };
  btn.addEventListener('click', (e) => {
    e.stopPropagation();
    if (!menu.hidden) return close();
    build();
    const r = btn.getBoundingClientRect();
    menu.style.left = `${r.left}px`;
    menu.style.top = `${r.bottom + 2}px`;
    menu.hidden = false;
    btn.setAttribute('aria-expanded', 'true');
  });
  document.addEventListener('click', (e) => {
    if (!menu.hidden && !menu.contains(e.target as Node)) close();
  });
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') close();
  });
})();

// 切換語言：applyStatic 已換掉 HTML 上的字，這裡重繪 JS 產生的部分（已顯示過的一次性訊息維持原語言）
onLangChange(() => {
  renderBackend();
  if (healthErr !== null) healthEl.textContent = t().healthErr(healthErr);
  setJawReTitle();
  renderPanel(); // 卡片整張重建（屬性存在 propsOf，不受影響），內含 renderInfo / updateJawBadges
  updateJawBadges();
  if (lastStats) renderStats(lastStats);
  renderBusy();
  if (lastUpdate) showUpdateBadge(lastUpdate); // 沒查過時 applyStatic 已換成「檢查更新」
  if (!updateEl.hidden && !updating && lastUpdate) renderUpdate(lastUpdate);
  if (!assocEl.hidden && lastAssoc) renderAssoc(lastAssoc);
});
renderPanel(); // 一開始就依目前語言顯示「物件 (0)」與空面板提示
