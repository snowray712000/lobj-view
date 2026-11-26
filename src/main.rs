mod assoc;
mod convert_dir;
mod dataset_stat;
mod instance;
mod lobj_api;
mod stats;
mod tls;
mod update;
mod zipfs;
mod zipsync;

use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use axum::{
    Router,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use axum_server::tls_rustls::RustlsConfig;
use clap::{Parser, Subcommand};
use rust_embed::{EmbeddedFile, RustEmbed};
use tokio::net::TcpListener;

/// 前端 build 結果（web/dist）。
/// debug build 時 rust-embed 直接從磁碟讀取，release build 才會真正嵌入執行檔。
#[derive(RustEmbed)]
#[folder = "web/dist/"]
#[allow_missing = true]
struct Assets;

/// 預設 port：開發建置用 8079，讓它能與已安裝的穩定版（8080）同時執行。
const DEFAULT_PORT: u16 = if cfg!(debug_assertions) { 8079 } else { 8080 };

/// 往上嘗試幾個 port，都被佔用時改由作業系統分配。
const PORT_SEARCH_RANGE: u16 = 50;

#[derive(Parser)]
#[command(version, about, args_conflicts_with_subcommands = true)]
struct Args {
    #[command(subcommand)]
    cmd: Option<Cmd>,

    /// 要開啟的檔案或資料夾（例如在檔案總管雙擊 .lobj）。
    /// 若已有執行中的 lobj-view，就交給它（切換資料夾並選取該檔）後結束；
    /// 否則啟動伺服器，根資料夾設為該檔所在的資料夾，並自動開瀏覽器。會覆蓋 --dir。
    path: Option<PathBuf>,

    /// 即使已有執行中的 lobj-view 也另起一個獨立實例（不轉交、也不登記為預設實例）。
    #[arg(long)]
    new: bool,

    /// 帶 PATH 啟動時，不自動開瀏覽器。
    #[arg(long)]
    no_browser: bool,

    /// 允許非本機的網頁觸發線上更新（搭配 --host 0.0.0.0 時）。預設關閉：
    /// 開啟後區網上任何能開這個網頁的人都能讓伺服器更新到 GitHub 最新版並重新啟動。
    #[arg(long)]
    allow_remote_update: bool,

    /// 監聽位址。預設只聽本機；0.0.0.0 會對外開放且沒有任何驗證。
    /// 非本機位址預設自動啟用 HTTPS（WebGPU 需要 secure context）。
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,

    /// 偏好的 port，被佔用時會自動往上找。開發（debug）建置預設 8079，發行（release）建置預設 8080。
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// 強制使用 HTTPS（本機位址預設是 HTTP）。
    #[arg(long, conflicts_with = "http")]
    https: bool,

    /// 強制使用 HTTP（即使監聽非本機位址）。此時其他裝置的瀏覽器沒有 WebGPU。
    #[arg(long)]
    http: bool,

    /// 自備憑證（PEM）。需與 --key 一起使用；不指定則自動產生本機 CA 與憑證。
    #[arg(long, requires = "key")]
    cert: Option<PathBuf>,

    /// 自備私鑰（PEM）。
    #[arg(long, requires = "cert")]
    key: Option<PathBuf>,

    /// 額外寫入憑證的名稱或 IP（例如 DNS 名稱），可重複指定。
    #[arg(long = "san")]
    sans: Vec<String>,

    /// 提供 .lobj / .lobj2 檔案的根資料夾（前端可瀏覽其子資料夾，但不能超出此資料夾）。
    #[arg(short, long, default_value = ".")]
    dir: PathBuf,
}

#[derive(Subcommand)]
enum Cmd {
    /// 把 lobj-view 註冊成副檔名的開啟程式（目前只支援 Windows；不需要系統管理員）。
    /// 預設處理全部五種副檔名：一律出現在「開啟檔案」清單；只有沒有其他預設程式的才會直接成為預設
    /// （.stl .obj .ply 通常已被佔用，不會被搶走，需要自己在系統設定指定）。
    Register {
        /// 只處理這些副檔名，逗號分隔（lobj,lobj2,stl,ply,obj）。
        #[arg(long, value_delimiter = ',')]
        ext: Vec<String>,
    },
    /// 取消註冊（不指定 --ext 就是全部）。
    Unregister {
        #[arg(long, value_delimiter = ',')]
        ext: Vec<String>,
    },
    /// 顯示各副檔名目前的關聯狀態。
    Status,
    /// 從 GitHub Release 下載最新版並替換這個執行檔（執行中的 lobj-view 要重新啟動才會生效）。
    Update {
        /// 只檢查有沒有新版，不下載。
        #[arg(long)]
        check: bool,
    },
    /// 把資料夾（含子資料夾）下所有檔名明確標示上顎 / 下顎（upper / lower）的 .lobj 轉成同名 .lobj2。
    ConvertDir {
        /// 要轉換的資料夾。
        dir: PathBuf,
        /// 轉換成功（寫出後讀回驗證）後刪除原本的 .lobj；預設保留。
        #[arg(long)]
        delete_old: bool,
        /// 只列出會做什麼，不寫入任何檔案。
        #[arg(long)]
        dry_run: bool,
    },
    /// 產生本機 CA 與憑證，並顯示其他裝置該怎麼信任它。
    Trust {
        /// 同時把 CA 匯入這台機器的信任清單（Windows / macOS）。
        #[arg(long)]
        install: bool,
        /// 額外寫入憑證的名稱或 IP，可重複指定。
        #[arg(long = "san")]
        sans: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if zipsync::askpass_mode() {
        return Ok(()); // 被 ssh 當 SSH_ASKPASS 叫起來：只印出金鑰密碼就結束
    }
    update::remember_exe();
    let args = Args::parse();

    match args.cmd {
        Some(Cmd::Update { check }) => return update::run_cli(check),
        Some(Cmd::ConvertDir { dir, delete_old, dry_run }) => return convert_dir::run(&dir, delete_old, dry_run),
        Some(Cmd::Trust { install, sans }) => return run_trust(install, &sans),
        Some(Cmd::Register { ext }) => {
            let exts = if ext.is_empty() {
                assoc::ALL_EXTS.iter().map(|s| s.to_string()).collect()
            } else {
                assoc::normalize_exts(&ext).map_err(anyhow::Error::msg)?
            };
            let st = assoc::register(&exts).map_err(anyhow::Error::msg)?;
            print!("{}", assoc::format_status(&st));
            if st.exts.iter().any(|e| exts.contains(&e.ext) && !e.is_default) {
                println!("
尚未成為預設的副檔名，請到 Windows「設定 > 預設應用程式」指定 lobj-view（或對檔案按右鍵 > 開啟檔案 > 選擇其他應用程式）。");
            }
            return Ok(());
        }
        Some(Cmd::Unregister { ext }) => {
            let exts = if ext.is_empty() {
                assoc::ALL_EXTS.iter().map(|s| s.to_string()).collect()
            } else {
                assoc::normalize_exts(&ext).map_err(anyhow::Error::msg)?
            };
            let st = assoc::unregister(&exts).map_err(anyhow::Error::msg)?;
            print!("{}", assoc::format_status(&st));
            return Ok(());
        }
        Some(Cmd::Status) => {
            print!("{}", assoc::format_status(&assoc::status().map_err(anyhow::Error::msg)?));
            return Ok(());
        }
        None => {}
    }

    // 線上更新後的重新啟動：等舊程序放掉 port 與登記，沿用同一個 port，不另開瀏覽器
    let restart = update::restart_info();
    if let Some((pid, _)) = restart {
        update::wait_pid_exit(pid);
    }

    // 帶檔案／資料夾：先試著交給執行中的實例，沒有才自己當伺服器
    let mut dir = args.dir.clone();
    let mut initial: Option<String> = None;
    let mut lock = None;
    if let Some(p) = &args.path {
        let abs = std::path::absolute(p)?;
        if !abs.exists() {
            anyhow::bail!("找不到 {}", abs.display());
        }
        if !args.new {
            match instance::claim(&abs) {
                instance::Claim::Forwarded => {
                    println!("已交給執行中的 lobj-view 開啟: {}", abs.display());
                    return Ok(());
                }
                instance::Claim::Rejected(msg) => anyhow::bail!("{msg}"),
                instance::Claim::Server(l) => lock = l,
            }
        }
        let (d, sel) = instance::split_target(&std::fs::canonicalize(&abs)?).map_err(anyhow::Error::msg)?;
        dir = d;
        initial = Some(sel.unwrap_or_default());
    }
    stats::start();
    std::thread::spawn(zipfs::cleanup_cache); // 清掉殘留與過期的 zip 解壓快取
    let shared = instance::Shared::new(dir, initial);
    shared.set_local_only(args.host.is_loopback());
    update::set_allow_remote(args.allow_remote_update);
    if args.allow_remote_update && !args.host.is_loopback() {
        println!("注意: 已開啟 --allow-remote-update，區網上的網頁也能觸發線上更新與重新啟動");
    }

    let use_tls = !args.http && (args.https || args.cert.is_some() || !args.host.is_loopback());

    // 先備妥憑證，失敗就不要繼續啟動
    let material = if use_tls {
        Some(match (&args.cert, &args.key) {
            (Some(c), Some(k)) => tls::Material {
                cert: c.clone(),
                key: k.clone(),
                ca: None,
                sans: Vec::new(),
            },
            _ => tls::ensure(&args.sans)?,
        })
    } else {
        None
    };
    let ca_pem = material
        .as_ref()
        .and_then(|m| m.ca.as_ref())
        .and_then(|p| std::fs::read(p).ok());

    let mut app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/version", get(|| async { env!("CARGO_PKG_VERSION") }))
        .route("/api/versions", get(|| async { include_str!("../doc/Versions.md") }))
        .route("/trust", get(trust_page))
        .merge(lobj_api::router(shared.clone()))
        .merge(dataset_stat::router(shared.clone()))
        .merge(assoc::router(shared.clone()))
        .merge(update::router(shared.clone()))
        .merge(zipsync::router(shared.clone()));
    if let Some(pem) = ca_pem {
        app = app.route(
            "/ca.crt",
            get(move || {
                let pem = pem.clone();
                async move {
                    (
                        [
                            (header::CONTENT_TYPE, "application/x-x509-ca-cert"),
                            (header::CONTENT_DISPOSITION, "attachment; filename=\"lobj-view-ca.crt\""),
                        ],
                        pem,
                    )
                }
            }),
        );
    }
    let app = app.fallback(static_handler);

    let listener = match restart {
        Some((_, port)) => bind_retry(args.host, port).await?,
        None => bind(args.host, args.port).await?,
    };
    let addr = listener.local_addr()?;
    if addr.port() != args.port {
        println!("port {} 已被佔用，改用 {}", args.port, addr.port());
    }
    print_urls(addr, material.as_ref());

    let scheme = if material.is_some() { "https" } else { "http" };
    let host = if addr.ip().is_unspecified() || addr.ip().is_loopback() {
        format!("localhost:{}", addr.port())
    } else {
        addr.to_string()
    };
    shared.set_url(format!("{scheme}://{host}"));

    // 登記為預設實例（之後 `lobj-view <檔案>` 會交給它）；登記完成才釋放啟動鎖
    let _registration = if args.new {
        None
    } else {
        match instance::serve_ipc(shared.clone()).await {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("警告: 無法登記為預設實例，之後開檔會另開新的: {e}");
                None
            }
        }
    };
    drop(lock);
    if args.path.is_some() && !args.no_browser && restart.is_none() {
        shared.open_browser();
    }

    let serve = async {
        match material {
            Some(m) => {
                let config = RustlsConfig::from_pem_file(&m.cert, &m.key).await?;
                axum_server::from_tcp_rustls(listener.into_std()?, config)?
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await?;
            }
            None => axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?,
        }
        anyhow::Ok(())
    };
    // Ctrl+C 時正常返回，讓登記檔被清掉
    tokio::select! {
        r = serve => r?,
        _ = tokio::signal::ctrl_c() => {}
        _ = update::restart_requested() => {
            drop(_registration);
            update::relaunch(addr.port());
        }
    }
    Ok(())
}

fn run_trust(install: bool, sans: &[String]) -> anyhow::Result<()> {
    let m = tls::ensure(sans)?;
    let ca = m.ca.expect("自動產生一定有 CA");
    println!("憑證目錄: {}", tls::tls_dir().display());
    println!("CA 憑證 : {}", ca.display());
    println!("憑證涵蓋: {}", m.sans.join(", "));
    println!();
    if install {
        println!("{}", tls::install_ca_locally(&ca)?);
    } else {
        println!("這台機器要信任它：lobj-view trust --install");
        println!("其他裝置：啟動 `lobj-view --host 0.0.0.0` 後開 https://<這台的IP>:<port>/trust，");
        println!("下載 ca.crt（第一次瀏覽器會警告，選擇繼續），再依下列方式匯入：");
        println!();
        println!("{}", tls::client_instructions(&ca));
    }
    Ok(())
}

async fn trust_page() -> impl IntoResponse {
    let ca = tls::tls_dir().join("ca.crt");
    let body = format!(
        "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width\">         <title>信任 lobj-view 憑證</title>         <body style=\"font-family:sans-serif;max-width:46em;margin:2em auto;padding:0 1em\">         <h1>信任 lobj-view 憑證</h1>         <p>為了讓瀏覽器在區網中啟用 WebGPU，需要 HTTPS。請先 <a href=\"/ca.crt\">下載 ca.crt</a>，         匯入為「受信任的根憑證授權單位」，之後不會再出現警告。</p>         <pre style=\"white-space:pre-wrap\">{}</pre>",
        tls::client_instructions(&ca)
    );
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], body)
}

