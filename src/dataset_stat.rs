//! 資料集統計頁（`/stat`）的後端：
//!   - `GET  /api/stat-case?path=<病例資料夾>`：讀該資料夾內所有 `.lobj2`，回傳有無上 / 下顎與出現過哪些 FDI。
//!   - `POST /api/stat-save`：把前端組好的 CSV 寫成 `<資料集資料夾>/stat.csv`（UTF-8，含 BOM；只限本機）。
//!
//! 一個病例資料夾（例如 `sets141_001`）可能有 2 或 4 個檔（`1_lower` / `1_lower_study` …），仍算一個病例：
//! 上 / 下顎、FDI 都取聯集。有異常（讀不到、檔名與 label 的上下顎矛盾、study 與原檔 FDI 不同）寫進 remark。

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use lobj::Jaw;
use lobj::labels::{LabelKind, classify};

use crate::instance::Shared;
use crate::lobj_api::resolve_ex;

const REMOTE_MSG: &str = "統計輸出會寫入伺服器上的檔案，只在伺服器監聽本機位址時開放";

pub fn router(shared: Arc<Shared>) -> Router {
    Router::new()
        .route("/api/stat-case", get(stat_case))
        .route("/api/stat-save", post(stat_save))
        .with_state(shared)
}

#[derive(Default, Debug, PartialEq)]
pub struct CaseStat {
    pub upper: bool,
    pub lower: bool,
    pub fdi: BTreeSet<i8>,
    pub remarks: Vec<String>,
}

fn jaw_name(j: Jaw) -> &'static str {
    match j {
        Jaw::Upper => "upper",
        Jaw::Lower => "lower",
    }
}

/// 統計一個病例資料夾（只看這一層的 `.lobj2`）。
pub fn stat_dir(dir: &Path) -> Result<CaseStat, String> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("無法列出資料夾：{e}"))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("lobj2")))
        .map(|e| e.path())
        .collect();
    files.sort();
    let mut st = CaseStat::default();
    if files.is_empty() {
        st.remarks.push("沒有 .lobj2".into());
        return Ok(st);
    }
    // (顎, 是否 study) → 該組的 FDI，用來比對 study 與原檔
    let mut groups: HashMap<(Jaw, bool), BTreeSet<i8>> = HashMap::new();
    for f in &files {
        let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let obj = match lobj::load(f, None) {
            Ok(o) => o,
            Err(e) => {
                st.remarks.push(format!("{name} 讀取失敗：{e}"));
                continue;
            }
        };
        let Some(labels) = obj.labels.as_ref() else {
            st.remarks.push(format!("{name} 沒有 labels"));
            continue;
        };
        let mut set = BTreeSet::new();
        let (mut has_u, mut has_l) = (false, false);
        for &l in labels {
            match classify(l) {
                LabelKind::PermanentTooth { jaw } | LabelKind::PrimaryTooth { jaw } => {
                    set.insert(l);
                    if jaw == Jaw::Upper { has_u = true } else { has_l = true }
                }
                _ => {}
            }
        }
        // 上下顎：以 label 實際出現的牙為準；檔名若明確標示卻與 label 矛盾就註記
        let by_name = Jaw::guess_from_path(f);
        match by_name {
            Some(Jaw::Upper) if has_l => st.remarks.push(format!("{name} 檔名為 upper 但有下顎的牙")),
            Some(Jaw::Lower) if has_u => st.remarks.push(format!("{name} 檔名為 lower 但有上顎的牙")),
            _ => {}
        }
        if set.is_empty() {
            // 沒有任何牙：只能靠檔名判斷這個顎存在
            match by_name {
                Some(j) => {
                    if j == Jaw::Upper { st.upper = true } else { st.lower = true }
                    st.remarks.push(format!("{name} 沒有任何牙的 label"));
                }
                None => st.remarks.push(format!("{name} 沒有牙的 label，檔名也無法判斷上 / 下顎")),
            }
            continue;
        }
        st.upper |= has_u;
        st.lower |= has_l;
        if let Some(j) = by_name.or(if has_u != has_l { Some(if has_u { Jaw::Upper } else { Jaw::Lower }) } else { None }) {
            let study = name.to_ascii_lowercase().contains("study");
            groups.entry((j, study)).or_default().extend(set.iter().copied());
        }
        st.fdi.extend(set);
    }
    for j in [Jaw::Upper, Jaw::Lower] {
        if let (Some(a), Some(b)) = (groups.get(&(j, false)), groups.get(&(j, true))) {
            if a != b {
                st.remarks.push(format!("{} 的 study 與原檔 FDI 不同", jaw_name(j)));
            }
        }
    }
    Ok(st)
}

async fn stat_case(State(shared): State<Arc<Shared>>, Query(q): Query<HashMap<String, String>>) -> Response {
    let root = shared.root();
    let unrestricted = shared.local_only();
    let input = q.get("path").cloned().unwrap_or_default();
    let job = move || -> Result<String, (StatusCode, String)> {
        let r = resolve_ex(&root, &input, &AtomicBool::new(false), unrestricted).map_err(|(c, m)| (c, m.to_string()))?;
        if r.in_zip {
            return Err((StatusCode::BAD_REQUEST, "zip 內的資料夾不能統計（請先解壓縮）".into()));
        }
        if !r.real.is_dir() {
            return Err((StatusCode::BAD_REQUEST, "不是資料夾".into()));
        }
        let st = stat_dir(&r.real).map_err(|m| (StatusCode::INTERNAL_SERVER_ERROR, m))?;
        Ok(serde_json::json!({
            "upper": st.upper,
            "lower": st.lower,
            "fdi": st.fdi.iter().collect::<Vec<_>>(),
            "remark": st.remarks.join("; "),
        })
        .to_string())
    };
    match tokio::task::spawn_blocking(job).await {
        Ok(Ok(body)) => ([(header::CONTENT_TYPE, "application/json")], body).into_response(),
        Ok(Err((c, m))) => (c, m).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// body：`{ "path": 資料集資料夾, "csv": 完整 CSV 文字 }`。固定寫成 `<path>/stat.csv`，已存在就覆蓋。
async fn stat_save(State(shared): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    if let Err(e) = crate::assoc::guard_with(&shared, &headers, true, REMOTE_MSG) {
        return e.into_response();
    }
    let (Some(path), Some(csv)) = (
        body.get("path").and_then(|v| v.as_str()).map(str::to_owned),
        body.get("csv").and_then(|v| v.as_str()).map(str::to_owned),
    ) else {
        return (StatusCode::BAD_REQUEST, "缺少 path 或 csv").into_response();
    };
    let root = shared.root();
    let unrestricted = shared.local_only();
    let job = move || -> Result<String, (StatusCode, String)> {
        let r = resolve_ex(&root, &path, &AtomicBool::new(false), unrestricted).map_err(|(c, m)| (c, m.to_string()))?;
        if r.in_zip || !r.real.is_dir() {
            return Err((StatusCode::BAD_REQUEST, "必須是磁碟上的資料夾".into()));
        }
        let out = r.real.join("stat.csv");
        // UTF-8 + BOM：Excel 開啟才不會把各國文字當成 ANSI 而亂碼；Python / pandas 用 utf-8-sig 或 utf-8 都能讀
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(csv.as_bytes());
        std::fs::write(&out, bytes).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(out.to_string_lossy().into_owned())
    };
    match tokio::task::spawn_blocking(job).await {
        Ok(Ok(p)) => ([(header::CONTENT_TYPE, "application/json")], serde_json::json!({ "path": p }).to_string()).into_response(),
        Ok(Err((c, m))) => (c, m).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
