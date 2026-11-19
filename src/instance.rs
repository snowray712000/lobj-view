//! 單一實例：同一台電腦只跑一個 lobj-view，之後再開檔案就交給它。
//!
//! 流程（`lobj-view <檔案或資料夾>`，例如在檔案總管雙擊 `.lobj`）：
//! 1. 讀 `<data_local_dir>/lobj-view/instance.txt`（port、token、pid），連到執行中那個實例的 IPC；
//!    送 `OPEN\t<token>\t<絕對路徑>\n`，收到 `OK` 就結束（實例會切換資料夾、通知網頁；
//!    若沒有網頁連著，才自動開瀏覽器）。
//! 2. 沒有可用的實例，就自己當伺服器：啟動後開 IPC、寫 `instance.txt`。
//!
//! IPC 刻意不走 HTTP API：它只聽 `127.0.0.1`、需要 token（檔案只有本機使用者讀得到），
//! 所以區網或網頁（含 DNS rebinding、CSRF）都沒有辦法改變伺服器的根資料夾。
//!
//! 同時啟動多個程序（檔案總管多選後開啟）時，用 `instance.lock` 避免同時都變成伺服器。

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{self, BufRead, BufReader as StdBufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream as AsyncTcpStream};
use tokio::sync::watch;

use crate::lobj_api::is_lobj_name;

/// 通知前端「有人要開這個」。`seq` 為 0 表示還沒有人開過（前端照一般流程瀏覽根目錄）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenEvent {
    pub seq: u64,
    /// 相對新根資料夾的路徑：檔案名稱，或資料夾時為空字串。
    pub path: String,
}

/// 伺服器端共用狀態：目前的根資料夾（可被 IPC 切換）、開檔事件、是否有網頁連著。
pub struct Shared {
    root: RwLock<PathBuf>,
    tx: watch::Sender<OpenEvent>,
    tabs: AtomicUsize,
    last_browser: Mutex<Option<Instant>>,
    url: Mutex<String>,
    local_only: AtomicBool,
}

impl Shared {
    /// `initial`：啟動時就指定了檔案（`Some(檔名)`）或資料夾（`Some("")`），前端第一次連線就會跳過去。
    pub fn new(root: PathBuf, initial: Option<String>) -> Arc<Self> {
        let root = std::fs::canonicalize(&root).unwrap_or(root);
        let ev = match initial {
            Some(path) => OpenEvent { seq: 1, path },
            None => OpenEvent { seq: 0, path: String::new() },
        };
        Arc::new(Shared {
            root: RwLock::new(root),
            tx: watch::channel(ev).0,
            tabs: AtomicUsize::new(0),
            last_browser: Mutex::new(None),
            url: Mutex::new(String::new()),
            local_only: AtomicBool::new(false),
        })
    }

    pub fn root(&self) -> PathBuf {
        self.root.read().unwrap().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<OpenEvent> {
        self.tx.subscribe()
    }

    /// 伺服器是否只監聽本機位址。會改系統設定的端點（副檔名關聯）只在這時開放。
    pub fn set_local_only(&self, v: bool) {
        self.local_only.store(v, Ordering::Relaxed);
    }

    pub fn local_only(&self) -> bool {
        self.local_only.load(Ordering::Relaxed)
    }

    pub fn set_url(&self, url: String) {
        *self.url.lock().unwrap() = url;
    }

    /// 網頁每開一條事件連線就拿一個；連線結束（含分頁關閉）時自動歸還。
    pub fn tab_guard(self: &Arc<Self>) -> TabGuard {
        self.tabs.fetch_add(1, Ordering::Relaxed);
        TabGuard(self.clone())
    }

    /// 開預設瀏覽器。剛開過（瀏覽器還在啟動、網頁還沒連上）的 10 秒內不重複開。
    pub fn open_browser(&self) {
        let mut last = self.last_browser.lock().unwrap();
        if last.is_some_and(|t| t.elapsed() < Duration::from_secs(10)) {
            return;
        }
        let url = self.url.lock().unwrap().clone();
        if url.is_empty() {
            return;
        }
        *last = Some(Instant::now());
        let _ = open::that_detached(url);
    }

    /// 把根資料夾切到 `path`（檔案則為其所在資料夾），通知網頁，必要時開瀏覽器。
    pub fn apply_open(&self, path: &str) -> Result<(), String> {
        let real = std::fs::canonicalize(path).map_err(|e| format!("無法開啟 {path}: {e}"))?;
        let (dir, sel) = split_target(&real)?;
        *self.root.write().unwrap() = dir;
        self.tx.send_modify(|ev| {
            ev.seq += 1;
            ev.path = sel.unwrap_or_default();
        });
        if self.tabs.load(Ordering::Relaxed) == 0 {
            self.open_browser();
        }
        Ok(())
    }
}

pub struct TabGuard(Arc<Shared>);

impl Drop for TabGuard {
    fn drop(&mut self) {
        self.0.tabs.fetch_sub(1, Ordering::Relaxed);
    }
}

/// 檔案 → (所在資料夾, Some(檔名))；資料夾 → (自己, None)。檔案必須是支援的副檔名。
pub fn split_target(real: &Path) -> Result<(PathBuf, Option<String>), String> {
    if real.is_file() {
        let name = real.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !is_lobj_name(&name) {
            return Err(format!("不是 .lobj / .lobj2 / .stl / .obj / .ply 檔: {name}"));
        }
        let dir = real.parent().ok_or("找不到所在資料夾")?.to_path_buf();
        Ok((dir, Some(name)))
    } else if real.is_dir() {
        Ok((real.to_path_buf(), None))
    } else {
        Err(format!("找不到 {}", real.display()))
    }
}

// ---------------------------------------------------------------- 狀態檔

fn state_dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("lobj-view")
}

