import './apple-color-ios.css';
import './style.css';
import * as THREE from 'three/webgpu';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { fetchLobj, labelCounts, listLobj, type LobjData } from './lobj';
import { applyProps, defaultProps, PALETTE, resolveCssColor, type ObjProps } from './props';

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
backendEl.textContent = isWebGPU ? 'WebGPU' : 'WebGL2 (fallback)';
backendEl.className = isWebGPU ? 'ok' : 'warn';
if (!isWebGPU && !window.isSecureContext) {
  backendEl.textContent += ' — 非 secure context，請改用 localhost / ssh -L';
}

// --- scene ---
const scene = new THREE.Scene();

const camera = new THREE.PerspectiveCamera(45, 1, 0.01, 1000);
camera.up.set(0, 0, 1); // CAD 資料多半 z 向上；必須在建立 OrbitControls 之前設定
camera.position.set(4, -3, 2.5);

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
const jawEl = document.getElementById('jaw') as HTMLSelectElement;
const infoEl = document.getElementById('info')!;

// 已載入的模型：key 為相對 --dir 的完整路徑。可同時顯示多個檔案。
interface Loaded {
  mesh: THREE.Mesh;
  info: string;
  /** 檔案原本的頂點顏色，color 屬性設回「不設定」時用來還原 */
  orig: Float32Array;
}
const loaded = new Map<string, Loaded>();
// 各檔案的屬性（alpha / color）。取消選取後仍保留，再選回來時沿用。
const propsOf = new Map<string, ObjProps>();
const getProps = (f: string): ObjProps => {
  let p = propsOf.get(f);
  if (!p) propsOf.set(f, (p = defaultProps()));
  return p;
};
const failed = new Map<string, string>();
// 目前選取的檔案（可多選，保留選取順序）
const selected = new Set<string>();
let syncToken = 0;

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
    new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.6, side: THREE.DoubleSide }),
  );
  const counts = [...labelCounts(data.labels)].map(([l, n]) => `${l}:${n}`).join(' ');
  const info = `頂點 ${data.vertices.length / 3}、三角形 ${data.triangles.length / 3}` + (counts ? `\nlabels ${counts}` : '');
  return { mesh, info, orig };
}

// --- 右側物件面板：每個已選檔案一張卡片（alpha、color） ---
const panelEl = document.getElementById('panel')!;
const panelBodyEl = document.getElementById('panel-body')!;
const panelToggleEl = document.getElementById('panel-toggle')!;

panelToggleEl.addEventListener('click', () => {
  const collapsed = panelEl.classList.toggle('collapsed');
  panelToggleEl.setAttribute('aria-expanded', String(!collapsed));
});

function updateProps(f: string): void {
  const l = loaded.get(f);
  if (l) applyProps(l.mesh, l.orig, getProps(f));
}

function renderPanel(): void {
  panelToggleEl.textContent = `物件 (${selected.size})`;
  panelBodyEl.replaceChildren();
  if (!selected.size) {
    const empty = document.createElement('div');
    empty.className = 'hint';
    empty.textContent = '尚未選取檔案';
    panelBodyEl.appendChild(empty);
    return;
  }
  for (const f of selected) panelBodyEl.appendChild(buildCard(f));
  renderInfo();
}

function buildCard(f: string): HTMLElement {
  const p = getProps(f);
  const card = document.createElement('div');
  card.className = 'card';

  const title = document.createElement('div');
  title.className = 'card-title';
  title.textContent = f.slice(f.lastIndexOf('/') + 1);
  title.title = f;
  card.appendChild(title);

  const info = document.createElement('div');
  info.className = 'card-info';
  info.dataset.path = f;
  card.appendChild(info);

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
      updateProps(f);
    }),
  );

  // color：第一格「原色」= 不設定
  const swatches = document.createElement('div');
  swatches.className = 'swatches';
  const mixRow = slider('疊加', p.mix, (v) => {
    p.mix = v;
    updateProps(f);
  });
  const refresh = () => {
    for (const b of swatches.querySelectorAll<HTMLButtonElement>('button')) {
      b.classList.toggle('on', (b.dataset.color || null) === p.color);
    }
    mixRow.hidden = p.color === null;
  };
  const addSwatch = (color: string | null, label: string) => {
    const b = document.createElement('button');
    b.type = 'button';
    b.title = label;
    b.dataset.color = color ?? '';
    if (color) b.style.background = `var(${color})`;
    else b.textContent = '原';
    b.addEventListener('click', () => {
      p.color = color;
      updateProps(f);
      refresh();
    });
    swatches.appendChild(b);
  };
  addSwatch(null, '原色（不設定）');
  for (const n of PALETTE) addSwatch(`--system${n}`, n);
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
  camera.near = radius / 100;
  camera.far = radius * 100;
  camera.position.copy(sphere.center).add(new THREE.Vector3(0, -radius * 2.6, radius * 0.6));
  camera.updateProjectionMatrix();
  controls.update();
}

