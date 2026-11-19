// 頂點曲率：Normal Variation（沿 edge），移植自 AppLabelTooth/curvature/compute_vertex_curvature_normal_variation_edge.py
//
//   κ_edge = (Δn · e) / |e|²      e = v₂ - v₁、Δn = n₂ - n₁（單位法向量差）
//
// 這是邊方向上的「法曲率」離散估計：半徑 R 的球（法向量朝外）每條邊都得到 +1/R。
// 法向量朝外時：凸（牙尖、牙脊）為正，凹（窩溝、牙齦線）為負。
// - max / min：與 Python 版相同，取 |κ_edge| 在每個頂點所有相鄰邊上的最大 / 最小值
// - signed：每個頂點相鄰邊 κ_edge（帶正負號）的平均，近似平均曲率，用來分凸凹
// 單位是「1 / 模型長度單位」（牙齒模型通常是 mm，所以是 1/mm；曲率半徑 = 1/κ）。
// 這裡不像 Python demo 先除以標準差做尺度正規化：顯示時用百分位截斷後的範圍上色，尺度不影響顏色，
// 保留原始單位讓圖例上的數字有物理意義。

export interface Curvature {
  max: Float32Array;
  min: Float32Array;
  signed: Float32Array;
  /** 已排序的值（max / min 原值、signed 取 |κ|），第一次查百分位才建立，之後拖滑桿只需 O(1) 查表 */
  sorted?: Partial<Record<CurvMode, Float32Array>>;
}

/**
 * `pos`：頂點座標（N*3）；`nor`：頂點法向量（N*3，內部會再正規化）；`tri`：三角形索引（M*3）。
 * 每個三角形的三條邊都走一次，內部邊會被兩個三角形各走一次：max / min 不受影響；
 * signed 的平均對內部頂點而言每條邊權重都一樣（都是 2 次），只有邊界頂點略有差異，可忽略。
 */
export function computeCurvature(pos: ArrayLike<number>, nor: ArrayLike<number>, tri: ArrayLike<number>): Curvature {
  const n = Math.floor(pos.length / 3);
  const un = new Float32Array(n * 3);
  for (let i = 0; i < n; i++) {
    const x = nor[i * 3], y = nor[i * 3 + 1], z = nor[i * 3 + 2];
    const l = Math.hypot(x, y, z) || 1;
    un[i * 3] = x / l;
    un[i * 3 + 1] = y / l;
    un[i * 3 + 2] = z / l;
  }
  const max = new Float32Array(n);
  const min = new Float32Array(n).fill(Infinity);
  const sum = new Float64Array(n);
  const cnt = new Uint32Array(n);
  const edge = (a: number, b: number) => {
    const ex = pos[b * 3] - pos[a * 3], ey = pos[b * 3 + 1] - pos[a * 3 + 1], ez = pos[b * 3 + 2] - pos[a * 3 + 2];
    const dx = un[b * 3] - un[a * 3], dy = un[b * 3 + 1] - un[a * 3 + 1], dz = un[b * 3 + 2] - un[a * 3 + 2];
    const k = (dx * ex + dy * ey + dz * ez) / (ex * ex + ey * ey + ez * ez + 1e-12);
    const ak = Math.abs(k);
    if (ak > max[a]) max[a] = ak;
    if (ak > max[b]) max[b] = ak;
    if (ak < min[a]) min[a] = ak;
    if (ak < min[b]) min[b] = ak;
    sum[a] += k;
    sum[b] += k;
    cnt[a]++;
    cnt[b]++;
  };
  for (let t = 0; t + 2 < tri.length; t += 3) {
    const a = tri[t], b = tri[t + 1], c = tri[t + 2];
    edge(a, b);
    edge(b, c);
    edge(c, a);
  }
  const signed = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    if (min[i] === Infinity) min[i] = 0; // 孤立點（沒有相鄰邊）
    signed[i] = cnt[i] ? sum[i] / cnt[i] : 0;
  }
  return { max, min, signed };
}

/**
 * 法向量平滑：每次把每個頂點的法向量換成「自己 + 相鄰頂點」的平均再正規化，頂點位置不動。
 * 曲率 κ_edge = Δn·e/|e|² 的雜訊主要來自法向量，直接平滑法向量最有效；
 * 試過先 Laplacian / Taubin 平滑頂點位置，頂點會沿切向漂移、三角形變細長，|e| 變小反而讓極值更大。
 * 鄰居沿三角形的邊累加（內部邊被走兩次，每個鄰居權重相同）。球面上平滑 10 次 κ 仍為 1/R。
 */
