import './style.css';
import * as THREE from 'three/webgpu';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

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

// --- 示範：以「頂點 label」上色的 mesh（之後由 .lobj 取代） ---
scene.add(createLabelDemo());

function createLabelDemo(): THREE.Mesh {
  const geom = new THREE.TorusKnotGeometry(0.8, 0.28, 160, 24);
  const pos = geom.getAttribute('position');
  const labels = new Uint32Array(pos.count);
  for (let i = 0; i < pos.count; i++) {
    // 假 label：依 x 座標分成 5 群
    labels[i] = Math.min(4, Math.max(0, Math.floor((pos.getX(i) + 1.2) / 0.5)));
  }
  const colors = new Float32Array(pos.count * 3);
  const c = new THREE.Color();
  for (let i = 0; i < pos.count; i++) {
    c.setHSL(labels[i] / 5, 0.65, 0.55);
    c.toArray(colors, i * 3);
  }
  geom.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  geom.translate(0, 1.2, 0);
  return new THREE.Mesh(geom, new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.6 }));
}

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
