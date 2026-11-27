//! 把本機的 `.zip` 用 rsync（經 ssh）同步到另一台電腦，以及刪除 `.zip`。
//!
//! - `POST /api/remote-ls` `{ host, path }`：用 ssh 列出遠端資料夾（只列子資料夾），給前端的目錄選擇用。
//!   `path` 空字串＝遠端家目錄；回傳 `{ path, dirs }`，`path` 是遠端 `pwd` 的結果，`writable` 表示目前帳號能否寫入該資料夾。
//! - `POST /api/zip-sync` `{ path, host, dir, passphrase? }`：`rsync -a` 把該 zip 傳到 `host:dir/`，回應是 NDJSON 進度串流（見 `zip_sync`）。
//! - `zip-sync` 帶 `background: true`：改在本機的 detached tmux session 裡跑 rsync（見 `start_bg`），立刻回 `{ id, session }`；
//!   關掉 ssh / 重啟 lobj-view 都不會中斷。`GET /api/sync-jobs` 回傳 `{ tmux, jobs }`（進度與狀態由工作資料夾裡的 log / status 檔判斷），
//!   `POST /api/sync-job-stop { id }` 結束該 session。工作資料夾在 `data_local_dir/lobj-view/sync-jobs/<id>/`，金鑰密碼只暫存在其中權限 0600 的 `pass`，結束或中止即刪。
//! - `POST /api/sync-job-clear { id? }`：刪除已結束（完成 / 失敗 / 中止）的工作紀錄；省略 `id` 就清掉全部已結束的，執行中的不動。
//! - `POST /api/zip-delete` `{ path }`：刪除該 zip（只刪 `.zip` 檔）。
//!
//! 都只在伺服器監聽本機位址或以 --admin 啟動時開放，且需 `X-Lobj-Requested`。ssh 預設 `BatchMode=yes`：沒有可用的金鑰就直接失敗，
//! 不會等密碼輸入。金鑰有密碼（ssh-agent 沒載入）時，前端可附 `passphrase`，改由 `SSH_ASKPASS`（本執行檔）供應；第一次連線的主機金鑰用 `accept-new` 自動記下（之後金鑰變了會拒絕）。
//! `host` 嚴格檢查（不能以 `-` 開頭、只允許 `[user@]host`），避免被當成 ssh / rsync 的選項。
//! 前端中斷連線時，子行程會被 `kill_on_drop` 殺掉。

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use tokio::process::Command;

use crate::instance::Shared;
use crate::lobj_api::{LineTx, json_string, line, ndjson_response, resolve_ex};

const REMOTE_MSG: &str = "同步、刪除、資料夾改名與拿掉一層只在伺服器監聽本機位址或以 --admin 啟動時開放";

/// ssh 的共用選項。沒給金鑰密碼：`BatchMode=yes`（沒有可用的金鑰就直接失敗，不等輸入）；
/// 有給：關掉 BatchMode，由 `SSH_ASKPASS` 回答 ssh 的詢問（金鑰密碼或登入密碼都用同一個字串），
/// 且只試一次（`NumberOfPasswordPrompts=1`），輸錯不會連續重試。
fn ssh_opts(pass: Option<&str>) -> String {
    let auth = if pass.is_some() { "-o NumberOfPasswordPrompts=1" } else { "-o BatchMode=yes" };
    format!("{auth} -o ConnectTimeout=10 -o StrictHostKeyChecking=accept-new")
}

const ASKPASS_FLAG: &str = "LOBJ_VIEW_ASKPASS";
const ASKPASS_PW: &str = "LOBJ_VIEW_PASSPHRASE";

/// 讓 ssh 透過「自己這個執行檔」取得金鑰密碼：密碼只放在子行程的環境變數，不進命令列、不寫檔、不存在伺服器。
fn apply_pass(cmd: &mut Command, pass: Option<&str>) {
    let Some(p) = pass else { return };
    if let Ok(exe) = std::env::current_exe() {
        cmd.env("SSH_ASKPASS", exe)
            .env("SSH_ASKPASS_REQUIRE", "force") // 沒有 DISPLAY / 沒有 tty 也照樣使用 SSH_ASKPASS
            .env(ASKPASS_FLAG, "1")
            .env(ASKPASS_PW, p);
    }
}

/// `main` 開頭呼叫：若是被 ssh 當 askpass 叫起來，印出金鑰密碼並回傳 true（呼叫端接著結束）。
pub fn askpass_mode() -> bool {
    if std::env::var_os(ASKPASS_FLAG).is_none() {
        return false;
    }
    if let Ok(p) = std::env::var(ASKPASS_PW) {
        println!("{p}");
    }
    true
}

