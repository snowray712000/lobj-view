import './style.css';
import * as THREE from 'three/webgpu';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { fetchLobj, labelCounts, listLobj, type LobjData } from './lobj';

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
scene.background = new THREE.Color(0x1e1f22);

const camera = new THREE.PerspectiveCamera(45, 1, 0.01, 1000);
camera.position.set(3, 2.5, 4);

const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;

scene.add(new THREE.HemisphereLight(0xffffff, 0x444444, 1.5));
const sun = new THREE.DirectionalLight(0xffffff, 2);
sun.position.set(5, 10, 7);
scene.add(sun);
scene.add(new THREE.GridHelper(10, 10, 0x555555, 0x333333));
scene.add(new THREE.AxesHelper(1.5));

// --- .lobj：後端以 binary stream 送來，前端解析後建立 three.js mesh ---
const filesEl = document.getElementById('files') as HTMLUListElement;
const pathEl = document.getElementById('path') as HTMLInputElement;
const jawEl = document.getElementById('jaw') as HTMLSelectElement;
const infoEl = document.getElementById('info')!;

// 已載入的模型：key 為相對 --dir 的完整路徑。可同時顯示多個檔案。
interface Loaded {
  mesh: THREE.Mesh;
  info: string;
}
const loaded = new Map<string, Loaded>();
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
  if (data.colors) geom.setAttribute('color', new THREE.BufferAttribute(data.colors, 3));
  geom.computeBoundingSphere();
  const mesh = new THREE.Mesh(
    geom,
    new THREE.MeshStandardMaterial({ vertexColors: !!data.colors, roughness: 0.6, side: THREE.DoubleSide }),
  );
  const counts = [...labelCounts(data.labels)].map(([l, n]) => `${l}:${n}`).join(' ');
  const info = `頂點 ${data.vertices.length / 3}、三角形 ${data.triangles.length / 3}` + (counts ? `\nlabels ${counts}` : '');
  return { mesh, info };
}

function disposeMesh(mesh: THREE.Mesh): void {
  scene.remove(mesh);
  mesh.geometry.dispose();
  (mesh.material as THREE.Material).dispose();
}

/** 相機對準所有已顯示的模型。 */
function fitCamera(): void {
  const box = new THREE.Box3();
  for (const l of loaded.values()) box.expandByObject(l.mesh);
  if (box.isEmpty()) return;
  const sphere = box.getBoundingSphere(new THREE.Sphere());
  const radius = Math.max(sphere.radius, 1e-6);
  controls.target.copy(sphere.center);
  camera.near = radius / 100;
  camera.far = radius * 100;
  camera.position.copy(sphere.center).add(new THREE.Vector3(0, radius * 0.6, radius * 2.6));
  camera.updateProjectionMatrix();
  controls.update();
}

function renderInfo(): void {
  const lines: string[] = [];
  const multi = selected.size > 1;
  if (multi) lines.push(`已選 ${selected.size} 個檔案`);
  for (const f of selected) {
    if (multi) lines.push(`■ ${f}`);
    const l = loaded.get(f);
    if (l) lines.push(l.info);
    else if (failed.has(f)) lines.push(`載入失敗：${failed.get(f)}`);
    else lines.push('載入中…');
  }
  infoEl.textContent = lines.join('\n');
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
  renderInfo();
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
  renderer.render(scene, camera);
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