fn instance_file() -> PathBuf {
    state_dir().join("instance.txt")
}

fn lock_file() -> PathBuf {
    state_dir().join("instance.lock")
}

/// 本機一次性的 token：以系統隨機種子的 hasher 產生（不另外引入 rand）。
fn random_token() -> String {
    let mut s = String::new();
    for _ in 0..2 {
        let mut h = RandomState::new().build_hasher();
        h.write_u32(std::process::id());
        s.push_str(&format!("{:016x}", h.finish()));
    }
    s
}

fn read_instance() -> Option<(u16, String)> {
    let text = std::fs::read_to_string(instance_file()).ok()?;
    let mut lines = text.lines();
    let port = lines.next()?.trim().parse().ok()?;
    let token = lines.next()?.trim().to_string();
    (!token.is_empty()).then_some((port, token))
}

/// 登記期間有效；結束時（若檔案仍是自己寫的）刪除。
pub struct Registration {
    token: String,
}

impl Drop for Registration {
    fn drop(&mut self) {
        if read_instance().is_some_and(|(_, t)| t == self.token) {
            let _ = std::fs::remove_file(instance_file());
        }
    }
}

/// 啟動 IPC 並寫入 `instance.txt`。
pub async fn serve_ipc(shared: Arc<Shared>) -> io::Result<Registration> {
    let token = random_token();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let port = listener.local_addr()?.port();

    let dir = state_dir();
    std::fs::create_dir_all(&dir)?;
    let tmp = dir.join(format!("instance.{}.tmp", std::process::id()));
    {
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp)?;
        write!(f, "{port}\n{token}\n{}\n", std::process::id())?;
    }
    std::fs::rename(&tmp, instance_file())?;

    let tk = token.clone();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else { continue };
            let shared = shared.clone();
            let tk = tk.clone();
            tokio::spawn(async move {
                let _ = handle(stream, shared, &tk).await;
            });
        }
    });
    Ok(Registration { token })
}

async fn handle(stream: AsyncTcpStream, shared: Arc<Shared>, token: &str) -> io::Result<()> {
    let (r, mut w) = stream.into_split();
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(3), BufReader::new(r.take(16 * 1024)).read_line(&mut line))
        .await
        .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))??;
    let reply = match parse_request(&line, token) {
        Ok(path) => match shared.apply_open(path) {
            Ok(()) => "OK\n".to_string(),
            Err(e) => format!("ERR\t{e}\n"),
        },
        Err(e) => format!("ERR\t{e}\n"),
    };
    w.write_all(reply.as_bytes()).await
}

/// 解析 `OPEN\t<token>\t<path>`，回傳 path。
fn parse_request<'a>(line: &'a str, token: &str) -> Result<&'a str, &'static str> {
    let mut it = line.trim_end_matches(['\r', '\n']).splitn(3, '\t');
    match (it.next(), it.next(), it.next()) {
        (Some("OPEN"), Some(t), Some(path)) if t == token && !path.is_empty() => Ok(path),
        (Some("OPEN"), Some(t), _) if t != token => Err("token 不符"),
        _ => Err("格式錯誤"),
    }
}

