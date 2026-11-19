//! 副檔名關聯：把 lobj-view 註冊成 `.lobj` `.lobj2` `.stl` `.ply` `.obj` 的開啟程式。
//!
//! 目前只實作 Windows（寫 `HKCU\Software\Classes`，不需要系統管理員）；其他平台回報「尚未支援」。
//!
//! 註冊內容：
//! - ProgId `lobj-view.file`，開啟指令為 `"<這個 exe>" "%1"`（即 1.2.2 的單一實例流程）。
//! - 每個副檔名：在 `.ext\OpenWithProgids` 加入 ProgId → 成為「開啟方式」的候選。
//! - 只有該副檔名**目前沒有任何預設程式**時，才把 `.ext` 的預設值設成 ProgId（`.lobj` / `.lobj2` 通常屬於這種）。
//!   已被其他程式佔用的（如 `.stl` `.obj`）不會被搶走：Windows 10 的 `UserChoice` 有雜湊保護，
//!   程式無法靜默覆蓋，只能由使用者在「設定 > 預設應用程式」自己指定。
//!
//! **還原**：第一次註冊某個副檔名前，會把原狀記在 `HKCU\Software\lobj-view\AssocBackup\.ext`
//! （`.ext` 機碼原本是否存在、類別預設值、`OpenWithProgids` 是否已含我們、當時生效的預設程式）。
//! `unregister` 依記錄還原：原本沒有的機碼就刪掉，原本有的值就寫回。唯一還原不了的是 `UserChoice`
//! （使用者在系統設定指定 lobj-view 後由 Windows 寫入，有雜湊保護）：這時會提示原本的預設程式，請使用者自己改回。
//!
//! 網頁設定頁與 CLI（`register` / `unregister` / `status`）共用這裡的邏輯。網頁端的寫入端點只在
//! 監聽本機位址時啟用，並檢查 `Host` 與自訂標頭（見 [`router`]）。

use std::sync::Arc;

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::{Value, json};

use crate::instance::Shared;

pub const PROG_ID: &str = "lobj-view.file";
/// 可註冊的副檔名（不含點）。
pub const ALL_EXTS: [&str; 5] = ["lobj", "lobj2", "stl", "ply", "obj"];
/// 不指定時預設處理的副檔名：這兩個幾乎不會與其他程式衝突。
pub const DEFAULT_EXTS: [&str; 2] = ["lobj", "lobj2"];

#[derive(Clone, Debug)]
pub struct ExtStatus {
    pub ext: String,
    /// `OpenWithProgids` 已含 lobj-view（至少是「開啟方式」候選）。
    pub registered: bool,
    /// 目前實際生效的預設程式就是 lobj-view。
    pub is_default: bool,
    /// 目前生效的預設程式 ProgId（沒有則 None）。
    pub current: Option<String>,
    /// 註冊前已記錄原狀（取消註冊時會還原）。
    pub has_backup: bool,
    /// 註冊前生效的預設程式 ProgId（記錄時就沒有、或就是 lobj-view 則為 None）。
    pub previous: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Status {
    pub supported: bool,
    pub exe: String,
    pub exts: Vec<ExtStatus>,
    /// 操作後仍需要使用者處理的事項（例如要到系統設定改回原本的預設程式）。
    pub notes: Vec<String>,
}

impl Status {
    fn to_json(&self) -> Value {
        json!({
            "supported": self.supported,
            "exe": self.exe,
            "exts": self.exts.iter().map(|e| json!({
                "ext": e.ext, "registered": e.registered, "isDefault": e.is_default, "current": e.current,
                "hasBackup": e.has_backup, "previous": e.previous,
            })).collect::<Vec<_>>(),
            "notes": self.notes,
        })
    }
}

/// 驗證並正規化副檔名（去掉開頭的點、轉小寫、去重），不認得的直接報錯。
pub fn normalize_exts<S: AsRef<str>>(input: &[S]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for s in input {
        let e = s.as_ref().trim().trim_start_matches('.').to_ascii_lowercase();
        if !ALL_EXTS.contains(&e.as_str()) {
            return Err(format!("不支援的副檔名: {}（可用: {}）", s.as_ref(), ALL_EXTS.join(", ")));
        }
        if !out.contains(&e) {
            out.push(e);
        }
    }
    Ok(out)
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::borrow::Cow;
    use winreg::enums::*;
    use winreg::{RegKey, RegValue};

    const CLASSES: &str = r"Software\Classes";
    const BACKUP: &str = r"Software\lobj-view\AssocBackup";

    fn hkcu() -> RegKey {
        RegKey::predef(HKEY_CURRENT_USER)
    }

    fn exe_path() -> Result<String, String> {
        std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string())
    }