/** 更新右側各卡片的資訊（頂點、三角形、labels；載入中 / 失敗）。 */
function renderInfo(): void {
  for (const el of panelBodyEl.querySelectorAll<HTMLElement>('.card-info')) {
    const f = el.dataset.path!;
    const l = loaded.get(f);
    el.textContent = l ? l.info : failed.has(f) ? `載入失敗：${failed.get(f)}` : '載入中…';
  }
}

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
  const todo = [...selected].filter((f) => !loaded.has(f));
  renderPanel();
  if (!todo.length) {
    fitCamera();
    return;
  }
  const jaw = (jawEl.value || undefined) as 'upper' | 'lower' | undefined;
  await Promise.all(
    todo.map(async (f) => {
      try {
        const built = buildMesh(await fetchLobj(f, jaw));
        // 期間選取已變（或已被取消）就丟棄，避免殘留
        if (!selected.has(f) || loaded.has(f)) {
          built.mesh.geometry.dispose();
          (built.mesh.material as THREE.Material).dispose();
          return;
        }
        loaded.set(f, built);
        applyProps(built.mesh, built.orig, getProps(f));
        failed.delete(f);
        scene.add(built.mesh);
      } catch (e) {
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

const join = (dir: string, name: string) => (dir ? `${dir}/${name}` : name);

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

/** 點檔案：一般點擊＝只選這個；Ctrl/⌘＝加選或取消；Shift＝從上次點的到這個。 */
function clickFile(full: string, e: MouseEvent): void {
  const additive = e.ctrlKey || e.metaKey;
  if (e.shiftKey && curFiles.includes(lastClicked)) {
    const a = curFiles.indexOf(lastClicked);
    const b = curFiles.indexOf(full);
    if (!additive) selected.clear();
    for (const f of curFiles.slice(Math.min(a, b), Math.max(a, b) + 1)) selected.add(f);
  } else if (additive) {
    if (!selected.delete(full)) selected.add(full);
    lastClicked = full;
  } else {
    selected.clear();
    selected.add(full);
    lastClicked = full;
  }
  markSelected();
  void syncSelection();
}

/** 進入資料夾（或貼上的路徑；若是檔案則進入它所在的資料夾並開啟它）。 */
async function browse(path: string): Promise<void> {
  try {
    const l = await listLobj(path);
    curDir = l.path;
    pathEl.value = l.path ? `${l.path}/` : '';
    filesEl.replaceChildren();
    if (l.parent) {
      const up = curDir.includes('/') ? curDir.slice(0, curDir.lastIndexOf('/')) : '';
      addItem('📁 ..', 'dir', () => void browse(up));
    }
    for (const d of l.dirs) addItem(`📁 ${d}`, 'dir', () => void browse(join(curDir, d)));
    curFiles = l.files.map((f) => join(curDir, f.name));
    for (const f of l.files) {
      const full = join(curDir, f.name);
      const li = addItem(`${f.name} (${(f.size / 1048576).toFixed(1)} MB)`, '', (e) => clickFile(full, e));
      li.dataset.path = full;
    }
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
        infoEl.textContent = '--dir 資料夾內沒有 .lobj / .lobj2 / .stl / .obj / .ply';
      }
    }
  } catch (e) {
    infoEl.textContent = `無法開啟路徑：${e instanceof Error ? e.message : e}`;
  }
}

pathEl.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') void browse(pathEl.value);
});
jawEl.addEventListener('change', () => {
  // jaw 會影響舊 .lobj 的解讀，需全部重新載入
  for (const l of loaded.values()) disposeMesh(l.mesh);
  loaded.clear();
  failed.clear();
  if (selected.size) void syncSelection();
});
void browse('');

// 依容器實際大小調整；分頁隱藏或尺寸為 0 時不繪製（WebGPU 不允許 0 大小的 texture）
let hasSize = false;
new ResizeObserver(() => {
  const w = container.clientWidth;
  const h = container.clientHeight;
  hasSize = w > 0 && h > 0;
  if (!hasSize) return;
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
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
fetch('/api/health')
  .then((r) => (r.ok ? r.text() : Promise.reject(r.status)))
  .then((t) => {
    healthEl.textContent = t;
    healthEl.className = 'ok';
  })
  .catch((e) => {
    healthEl.textContent = `無法連線 (${e})`;
    healthEl.className = 'err';
  });

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
    versionsBodyEl.textContent = `無法取得版本紀錄：${e instanceof Error ? e.message : e}`;
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