pub fn router(shared: Arc<Shared>) -> Router {
    Router::new()
        .route("/api/remote-ls", post(remote_ls))
        .route("/api/zip-sync", post(zip_sync))
        .route("/api/zip-delete", post(zip_delete))
        .route("/api/dir-rename", post(dir_rename))
        .route("/api/dir-flatten", post(dir_flatten))
        .route("/api/dir-mask", post(dir_mask))
        .route("/api/sync-jobs", get(sync_jobs))
        .route("/api/sync-job-stop", post(sync_job_stop))
        .route("/api/sync-job-clear", post(sync_job_clear))
        .with_state(shared)
}

type Fail = (StatusCode, String);

fn bad(m: impl Into<String>) -> Fail {
    (StatusCode::BAD_REQUEST, m.into())
}

/// `[user@]host`：英數與 `._-`，不能以 `-` 開頭（也不允許 `:`，連接埠請寫在 ~/.ssh/config）。
fn valid_host(h: &str) -> bool {
    let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
    !h.starts_with('-')
        && match h.split_once('@') {
            Some((u, host)) => ok(u) && ok(host),
            None => ok(h),
        }
}

fn str_of(body: &serde_json::Value, key: &str) -> String {
    body.get(key).and_then(|v| v.as_str()).unwrap_or("").trim().to_owned()
}

/// 金鑰密碼（可省略）；不 trim，空字串視為沒給。
fn pass_of(body: &serde_json::Value) -> Option<String> {
    body.get("passphrase").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(str::to_owned)
}

fn host_of(body: &serde_json::Value) -> Result<String, Fail> {
    let h = str_of(body, "host");
    if valid_host(&h) { Ok(h) } else { Err(bad("主機格式不正確，請輸入 user@host")) }
}

fn dir_of(body: &serde_json::Value, key: &str) -> Result<String, Fail> {
    let d = str_of(body, key);
    if d.contains(['\0', '\n', '\r']) { Err(bad("路徑不可含換行")) } else { Ok(d) }
}

fn guard(shared: &Shared, headers: &HeaderMap) -> Result<(), Fail> {
    crate::assoc::guard_with(shared, headers, true, REMOTE_MSG).map_err(|(c, m)| (c, m.to_string()))
}

fn respond(r: Result<String, Fail>) -> Response {
    match r {
        Ok(json) => ([(header::CONTENT_TYPE, "application/json")], json).into_response(),
        Err((c, m)) => (c, m).into_response(),
    }
}

/// 子行程的錯誤輸出取最後幾行，給使用者看。
fn tail(s: &[u8]) -> String {
    let s = String::from_utf8_lossy(s);
    let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty() && !l.starts_with("Warning: Permanently added")).collect();
    lines[lines.len().saturating_sub(5)..].join("\n")
}

async fn run(mut cmd: Command, timeout: Duration, what: &str) -> Result<std::process::Output, Fail> {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    let child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            (StatusCode::NOT_IMPLEMENTED, format!("找不到 {what}，請先安裝並放進 PATH"))
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    })?;
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(o)) => Ok(o),
        Ok(Err(e)) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        Err(_) => Err((StatusCode::GATEWAY_TIMEOUT, format!("{what} 逾時"))),
    }
}

/// 單引號包起來給遠端 shell。
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

async fn remote_ls(State(shared): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    respond(async {
        guard(&shared, &headers)?;
        let host = host_of(&body)?;
        let path = dir_of(&body, "path")?;
        let cd = if path.is_empty() { "cd".to_string() } else { format!("cd -- {}", sh_quote(&path)) };
        let pass = pass_of(&body);
        let mut cmd = Command::new("ssh");
        cmd.args(ssh_opts(pass.as_deref()).split(' ')).arg("--")
            .arg(&host)
            .arg(format!("{cd} && pwd && {{ test -w . && echo W || echo R; }} && ls -1Ap"));
        apply_pass(&mut cmd, pass.as_deref());
        let out = run(cmd, Duration::from_secs(30), "ssh").await?;
        if !out.status.success() {
            return Err((StatusCode::BAD_GATEWAY, tail(&out.stderr)));
        }
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let mut lines = text.lines();
        let pwd = lines.next().unwrap_or("/").to_string();
        let writable = lines.next() == Some("W");
        let mut dirs: Vec<&str> = lines.filter_map(|l| l.strip_suffix('/')).collect();
        dirs.sort_unstable();
        let dirs = dirs.iter().map(|d| json_string(d)).collect::<Vec<_>>().join(",");
        Ok(format!("{{\"path\":{},\"writable\":{writable},\"dirs\":[{dirs}]}}", json_string(&pwd)))
    }
    .await)
}