export function smoothNormals(nor: ArrayLike<number>, tri: ArrayLike<number>, iters: number): Float32Array {
  let cur = Float32Array.from(nor);
  const n = Math.floor(cur.length / 3);
  for (let k = 0; k < iters; k++) {
    const acc = Float64Array.from(cur);
    const add = (a: number, b: number) => {
      for (let c = 0; c < 3; c++) {
        acc[a * 3 + c] += cur[b * 3 + c];
        acc[b * 3 + c] += cur[a * 3 + c];
      }
    };
    for (let t = 0; t + 2 < tri.length; t += 3) {
      add(tri[t], tri[t + 1]);
      add(tri[t + 1], tri[t + 2]);
      add(tri[t + 2], tri[t]);
    }
    const next = new Float32Array(cur.length);
    for (let i = 0; i < n; i++) {
      const l = Math.hypot(acc[i * 3], acc[i * 3 + 1], acc[i * 3 + 2]) || 1;
      next[i * 3] = acc[i * 3] / l;
      next[i * 3 + 1] = acc[i * 3 + 1] / l;
      next[i * 3 + 2] = acc[i * 3 + 2] / l;
    }
    cur = next;
  }
  return cur;
}

/** 已排序陣列的第 p 百分位（0–100，最近秩）。 */
export function percentileSorted(s: Float32Array, p: number): number {
  if (!s.length) return 0;
  return s[Math.min(s.length - 1, Math.max(0, Math.round((p / 100) * (s.length - 1))))];
}

/** 第 p 百分位（0–100，最近秩）。 */
export function percentile(v: Float32Array, p: number): number {
  return percentileSorted(Float32Array.from(v).sort(), p);
}

function sortedOf(c: Curvature, mode: CurvMode): Float32Array {
  c.sorted ??= {};
  return (c.sorted[mode] ??= (mode === 'signed' ? c.signed.map(Math.abs) : Float32Array.from(c[mode])).sort());
}

// viridis（matplotlib，與 Python color_curvature 相同色表）取 9 個控制點線性內插
const VIRIDIS = [
  [0.267, 0.005, 0.329], [0.283, 0.141, 0.458], [0.254, 0.265, 0.53], [0.207, 0.372, 0.553], [0.164, 0.471, 0.558],
  [0.128, 0.567, 0.551], [0.135, 0.659, 0.518], [0.267, 0.749, 0.441], [0.478, 0.821, 0.318], [0.741, 0.873, 0.15],
  [0.993, 0.906, 0.144],
];
// 發散色：藍（凹）– 淺灰（平）– 紅（凸）
const DIVERGING = [
  [0.13, 0.4, 0.85], [0.55, 0.72, 0.95], [0.92, 0.92, 0.92], [0.97, 0.62, 0.5], [0.85, 0.15, 0.12],
];

function lut(table: number[][], t: number, out: Float32Array, o: number): void {
  const x = Math.min(1, Math.max(0, t)) * (table.length - 1);
  const i = Math.min(table.length - 2, Math.floor(x));
  const f = x - i;
  for (let c = 0; c < 3; c++) out[o + c] = table[i][c] * (1 - f) + table[i + 1][c] * f;
}

/** CSS linear-gradient 字串，給圖例用。 */
export function gradientCss(kind: 'seq' | 'div'): string {
  const table = kind === 'seq' ? VIRIDIS : DIVERGING;
  const stops = table.map((c, i) => `rgb(${c.map((v) => Math.round(v * 255)).join(',')}) ${((i / (table.length - 1)) * 100).toFixed(0)}%`);
  return `linear-gradient(to right, ${stops.join(', ')})`;
}

export type CurvMode = 'signed' | 'max' | 'min';

export interface CurvColoring {
  colors: Float32Array;
  lo: number;
  hi: number;
}

/** 截斷上界百分位的預設值（物件卡片的「截斷」滑桿） */
export const DEFAULT_CLIP: Record<CurvMode, number> = { signed: 90, max: 95, min: 95 };

/**
 * 依模式上色，`clip` 為截斷上界百分位（50–100）。
 * - max / min：截斷到 [min(1, 100-clip)%, clip%] 後 min-max 正規化、viridis。
 *   clip = 99.5 即 Python clip_percentile 的 0.5%–99.5%；但口掃 mesh 的 κ 分佈尾巴很長（雜訊、破面），
 *   上界取 99.5% 時幾乎整個模型都擠在最暗的那一端，所以預設 95（下界 1%）
 * - signed：以 |κ| 的 clip 百分位為對稱範圍 [-a, a]，0 對到中間的灰色；預設 90
 */
export function colorCurvature(c: Curvature, mode: CurvMode, clip: number): CurvColoring {
  const v = c[mode];
  const s = sortedOf(c, mode);
  const out = new Float32Array(v.length * 3);
  if (mode === 'signed') {
    const a = percentileSorted(s, clip) || 1e-12;
    for (let i = 0; i < v.length; i++) lut(DIVERGING, (v[i] / a + 1) / 2, out, i * 3);
    return { colors: out, lo: -a, hi: a };
  }
  const lo = percentileSorted(s, Math.min(1, 100 - clip));
  const hi = percentileSorted(s, clip);
  const d = hi - lo + 1e-12;
  for (let i = 0; i < v.length; i++) lut(VIRIDIS, (v[i] - lo) / d, out, i * 3);
  return { colors: out, lo, hi };
}
