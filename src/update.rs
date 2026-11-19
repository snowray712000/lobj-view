//! 線上更新：查 GitHub 最新 Release → 下載這個平台的壓縮檔 → 替換執行檔 → 重新啟動。
//!
//! - 檔名規則跟 `.github/workflows/release.yml` 一致：`lobj-view-v{版本}-{target}.zip|tar.gz`，
//!   裡面一個資料夾放 `lobj-view(.exe)`。
//! - GitHub 有提供 asset 的 `digest`（sha256）時會驗證；下載完先跑一次 `新檔 --version` 確認能執行、版本正確，
//!   才用 `self_replace` 換掉正在執行的檔案（Windows 不能直接覆蓋執行中的 exe，它會處理）。
//! - 重新啟動：先停掉伺服器、清掉 `instance.txt`，再用同樣的參數啟動新版，並以環境變數
//!   `LOBJ_VIEW_RESTART=<pid>:<port>` 告訴新程序「沿用同一個 port、不要開瀏覽器、等舊程序結束」。
//!   Unix 用 `exec`（同一個 pid，終端機的 Ctrl+C 照常有效，pid 傳 0 表示不用等）；
//!   Windows 沒有 exec，改為啟動子程序後舊程序立即結束。
//! - 網頁端的更新預設只允許本機（與副檔名關聯同樣的檢查）；監聽 0.0.0.0 時請在伺服器上執行 `lobj-view update`，
//!   或以 `--allow-remote-update` 啟動，讓區網上的網頁也能更新（仍需 `X-Lobj-Requested` 標頭）。

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::{
    Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::Notify;

use crate::instance::Shared;

const REPO: &str = "snowray712000/lobj-view";
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
const BUILD_TARGET: &str = env!("LOBJ_VIEW_TARGET");
/// 同一個伺服器查 GitHub 的結果快取多久（未登入的 GitHub API 每小時只有 60 次）。
const CACHE_TTL: Duration = Duration::from_secs(600);
const MAX_DOWNLOAD: u64 = 300 * 1024 * 1024;
const RESTART_ENV: &str = "LOBJ_VIEW_RESTART";

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub asset: Option<Asset>,
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: Option<String>,
}

/// 這個執行檔對應的 Release 檔 target（本機自己編的 linux-gnu 也用 musl 版，它是靜態連結的）。
pub fn asset_target() -> Option<&'static str> {
    let arch = BUILD_TARGET.split('-').next()?;
    let t = BUILD_TARGET;
    match (arch, t) {
        ("x86_64", t) if t.contains("windows") => Some("x86_64-pc-windows-msvc"),
        ("x86_64", t) if t.contains("linux") => Some("x86_64-unknown-linux-musl"),
        ("aarch64", t) if t.contains("linux") => Some("aarch64-unknown-linux-musl"),
        ("x86_64", t) if t.contains("apple-darwin") => Some("x86_64-apple-darwin"),
        ("aarch64", t) if t.contains("apple-darwin") => Some("aarch64-apple-darwin"),
        _ => None,
    }
}

/// `v1.8.1` / `1.8.1` → (1, 8, 1)。
pub fn parse_ver(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches('v');
    let s = s.split(['-', '+']).next()?;
    let mut it = s.split('.').map(|p| p.parse::<u64>().ok());
    let v = (it.next()??, it.next()??, it.next()??);
    it.next().is_none().then_some(v)
}

pub fn is_newer(latest: &str) -> bool {
    matches!((parse_ver(latest), parse_ver(CURRENT)), (Some(l), Some(c)) if l > c)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(600)))
        .user_agent(concat!("lobj-view/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// 查 GitHub 最新的正式 Release（不含 draft / prerelease）。會阻塞，請在 blocking 執行緒呼叫。
pub fn fetch_latest() -> Result<Release, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let mut resp = agent()
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(403 | 429) => "GitHub API 達到次數限制，請稍後再試".to_string(),
            ureq::Error::StatusCode(404) => "GitHub 上還沒有正式發佈的 Release".to_string(),
            e => format!("無法連線到 GitHub：{e}"),
        })?;
    let text = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("讀取 GitHub 回應失敗：{e}"))?;
    parse_release(&text)
}