/// 解析成「磁碟上真實存在、不在 zip 快取內的 .zip 檔」。
fn real_zip(shared: &Shared, input: &str) -> Result<PathBuf, Fail> {
    let r = resolve_ex(&shared.root(), input, &AtomicBool::new(false), shared.local_only()).map_err(|(c, m)| (c, m.to_string()))?;
    let is_zip = r.real.file_name().is_some_and(|n| crate::zipfs::is_zip_name(&n.to_string_lossy()));
    if r.in_zip || !r.real.is_file() || !is_zip {
        return Err(bad("只能操作磁碟上的 .zip 檔"));
    }
    Ok(r.real)
}

/// 解析成「磁碟上真實存在、不在 zip 快取內的資料夾」，且不是根資料夾、也不是根資料夾的上層
/// （改名或搬動它會讓伺服器的根資料夾失效）。
fn real_dir(shared: &Shared, input: &str) -> Result<PathBuf, Fail> {
    let root = shared.root();
    let r = resolve_ex(&root, input, &AtomicBool::new(false), shared.local_only()).map_err(|(c, m)| (c, m.to_string()))?;
    if r.in_zip || !r.real.is_dir() {
        return Err(bad("只能操作磁碟上的資料夾"));
    }
    if r.real.parent().is_none() || root.starts_with(&r.real) {
        return Err(bad("不能操作根資料夾（或它的上層）"));
    }
    Ok(r.real)
}

fn valid_dir_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 255
        && n != "."
        && n != ".."
        && !n.ends_with(['.', ' '])
        && !n.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
}

/// 資料夾改名（同一層，不能搬動）。body：`{ "path": 資料夾, "name": 新名稱 }`；新名稱已存在回 409。
/// 回 `{ "path": 新的完整路徑 }`。
async fn dir_rename(State(shared): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    respond(async {
        crate::assoc::guard_with(&shared, &headers, true, REMOTE_MSG).map_err(|(c, m)| (c, m.to_string()))?;
        let dir = real_dir(&shared, &str_of(&body, "path"))?;
        let name = str_of(&body, "name");
        if !valid_dir_name(&name) {
            return Err(bad("名稱不正確（不可空白、含 / \\ : * ? \" < > |，或以 . 空白結尾）"));
        }
        let to = dir.with_file_name(&name);
        if to == dir {
            return Err(bad("名稱沒有改變"));
        }
        if to.exists() {
            return Err((StatusCode::CONFLICT, format!("{name} 已存在")));
        }
        std::fs::rename(&dir, &to).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(format!("{{\"path\":{}}}", json_string(&to.to_string_lossy())))
    }
    .await)
}

/// 解析成磁碟上的資料夾（不在 zip 內）；根資料夾與它的上層是否可用由呼叫端決定。
fn any_dir(shared: &Shared, input: &str) -> Result<PathBuf, Fail> {
    let r = resolve_ex(&shared.root(), input, &AtomicBool::new(false), shared.local_only()).map_err(|(c, m)| (c, m.to_string()))?;
    if r.in_zip || !r.real.is_dir() {
        return Err(bad("只能操作磁碟上的資料夾"));
    }
    Ok(r.real)
}

/// `dir` 底下的子資料夾（不含符號連結），依名稱排序。
fn sub_dirs(dir: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .collect();
    v.sort();
    Ok(v)
}