async fn bind(host: IpAddr, port: u16) -> anyhow::Result<TcpListener> {
    let last = port.saturating_add(PORT_SEARCH_RANGE);
    for p in port..=last {
        match TcpListener::bind(SocketAddr::new(host, p)).await {
            Ok(l) => return Ok(l),
            Err(e) if matches!(e.kind(), ErrorKind::AddrInUse | ErrorKind::PermissionDenied) => {
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(TcpListener::bind(SocketAddr::new(host, 0)).await?)
}

/// 重新啟動時舊程序可能還沒完全放掉 port：同一個 port 重試幾秒，不行才照一般規則往上找。
async fn bind_retry(host: IpAddr, port: u16) -> anyhow::Result<TcpListener> {
    for _ in 0..50 {
        if let Ok(l) = TcpListener::bind(SocketAddr::new(host, port)).await {
            return Ok(l);
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    bind(host, port).await
}

fn print_urls(addr: SocketAddr, tls: Option<&tls::Material>) {
    let port = addr.port();
    let scheme = if tls.is_some() { "https" } else { "http" };
    if addr.ip().is_unspecified() {
        println!("lobj-view 監聽所有介面 (port {port}, {scheme})");
        println!("  本機:  {scheme}://localhost:{port}");
        if tls.is_some() {
            for ip in tls::collect_sans(&[]).iter().filter(|s| s.parse::<IpAddr>().is_ok_and(|i| i.is_ipv4() && !i.is_loopback())) {
                println!("  區網:  {scheme}://{ip}:{port}");
            }
            if tls.is_some_and(|m| m.ca.is_some()) {
                println!("  其他裝置第一次使用請先開 /trust 下載並信任 CA，否則瀏覽器會顯示憑證警告");
            }
        } else {
            println!("  注意: 以區網 IP 的 http 連線時，瀏覽器不提供 WebGPU（會退回 WebGL2）");
        }
    } else if addr.ip().is_loopback() && tls.is_none() {
        println!("lobj-view: http://localhost:{port}");
        println!("  遠端使用: ssh -L {port}:localhost:{port} <這台主機>，再開上面的網址");
    } else {
        println!("lobj-view: {scheme}://{addr}");
    }
}

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    // 獨立的批次轉檔頁（web/public/to-lobj2.html），不屬於主程式 SPA
    let path = match path {
        "" => "index.html",
        "to-lobj2" | "to-lobj2/" => "to-lobj2.html",
        "stat" | "stat/" => "stat.html",
        p => p,
    };

    if let Some(file) = Assets::get(path) {
        return serve_file(path, file);
    }
    // 未知的 API 不要回 index.html，以免前端拿到 HTML 誤當資料解析
    if path.starts_with("api/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    // SPA fallback
    match Assets::get("index.html") {
        Some(file) => serve_file("index.html", file),
        None => (
            StatusCode::NOT_FOUND,
            "前端尚未 build：請先在 web/ 執行 npm install && npm run build",
        )
            .into_response(),
    }
}

fn serve_file(path: &str, file: EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    // Vite 產出的 assets/ 檔名含 hash，可永久快取；index.html 則每次重新驗證
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        [
            (header::CONTENT_TYPE, mime.as_ref().to_owned()),
            (header::CACHE_CONTROL, cache.to_owned()),
        ],
        file.data,
    )
        .into_response()
}