fn parse_release(text: &str) -> Result<Release, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("GitHub 回應格式錯誤：{e}"))?;
    let tag = v["tag_name"].as_str().ok_or("GitHub 回應沒有 tag_name")?;
    let version = tag.trim_start_matches('v').to_string();
    let url = v["html_url"].as_str().unwrap_or_default().to_string();
    let asset = asset_target().and_then(|t| {
        v["assets"].as_array()?.iter().find_map(|a| {
            let name = a["name"].as_str()?;
            let ok = name.starts_with("lobj-view-")
                && name.contains(t)
                && (name.ends_with(".zip") || name.ends_with(".tar.gz"));
            ok.then(|| Asset {
                name: name.to_string(),
                url: a["browser_download_url"].as_str().unwrap_or_default().to_string(),
                size: a["size"].as_u64().unwrap_or(0),
                sha256: a["digest"]
                    .as_str()
                    .and_then(|d| d.strip_prefix("sha256:"))
                    .map(|d| d.to_ascii_lowercase()),
            })
            .filter(|a| !a.url.is_empty())
        })
    });
    Ok(Release { version, url, asset })
}

/// 下載、驗證、替換執行檔。成功回傳新版本號。會阻塞。
pub fn install(rel: &Release) -> Result<String, String> {
    let asset = rel.asset.as_ref().ok_or_else(|| no_asset_msg(&rel.version))?;
    let mut resp = agent().get(&asset.url).call().map_err(|e| format!("下載 {} 失敗：{e}", asset.name))?;
    let data = resp
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD)
        .read_to_vec()
        .map_err(|e| format!("下載 {} 失敗：{e}", asset.name))?;
    if asset.size != 0 && data.len() as u64 != asset.size {
        return Err(format!("下載的檔案大小不符（{} / {} bytes）", data.len(), asset.size));
    }
    if let Some(want) = &asset.sha256 {
        let got: String = Sha256::digest(&data).iter().map(|b| format!("{b:02x}")).collect();
        if &got != want {
            return Err("下載的檔案 sha256 不符，已放棄更新".into());
        }
    }
    let exe = extract_exe(&asset.name, &data)?;

    // 先寫到暫存檔並試跑，確定是能執行、版本正確的 lobj-view
    let tmp = std::env::temp_dir().join(format!("lobj-view-update-{}{}", std::process::id(), std::env::consts::EXE_SUFFIX));
    std::fs::write(&tmp, &exe).map_err(|e| format!("寫入暫存檔失敗：{e}"))?;
    let result = verify_and_replace(&tmp, &rel.version);
    let _ = std::fs::remove_file(&tmp);
    result.map(|_| rel.version.clone())
}