/// 「拿掉這層」：把目標資料夾裡的項目全部移到它的上一層，再刪掉這個（已清空的）資料夾。
/// body：`{ "path": 資料夾, "level"?: N }`。`level` 省略或 0＝所選資料夾本身；
/// N ≥ 1＝所選資料夾底下第 N 層的每一個資料夾（1＝它的子資料夾，2＝孫資料夾…），一次全部處理
/// （例如在 R 選 2：`R/aa1/lobj`、`R/aa2/lobj` 都被拿掉，內容移到 `R/aa1`、`R/aa2`）。
/// 先檢查全部目標，上一層已有同名項目（含與目標自己同名、或同一層的兩個目標內容互相同名）就整個不動並回 409；
/// 最後用 `remove_dir`（只刪空資料夾）所以不會誤刪內容。回 `{ "moved": 搬了幾個項目, "folders": 處理幾個資料夾 }`。
/// `dry_run: true`：只預覽不動，回 `{ folders, moved, targets:[{path,items}], clashes, clash_list:[{from,to}] }`（清單最多各 300 筆）。
async fn dir_flatten(State(shared): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    respond(async {
        crate::assoc::guard_with(&shared, &headers, true, REMOTE_MSG).map_err(|(c, m)| (c, m.to_string()))?;
        let level = body.get("level").and_then(|v| v.as_u64()).unwrap_or(0);
        if level > 8 {
            return Err(bad("層數太大（最多 8）"));
        }
        let io = |e: std::io::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        let top = if level == 0 { real_dir(&shared, &str_of(&body, "path"))? } else { any_dir(&shared, &str_of(&body, "path"))? };
        let mut targets = vec![top.clone()];
        if level > 0 {
            for _ in 0..level {
                let mut next = Vec::new();
                for d in &targets {
                    next.extend(sub_dirs(d).map_err(io)?);
                }
                targets = next;
            }
            if targets.is_empty() {
                return Err(bad(format!("第 {level} 層沒有資料夾")));
            }
        }
        let root = shared.root();
        // (搬的來源, 目的地)
        let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut planned: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
        let mut clash: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut per_target: Vec<(PathBuf, usize)> = Vec::new();
        for t in &targets {
            let Some(parent) = t.parent() else { return Err(bad("沒有上一層")) };
            if root.starts_with(t) {
                return Err(bad("不能操作根資料夾（或它的上層）"));
            }
            let mut n = 0;
            for e in std::fs::read_dir(t).map_err(io)?.filter_map(Result::ok) {
                let to = parent.join(e.file_name());
                if to.exists() || !planned.insert(to.clone()) {
                    clash.push((e.path(), to.clone()));
                }
                moves.push((e.path(), to));
                n += 1;
            }
            per_target.push((t.clone(), n));
        }
        if body.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false) {
            // 預覽：不動任何東西，列出會處理的資料夾（含各有幾個項目）與不符合（會衝突）的路徑
            let lim = 300;
            let ts = per_target
                .iter()
                .take(lim)
                .map(|(p, n)| format!("{{\"path\":{},\"items\":{n}}}", json_string(&p.to_string_lossy())))
                .collect::<Vec<_>>()
                .join(",");
            let cs = clash
                .iter()
                .take(lim)
                .map(|(f, t)| format!("{{\"from\":{},\"to\":{}}}", json_string(&f.to_string_lossy()), json_string(&t.to_string_lossy())))
                .collect::<Vec<_>>()
                .join(",");
            return Ok(format!(
                "{{\"folders\":{},\"moved\":{},\"targets\":[{ts}],\"clashes\":{},\"clash_list\":[{cs}]}}",
                targets.len(),
                moves.len(),
                clash.len()
            ));
        }
        if !clash.is_empty() {
            let shown = clash.iter().take(5).map(|(_, t)| t.to_string_lossy().into_owned()).collect::<Vec<_>>().join("、");
            let more = if clash.len() > 5 { format!(" 等 {} 項", clash.len()) } else { String::new() };
            return Err((StatusCode::CONFLICT, format!("已有同名項目：{shown}{more}，未做任何變更")));
        }
        for (from, to) in &moves {
            std::fs::rename(from, to).map_err(io)?;
        }
        for t in &targets {
            std::fs::remove_dir(t).map_err(io)?;
        }
        Ok(format!("{{\"moved\":{},\"folders\":{}}}", moves.len(), targets.len()))
    }
    .await)
}

