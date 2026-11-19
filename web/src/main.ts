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

let current: THREE.Mesh | null = null;

function showLobj(data: LobjData): void {
  if (current) {
    scene.remove(current);
    current.geometry.dispose();
    (current.material as THREE.Material).dispose();
  }
  const geom = new THREE.BufferGeometry();
  geom.setAttribute('position', new THREE.BufferAttribute(data.vertices, 3));
  geom.setIndex(new THREE.BufferAttribute(data.triangles, 1));
  if (data.normals) geom.setAttribute('normal', new THREE.BufferAttribute(data.normals, 3));
  else geom.computeVertexNormals();
  if (data.colors) geom.setAttribute('color', new THREE.BufferAttribute(data.colors, 3));
  geom.computeBoundingSphere();

  current = new THREE.Mesh(
    geom,
    new THREE.MeshStandardMaterial({ vertexColors: !!data.colors, roughness: 0.6, side: THREE.DoubleSide }),
  );
  scene.add(current);

  // 相機對準物體
  const { center, radius } = geom.boundingSphere!;
  controls.target.copy(center);
  camera.near = radius / 100;
  camera.far = radius * 100;
  camera.position.copy(center).add(new THREE.Vector3(0, radius * 0.6, radius * 2.6));
  camera.updateProjectionMatrix();
  controls.update();

  const counts = [...labelCounts(data.labels)].map(([l, n]) => `${l}:${n}`).join(' ');
  infoEl.textContent = `頂點 ${data.vertices.length / 3}、三角形 ${data.triangles.length / 3}` + (counts ? `\nlabels ${counts}` : '');
}

async function openFile(name: string): Promise<void> {
  if (!name) return;
  infoEl.textContent = '載入中…';
  try {
    showLobj(await fetchLobj(name, (jawEl.value || undefined) as 'upper' | 'lower' | undefined));
  } catch (e) {
    infoEl.textContent = `載入失敗：${e instanceof Error ? e.message : e}`;
  }
}

// 目前資料夾（相對 --dir，以 / 分隔）與目前選取的檔案（相對 --dir 的完整路徑）
let curDir = '';
let curFile = '';

const join = (dir: string, name: string) => (dir ? `${dir}/${name}` : name);

function addItem(text: string, cls: string, onClick: () => void): HTMLLIElement {
  const li = document.createElement('li');
  li.textContent = text;
  li.className = cls;
  li.addEventListener('click', onClick);
  filesEl.appendChild(li);
  return li;
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
    for (const f of l.files) {
      const full = join(curDir, f.name);
      const li = addItem(`${f.name} (${(f.size / 1048576).toFixed(1)} MB)`, full === curFile ? 'sel' : '', () => {
        curFile = full;
        for (const x of filesEl.children) x.classList.remove('sel');
        li.classList.add('sel');
        void openFile(full);
      });
    }
    if (l.selected) {
      curFile = join(curDir, l.selected);
      await browse(curDir); // 重畫一次以標示選取
      await openFile(curFile);
    } else if (!l.dirs.length && !l.files.length && !l.parent) {
      infoEl.textContent = '--dir 資料夾內沒有 .lobj / .lobj2 / .stl / .obj / .ply';
    }
  } catch (e) {
    infoEl.textContent = `無法開啟路徑：${e instanceof Error ? e.message : e}`;
  }
}

pathEl.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') void browse(pathEl.value);
});
jawEl.addEventListener('change', () => curFile && void openFile(curFile));
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