    /// 目前生效的預設程式 ProgId：先看使用者的選擇（UserChoice），再看 HKCU / HKLM 的類別預設值。
    fn effective(ext: &str) -> Option<String> {
        let user_choice = format!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.{ext}\UserChoice");
        if let Ok(k) = hkcu().open_subkey(user_choice) {
            if let Ok(v) = k.get_value::<String, _>("ProgId") {
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
        for root in [hkcu(), RegKey::predef(HKEY_LOCAL_MACHINE)] {
            if let Ok(k) = root.open_subkey(format!(r"{CLASSES}\.{ext}")) {
                if let Ok(v) = k.get_value::<String, _>("") {
                    if !v.is_empty() {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    fn has_open_with(ext: &str) -> bool {
        hkcu()
            .open_subkey(format!(r"{CLASSES}\.{ext}\OpenWithProgids"))
            .map(|k| k.get_raw_value(PROG_ID).is_ok())
            .unwrap_or(false)
    }

    pub fn status() -> Result<Status, String> {
        let exts = ALL_EXTS
            .iter()
            .map(|e| {
                let current = effective(e);
                let backup = backup_read(e);
                ExtStatus {
                    ext: e.to_string(),
                    registered: has_open_with(e),
                    is_default: current.as_deref() == Some(PROG_ID),
                    current,
                    has_backup: backup.is_some(),
                    previous: backup.and_then(|b| b.prev_effective),
                }
            })
            .collect();
        Ok(Status { supported: true, exe: exe_path()?, exts, notes: Vec::new() })
    }

    /// 註冊前的原狀。
    struct Backup {
        ext_existed: bool,
        had_open_with: bool,
        class_default: Option<String>,
        prev_effective: Option<String>,
    }

    fn backup_path(ext: &str) -> String {
        format!(r"{BACKUP}\.{ext}")
    }

    fn backup_read(ext: &str) -> Option<Backup> {
        let k = hkcu().open_subkey(backup_path(ext)).ok()?;
        Some(Backup {
            ext_existed: k.get_value::<u32, _>("ExtKeyExisted").unwrap_or(1) != 0,
            had_open_with: k.get_value::<u32, _>("HadOpenWith").unwrap_or(0) != 0,
            class_default: k.get_value::<String, _>("ClassDefault").ok(),
            prev_effective: k.get_value::<String, _>("PrevEffective").ok(),
        })
    }

    /// 只在第一次記錄；已有記錄就不覆蓋（否則會把「已註冊後」的狀態誤當成原狀）。
    fn backup_save(ext: &str) -> Result<(), String> {
        let root = hkcu();
        if root.open_subkey(backup_path(ext)).is_ok() {
            return Ok(());
        }
        let class_key = root.open_subkey(format!(r"{CLASSES}\.{ext}")).ok();
        let class_default = class_key.as_ref().and_then(|k| k.get_value::<String, _>("").ok());
        let prev = effective(ext).filter(|p| p != PROG_ID);
        let (k, _) = root.create_subkey(backup_path(ext)).map_err(io)?;
        k.set_value("ExtKeyExisted", &(class_key.is_some() as u32)).map_err(io)?;
        k.set_value("HadOpenWith", &(has_open_with(ext) as u32)).map_err(io)?;
        if let Some(v) = class_default {
            k.set_value("ClassDefault", &v).map_err(io)?;
        }
        if let Some(v) = prev {
            k.set_value("PrevEffective", &v).map_err(io)?;
        }
        Ok(())
    }

    fn notify() {
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn SHChangeNotify(event: i32, flags: u32, a: *const core::ffi::c_void, b: *const core::ffi::c_void);
        }
        // SHCNE_ASSOCCHANGED, SHCNF_IDLIST：讓檔案總管重新讀取關聯與圖示
        unsafe { SHChangeNotify(0x0800_0000, 0, std::ptr::null(), std::ptr::null()) };
    }

    fn io(e: std::io::Error) -> String {
        e.to_string()
    }

    pub fn register(exts: &[String]) -> Result<Status, String> {
        let exe = exe_path()?;
        let root = hkcu();
        let (prog, _) = root.create_subkey(format!(r"{CLASSES}\{PROG_ID}")).map_err(io)?;
        prog.set_value("", &"lobj-view 3D 模型").map_err(io)?;
        let (icon, _) = prog.create_subkey("DefaultIcon").map_err(io)?;
        icon.set_value("", &format!("\"{exe}\",0")).map_err(io)?;
        let (cmd, _) = prog.create_subkey(r"shell\open\command").map_err(io)?;
        cmd.set_value("", &format!("\"{exe}\" \"%1\"")).map_err(io)?;

        for ext in exts {
            backup_save(ext)?; // 動任何東西之前先記錄原狀
            let (key, _) = root.create_subkey(format!(r"{CLASSES}\.{ext}")).map_err(io)?;
            let (owp, _) = key.create_subkey("OpenWithProgids").map_err(io)?;
            owp.set_raw_value(PROG_ID, &RegValue { bytes: Cow::Borrowed(&[]), vtype: REG_NONE })
                .map_err(io)?;
            // 沒有任何預設程式時才當預設，不搶別人的
            if effective(ext).is_none() {
                key.set_value("", &PROG_ID).map_err(io)?;
            }
        }
        notify();
        status()
    }

    /// 取消註冊並依註冊前的記錄還原。沒有記錄（1.2.4 以前註冊的）就只移除我們加的部分。
    pub fn unregister(exts: &[String]) -> Result<Status, String> {
        let root = hkcu();
        let mut notes = Vec::new();
        for ext in exts {
            // 沒有記錄時的假設：機碼原本就在、OpenWithProgids 原本沒有我們、原本沒有類別預設值
            let b = backup_read(ext).unwrap_or(Backup {
                ext_existed: true,
                had_open_with: false,
                class_default: None,
                prev_effective: None,
            });

            let owp_path = format!(r"{CLASSES}\.{ext}\OpenWithProgids");
            if !b.had_open_with {
                if let Ok(owp) = root.open_subkey_with_flags(&owp_path, KEY_SET_VALUE | KEY_READ) {
                    let _ = owp.delete_value(PROG_ID);
                    let empty = owp.enum_values().next().is_none() && owp.enum_keys().next().is_none();
                    drop(owp);
                    if empty {
                        let _ = root.delete_subkey(&owp_path);
                    }
                }
            }
            let class_path = format!(r"{CLASSES}\.{ext}");
            if let Ok(key) = root.open_subkey_with_flags(&class_path, KEY_READ | KEY_SET_VALUE) {
                if key.get_value::<String, _>("").is_ok_and(|v| v == PROG_ID) {
                    match &b.class_default {
                        Some(v) if v != PROG_ID => key.set_value("", v).map_err(io)?,
                        _ => {
                            let _ = key.delete_value("");
                        }
                    }
                }
                let empty = key.enum_values().next().is_none() && key.enum_keys().next().is_none();
                drop(key);
                // 這個機碼是我們建立的、現在也空了 → 刪掉，回到註冊前
                if empty && !b.ext_existed {
                    let _ = root.delete_subkey(&class_path);
                }
            }
            // UserChoice 有雜湊保護，只能由使用者在系統設定改
            if effective(ext).as_deref() == Some(PROG_ID) {
                notes.push(match &b.prev_effective {
                    Some(p) => format!(".{ext} 目前的預設程式仍是 lobj-view（你在系統設定指定過）；原本是 {p}，請到 Windows「設定 > 預設應用程式」改回"),
                    None => format!(".{ext} 目前的預設程式仍是 lobj-view（你在系統設定指定過），請到 Windows「設定 > 預設應用程式」改選其他程式"),
                });
            }
            let _ = root.delete_subkey_all(backup_path(ext));
        }
        // 沒有任何記錄了，就把備份機碼一併清掉
        if root.open_subkey(BACKUP).is_ok_and(|k| k.enum_keys().next().is_none()) {
            let _ = root.delete_subkey(BACKUP);
            if root.open_subkey(r"Software\lobj-view").is_ok_and(|k| k.enum_keys().next().is_none() && k.enum_values().next().is_none()) {
                let _ = root.delete_subkey(r"Software\lobj-view");
            }
        }
        // 已經沒有任何副檔名指向它，就把 ProgId 一併移除
        if !ALL_EXTS.iter().any(|e| has_open_with(e)) {
            let _ = root.delete_subkey_all(format!(r"{CLASSES}\{PROG_ID}"));
        }
        notify();
        let mut st = status()?;
        st.notes = notes;
        Ok(st)
    }

    pub fn open_default_apps_settings() -> Result<(), String> {
        open::that_detached("ms-settings:defaultapps").map_err(io)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    const MSG: &str = "目前只支援 Windows 的副檔名關聯（Linux / macOS 尚未支援）";

    pub fn status() -> Result<Status, String> {
        Ok(Status {
            supported: false,
            exe: std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
            exts: ALL_EXTS
                .iter()
                .map(|e| ExtStatus {
                    ext: e.to_string(),
                    registered: false,
                    is_default: false,
                    current: None,
                    has_backup: false,
                    previous: None,
                })
                .collect(),
            notes: Vec::new(),
        })
    }
    pub fn register(_: &[String]) -> Result<Status, String> {
        Err(MSG.into())
    }
    pub fn unregister(_: &[String]) -> Result<Status, String> {
        Err(MSG.into())
    }
    pub fn open_default_apps_settings() -> Result<(), String> {
        Err(MSG.into())
    }
}

pub use imp::{open_default_apps_settings, register, status, unregister};

/// CLI 輸出用的狀態表。
pub fn format_status(s: &Status) -> String {
    let mut out = format!("執行檔: {}\n", s.exe);
    for e in &s.exts {
        let state = if e.is_default {
            "✓ 預設開啟程式".to_string()
        } else if e.registered {
            match &e.current {
                Some(c) => format!("已註冊（目前預設: {c}；請到 Windows「設定 > 預設應用程式」改選）"),
                None => "已註冊".to_string(),
            }
        } else {
            match &e.current {
                Some(c) => format!("未註冊（目前預設: {c}）"),
                None => "未註冊".to_string(),
            }
        };
        let restore = match (e.has_backup, &e.previous) {
            (true, Some(p)) => format!("  [取消註冊會還原為: {p}]"),
            (true, None) => "  [取消註冊會還原為: 無預設]".to_string(),
            _ => String::new(),
        };
        out.push_str(&format!("  .{:<6} {state}{restore}\n", e.ext));
    }
    for n in &s.notes {
        out.push_str(&format!("\n注意: {n}"));
    }
    if !s.notes.is_empty() {
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------- 網頁端

/// 讓網頁設定頁能查詢／變更關聯。
///
/// 寫入註冊表等於改系統設定，所以：
/// - 伺服器必須只監聽本機位址（`--host 0.0.0.0` 時一律拒絕，區網上的人不能動你的註冊表）；
/// - `Host` 必須是 localhost / 127.0.0.1 / ::1（擋 DNS rebinding）；
/// - POST 必須帶 `X-Lobj-Requested: 1`（自訂標頭會觸發 CORS 預檢，本伺服器不回應預檢，其他網站的頁面送不出來，擋 CSRF）。
pub fn router(shared: Arc<Shared>) -> Router {
    Router::new()
        .route("/api/assoc", get(get_status).post(post_assoc))
        .route("/api/assoc/settings", post(post_settings))
        .with_state(shared)
}

type ApiErr = (StatusCode, &'static str);

fn guard(shared: &Shared, headers: &HeaderMap, write: bool) -> Result<(), ApiErr> {
    if !shared.local_only() {
        return Err((StatusCode::FORBIDDEN, "伺服器監聽非本機位址，不開放變更副檔名關聯（請用 lobj-view register）"));
    }
    let host = headers.get(header::HOST).and_then(|h| h.to_str().ok()).unwrap_or("");
    let name = host.rsplit_once(':').map_or(host, |(h, p)| if p.chars().all(|c| c.is_ascii_digit()) { h } else { host });
    if !matches!(name, "localhost" | "127.0.0.1" | "[::1]") {
        return Err((StatusCode::FORBIDDEN, "Host 不是本機位址"));
    }
    if write && headers.get("x-lobj-requested").and_then(|h| h.to_str().ok()) != Some("1") {
        return Err((StatusCode::FORBIDDEN, "缺少 X-Lobj-Requested 標頭"));
    }
    Ok(())
}

fn json_response(r: Result<Status, String>) -> Response {
    match r {
        Ok(s) => ([(header::CONTENT_TYPE, "application/json")], s.to_json().to_string()).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

async fn get_status(State(shared): State<Arc<Shared>>, headers: HeaderMap) -> Response {
    if let Err(e) = guard(&shared, &headers, false) {
        return e.into_response();
    }
    json_response(tokio::task::spawn_blocking(status).await.unwrap_or_else(|e| Err(e.to_string())))
}

async fn post_assoc(State(shared): State<Arc<Shared>>, headers: HeaderMap, body: Bytes) -> Response {
    if let Err(e) = guard(&shared, &headers, true) {
        return e.into_response();
    }
    let parsed = serde_json::from_slice::<Value>(&body).ok();
    let action = parsed.as_ref().and_then(|v| v["action"].as_str()).unwrap_or("").to_string();
    let exts: Vec<String> = parsed
        .as_ref()
        .and_then(|v| v["exts"].as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let exts = match normalize_exts(&exts) {
        Ok(e) if !e.is_empty() => e,
        Ok(_) => return (StatusCode::BAD_REQUEST, "請至少選一個副檔名").into_response(),
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    let result = tokio::task::spawn_blocking(move || match action.as_str() {
        "register" => register(&exts),
        "unregister" => unregister(&exts),
        _ => Err("action 只能是 register 或 unregister".to_string()),
    })
    .await
    .unwrap_or_else(|e| Err(e.to_string()));
    json_response(result)
}

async fn post_settings(State(shared): State<Arc<Shared>>, headers: HeaderMap) -> Response {
    if let Err(e) = guard(&shared, &headers, true) {
        return e.into_response();
    }
    match open_default_apps_settings() {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exts_normalize() {
        assert_eq!(normalize_exts(&[".LOBJ", "lobj", "Stl"]).unwrap(), vec!["lobj", "stl"]);
        assert!(normalize_exts(&["exe"]).is_err());
    }

    #[test]
    fn guard_rules() {
        let sh = Shared::new(std::env::temp_dir(), None);
        let mut h = HeaderMap::new();
        h.insert(header::HOST, "localhost:8080".parse().unwrap());
        assert!(guard(&sh, &h, false).is_err()); // 預設不是 local_only
        sh.set_local_only(true);
        assert!(guard(&sh, &h, false).is_ok());
        assert!(guard(&sh, &h, true).is_err()); // 缺自訂標頭
        h.insert("x-lobj-requested", "1".parse().unwrap());
        assert!(guard(&sh, &h, true).is_ok());
        h.insert(header::HOST, "evil.example:8080".parse().unwrap());
        assert!(guard(&sh, &h, true).is_err());
        h.insert(header::HOST, "[::1]:8080".parse().unwrap());
        assert!(guard(&sh, &h, true).is_ok());
    }
}