/// 遮蔽 / 還原資料夾名稱（病患姓名等），演算法見 `namemask`，與 `no_user_info.py` 相容。
/// body：`{ "path": 資料夾, "mode": "encode" | "decode", "password": 密碼 }`；只改所選資料夾底下第一層的子資料夾名稱
/// （所選資料夾可以是根資料夾）。先算好全部新名稱，任何一個失敗（例如還原時密碼錯誤）或新名稱衝突就整個不動。
/// 密碼只用來導出金鑰，不記錄、不儲存。回 `{ "count": 改了幾個 }`。
async fn dir_mask(State(shared): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    respond(async {
        crate::assoc::guard_with(&shared, &headers, true, REMOTE_MSG).map_err(|(c, m)| (c, m.to_string()))?;
        let encode = match str_of(&body, "mode").as_str() {
            "encode" => true,
            "decode" => false,
            _ => return Err(bad("mode 只能是 encode 或 decode")),
        };
        let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("");
        if password.is_empty() {
            return Err(bad("請輸入密碼"));
        }
        let top = any_dir(&shared, &str_of(&body, "path"))?;
        let root = shared.root();
        let password = password.to_owned();
        let (plan, file_plan) = tokio::task::spawn_blocking(move || -> Result<MaskPlan, Fail> {
            let key = crate::namemask::derive_key(&password);
            let dirs = sub_dirs(&top).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let mut plan = Vec::new();
            let mut file_plan: Vec<(PathBuf, PathBuf)> = Vec::new();
            for d in dirs {
                if root.starts_with(&d) {
                    return Err(bad("根資料夾在這些資料夾裡面，無法改名"));
                }
                let old = d.file_name().unwrap_or_default().to_string_lossy().into_owned();
                let new = if encode { crate::namemask::encode(&old, &key) } else { crate::namemask::decode(&old, &key).map_err(|m| bad(format!("{old}: {m}")))? };
                if !valid_dir_name(&new) {
                    return Err(bad(format!("{old}: 還原後的名稱不合法")));
                }
                // 檔名裡常內嵌資料夾名稱（例如 `Template4_upper_..._93413335A_姓名.lobj`），一併換掉；
                // 只替換資料夾名稱那一段，其餘（upper/lower、step…）保持原樣，才不影響依檔名判斷上下顎。
                let mut fs = Vec::new();
                for f in files_under(&d).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))? {
                    let fname = f.file_name().unwrap_or_default().to_string_lossy().into_owned();
                    if !fname.contains(&old) {
                        continue;
                    }
                    let nf = fname.replace(&old, &new);
                    if !valid_dir_name(&nf) {
                        return Err(bad(format!("{fname}: 換名後檔名不合法（可能太長）")));
                    }
                    fs.push((f.clone(), f.with_file_name(nf)));
                }
                let mut seen = std::collections::HashSet::new();
                if fs.iter().any(|(_, n)| !seen.insert(n.clone()) || n.exists()) {
                    return Err((StatusCode::CONFLICT, "檔名衝突，未做任何變更".into()));
                }
                file_plan.extend(fs);
                plan.push((d.clone(), d.with_file_name(new)));
            }
            let olds: std::collections::HashSet<&PathBuf> = plan.iter().map(|(o, _)| o).collect();
            let news: std::collections::HashSet<&PathBuf> = plan.iter().map(|(_, n)| n).collect();
            if news.len() != plan.len() || plan.iter().any(|(_, n)| n.exists() && !olds.contains(n)) {
                return Err((StatusCode::CONFLICT, "名稱衝突，未做任何變更".into()));
            }
            Ok((plan, file_plan))
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
        // 先改檔名（此時資料夾還在原路徑），再改資料夾名稱
        for (old, new) in &file_plan {
            std::fs::rename(old, new).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        // 新名稱可能剛好是另一個要改名的舊名稱：先全部改成暫名再改成最終名稱，避免互相覆蓋
        let mut stage = Vec::new();
        for (i, (old, new)) in plan.iter().enumerate() {
            let tmp = old.with_file_name(format!(".lobj-mask-{}-{i}", std::process::id()));
            std::fs::rename(old, &tmp).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            stage.push((tmp, new.clone()));
        }
        for (tmp, new) in &stage {
            std::fs::rename(tmp, new).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        Ok(format!("{{\"count\":{},\"files\":{}}}", plan.len(), file_plan.len()))
    }
    .await)
}

type MaskPlan = (Vec<(PathBuf, PathBuf)>, Vec<(PathBuf, PathBuf)>);

/// `dir` 底下所有檔案（遞迴，不含符號連結）。
fn files_under(dir: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir)?.filter_map(Result::ok) {
        let t = e.file_type()?;
        if t.is_dir() {
            out.extend(files_under(&e.path())?);
        } else if t.is_file() {
            out.push(e.path());
        }
    }
    Ok(out)
}

/// rsync 的 `--info=progress2` 輸出形如 `  12,345,678  37%   10.50MB/s    0:00:05`（以 `\r` 更新同一行）。
fn parse_progress(seg: &str) -> Option<(u32, String)> {
    let mut pct = None;
    let mut speed = String::new();
    for tok in seg.split_whitespace() {
        if let Some(n) = tok.strip_suffix('%') {
            pct = n.parse().ok();
        } else if tok.ends_with("/s") {
            speed = tok.to_string();
        }
    }
    pct.map(|p| (p, speed))
}

/// 執行 rsync 並把進度寫進 `tx`。`tx` 送不出去（前端中斷）就直接返回，`kill_on_drop` 會殺掉 rsync。
async fn rsync_run(mut cmd: Command, tx: &LineTx) -> Result<(), String> {
    use tokio::io::AsyncReadExt;
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { "找不到 rsync，請先安裝並放進 PATH".to_string() } else { e.to_string() }
    })?;
    let mut stderr = child.stderr.take().ok_or("無法讀取 rsync 輸出")?;
    let err_task = tokio::spawn(async move {
        let mut v = Vec::new();
        let mut buf = [0u8; 4096];
        while let Ok(n) = stderr.read(&mut buf).await {
            if n == 0 || v.len() > 64 * 1024 {
                break;
            }
            v.extend_from_slice(&buf[..n]);
        }
        v
    });
    let mut stdout = child.stdout.take().ok_or("無法讀取 rsync 輸出")?;
    let (mut buf, mut acc, mut last) = ([0u8; 4096], String::new(), None);
    loop {
        let n = stdout.read(&mut buf).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        acc.push_str(&String::from_utf8_lossy(&buf[..n]));
        // 以 \r / \n 切段，最後一段可能還沒寫完，留到下次
        while let Some(i) = acc.find(['\r', '\n']) {
            let seg: String = acc.drain(..=i).collect();
            if let Some((p, speed)) = parse_progress(&seg) {
                if last != Some(p) {
                    last = Some(p);
                    let l = format!("{{\"progress\":{p},\"speed\":{}}}", json_string(&speed));
                    if tx.send(line(l)).await.is_err() {
                        return Err("連線已中斷".into());
                    }
                }
            }
        }
    }
    let status = child.wait().await.map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(tail(&err_task.await.unwrap_or_default()))
    }
}

