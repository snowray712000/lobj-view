//! 把本機的 `.zip` 用 rsync（經 ssh）同步到另一台電腦，以及刪除 `.zip`。
//!
//! - `POST /api/remote-ls` `{ host, path }`：用 ssh 列出遠端資料夾（只列子資料夾），給前端的目錄選擇用。
//!   `path` 空字串＝遠端家目錄；回傳 `{ path, dirs }`，`path` 是遠端 `pwd` 的結果，`writable` 表示目前帳號能否寫入該資料夾。
//! - `POST /api/zip-sync` `{ path, host, dir, passphrase? }`：`rsync -a` 把該 zip 傳到 `host:dir/`，回應是 NDJSON 進度串流（見 `zip_sync`）。
//! - `POST /api/zip-delete` `{ path }`：刪除該 zip（只刪 `.zip` 檔）。
//!
//! 都只在伺服器監聽本機位址時開放，且需 `X-Lobj-Requested`。ssh 預設 `BatchMode=yes`：沒有可用的金鑰就直接失敗，
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
    routing::post,
};
use tokio::process::Command;

use crate::instance::Shared;
use crate::lobj_api::{LineTx, json_string, line, ndjson_response, resolve_ex};

const REMOTE_MSG: &str = "同步與刪除只在伺服器監聽本機位址時開放";

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
