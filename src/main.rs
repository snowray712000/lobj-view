mod lobj_api;
mod tls;

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

/// 往上嘗試幾個 port，都被佔用時改由作業系統分配。
const PORT_SEARCH_RANGE: u16 = 50;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    cmd: Option<Cmd>,

    /// 監聽位址。預設只聽本機；0.0.0.0 會對外開放且沒有任何驗證。
    /// 非本機位址預設自動啟用 HTTPS（WebGPU 需要 secure context）。
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,

    /// 偏好的 port，被佔用時會自動往上找。
    #[arg(short, long, default_value_t = 8080)]
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
    let args = Args::parse();

    if let Some(Cmd::Trust { install, sans }) = args.cmd {
        return run_trust(install, &sans);
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
        .merge(lobj_api::router(args.dir.clone()));
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

    let listener = bind(args.host, args.port).await?;
    let addr = listener.local_addr()?;
    if addr.port() != args.port {
        println!("port {} 已被佔用，改用 {}", args.port, addr.port());
    }
    print_urls(addr, material.as_ref());

    match material {
        Some(m) => {
            let config = RustlsConfig::from_pem_file(&m.cert, &m.key).await?;
            axum_server::from_tcp_rustls(listener.into_std()?, config)?
                .serve(app.into_make_service())
                .await?;
        }
        None => axum::serve(listener, app).await?,
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
    let path = if path.is_empty() { "index.html" } else { path };

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