/// 同步成功前的檢查失敗回一般 HTTP 錯誤；開始傳輸後回 NDJSON：`{"progress":0-100,"speed":".."}`…，最後 `{"done":{"dest"}}` 或 `{"error":".."}`。
async fn zip_sync(State(shared): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    let prep = (|| -> Result<(String, String, PathBuf, Option<String>), Fail> {
        guard(&shared, &headers)?;
        Ok((host_of(&body)?, dir_of(&body, "dir")?, real_zip(&shared, &str_of(&body, "path"))?, pass_of(&body)))
    })();
    let (host, dir, zip, pass) = match prep {
        Ok(p) => p,
        Err((c, m)) => return (c, m).into_response(),
    };
    let name = zip.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let dest = format!("{host}:{}", if dir.is_empty() { String::new() } else { format!("{}/", dir.trim_end_matches('/')) });
    if body.get("background").and_then(|v| v.as_bool()).unwrap_or(false) {
        return respond(start_bg(&zip, &dest, &name, pass.as_deref()).await);
    }
    let mut cmd = Command::new("rsync");
    // -s：遠端路徑不經遠端 shell 解析（空白、特殊字元安全）
    cmd.args(["-a", "-s", "--partial", "--info=progress2", "-e", &format!("ssh {}", ssh_opts(pass.as_deref())), "--"]).arg(&zip).arg(&dest);
    apply_pass(&mut cmd, pass.as_deref());
    let (tx, rx) = tokio::sync::mpsc::channel(16);
    tokio::spawn(async move {
        let res = match tokio::time::timeout(Duration::from_secs(6 * 3600), rsync_run(cmd, &tx)).await {
            Ok(r) => r,
            Err(_) => Err("rsync 逾時".into()),
        };
        let last = match res {
            Ok(()) => format!("{{\"done\":{{\"dest\":{}}}}}", json_string(&format!("{dest}{name}"))),
            Err(e) => format!("{{\"error\":{}}}", json_string(&e)),
        };
        tx.send(line(last)).await.ok();
    });
    ndjson_response(rx)
}

async fn zip_delete(State(shared): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    respond(async {
        guard(&shared, &headers)?;
        let zip = real_zip(&shared, &str_of(&body, "path"))?;
        std::fs::remove_file(&zip).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok("{}".to_string())
    }
    .await)
}


// ---------------------------------------------------------------------------------------------
// 背景執行（tmux）
// ---------------------------------------------------------------------------------------------

fn jobs_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("lobj-view").join("sync-jobs"))
}

/// 工作 id 只允許數字（毫秒時間戳），同時也是資料夾名與 tmux session 名（`lobj-sync-<id>`）的一部分。
fn valid_job_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 20 && id.chars().all(|c| c.is_ascii_digit())
}

fn session_of(id: &str) -> String {
    format!("lobj-sync-{id}")
}