// ---------------------------------------------------------------- 用戶端

enum Forward {
    /// 沒有可用的實例（沒登記、連不上、回應不對）。
    NoInstance,
    Done,
    /// 實例活著但拒絕了（例如不支援的檔案）。
    Rejected(String),
}

/// 把 `abs` 交給執行中的實例。
fn forward(abs: &Path) -> Forward {
    use Forward::*;
    let Some((port, token)) = read_instance() else { return NoInstance };
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(500)) else { return NoInstance };
    let t = Some(Duration::from_secs(3));
    let _ = s.set_read_timeout(t);
    let _ = s.set_write_timeout(t);
    if write!(s, "OPEN\t{token}\t{}\n", abs.display()).is_err() {
        return NoInstance;
    }
    let mut reply = String::new();
    if StdBufReader::new(s).read_line(&mut reply).is_err() {
        return NoInstance;
    }
    match reply.trim_end().split_once('\t') {
        None if reply.trim_end() == "OK" => Done,
        Some(("ERR", msg)) => Rejected(msg.to_string()),
        _ => NoInstance,
    }
}

/// 啟動競爭時的鎖；釋放（或 drop）後其他程序才能成為伺服器。
pub struct Lock(PathBuf);

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub enum Claim {
    /// 已交給執行中的實例，這個程序可以結束了。
    Forwarded,
    /// 執行中的實例拒絕了這個路徑（原因在內）；不要另開實例。
    Rejected(String),
    /// 沒有可用的實例，自己當伺服器（持有鎖，登記完成後再釋放）。
    Server(Option<Lock>),
}

pub fn claim(path: &Path) -> Claim {
    let lock = lock_file();
    let _ = std::fs::create_dir_all(state_dir());
    for _ in 0..60 {
        match forward(path) {
            Forward::Done => return Claim::Forwarded,
            Forward::Rejected(msg) => return Claim::Rejected(msg),
            Forward::NoInstance => {}
        }
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&lock) {
            Ok(_) => return Claim::Server(Some(Lock(lock))),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                // 別的程序正在啟動：稍候再試；鎖卡太久（上次當掉）就清掉
                let stale = std::fs::metadata(&lock)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .is_some_and(|age| age > Duration::from_secs(15));
                if stale {
                    let _ = std::fs::remove_file(&lock);
                }
            }
            Err(_) => return Claim::Server(None),
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Claim::Server(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_parse() {
        assert_eq!(parse_request("OPEN\ttk\tC:\\a b\\x.lobj\n", "tk"), Ok("C:\\a b\\x.lobj"));
        assert_eq!(parse_request("OPEN\tbad\tx\n", "tk"), Err("token 不符"));
        assert_eq!(parse_request("HELLO\n", "tk"), Err("格式錯誤"));
        assert_eq!(parse_request("OPEN\ttk\t\n", "tk"), Err("格式錯誤"));
    }

    #[test]
    fn open_switches_root() {
        let base = std::env::temp_dir().join(format!("lobj_view_inst_{}", std::process::id()));
        let a = base.join("a");
        let b = base.join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(b.join("m.lobj"), b"x").unwrap();
        std::fs::write(b.join("n.txt"), b"x").unwrap();

        let sh = Shared::new(a.clone(), None);
        assert_eq!(sh.subscribe().borrow().seq, 0);
        sh.set_url(String::new()); // 測試時不開瀏覽器

        sh.apply_open(b.join("m.lobj").to_str().unwrap()).unwrap();
        assert_eq!(sh.root(), std::fs::canonicalize(&b).unwrap());
        assert_eq!(*sh.subscribe().borrow(), OpenEvent { seq: 1, path: "m.lobj".into() });

        sh.apply_open(a.to_str().unwrap()).unwrap();
        assert_eq!(*sh.subscribe().borrow(), OpenEvent { seq: 2, path: String::new() });

        assert!(sh.apply_open(b.join("n.txt").to_str().unwrap()).is_err());
        assert!(sh.apply_open(b.join("none.lobj").to_str().unwrap()).is_err());
        assert_eq!(sh.subscribe().borrow().seq, 2);
        std::fs::remove_dir_all(&base).ok();
    }
}
