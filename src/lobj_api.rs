//! 後端的 lobj API。前後端之間的資料就是 `.lobj` 的二進位格式（binary stream）。
//!
//! - `GET /api/lobj[?path=...]`：列出 `--dir`（或其子資料夾）的內容
//!   （JSON：`{ path, parent, dirs: [name], zips: [name], files: [{ name, size }], selected }`）。
//!   `path` 可用 `/` 或 `\`，可以是相對 `--dir` 的路徑，也可以是 `--dir` 底下的絕對路徑；
//!   若指到檔案，則列出它所在的資料夾並以 `selected` 回報該檔名。
//! - `GET /api/lobj/{*path}[?jaw=upper|lower]`：回傳該檔的 `.lobj` 二進位串流
//!   （`application/octet-stream`，版面見 `lobj::format`），與磁碟檔案的差別：
//!     - labels 一律是 **FDI**（舊 `.lobj` 會在後端先換算，所以前端不需要知道 arch_index）；
//!     - 檔案沒有 colors 時，由後端依 labels 補上（`Palette::Unique`），調色盤只在 Rust 維護一份。
//!
//! - `GET /api/places`：資料夾選擇對話框左側的「常用位置」
//!   （JSON：`[{ kind, name, path }]`；`kind` 為 root / home / desktop / documents / downloads / drive）。
//!   只監聽本機位址時才列出家目錄、桌面、文件、下載與磁碟機；否則只有 `--dir` 本身。
//! - `POST /api/zip-dir`（只限本機、需 `X-Lobj-Requested`）：把資料夾（含子資料夾）壓成與它同層的 `<資料夾名>.zip`，
//!   用來把整個資料夾傳到另一台電腦（見 `zip_dir`）；回應是 NDJSON 進度串流。zip 內的路徑不能壓縮。
//! - `POST /api/lobj-convert`（只限本機、需 `X-Lobj-Requested`）：把舊 `.lobj` 依指定的 jaw 轉存成同資料夾的 `.lobj2`，
//!   新檔沿用舊檔的建立 / 修改 / 存取時間，可選擇刪除舊檔（見 `convert`）。
//!
//! 所有路徑都會 canonicalize 後確認仍在 `--dir` 之內（含 `..`、符號連結），超出一律拒絕。
//! 例外：伺服器只監聽本機位址（localhost）時不限制，可瀏覽整台電腦；`--dir` 之外的路徑以絕對路徑（`/` 分隔）表示。
//!
//! `.zip` 檔視為資料夾（`dirs` 內包含它，並在 `zips` 標出）：第一次進入時由 `zipfs` 解壓到快取，
//! 原 zip 不動；路徑可以穿過 zip（`a.zip/sub/x.stl`），內層 zip 也可以。解壓期間若前端中斷連線
//! （`AbortController`），handler 被丟棄，`CancelOnDrop` 會讓解壓停下來。

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::{
    Router,
    Json,
    extract::{ConnectInfo, Path as UrlPath, Query, State},
    http::{HeaderMap, HeaderName, StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use lobj::colors::{self, Palette};
use lobj::{Jaw, LobjError};
use tokio_stream::{Stream, StreamExt, wrappers::WatchStream};

use crate::instance::Shared;
use crate::zipfs::{self, ZipError};

/// 根資料夾由 `Shared` 持有，執行中可被切換（見 `instance.rs`），所以每個請求都重新讀取。
pub fn router(shared: Arc<Shared>) -> Router {
    Router::new()
        .route("/api/lobj", get(list))
        .route("/api/lobj/{*path}", get(fetch))
        .route("/api/lobj-convert", post(convert))
        .route("/api/zip-dir", post(zip_dir))
        .route("/api/places", get(places))
        .route("/api/events", get(events))
        .route("/api/stats", get(stats))
        .with_state(shared)
}

/// SSE：連上時先送目前的開檔事件，之後每次有人用 `lobj-view <檔案>` 開檔就再送一次。
/// 連線存在即代表「有網頁開著」，此時就不會再自動開新的瀏覽器分頁。
async fn events(
    State(shared): State<Arc<Shared>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let guard = shared.tab_guard(peer.ip());
    let stream = WatchStream::new(shared.subscribe()).map(move |ev| {
        let _keep = &guard;
        Ok(Event::default().data(format!("{{\"seq\":{},\"path\":{}}}", ev.seq, json_string(&ev.path))))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// 左上角狀態列用：使用人數 / 分頁數 + 伺服器資源（取自背景取樣的快取，見 `stats.rs`）。
async fn stats(State(shared): State<Arc<Shared>>) -> Response {
    let (people, tabs) = shared.usage();
    let s = crate::stats::read();
    let gpu = match &s.gpu {
        Some(g) => format!(
            "{{\"name\":{},\"util\":{:.0},\"mem_used_mb\":{:.0},\"mem_total_mb\":{:.0}}}",
            json_string(&g.name),
            g.util,
            g.mem_used_mb,
            g.mem_total_mb
        ),
        None => "null".into(),
    };
    (
        [(header::CONTENT_TYPE, "application/json")],
        format!(
            "{{\"people\":{people},\"tabs\":{tabs},\"ready\":{},\"cpu\":{:.0},\"cores\":{},\"mem_used\":{},\"mem_total\":{},\"proc_cpu\":{:.0},\"proc_mem\":{},\"gpu\":{gpu}}}",
            s.ready, s.cpu, s.cores, s.mem_used, s.mem_total, s.proc_cpu, s.proc_mem
        ),
    )
        .into_response()
}

/// 常用位置：`--dir` 一定有；本機模式再加上使用者資料夾與（Windows）磁碟機。路徑格式同 `list` 的 `path`。
fn places_for(root: &Path, unrestricted: bool) -> Vec<(&'static str, String, String)> {
    let mut v = vec![("root", String::new(), String::new())];
    if !unrestricted {
        return v;
    }
    let known: [(&'static str, Option<PathBuf>); 4] = [
        ("home", dirs::home_dir()),
        ("desktop", dirs::desktop_dir()),
        ("documents", dirs::document_dir()),
        ("downloads", dirs::download_dir()),
    ];
    for (kind, p) in known {
        if let Some(real) = p.and_then(|p| std::fs::canonicalize(p).ok()) {
            if real.is_dir() {
                v.push((kind, String::new(), virt_of(root, &real)));
            }
        }
    }
    #[cfg(windows)]
    for c in b'A'..=b'Z' {
        let drive = format!("{}:/", c as char);
        if Path::new(&drive).exists() {
            v.push(("drive", format!("{}:", c as char), drive));
        }
    }
    v
}

async fn places(State(shared): State<Arc<Shared>>) -> Response {
    let root = shared.root();
    let unrestricted = shared.local_only();
    let list = tokio::task::spawn_blocking(move || places_for(&root, unrestricted)).await.unwrap_or_default();
    let body = list
        .iter()
        .map(|(k, n, p)| format!("{{\"kind\":{},\"name\":{},\"path\":{}}}", json_string(k), json_string(n), json_string(p)))
        .collect::<Vec<_>>()
        .join(",");
    ([(header::CONTENT_TYPE, "application/json")], format!("[{body}]")).into_response()
}

pub fn is_lobj_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".lobj") || lower.ends_with(".lobj2") || lower.ends_with(".stl") || lower.ends_with(".obj") || lower.ends_with(".ply")
}

pub(crate) type ApiErr = (StatusCode, std::borrow::Cow<'static, str>);

fn err(code: StatusCode, m: &'static str) -> ApiErr {
    (code, m.into())
}

/// 前端中斷連線 → handler 被丟棄 → 通知背景的解壓停下來。
struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

fn zip_err(e: ZipError) -> ApiErr {
    match e {
        ZipError::Cancelled => (StatusCode::from_u16(499).unwrap(), "已取消".into()),
        ZipError::Msg(m) => (StatusCode::UNPROCESSABLE_ENTITY, m.into()),
    }
}

/// 解析結果：`real` 是磁碟上的真實路徑（可能在 zip 快取內），`virt` 是前端看到的路徑（`/` 分隔，
/// 相對 `--dir`，穿過 zip 時形如 `a.zip/sub`），`base` 是這個路徑所屬的限制範圍（`--dir` 或某個 zip 的快取）。
#[derive(Debug)]
pub(crate) struct Resolved {
    pub(crate) real: PathBuf,
    pub(crate) virt: String,
    base: PathBuf,
    /// 是否位於 zip 解壓快取內（zip 內的檔案不能轉檔）
    pub(crate) in_zip: bool,
}

/// 前端看到的路徑：在 `root` 之內是相對路徑，否則（只有不限制範圍時才會發生）是絕對路徑，一律 `/` 分隔。
fn virt_of(root: &Path, real: &Path) -> String {
    if real.starts_with(root) {
        return rel_string(root, real);
    }
    let s = real.to_string_lossy().replace('\\', "/");
    if let Some(r) = s.strip_prefix("//?/UNC/") {
        format!("//{r}")
    } else if let Some(r) = s.strip_prefix("//?/") {
        r.to_string()
    } else {
        s
    }
}

/// 前端路徑的上一層（純字串）；已是檔案系統根目錄則為 `None`。相對路徑的最上層回傳 `Some("")`。
fn parent_virt(virt: &str) -> Option<String> {
    if virt.is_empty() {
        return None;
    }
    if Path::new(virt).is_absolute() && Path::new(virt).parent().is_none() {
        return None;
    }
    Some(match virt.trim_end_matches('/').rfind('/') {
        None => String::new(),
        Some(0) => "/".into(),
        Some(i) if virt[..i].ends_with(':') => format!("{}/", &virt[..i]),
        Some(i) => virt[..i].to_string(),
    })
}

fn virt_join(a: &str, rel: &Path) -> String {
    let mut out = a.to_string();
    for c in rel.components() {
        if let std::path::Component::Normal(n) = c {
            if !out.is_empty() {
                out.push('/');
            }
            out.push_str(&n.to_string_lossy());
        }
    }
    out
}

/// 只在字串層面處理 `.` 與 `..`（路徑穿過 zip 時磁碟上並不存在，無法 canonicalize）。
fn lexical_normalize(p: &Path) -> PathBuf {
    use std::path::Component::*;
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            CurDir => {}
            ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// 進入 zip（必要時解壓），`inner` 是 zip 內的相對路徑；`inner` 中若又穿過內層 zip 就繼續往下。
fn descend(mut zip_real: PathBuf, mut zip_virt: String, mut inner: PathBuf, cancel: &AtomicBool) -> Result<Resolved, ApiErr> {
    loop {
        let dir = zipfs::ensure_extracted(&zip_real, cancel).map_err(zip_err)?;
        let base = std::fs::canonicalize(&dir).map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "解壓快取不可用"))?;
        let target = if inner.as_os_str().is_empty() { base.clone() } else { base.join(&inner) };
        if let Ok(real) = std::fs::canonicalize(&target) {
            if !real.starts_with(&base) {
                return Err(err(StatusCode::FORBIDDEN, "路徑不可超出 zip"));
            }
            let virt = virt_join(&zip_virt, &inner);
            return Ok(Resolved { real, virt, base, in_zip: true });
        }
        // 內層 zip？
        let mut next = None;
        for anc in target.ancestors().skip(1) {
            if !anc.starts_with(&base) || anc == base {
                break;
            }
            if anc.file_name().is_some_and(|n| zipfs::is_zip_name(&n.to_string_lossy())) && anc.is_file() {
                next = Some(anc.to_path_buf());
                break;
            }
        }
        let Some(anc) = next else { return Err(err(StatusCode::NOT_FOUND, "zip 內找不到路徑")) };
        let rel = anc.strip_prefix(&base).unwrap_or(&anc).to_path_buf();
        zip_virt = virt_join(&zip_virt, &rel);
        inner = target.strip_prefix(&anc).unwrap_or(Path::new("")).to_path_buf();
        zip_real = std::fs::canonicalize(&anc).map_err(|_| err(StatusCode::NOT_FOUND, "找不到路徑"))?;
    }
}

/// 把使用者輸入的路徑（`/` 或 `\`、相對或絕對）解析成 `root` 之內（或 zip 之內）的真實路徑。
#[cfg(test)]
fn resolve(root: &Path, input: &str, cancel: &AtomicBool) -> Result<Resolved, ApiErr> {
    resolve_ex(root, input, cancel, false)
}

/// `unrestricted`：不限制在 `root` 內（伺服器只監聽本機位址時）。
pub(crate) fn resolve_ex(root: &Path, input: &str, cancel: &AtomicBool, unrestricted: bool) -> Result<Resolved, ApiErr> {
    let s = input.trim().trim_matches('"').replace('\\', "/");
    let rel = s.trim_start_matches('/');
    let mut candidates = Vec::new();
    if Path::new(&s).is_absolute() {
        candidates.push(PathBuf::from(&s));
    }
    candidates.push(root.join(rel));

    let mut outside = false;
    for c in candidates {
        if let Ok(real) = std::fs::canonicalize(&c) {
            if unrestricted {
                let virt = virt_of(root, &real);
                return Ok(Resolved { real, virt, base: PathBuf::new(), in_zip: false });
            }
            if real.starts_with(root) {
                let virt = rel_string(root, &real);
                return Ok(Resolved { real, virt, base: root.to_path_buf(), in_zip: false });
            }
            outside = true;
            continue;
        }
        // 磁碟上不存在：也許路徑穿過了某個 .zip
        let n = lexical_normalize(&c);
        for anc in n.ancestors().skip(1) {
            if !anc.file_name().is_some_and(|f| zipfs::is_zip_name(&f.to_string_lossy())) || !anc.is_file() {
                continue;
            }
            let Ok(zr) = std::fs::canonicalize(anc) else { continue };
            if !unrestricted && !zr.starts_with(root) {
                outside = true;
                break;
            }
            let inner = n.strip_prefix(anc).unwrap_or(Path::new("")).to_path_buf();
            let zv = virt_of(root, &zr);
            return descend(zr, zv, inner, cancel);
        }
    }
    if outside {
        Err(err(StatusCode::FORBIDDEN, "路徑不可超出 --dir"))
    } else {
        Err(err(StatusCode::NOT_FOUND, "找不到路徑"))
    }
}

/// `root` 之下的相對路徑，一律用 `/` 分隔（根目錄為空字串）。
fn rel_string(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .map(|r| {
            r.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_default()
}

#[derive(Debug)]
struct Listing {
    path: String,
    dirs: Vec<String>,
    /// dirs 之中其實是 .zip 檔的名稱
    zips: Vec<String>,
    files: Vec<(String, u64)>,
    selected: Option<String>,
    /// 「..」要去的路徑；`None` 表示沒有上一層
    up: Option<String>,
}

#[cfg(test)]
fn list_dir(root: &Path, input: &str, cancel: &AtomicBool) -> Result<Listing, ApiErr> {
    list_dir_ex(root, input, cancel, false)
}

fn list_dir_ex(root: &Path, input: &str, cancel: &AtomicBool, unrestricted: bool) -> Result<Listing, ApiErr> {
    let mut r = resolve_ex(root, input, cancel, unrestricted)?;
    let fname = r.real.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if r.real.is_file() && zipfs::is_zip_name(&fname) {
        // 指到 .zip 本身 → 當成資料夾進入
        r = descend(r.real.clone(), r.virt.clone(), PathBuf::new(), cancel)?;
    }
    let (dir, dir_virt, selected) = if r.real.is_file() {
        if !is_lobj_name(&fname) {
            return Err(err(StatusCode::BAD_REQUEST, "不是 .lobj / .lobj2 / .stl / .obj / .ply 檔"));
        }
        let pv = parent_virt(&r.virt).unwrap_or_default();
        (r.real.parent().unwrap_or(&r.base).to_path_buf(), pv, Some(fname))
    } else {
        (r.real.clone(), r.virt.clone(), None)
    };
    let io = |_| err(StatusCode::INTERNAL_SERVER_ERROR, "無法列出資料夾");
    let mut dirs = Vec::new();
    let mut zips = Vec::new();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(io)? {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        // 以 canonicalize 後仍在範圍內為準（符號連結指到外面的就不列出）
        let Ok(real) = std::fs::canonicalize(entry.path()) else { continue };
        if !real.starts_with(&r.base) {
            continue;
        }
        if real.is_dir() {
            dirs.push(name);
        } else if zipfs::is_zip_name(&name) {
            zips.push(name.clone());
            dirs.push(name);
        } else if is_lobj_name(&name) {
            files.push((name, real.metadata().map(|m| m.len()).unwrap_or(0)));
        }
    }
    dirs.sort_by_key(|n| n.to_lowercase());
    files.sort_by_key(|(n, _)| n.to_lowercase());
    // 本機模式下，--dir 的最上層也能再往上（到磁碟上的上一層）
    let up = match parent_virt(&dir_virt) {
        Some(u) => Some(u),
        None if unrestricted && !r.in_zip => dir.parent().map(|p| virt_of(root, p)),
        None => None,
    };
    Ok(Listing { path: dir_virt, dirs, zips, files, selected, up })
}

async fn list(State(shared): State<Arc<Shared>>, Query(q): Query<HashMap<String, String>>) -> Response {
    let root = shared.root();
    let unrestricted = shared.local_only();
    let input = q.get("path").cloned().unwrap_or_default();
    // 前端中斷連線時這個 future 會被丟棄，guard 隨之讓背景的解壓停下來
    let cancel = Arc::new(AtomicBool::new(false));
    let _guard = CancelOnDrop(cancel.clone());
    match tokio::task::spawn_blocking(move || list_dir_ex(&root, &input, &cancel, unrestricted)).await {
        Ok(Ok(l)) => {
            let names = |v: &[String]| v.iter().map(|d| json_string(d)).collect::<Vec<_>>().join(",");
            let (dirs, zips) = (names(&l.dirs), names(&l.zips));
            let files = l
                .files
                .iter()
                .map(|(n, s)| format!("{{\"name\":{},\"size\":{s}}}", json_string(n)))
                .collect::<Vec<_>>()
                .join(",");
            let selected = l.selected.as_deref().map(json_string).unwrap_or_else(|| "null".into());
            (
                [(header::CONTENT_TYPE, "application/json")],
                format!(
                    "{{\"path\":{},\"parent\":{},\"up\":{},\"dirs\":[{dirs}],\"zips\":[{zips}],\"files\":[{files}],\"selected\":{selected}}}",
                    json_string(&l.path),
                    l.up.is_some(),
                    l.up.as_deref().map(json_string).unwrap_or_else(|| "null".into())
                ),
            )
                .into_response()
        }
        Ok(Err(e)) => e.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn fetch(
    State(shared): State<Arc<Shared>>,
    UrlPath(name): UrlPath<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    if !is_lobj_name(&name) {
        return (StatusCode::BAD_REQUEST, "檔名不合法").into_response();
    }
    let jaw = match q.get("jaw").map(String::as_str) {
        Some("upper") => Some(Jaw::Upper),
        Some("lower") => Some(Jaw::Lower),
        Some(_) => return (StatusCode::BAD_REQUEST, "jaw 只能是 upper 或 lower").into_response(),
        None => None,
    };
    let root = shared.root();
    let unrestricted = shared.local_only();
    let cancel = Arc::new(AtomicBool::new(false));
    let _guard = CancelOnDrop(cancel.clone());
    // 路徑可能穿過 .zip（需要解壓），所以解析也在背景執行緒做
    let job = move || -> Result<Result<(Vec<u8>, bool), LobjError>, ApiErr> {
        let r = resolve_ex(&root, &name, &cancel, unrestricted)?;
        if !r.real.is_file() {
            return Err(err(StatusCode::BAD_REQUEST, "不是檔案"));
        }
        Ok(encode(&r.real, jaw))
    };
    let result = match tokio::task::spawn_blocking(job).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return e.into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    match result {
        Ok((bytes, file_has_colors)) => (
            [
                (header::CONTENT_TYPE, "application/octet-stream"),
                // 檔案本身是否帶有顏色（後端補的不算），前端據此決定自動配色的預設疊加比例
                (HeaderName::from_static("x-lobj-file-colors"), if file_has_colors { "1" } else { "0" }),
            ],
            bytes,
        )
            .into_response(),
        Err(LobjError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            (StatusCode::NOT_FOUND, "找不到檔案").into_response()
        }
        Err(e) => (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response(),
    }
}

const CONVERT_REMOTE_MSG: &str = "轉檔會寫入並可能刪除伺服器上的檔案，只在伺服器監聽本機位址時開放";

/// 把新檔的時間設成與舊檔相同（Windows 連建立時間一起）。
fn copy_times(from: &std::fs::Metadata, to: &Path) -> std::io::Result<()> {
    let mut t = std::fs::FileTimes::new();
    if let Ok(a) = from.accessed() {
        t = t.set_accessed(a);
    }
    if let Ok(m) = from.modified() {
        t = t.set_modified(m);
    }
    #[cfg(windows)]
    if let Ok(c) = from.created() {
        use std::os::windows::fs::FileTimesExt;
        t = t.set_created(c);
    }
    std::fs::OpenOptions::new().write(true).open(to)?.set_times(t)
}

/// 舊 `.lobj`（或 `.stl` / `.ply` / `.obj`）→ 同資料夾、同主檔名的 `.lobj2`。body：`{ "path": 相對 --dir 的路徑, "jaw": "upper"|"lower"|null, "delete_old": bool }`。
/// 新檔已存在就拒絕（不覆蓋）；寫完會重新讀回比對頂點 / 三角形數，確認無誤才依 `delete_old` 刪舊檔。
async fn convert(State(shared): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    if let Err(e) = crate::assoc::guard_with(&shared, &headers, true, CONVERT_REMOTE_MSG) {
        return e.into_response();
    }
    let Some(path) = body.get("path").and_then(|v| v.as_str()).map(str::to_owned) else {
        return (StatusCode::BAD_REQUEST, "缺少 path").into_response();
    };
    let jaw = match body.get("jaw").and_then(|v| v.as_str()) {
        Some("upper") => Some(Jaw::Upper),
        Some("lower") => Some(Jaw::Lower),
        Some(_) => return (StatusCode::BAD_REQUEST, "jaw 只能是 upper 或 lower").into_response(),
        None => None,
    };
    let delete_old = body.get("delete_old").and_then(|v| v.as_bool()).unwrap_or(false);
    let root = shared.root();
    let unrestricted = shared.local_only();
    let job = move || -> Result<String, (StatusCode, String)> {
        let bad = |c: StatusCode, m: &str| (c, m.to_string());
        let r = resolve_ex(&root, &path, &AtomicBool::new(false), unrestricted).map_err(|(c, m)| (c, m.to_string()))?;
        if r.in_zip {
            return Err(bad(StatusCode::BAD_REQUEST, "zip 內的檔案不能轉檔（請先解壓縮）"));
        }
        let ext = r.real.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        if !r.real.is_file() || !matches!(ext.as_str(), "lobj" | "stl" | "ply" | "obj") {
            return Err(bad(StatusCode::BAD_REQUEST, "只能轉換 .lobj / .stl / .ply / .obj 檔案"));
        }
        // 只換副檔名，不改檔名（.stl / .ply / .obj 沒有 label，轉出的 .lobj2 labels 全是 -1）
        let new = r.real.with_extension("lobj2");
        if new.exists() {
            return Err(bad(StatusCode::CONFLICT, "同名的 .lobj2 已存在，未轉檔"));
        }
        let io = |e: std::io::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        let le = |e: LobjError| (StatusCode::UNPROCESSABLE_ENTITY, e.to_string());
        let meta = std::fs::metadata(&r.real).map_err(io)?; // 先取時間：讀檔會更動存取時間
        let obj = lobj::load(&r.real, jaw).map_err(le)?;
        let (nv, nt) = (obj.vertices.len(), obj.triangles.len());
        let cleanup = |m: (StatusCode, String)| {
            std::fs::remove_file(&new).ok();
            m
        };
        lobj::save(&new, &obj).map_err(|e| cleanup(le(e)))?;
        copy_times(&meta, &new).map_err(|e| cleanup(io(e)))?;
        let back = lobj::load(&new, None).map_err(|e| cleanup(le(e)))?;
        if back.vertices.len() != nv || back.triangles.len() != nt || back.labels != obj.labels {
            return Err(cleanup((StatusCode::INTERNAL_SERVER_ERROR, "轉出的檔案與原檔不一致，已放棄".into())));
        }
        if delete_old {
            std::fs::remove_file(&r.real).map_err(io)?;
        }
        Ok(virt_of(&root, &new))
    };
    match tokio::task::spawn_blocking(job).await {
        Ok(Ok(new_path)) => (
            [(header::CONTENT_TYPE, "application/json")],
            format!("{{\"new_path\":{},\"deleted\":{delete_old}}}", json_string(&new_path)),
        )
            .into_response(),
        Ok(Err((c, m))) => (c, m).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// 讀檔 → labels 轉 FDI → 補 colors → 編成 `.lobj` 二進位。
fn encode(path: &Path, jaw: Option<Jaw>) -> Result<(Vec<u8>, bool), LobjError> {
    let mut obj = lobj::load(path, jaw)?;
    let file_has_colors = obj.colors.is_some();
    if obj.colors.is_none() {
        if let Some(labels) = &obj.labels {
            obj.colors = Some(colors::labels_to_colors(labels, Palette::Unique));
        }
    }
    Ok((lobj::format::to_bytes(&obj)?, file_has_colors))
}

pub(crate) fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn no() -> AtomicBool {
        AtomicBool::new(false)
    }

    #[test]
    fn json_escape() {
        assert_eq!(json_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }

    #[test]
    fn resolve_confined() {
        let base = std::env::temp_dir().join(format!("lobj_view_test_{}", std::process::id()));
        let root = base.join("root");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(base.join("secret.lobj"), b"x").unwrap();
        std::fs::write(root.join("sub").join("a.lobj"), b"x").unwrap();
        let root = std::fs::canonicalize(&root).unwrap();

        assert_eq!(resolve(&root, "", &no()).unwrap().real, root);
        assert!(resolve(&root, "sub\\a.lobj", &no()).unwrap().real.ends_with("a.lobj"));
        assert!(resolve(&root, "sub/../sub", &no()).is_ok());
        assert_eq!(resolve(&root, "..", &no()).unwrap_err().0, StatusCode::FORBIDDEN);
        assert_eq!(resolve(&root, "sub/../../secret.lobj", &no()).unwrap_err().0, StatusCode::FORBIDDEN);
        let abs = root.join("sub").join("a.lobj");
        assert!(resolve(&root, abs.to_str().unwrap(), &no()).is_ok());
        assert!(resolve(&root, base.to_str().unwrap(), &no()).is_err());
        let l = list_dir(&root, "sub/a.lobj", &no()).unwrap();
        assert_eq!((l.path.as_str(), l.selected.as_deref()), ("sub", Some("a.lobj")));
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn unrestricted_local() {
        let base = std::env::temp_dir().join(format!("lobj_view_unres_{}", std::process::id()));
        let root = base.join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(base.join("out.lobj"), b"x").unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        let l = list_dir_ex(&root, "", &no(), true).unwrap();
        assert!(l.up.is_some());
        let up = l.up.unwrap();
        let l = list_dir_ex(&root, &up, &no(), true).unwrap();
        assert!(l.files.iter().any(|f| f.0 == "out.lobj"));
        assert!(l.dirs.contains(&"root".to_string()));
        assert!(resolve_ex(&root, "../out.lobj", &no(), true).is_ok());
        assert!(resolve_ex(&root, "../out.lobj", &no(), false).is_err());
        assert_eq!(parent_virt("D:/a"), Some("D:/".into()));
        assert_eq!(parent_virt("a"), Some(String::new()));
        std::fs::remove_dir_all(&base).ok();
    }

    fn make_zip(path: &Path, entries: &[(&str, Vec<u8>)]) {
        let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (n, d) in entries {
            w.start_file(*n, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(d).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn zip_as_folder() {
        let base = std::env::temp_dir().join(format!("lobj_view_ziptest_{}", std::process::id()));
        let root = base.join("root");
        std::fs::create_dir_all(root.join("d")).unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        let inner = base.join("inner.zip");
        make_zip(&inner, &[("deep/c.stl", b"solid x".to_vec())]);
        let inner_bytes = std::fs::read(&inner).unwrap();
        make_zip(&root.join("d").join("pack.ZIP"), &[("m/a.stl", b"solid a".to_vec()), ("b.ply", b"ply".to_vec()), ("in.zip", inner_bytes)]);
        let before = std::fs::read(root.join("d").join("pack.ZIP")).unwrap();

        // 資料夾列表：zip 出現在 dirs 與 zips
        let l = list_dir(&root, "d", &no()).unwrap();
        assert_eq!((l.dirs.as_slice(), l.zips.as_slice()), (&["pack.ZIP".to_string()][..], &["pack.ZIP".to_string()][..]));
        // 進入 zip，看到內容
        let l = list_dir(&root, "d/pack.ZIP", &no()).unwrap();
        assert_eq!(l.path, "d/pack.ZIP");
        assert_eq!(l.dirs, vec!["in.zip".to_string(), "m".to_string()]);
        assert_eq!(l.files[0].0, "b.ply");
        // 穿過 zip 的子資料夾與檔案
        let l = list_dir(&root, "d/pack.ZIP/m", &no()).unwrap();
        assert_eq!((l.path.as_str(), l.files[0].0.as_str()), ("d/pack.ZIP/m", "a.stl"));
        let l = list_dir(&root, "d/pack.ZIP/m/a.stl", &no()).unwrap();
        assert_eq!((l.path.as_str(), l.selected.as_deref()), ("d/pack.ZIP/m", Some("a.stl")));
        assert!(resolve(&root, "d/pack.ZIP/m/a.stl", &no()).unwrap().real.is_file());
        // zip 內的 .. 回到 zip 根；跳出 zip 根則回到 zip 所在資料夾（把 zip 當資料夾的自然結果）
        assert_eq!(list_dir(&root, "d/pack.ZIP/m/..", &no()).unwrap().path, "d/pack.ZIP");
        assert_eq!(list_dir(&root, "d/pack.ZIP/..", &no()).unwrap().path, "d");
        // 內層 zip
        let l = list_dir(&root, "d/pack.ZIP/in.zip/deep", &no()).unwrap();
        assert_eq!((l.path.as_str(), l.files[0].0.as_str()), ("d/pack.ZIP/in.zip/deep", "c.stl"));
        // 不存在、穿越
        assert!(resolve(&root, "d/pack.ZIP/none.stl", &no()).is_err());
        assert!(resolve(&root, "d/pack.ZIP/../../../x", &no()).is_err());
        // 取消：新的 zip 一開始就取消 → 不留下快取
        make_zip(&root.join("d").join("other.zip"), &[("z.stl", b"solid".to_vec())]);
        let yes = AtomicBool::new(true);
        assert_eq!(list_dir(&root, "d/other.zip", &yes).unwrap_err().0.as_u16(), 499);
        assert!(zipfs::cached_dir(&std::fs::canonicalize(root.join("d").join("other.zip")).unwrap()).is_none());
        // 原 zip 沒有被改
        assert_eq!(before, std::fs::read(root.join("d").join("pack.ZIP")).unwrap());
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn name_filter() {
        assert!(is_lobj_name("x.LOBJ2"));
        assert!(is_lobj_name("x.STL"));
        assert!(is_lobj_name("x.OBJ"));
        assert!(is_lobj_name("x.ply"));
        assert!(!is_lobj_name("x.txt"));
    }
}


/// NDJSON 串流（一行一個 JSON）：`{"progress":..}` 若干行，最後一行是 `{"done":..}` 或 `{"error":".."}`。
/// 開始串流之後 HTTP 狀態碼已是 200，所以失敗要靠最後一行的 `error` 表達；開始之前的錯誤仍用一般的狀態碼。
pub(crate) type LineTx = tokio::sync::mpsc::Sender<Result<Vec<u8>, std::io::Error>>;

pub(crate) fn line(json: String) -> Result<Vec<u8>, std::io::Error> {
    Ok(format!("{json}\n").into_bytes())
}

pub(crate) fn ndjson_response(rx: tokio::sync::mpsc::Receiver<Result<Vec<u8>, std::io::Error>>) -> Response {
    (
        [(header::CONTENT_TYPE, "application/x-ndjson"), (header::CACHE_CONTROL, "no-store")],
        axum::body::Body::from_stream(tokio_stream::wrappers::ReceiverStream::new(rx)),
    )
        .into_response()
}

/// 壓縮進度：以「已處理位元組 / 總位元組」換算百分比，百分比改變且距上次回報 ≥ 200ms 才送；
/// 送不出去（前端已中斷連線）就回傳錯誤，讓壓縮停下來。
struct ZipProgress<'a> {
    tx: &'a LineTx,
    done: u64,
    total: u64,
    last_pct: u64,
    last: std::time::Instant,
}

impl ZipProgress<'_> {
    fn add(&mut self, n: u64) -> anyhow::Result<()> {
        self.done += n;
        let pct = if self.total == 0 { 0 } else { (self.done * 100 / self.total).min(100) };
        if pct != self.last_pct && self.last.elapsed() >= Duration::from_millis(200) {
            self.last_pct = pct;
            self.last = std::time::Instant::now();
            self.tx.blocking_send(line(format!("{{\"progress\":{pct}}}"))).map_err(|_| anyhow::anyhow!("連線已中斷"))?;
        }
        Ok(())
    }
}

/// `dir` 底下所有一般檔案的位元組總和（略過符號連結），當作進度的分母。
fn dir_bytes(dir: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    rd.filter_map(Result::ok)
        .map(|e| match e.file_type() {
            Ok(ft) if ft.is_dir() => dir_bytes(&e.path()),
            Ok(ft) if ft.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

/// 遞迴把 `dir` 寫進 zip，entry 名稱是相對 `base` 的 `/` 路徑（含最上層資料夾名）；略過符號連結。
fn zip_add_dir<W: std::io::Write + std::io::Seek>(
    zw: &mut zip::ZipWriter<W>,
    base: &Path,
    dir: &Path,
    opt: zip::write::SimpleFileOptions,
    count: &mut usize,
    prog: &mut ZipProgress,
) -> anyhow::Result<()> {
    use std::io::{Read, Write};
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        let name = path.strip_prefix(base)?.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
        if ft.is_dir() {
            zw.add_directory(format!("{name}/"), opt)?;
            zip_add_dir(zw, base, &path, opt, count, prog)?;
        } else if ft.is_file() {
            // 檔案被占用 / 無權限就略過，不讓整包失敗
            let Ok(mut f) = std::fs::File::open(&path) else { continue };
            zw.start_file(name, opt.large_file(e.metadata().map(|m| m.len() > u32::MAX as u64).unwrap_or(false)))?;
            let mut buf = vec![0u8; 256 * 1024];
            loop {
                let n = f.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                zw.write_all(&buf[..n])?;
                prog.add(n as u64)?;
            }
            *count += 1;
        }
    }
    Ok(())
}

const ZIP_REMOTE_MSG: &str = "壓縮會在伺服器上寫入檔案，只在伺服器監聽本機位址時開放";

/// 壓縮前的檢查結果（這段失敗時還能回一般的 HTTP 錯誤）。
struct ZipPlan {
    dir: PathBuf,
    base: PathBuf,
    name: String,
    out: PathBuf,
    tmp: PathBuf,
    total: u64,
}

/// 把資料夾壓成與它同層的 `<資料夾名>.zip`（用來傳到別台電腦）。body：`{ "path": 資料夾, "overwrite": bool }`。
/// 先寫 `.partial` 完成才改名；同名檔已存在且未指定 overwrite 回 409。
/// 檢查通過後回傳 NDJSON 串流：`{"progress":0-100}`…，最後 `{"done":{"zip_path","size","files"}}` 或 `{"error":".."}`；前端中斷連線會停止壓縮並刪掉 `.partial`。
async fn zip_dir(State(shared): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    if let Err(e) = crate::assoc::guard_with(&shared, &headers, true, ZIP_REMOTE_MSG) {
        return e.into_response();
    }
    let input = body.get("path").and_then(|v| v.as_str()).unwrap_or("").to_owned();
    let overwrite = body.get("overwrite").and_then(|v| v.as_bool()).unwrap_or(false);
    let root = shared.root();
    let unrestricted = shared.local_only();
    let plan = move || -> Result<ZipPlan, (StatusCode, String)> {
        let r = resolve_ex(&root, &input, &AtomicBool::new(false), unrestricted).map_err(|(c, m)| (c, m.to_string()))?;
        if r.in_zip {
            return Err((StatusCode::BAD_REQUEST, "zip 內的資料夾不能壓縮".into()));
        }
        if !r.real.is_dir() {
            return Err((StatusCode::BAD_REQUEST, "不是資料夾".into()));
        }
        let (Some(name), Some(base)) = (r.real.file_name().map(|n| n.to_string_lossy().into_owned()), r.real.parent()) else {
            return Err((StatusCode::BAD_REQUEST, "根目錄無法壓縮（沒有上一層可放 zip）".into()));
        };
        let out = base.join(format!("{name}.zip"));
        if out.exists() && !overwrite {
            return Err((StatusCode::CONFLICT, format!("{} 已存在", out.display())));
        }
        Ok(ZipPlan { tmp: base.join(format!("{name}.zip.partial")), base: base.to_path_buf(), total: dir_bytes(&r.real), dir: r.real, name, out })
    };
    let plan = match tokio::task::spawn_blocking(plan).await {
        Ok(Ok(p)) => p,
        Ok(Err((c, m))) => return (c, m).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let (tx, rx) = tokio::sync::mpsc::channel(16);
    tokio::task::spawn_blocking(move || {
        let work = || -> anyhow::Result<(u64, usize)> {
            let file = std::fs::File::create(&plan.tmp)?;
            let mut zw = zip::ZipWriter::new(std::io::BufWriter::new(file));
            let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zw.add_directory(format!("{}/", plan.name), opt)?;
            let mut count = 0usize;
            let mut prog = ZipProgress { tx: &tx, done: 0, total: plan.total, last_pct: 0, last: std::time::Instant::now() };
            zip_add_dir(&mut zw, &plan.base, &plan.dir, opt, &mut count, &mut prog)?;
            zw.finish()?;
            std::fs::rename(&plan.tmp, &plan.out)?;
            Ok((std::fs::metadata(&plan.out).map(|m| m.len()).unwrap_or(0), count))
        };
        let last = match work() {
            Ok((size, files)) => format!("{{\"done\":{{\"zip_path\":{},\"size\":{size},\"files\":{files}}}}}", json_string(&plan.out.to_string_lossy())),
            Err(e) => {
                std::fs::remove_file(&plan.tmp).ok();
                format!("{{\"error\":{}}}", json_string(&e.to_string()))
            }
        };
        tx.blocking_send(line(last)).ok();
    });
    ndjson_response(rx)
}