async fn tmux_available() -> bool {
    let mut cmd = Command::new("tmux");
    cmd.arg("-V");
    run(cmd, Duration::from_secs(5), "tmux").await.is_ok_and(|o| o.status.success())
}

/// 在 detached tmux session 裡執行 rsync。回傳 `{ id, session }`。
async fn start_bg(zip: &std::path::Path, dest: &str, name: &str, pass: Option<&str>) -> Result<String, Fail> {
    if cfg!(not(unix)) {
        return Err((StatusCode::NOT_IMPLEMENTED, "背景執行（tmux）只支援 Linux / macOS".into()));
    }
    if !tmux_available().await {
        return Err((StatusCode::NOT_IMPLEMENTED, "找不到 tmux，請先安裝（或取消勾選「背景執行」）".into()));
    }
    let io = |e: std::io::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0).to_string();
    let dir = jobs_dir().ok_or((StatusCode::INTERNAL_SERVER_ERROR, "找不到資料夾可存放工作紀錄".to_string()))?.join(&id);
    std::fs::create_dir_all(&dir).map_err(io)?;
    let write = |file: &str, text: &str, mode: u32| -> std::io::Result<()> {
        use std::io::Write;
        let mut o = std::fs::OpenOptions::new();
        o.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut o, mode);
        let _ = mode;
        o.open(dir.join(file))?.write_all(text.as_bytes())
    };
    let q = |p: &std::path::Path| sh_quote(&p.to_string_lossy());
    let mut script = format!("#!/bin/sh\ncd {}\n", q(&dir));
    if let Ok(sock) = std::env::var("SSH_AUTH_SOCK") {
        script += &format!("export SSH_AUTH_SOCK={}\n", sh_quote(&sock));
    }
    if let Some(p) = pass {
        write("pass", &format!("{p}\n"), 0o600).map_err(io)?;
        write("askpass.sh", &format!("#!/bin/sh\ncat {}\n", q(&dir.join("pass"))), 0o700).map_err(io)?;
        script += &format!("export SSH_ASKPASS={} SSH_ASKPASS_REQUIRE=force\n", q(&dir.join("askpass.sh")));
    }
    // tee：tmux 畫面看得到 rsync 輸出（可 attach），同時存成 log 給 UI 解析；退出碼先寫 rc 再改名 status（避免讀到寫一半的檔）
    script += &format!(
        "{{ rsync -a -s --partial --info=progress2 -e {} -- {} {}; echo $? > rc; }} 2>&1 | tee log\nmv rc status\nrm -f pass askpass.sh\n",
        sh_quote(&format!("ssh {}", ssh_opts(pass))),
        q(zip),
        sh_quote(dest)
    );
    write("run.sh", &script, 0o700).map_err(io)?;
    let meta = serde_json::json!({ "zip": zip.to_string_lossy(), "dest": format!("{dest}{name}"), "started": id });
    write("job.json", &meta.to_string(), 0o600).map_err(io)?;

    let mut cmd = Command::new("tmux");
    cmd.args(["new-session", "-d", "-s", &session_of(&id), &format!("sh {}", q(&dir.join("run.sh")))]);
    match run(cmd, Duration::from_secs(10), "tmux").await {
        Ok(o) if o.status.success() => Ok(format!("{{\"id\":\"{id}\",\"session\":{}}}", json_string(&session_of(&id)))),
        Ok(o) => {
            std::fs::remove_dir_all(&dir).ok();
            Err((StatusCode::BAD_GATEWAY, tail(&o.stderr)))
        }
        Err(e) => {
            std::fs::remove_dir_all(&dir).ok();
            Err(e)
        }
    }
}

/// log 最後幾段中，把 rsync 的進度行（`37%`）去掉，剩下的當錯誤訊息。
fn log_error(log: &[u8]) -> String {
    let s = String::from_utf8_lossy(log);
    let lines: Vec<&str> = s.split(['\r', '\n']).filter(|l| !l.trim().is_empty() && parse_progress(l).is_none()).collect();
    lines[lines.len().saturating_sub(5)..].join("\n")
}

fn read_tail(path: &std::path::Path, max: u64) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(path) else { return Vec::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    f.seek(SeekFrom::Start(len.saturating_sub(max))).ok();
    let mut v = Vec::new();
    f.read_to_end(&mut v).ok();
    v
}

async fn session_alive(id: &str) -> bool {
    let mut cmd = Command::new("tmux");
    cmd.args(["has-session", "-t", &format!("={}", session_of(id))]);
    run(cmd, Duration::from_secs(5), "tmux").await.is_ok_and(|o| o.status.success())
}

