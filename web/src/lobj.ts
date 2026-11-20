// .lobj 二進位格式解析（對應 Rust `lobj::format`，兩邊版面必須一致）。
//
// little-endian，沒有 magic / 版本號：
//   i32 × 7        counts = [nV, nNormal, nColor, nLabel, nTex2d, nTri, nTexId]
//   f32 × nV*3     vertices   必要
//   f32 × nNormal*3 normals   可選，若有則 == nV
//   f32 × nColor*3 colors     可選，若有則 == nV
//   i8  × nLabel   labels     可選，若有則 == nV
//   f32 × nTex2d*2 tex2d      可選 (u, v)
//   u32 × nTri*3   triangles  必要，0 起算
//   u32 × nTexId*3 texId      可選，若有則 == nTri
//
// 後端 /api/lobj/{name} 送來的 labels 一律是 FDI，且沒有 colors 時後端會補上，
// 所以前端不需要處理 arch_index 或調色盤。

export interface LobjData {
  vertices: Float32Array; // 長度 nV*3
  normals: Float32Array | null;
  colors: Float32Array | null;
  labels: Int8Array | null;
  /** 檔案本身是否帶顏色（後端補的不算；由 X-Lobj-File-Colors 標頭得知，解析 bytes 時預設 true） */
  fileHasColors?: boolean;
  tex2d: Float32Array | null;
  triangles: Uint32Array; // 長度 nTri*3
  texId: Uint32Array | null;
}

const HEADER_BYTES = 7 * 4;
// 每個欄位「一筆」的 bytes 數，順序同 header
const ITEM_BYTES = [12, 12, 12, 1, 8, 12, 12];
const FIELD_NAMES = ['vertices', 'normals', 'colors', 'labels', 'tex2d', 'triangles', 'texId'];

export function parseLobj(buf: ArrayBuffer): LobjData {
  if (buf.byteLength < HEADER_BYTES) {
    throw new Error(`資料太短：需要 ${HEADER_BYTES} bytes，實際 ${buf.byteLength}`);
  }
  const view = new DataView(buf);
  const counts: number[] = [];
  let expected = HEADER_BYTES;
  for (let i = 0; i < 7; i++) {
    const n = view.getInt32(i * 4, true);
    if (n < 0) throw new Error(`${FIELD_NAMES[i]} 的數量是負數`);
    counts.push(n);
    expected += n * ITEM_BYTES[i];
  }
  if (buf.byteLength !== expected) {
    throw new Error(`檔案長度應為 ${expected} bytes，實際 ${buf.byteLength}`);
  }
  const [nV, nNormal, nColor, nLabel, nTex2d, nTri, nTexId] = counts;
  if (nV === 0 || nTri === 0) throw new Error('頂點數或三角形數為 0');
  if (nNormal && nNormal !== nV) throw new Error('normals 數量與頂點數不符');
  if (nColor && nColor !== nV) throw new Error('colors 數量與頂點數不符');
  if (nLabel && nLabel !== nV) throw new Error('labels 數量與頂點數不符');
  if (nTexId && nTexId !== nTri) throw new Error('texId 數量與三角形數不符');

  // 各段起點未必 4-byte 對齊（labels 的長度可以是任意值），所以用 slice 複製出來再建立 typed array。
  let pos = HEADER_BYTES;
  const take = (bytes: number) => {
    const part = buf.slice(pos, pos + bytes);
    pos += bytes;
    return part;
  };
  const vertices = new Float32Array(take(nV * 12));
  const normals = nNormal ? new Float32Array(take(nNormal * 12)) : null;
  const colors = nColor ? new Float32Array(take(nColor * 12)) : null;
  const labels = nLabel ? new Int8Array(take(nLabel)) : null;
  const tex2d = nTex2d ? new Float32Array(take(nTex2d * 8)) : null;
  const triangles = new Uint32Array(take(nTri * 12));
  const texId = nTexId ? new Uint32Array(take(nTexId * 12)) : null;

  for (let i = 0; i < triangles.length; i++) {
    if (triangles[i] >= nV) throw new Error(`三角形引用頂點 ${triangles[i]}，但只有 ${nV} 個頂點`);
  }
  return { vertices, normals, colors, labels, tex2d, triangles, texId };
}

/** 各 label 出現次數（由小到大）。 */
export function labelCounts(labels: Int8Array | null): Map<number, number> {
  const m = new Map<number, number>();
  if (labels) for (const l of labels) m.set(l, (m.get(l) ?? 0) + 1);
  return new Map([...m].sort((a, b) => a[0] - b[0]));
}

export interface LobjFileInfo {
  name: string;
  size: number;
}

export interface LobjListing {
  /** 目前資料夾，相對 --dir，以 / 分隔；根目錄為空字串 */
  path: string;
  /** 是否有上一層 */
  parent: boolean;
  /** 「..」要去的路徑（本機模式下可超出 --dir，為絕對路徑）；沒有上一層為 null */
  up?: string | null;
  dirs: string[];
  /** dirs 之中其實是 .zip 檔的名稱（後端偷偷解壓，看起來像資料夾） */
  zips: string[];
  files: LobjFileInfo[];
  /** 輸入的路徑若指到檔案，這裡是該檔名 */
  selected: string | null;
}

/** 列出資料夾；path 可用 / 或 \，也可以直接指到檔案。不可超出後端的 --dir。 */
export async function listLobj(path = '', signal?: AbortSignal): Promise<LobjListing> {
  const r = await fetch(`/api/lobj?path=${encodeURIComponent(path)}`, { signal });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

/** 從後端取一個檔案（binary stream）。`.lobj`（舊格式）可用 jaw 指定上 / 下顎。 */
export async function fetchLobj(name: string, jaw?: 'upper' | 'lower', signal?: AbortSignal): Promise<LobjData> {
  const q = jaw ? `?jaw=${jaw}` : '';
  const r = await fetch(`/api/lobj/${name.split("/").map(encodeURIComponent).join("/")}${q}`, { signal });
  if (!r.ok) throw new Error(await r.text());
  const data = parseLobj(await r.arrayBuffer());
  data.fileHasColors = r.headers.get('X-Lobj-File-Colors') !== '0';
  return data;
}
