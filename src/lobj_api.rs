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
//! 所有路徑都會 canonicalize 後確認仍在 `--dir` 之內（含 `..`、符號連結），超出一律拒絕。
//!
//! `.zip` 檔視為資料夾（`dirs` 內包含它，並在 `zips` 標出）：第一次進入時由 `zipfs` 解壓到快取，
//! 原 zip 不動；路徑可以穿過 zip（`a.zip/sub/x.stl`），內層 zip 也可以。解壓期間若前端中斷連線
//! （`AbortController`），handler 被丟棄，`CancelOnDrop` 會讓解壓停下來。

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::{
    Router,
    extract::{Path as UrlPath, Query, State},
    http::{HeaderName, StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::get,
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
        .route("/api/events", get(events))
        .with_state(shared)
}

/// SSE：連上時先送目前的開檔事件，之後每次有人用 `lobj-view <檔案>` 開檔就再送一次。
/// 連線存在即代表「有網頁開著」，此時就不會再自動開新的瀏覽器分頁。
async fn events(State(shared): State<Arc<Shared>>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let guard = shared.tab_guard();
    let stream = WatchStream::new(shared.subscribe()).map(move |ev| {
        let _keep = &guard;
        Ok(Event::default().data(format!("{{\"seq\":{},\"path\":{}}}", ev.seq, json_string(&ev.path))))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub fn is_lobj_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".lobj") || lower.ends_with(".lobj2") || lower.ends_with(".stl") || lower.ends_with(".obj") || lower.ends_with(".ply")
}

type ApiErr = (StatusCode, std::borrow::Cow<'static, str>);

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
struct Resolved {
    real: PathBuf,
    virt: String,
    base: PathBuf,
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
            return Ok(Resolved { real, virt, base });
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
fn resolve(root: &Path, input: &str, cancel: &AtomicBool) -> Result<Resolved, ApiErr> {
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
            if real.starts_with(root) {
                let virt = rel_string(root, &real);
                return Ok(Resolved { real, virt, base: root.to_path_buf() });
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
            if !zr.starts_with(root) {
                outside = true;
                break;
            }
            let inner = n.strip_prefix(anc).unwrap_or(Path::new("")).to_path_buf();
            let zv = rel_string(root, &zr);
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
}

fn list_dir(root: &Path, input: &str, cancel: &AtomicBool) -> Result<Listing, ApiErr> {
    let mut r = resolve(root, input, cancel)?;
    let fname = r.real.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if r.real.is_file() && zipfs::is_zip_name(&fname) {
        // 指到 .zip 本身 → 當成資料夾進入
        r = descend(r.real.clone(), r.virt.clone(), PathBuf::new(), cancel)?;
    }
    let (dir, dir_virt, selected) = if r.real.is_file() {
        if !is_lobj_name(&fname) {
            return Err(err(StatusCode::BAD_REQUEST, "不是 .lobj / .lobj2 / .stl / .obj / .ply 檔"));
        }
        let pv = r.virt.rsplit_once('/').map(|x| x.0.to_string()).unwrap_or_default();
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
    Ok(Listing { path: dir_virt, dirs, zips, files, selected })
}

async fn list(State(shared): State<Arc<Shared>>, Query(q): Query<HashMap<String, String>>) -> Response {
    let root = shared.root();
    let input = q.get("path").cloned().unwrap_or_default();
    // 前端中斷連線時這個 future 會被丟棄，guard 隨之讓背景的解壓停下來
    let cancel = Arc::new(AtomicBool::new(false));
    let _guard = CancelOnDrop(cancel.clone());
    match tokio::task::spawn_blocking(move || list_dir(&root, &input, &cancel)).await {
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
                    "{{\"path\":{},\"parent\":{},\"dirs\":[{dirs}],\"zips\":[{zips}],\"files\":[{files}],\"selected\":{selected}}}",
                    json_string(&l.path),
                    !l.path.is_empty()
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
    let cancel = Arc::new(AtomicBool::new(false));
    let _guard = CancelOnDrop(cancel.clone());
    // 路徑可能穿過 .zip（需要解壓），所以解析也在背景執行緒做
    let job = move || -> Result<Result<(Vec<u8>, bool), LobjError>, ApiErr> {
        let r = resolve(&root, &name, &cancel)?;
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

fn json_string(s: &str) -> String {
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