/// 工作結束或中止後不再需要密碼：刪掉暫存的 pass / askpass.sh。
fn drop_secrets(dir: &std::path::Path) {
    std::fs::remove_file(dir.join("pass")).ok();
    std::fs::remove_file(dir.join("askpass.sh")).ok();
}

async fn sync_jobs(State(shared): State<Arc<Shared>>, headers: HeaderMap) -> Response {
    respond(async {
        crate::assoc::guard_with(&shared, &headers, false, REMOTE_MSG).map_err(|(c, m)| (c, m.to_string()))?;
        let tmux = tmux_available().await;
        let mut jobs = Vec::new();
        let Some(root) = jobs_dir() else { return Ok("{\"tmux\":false,\"jobs\":[]}".to_string()) };
        let mut ids: Vec<String> =
            std::fs::read_dir(&root).map(|rd| rd.filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
        ids.retain(|i| valid_job_id(i));
        ids.sort_by(|a, b| b.cmp(a)); // 新的在前
        let week_ms = 7u128 * 24 * 3600 * 1000;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        for id in ids {
            let dir = root.join(&id);
            let meta: serde_json::Value = std::fs::read_to_string(dir.join("job.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            let status = std::fs::read_to_string(dir.join("status")).ok().map(|t| t.trim().to_string());
            let log = read_tail(&dir.join("log"), 8192);
            let (state, error) = match status.as_deref() {
                Some("0") => ("done", String::new()),
                Some("stopped") => ("stopped", String::new()),
                Some(_) => ("failed", log_error(&log)),
                None if tmux && session_alive(&id).await => ("running", String::new()),
                None => {
                    drop_secrets(&dir);
                    ("stopped", String::new()) // session 不見了又沒有 status：被手動結束或機器重開
                }
            };
            // 超過 7 天的已結束工作順手清掉
            if state != "running" && id.parse::<u128>().is_ok_and(|t| now.saturating_sub(t) > week_ms) {
                std::fs::remove_dir_all(&dir).ok();
                continue;
            }
            let (pct, speed) = if state == "done" {
                (100, String::new())
            } else {
                String::from_utf8_lossy(&log).split(['\r', '\n']).rev().find_map(parse_progress).unwrap_or((0, String::new()))
            };
            jobs.push(serde_json::json!({
                "id": id, "session": session_of(&id), "zip": meta["zip"], "dest": meta["dest"],
                "state": state, "progress": pct, "speed": speed, "error": error,
            }));
        }
        Ok(serde_json::json!({ "tmux": tmux, "jobs": jobs }).to_string())
    }
    .await)
}

async fn sync_job_stop(State(shared): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    respond(async {
        guard(&shared, &headers)?;
        let id = str_of(&body, "id");
        if !valid_job_id(&id) {
            return Err(bad("工作 id 不正確"));
        }
        let mut cmd = Command::new("tmux");
        cmd.args(["kill-session", "-t", &format!("={}", session_of(&id))]);
        run(cmd, Duration::from_secs(10), "tmux").await?; // session 已經不在也無妨，結果不檢查
        if let Some(dir) = jobs_dir().map(|d| d.join(&id)) {
            drop_secrets(&dir);
            if !dir.join("status").exists() {
                std::fs::write(dir.join("status"), "stopped").ok();
            }
        }
        Ok("{}".to_string())
    }
    .await)
}

/// 清除已結束的工作紀錄（`id` 省略＝全部）。執行中的（沒有 status 檔且 tmux session 還在）一律保留。
async fn sync_job_clear(State(shared): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    respond(async {
        guard(&shared, &headers)?;
        let Some(root) = jobs_dir() else { return Ok("{\"removed\":0}".to_string()) };
        let one = str_of(&body, "id");
        let ids: Vec<String> = if one.is_empty() {
            std::fs::read_dir(&root).map(|rd| rd.filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default()
        } else if valid_job_id(&one) {
            vec![one]
        } else {
            return Err(bad("工作 id 不正確"));
        };
        let mut removed = 0;
        for id in ids.into_iter().filter(|i| valid_job_id(i)) {
            let dir = root.join(&id);
            if !dir.join("status").exists() && session_alive(&id).await {
                continue;
            }
            drop_secrets(&dir);
            if std::fs::remove_dir_all(&dir).is_ok() {
                removed += 1;
            }
        }
        Ok(format!("{{\"removed\":{removed}}}"))
    }
    .await)
}
