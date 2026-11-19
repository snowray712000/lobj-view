// 物件屬性（右側面板）：alpha 與 color。
// 顏色一律用 CSS 變數名稱（Apple 色票，例如 --systemBlue）存，實際色值在套用時才由目前主題解析，
// 所以切換淺色 / 深色主題後重新套用，3D 模型的色調也會跟著主題走。

import * as THREE from 'three/webgpu';

export interface ObjProps {
  /** 0–1，預設 1（不透明） */
  alpha: number;
  /** CSS 變數名稱（如 `--systemBlue`）；null = 不設定，維持檔案原本的顏色（牙齒標記色 / 頂點色） */
  color: string | null;
  /** 0–1，設了 color 之後與原色的混合比例：1 = 完全換成該色，越小越保留原本標記色 */
  mix: number;
  /** 是否顯示 mesh 的邊線（黑線） */
  wire: boolean;
}

export const defaultProps = (): ObjProps => ({ alpha: 1, color: null, mix: 0.6, wire: false });

/** 可選的顏色：Apple system colors（--system<名稱>） */
export const PALETTE = ['Red', 'Orange', 'Yellow', 'Green', 'Teal', 'Blue', 'Indigo', 'Purple', 'Pink', 'Gray'] as const;

/** 自動配色用：排除 Gray（跟「不確定」的灰色太像，分不出來） */
export const AUTO_PALETTE = PALETTE.filter((n) => n !== 'Gray').map((n) => `--system${n}`);

/**
 * 為沒有 label 的新模型挑一個顏色：優先挑 `used`（目前畫面上各模型已用的顏色）裡沒出現過的；
 * 同時載入的模型比色票還多時，顏色才重複，這時挑被用最少次的（平均分攤）。
 */
export function pickAutoColor(used: Iterable<string | null>): string {
  const cnt = new Map<string, number>(AUTO_PALETTE.map((c) => [c, 0]));
  for (const c of used) if (c && cnt.has(c)) cnt.set(c, cnt.get(c)! + 1);
  let best = AUTO_PALETTE[0];
  for (const c of AUTO_PALETTE) if (cnt.get(c)! < cnt.get(best)!) best = c; // 嚴格小於 → 同分取色票順序在前者
  return best;
}

let probe: HTMLElement | null = null;

/** 把 CSS 變數（隨目前主題）解析成 three.js 的顏色。 */
export function resolveCssColor(varName: string): THREE.Color {
  if (!probe) {
    probe = document.createElement('span');
    probe.style.display = 'none';
    document.body.appendChild(probe);
  }
  probe.style.color = `var(${varName})`;
  return new THREE.Color().setStyle(getComputedStyle(probe).color);
}

/**
 * 把屬性套到 mesh。`orig` 是檔案原本的頂點顏色（長度同頂點數*3）；
 * 沒設 color 時還原成 orig，設了則依 `mix` 混合。
 */
export function applyProps(mesh: THREE.Mesh, orig: Float32Array, p: ObjProps): void {
  const mat = mesh.material as THREE.MeshStandardMaterial;
  const transparent = p.alpha < 1;
  if (mat.transparent !== transparent) {
    mat.transparent = transparent;
    mat.needsUpdate = true;
  }
  mat.opacity = p.alpha;
  mat.depthWrite = !transparent; // 半透明時不寫深度，才看得到後面的模型

  // 邊線：第一次勾選才建立（所有三角形的邊，黑色），掛在 mesh 底下跟著動；alpha 一併套用
  let wire = mesh.userData.wire as THREE.LineSegments | undefined;
  if (p.wire && !wire) {
    wire = new THREE.LineSegments(
      new THREE.WireframeGeometry(mesh.geometry),
      new THREE.LineBasicMaterial({ color: 0x000000 }),
    );
    mesh.userData.wire = wire;
    mesh.add(wire);
  }
  if (wire) {
    wire.visible = p.wire;
    const lm = wire.material as THREE.LineBasicMaterial;
    lm.transparent = transparent;
    lm.opacity = p.alpha;
    lm.depthWrite = !transparent;
  }
  // 面稍微往後推，邊線才不會被同位置的面蓋住（z-fighting）
  if (mat.polygonOffset !== p.wire) {
    mat.polygonOffset = p.wire;
    mat.polygonOffsetFactor = 1;
    mat.polygonOffsetUnits = 1;
    mat.needsUpdate = true;
  }

  const attr = mesh.geometry.getAttribute('color') as THREE.BufferAttribute;
  const arr = attr.array as Float32Array;
  if (!p.color) {
    arr.set(orig);
  } else {
    const t = resolveCssColor(p.color);
    const k = p.mix;
    for (let i = 0; i < arr.length; i += 3) {
      arr[i] = orig[i] * (1 - k) + t.r * k;
      arr[i + 1] = orig[i + 1] * (1 - k) + t.g * k;
      arr[i + 2] = orig[i + 2] * (1 - k) + t.b * k;
    }
  }
  attr.needsUpdate = true;
}