fn verify_and_replace(tmp: &Path, version: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(tmp, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    let out = std::process::Command::new(tmp)
        .arg("--version")
        .output()
        .map_err(|e| format!("新版無法執行：{e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() || !text.contains(version) {
        return Err(format!("新版自我檢查失敗（--version 輸出：{}）", text.trim()));
    }
    self_replace::self_replace(tmp).map_err(|e| format!("替換執行檔失敗（可能沒有寫入權限）：{e}"))
}

fn extract_exe(name: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let want = format!("lobj-view{}", std::env::consts::EXE_SUFFIX);
    let is_want = |p: &str| p.rsplit(['/', '\\']).next() == Some(want.as_str());
    let mut out = Vec::new();
    if name.ends_with(".zip") {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(data)).map_err(|e| format!("壓縮檔損壞：{e}"))?;
        for i in 0..z.len() {
            let mut f = z.by_index(i).map_err(|e| format!("壓縮檔損壞：{e}"))?;
            if f.is_file() && f.name().is_ok_and(|n| is_want(&n)) {
                f.read_to_end(&mut out).map_err(|e| format!("解壓失敗：{e}"))?;
                return Ok(out);
            }
        }
    } else {
        let mut ar = tar::Archive::new(flate2::read::GzDecoder::new(data));
        for entry in ar.entries().map_err(|e| format!("壓縮檔損壞：{e}"))? {
            let mut entry = entry.map_err(|e| format!("壓縮檔損壞：{e}"))?;
            let path = entry.path().map_err(|e| e.to_string())?.to_string_lossy().into_owned();
            if entry.header().entry_type().is_file() && is_want(&path) {
                entry.read_to_end(&mut out).map_err(|e| format!("解壓失敗：{e}"))?;
                return Ok(out);
            }
        }
    }
    Err(format!("{name} 裡找不到 {want}"))
}

fn no_asset_msg(version: &str) -> String {
    match asset_target() {
        Some(t) => format!("v{version} 還沒有 {t} 的檔案（GitHub 可能還在建置，請過幾分鐘再試）"),
        None => format!("沒有這個平台（{BUILD_TARGET}）的預編譯檔，請用 cargo install 更新"),
    }
}

// ---------------------------------------------------------------- 重新啟動

static EXE: OnceLock<PathBuf> = OnceLock::new();
static RESTART: LazyLock<Notify> = LazyLock::new(Notify::new);

/// 啟動時記下執行檔路徑：替換之後 Linux 的 `current_exe()` 會變成「… (deleted)」。
pub fn remember_exe() {
    if let Ok(p) = std::env::current_exe() {
        let _ = EXE.set(p);
    }
}

/// 更新後重新啟動時，舊程序傳來的 (pid, port)。
pub fn restart_info() -> Option<(u32, u16)> {
    let v = std::env::var(RESTART_ENV).ok()?;
    let (pid, port) = v.split_once(':')?;
    Some((pid.parse().ok()?, port.parse().ok()?))
}

/// 等舊程序結束（它要先放掉 port 與 instance.txt），最多 15 秒。
pub fn wait_pid_exit(pid: u32) {
    if pid == 0 || pid == std::process::id() {
        return;
    }
    use sysinfo::{Pid, ProcessesToUpdate, System};
    let pid = Pid::from_u32(pid);
    let Ok(mut sys) = System::new() else {
        std::thread::sleep(Duration::from_secs(2)); // 查不到行程時至少等一下
        return;
    };
    let end = Instant::now() + Duration::from_secs(15);
    while Instant::now() < end {
        sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
        if sys.process(pid).is_none() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// 伺服器主迴圈等這個；網頁按下更新並成功替換後觸發。
pub async fn restart_requested() {
    RESTART.notified().await
}

/// 以同樣的參數啟動新版並結束這個程序（呼叫前請先停掉伺服器、清掉登記）。
pub fn relaunch(port: u16) -> ! {
    let exe = EXE.get().cloned().or_else(|| std::env::current_exe().ok()).expect("找不到執行檔路徑");
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(std::env::args_os().skip(1));
    println!("lobj-view 已更新，重新啟動中…");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.env(RESTART_ENV, format!("0:{port}"));
        let e = cmd.exec(); // 成功就不會回來
        eprintln!("重新啟動失敗：{e}，請手動執行 {}", exe.display());
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    {
        cmd.env(RESTART_ENV, format!("{}:{port}", std::process::id()));
        match cmd.spawn() {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("重新啟動失敗：{e}，請手動執行 {}", exe.display());
                std::process::exit(1);
            }
        }
    }
}

// ---------------------------------------------------------------- 網頁端

/// `GET /api/update`：查詢有沒有新版（`?force=1` 不用快取）；`POST /api/update`：下載、替換、重新啟動。
pub fn router(shared: Arc<Shared>) -> Router {
    Router::new()
        .route("/api/update", get(get_check).post(post_apply))
        .with_state(shared)
}

static CACHE: Mutex<Option<(Instant, Release)>> = Mutex::new(None);
static BUSY: AtomicBool = AtomicBool::new(false);

const REMOTE_MSG: &str =
    "伺服器監聽非本機位址，網頁預設不開放更新（請在伺服器上執行 lobj-view update，或以 --allow-remote-update 啟動）";

/// `--allow-remote-update`：監聽非本機位址時也讓網頁更新（預設關閉）。
static ALLOW_REMOTE: AtomicBool = AtomicBool::new(false);

pub fn set_allow_remote(v: bool) {
    ALLOW_REMOTE.store(v, Ordering::Relaxed);
}

/// 預設與副檔名關聯相同（只限本機）；`--allow-remote-update` 時放寬來源與 Host，
/// 但寫入仍要 `X-Lobj-Requested` 標頭（擋其他網站的 CSRF）。
fn guard(shared: &Shared, headers: &HeaderMap, write: bool) -> Result<(), (StatusCode, &'static str)> {
    if !ALLOW_REMOTE.load(Ordering::Relaxed) {
        return crate::assoc::guard_with(shared, headers, write, REMOTE_MSG);
    }
    if write && headers.get("x-lobj-requested").and_then(|h| h.to_str().ok()) != Some("1") {
        return Err((StatusCode::FORBIDDEN, "缺少 X-Lobj-Requested 標頭"));
    }
    Ok(())
}

async fn latest_cached(force: bool) -> Result<Release, String> {
    if !force
        && let Some((t, r)) = CACHE.lock().unwrap().as_ref()
        && t.elapsed() < CACHE_TTL
    {
        return Ok(r.clone());
    }
    let r = tokio::task::spawn_blocking(fetch_latest)
        .await
        .map_err(|e| e.to_string())??;
    *CACHE.lock().unwrap() = Some((Instant::now(), r.clone()));
    Ok(r)
}

async fn get_check(State(shared): State<Arc<Shared>>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let force = q.get("force").is_some_and(|v| v == "1");
    match latest_cached(force).await {
        Ok(r) => {
            let newer = is_newer(&r.version);
            let blocker = if !newer {
                None
            } else if let Err((_, m)) = guard(&shared, &headers, false) {
                Some(m.to_string())
            } else if r.asset.is_none() {
                Some(no_asset_msg(&r.version))
            } else {
                None
            };
            json_ok(json!({
                "current": CURRENT,
                "latest": r.version,
                "newer": newer,
                "url": r.url,
                "canApply": newer && blocker.is_none(),
                "reason": blocker,
            }))
        }
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

async fn post_apply(State(shared): State<Arc<Shared>>, headers: HeaderMap) -> Response {
    if let Err(e) = guard(&shared, &headers, true) {
        return e.into_response();
    }
    if BUSY.swap(true, Ordering::SeqCst) {
        return (StatusCode::CONFLICT, "已經在更新中").into_response();
    }
    let result = async {
        let rel = latest_cached(true).await?;
        if !is_newer(&rel.version) {
            return Err(format!("目前的 v{CURRENT} 已是最新版"));
        }
        tokio::task::spawn_blocking(move || install(&rel)).await.map_err(|e| e.to_string())?
    }
    .await;
    match result {
        Ok(v) => {
            // 讓回應先送出去，再停伺服器重新啟動
            tokio::spawn(async {
                tokio::time::sleep(Duration::from_millis(500)).await;
                RESTART.notify_one();
            });
            json_ok(json!({ "version": v }))
        }
        Err(e) => {
            BUSY.store(false, Ordering::SeqCst);
            (StatusCode::BAD_GATEWAY, e).into_response()
        }
    }
}

fn json_ok(v: serde_json::Value) -> Response {
    ([(header::CONTENT_TYPE, "application/json")], v.to_string()).into_response()
}

// ---------------------------------------------------------------- 命令列

/// `lobj-view update [--check]`
pub fn run_cli(check_only: bool) -> anyhow::Result<()> {
    println!("目前版本: v{CURRENT}");
    let rel = fetch_latest().map_err(anyhow::Error::msg)?;
    if !is_newer(&rel.version) {
        println!("已是最新版（GitHub 最新為 v{}）", rel.version);
        return Ok(());
    }
    println!("有新版: v{}  {}", rel.version, rel.url);
    if check_only {
        println!("執行 lobj-view update 進行更新");
        return Ok(());
    }
    println!("下載並替換中…");
    let v = install(&rel).map_err(anyhow::Error::msg)?;
    println!("已更新到 v{v}。執行中的 lobj-view 要重新啟動才會換成新版。");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(parse_ver("v1.8.1"), Some((1, 8, 1)));
        assert_eq!(parse_ver("1.10.0-rc1"), Some((1, 10, 0)));
        assert_eq!(parse_ver("1.8"), None);
        assert!(is_newer("v999.0.1"));
        assert!(!is_newer(CURRENT));
        assert!(!is_newer("0.0.1"));
    }

    #[test]
    fn release_json() {
        let t = asset_target().unwrap_or("x86_64-pc-windows-msvc");
        let ext = if t.contains("windows") { "zip" } else { "tar.gz" };
        let text = format!(
            r#"{{"tag_name":"v9.9.1","html_url":"https://example/r","assets":[
              {{"name":"lobj-view-v9.9.1-other-target.zip","browser_download_url":"x","size":1}},
              {{"name":"lobj-view-v9.9.1-{t}.{ext}","browser_download_url":"https://example/a","size":42,"digest":"sha256:ABCD"}}]}}"#
        );
        let r = parse_release(&text).unwrap();
        assert_eq!(r.version, "9.9.1");
        if asset_target().is_some() {
            let a = r.asset.unwrap();
            assert_eq!(a.url, "https://example/a");
            assert_eq!(a.size, 42);
            assert_eq!(a.sha256.as_deref(), Some("abcd"));
        }
    }
}
