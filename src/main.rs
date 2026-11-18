use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};

use axum::{
    Router,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use clap::Parser;
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
    /// 監聽位址。預設只聽本機，遠端請用 `ssh -L` 轉發；0.0.0.0 會對外開放且沒有任何驗證。
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,

    /// 偏好的 port，被佔用時會自動往上找。
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .fallback(static_handler);

    let listener = bind(args.host, args.port).await?;
    let addr = listener.local_addr()?;
    if addr.port() != args.port {
        println!("port {} 已被佔用，改用 {}", args.port, addr.port());
    }
    print_urls(addr);

    axum::serve(listener, app).await?;
    Ok(())
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

fn print_urls(addr: SocketAddr) {
    let port = addr.port();
    if addr.ip().is_unspecified() {
        println!("lobj-view 監聽所有介面 (port {port})");
        println!("  本機:  http://localhost:{port}");
        println!("  注意: 以區網 IP 的 http 連線時，瀏覽器不提供 WebGPU（會退回 WebGL2）");
    } else if addr.ip().is_loopback() {
        println!("lobj-view: http://localhost:{port}");
        println!("  遠端使用: ssh -L {port}:localhost:{port} <這台主機>，再開上面的網址");
    } else {
        println!("lobj-view: http://{addr}");
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
