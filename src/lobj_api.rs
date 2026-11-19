//! 後端的 lobj API。前後端之間的資料就是 `.lobj` 的二進位格式（binary stream）。
//!
//! - `GET /api/lobj[?path=...]`：列出 `--dir`（或其子資料夾）的內容
//!   （JSON：`{ path, parent, dirs: [name], files: [{ name, size }], selected }`）。
//!   `path` 可用 `/` 或 `\`，可以是相對 `--dir` 的路徑，也可以是 `--dir` 底下的絕對路徑；
//!   若指到檔案，則列出它所在的資料夾並以 `selected` 回報該檔名。
//! - `GET /api/lobj/{*path}[?jaw=upper|lower]`：回傳該檔的 `.lobj` 二進位串流
//!   （`application/octet-stream`，版面見 `lobj::format`），與磁碟檔案的差別：
//!     - labels 一律是 **FDI**（舊 `.lobj` 會在後端先換算，所以前端不需要知道 arch_index）；
//!     - 檔案沒有 colors 時，由後端依 labels 補上（`Palette::Unique`），調色盤只在 Rust 維護一份。
//!
//! 所有路徑都會 canonicalize 後確認仍在 `--dir` 之內（含 `..`、符號連結），超出一律拒絕。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::{
    Router,
    extract::{Path as UrlPath, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use lobj::colors::{self, Palette};
use lobj::{Jaw, LobjError};

pub fn router(dir: PathBuf) -> Router {
    let root = std::fs::canonicalize(&dir).unwrap_or(dir);
    Router::new()
        .route("/api/lobj", get(list))
        .route("/api/lobj/{*path}", get(fetch))
        .with_state(Arc::new(root))
}

fn is_lobj_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".lobj") || lower.ends_with(".lobj2") || lower.ends_with(".stl") || lower.ends_with(".obj") || lower.ends_with(".ply")
}

type ApiErr = (StatusCode, &'static str);

/// 把使用者輸入的路徑（`/` 或 `\`、相對或絕對）解析成 `root` 之內的真實路徑。
fn resolve(root: &Path, input: &str) -> Result<PathBuf, ApiErr> {
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
                return Ok(real);
            }
            outside = true;
        }
    }
    if outside {
        Err((StatusCode::FORBIDDEN, "路徑不可超出 --dir"))
    } else {
        Err((StatusCode::NOT_FOUND, "找不到路徑"))
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

struct Listing {
    path: String,
    dirs: Vec<String>,
    files: Vec<(String, u64)>,
    selected: Option<String>,
}

fn list_dir(root: &Path, input: &str) -> Result<Listing, ApiErr> {
    let real = resolve(root, input)?;
    let (dir, selected) = if real.is_file() {
        let name = real.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !is_lobj_name(&name) {
            return Err((StatusCode::BAD_REQUEST, "不是 .lobj / .lobj2 / .stl / .obj / .ply 檔"));
        }
        (real.parent().unwrap_or(root).to_path_buf(), Some(name))
    } else {
        (real, None)
    };
    let io = |_| (StatusCode::INTERNAL_SERVER_ERROR, "無法列出資料夾");
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(io)? {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        // 以 canonicalize 後仍在 root 內為準（符號連結指到外面的就不列出）
        let Ok(real) = std::fs::canonicalize(entry.path()) else { continue };
        if !real.starts_with(root) {
            continue;
        }
        if real.is_dir() {
            dirs.push(name);
        } else if is_lobj_name(&name) {
            files.push((name, real.metadata().map(|m| m.len()).unwrap_or(0)));
        }
    }
    dirs.sort_by_key(|n| n.to_lowercase());
    files.sort_by_key(|(n, _)| n.to_lowercase());
    Ok(Listing { path: rel_string(root, &dir), dirs, files, selected })
}

async fn list(State(root): State<Arc<PathBuf>>, Query(q): Query<HashMap<String, String>>) -> Response {
    let input = q.get("path").cloned().unwrap_or_default();
    match tokio::task::spawn_blocking(move || list_dir(&root, &input)).await {
        Ok(Ok(l)) => {
            let dirs = l.dirs.iter().map(|d| json_string(d)).collect::<Vec<_>>().join(",");
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
                    "{{\"path\":{},\"parent\":{},\"dirs\":[{dirs}],\"files\":[{files}],\"selected\":{selected}}}",
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
    State(root): State<Arc<PathBuf>>,
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
    let path = match resolve(&root, &name) {
        Ok(p) if p.is_file() => p,
        Ok(_) => return (StatusCode::BAD_REQUEST, "不是檔案").into_response(),
        Err(e) => return e.into_response(),
    };

    match tokio::task::spawn_blocking(move || encode(&path, jaw)).await {
        Ok(Ok(bytes)) => ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response(),
        Ok(Err(LobjError::Io(e))) if e.kind() == std::io::ErrorKind::NotFound => {
            (StatusCode::NOT_FOUND, "找不到檔案").into_response()
        }
        Ok(Err(e @ LobjError::JawRequired)) => (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response(),
        Ok(Err(e)) => (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// 讀檔 → labels 轉 FDI → 補 colors → 編成 `.lobj` 二進位。
fn encode(path: &Path, jaw: Option<Jaw>) -> Result<Vec<u8>, LobjError> {
    let mut obj = lobj::load(path, jaw)?;
    if obj.colors.is_none() {
        if let Some(labels) = &obj.labels {
            obj.colors = Some(colors::labels_to_colors(labels, Palette::Unique));
        }
    }
    lobj::format::to_bytes(&obj)
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

        assert_eq!(resolve(&root, "").unwrap(), root);
        assert!(resolve(&root, "sub\\a.lobj").unwrap().ends_with("a.lobj"));
        assert!(resolve(&root, "sub/../sub").is_ok());
        assert_eq!(resolve(&root, "..").unwrap_err().0, StatusCode::FORBIDDEN);
        assert_eq!(resolve(&root, "sub/../../secret.lobj").unwrap_err().0, StatusCode::FORBIDDEN);
        let abs = root.join("sub").join("a.lobj");
        assert!(resolve(&root, abs.to_str().unwrap()).is_ok());
        assert!(resolve(&root, base.to_str().unwrap()).is_err());
        let l = list_dir(&root, "sub/a.lobj").unwrap();
        assert_eq!((l.path.as_str(), l.selected.as_deref()), ("sub", Some("a.lobj")));
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
