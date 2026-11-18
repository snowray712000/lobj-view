//! 本機 HTTPS 憑證：自動建立一個本機 CA，再用它簽發伺服器憑證。
//!
//! 為什麼不是單純的自簽憑證：瀏覽器的 WebGPU 需要 secure context。
//! 只要使用者把 CA 匯入一次信任，之後 IP 變動、重簽伺服器憑證都不必再重做。
//! 檔案放在使用者資料目錄（`tls_dir()`），CA 私鑰不會離開這台機器，
//! 只有 `ca.crt`（公開憑證）會透過 `/ca.crt` 提供下載。

use std::collections::BTreeSet;
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa,
    Issuer, KeyPair, KeyUsagePurpose,
};
use time::OffsetDateTime;

/// 伺服器憑證效期 1 年（Apple 等平台對超過 398 天的憑證會拒絕），超過 300 天就重簽。
const LEAF_DAYS: i64 = 365;
const LEAF_RENEW_AFTER: Duration = Duration::from_secs(300 * 24 * 3600);

pub struct Material {
    pub cert: PathBuf,
    pub key: PathBuf,
    /// 只有自動產生時才有；使用者自備 `--cert/--key` 時為 None。
    pub ca: Option<PathBuf>,
    pub sans: Vec<String>,
}

pub fn tls_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lobj-view")
        .join("tls")
}

fn ca_params() -> Result<CertificateParams> {
    let mut p = CertificateParams::new(Vec::<String>::new())?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "lobj-view local CA");
    dn.push(DnType::OrganizationName, "lobj-view");
    p.distinguished_name = dn;
    p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    p.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let now = OffsetDateTime::now_utc();
    p.not_before = now - time::Duration::days(1);
    p.not_after = now + time::Duration::days(3650);
    Ok(p)
}

/// 這台機器目前會用到的名稱與 IP，加上使用者額外指定的。
pub fn collect_sans(extra: &[String]) -> Vec<String> {
    let mut set: BTreeSet<String> = ["localhost", "127.0.0.1", "::1"]
        .into_iter()
        .map(String::from)
        .collect();
    if let Ok(name) = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")) {
        if !name.is_empty() {
            set.insert(name);
        }
    }
    if let Ok(ifs) = local_ip_address::list_afinet_netifas() {
        for (_, ip) in ifs {
            // 只放 IPv4：IPv6 link-local 位址每次連線形式不同，放進去沒有意義
            if let IpAddr::V4(v4) = ip {
                if !v4.is_loopback() && !v4.is_link_local() {
                    set.insert(v4.to_string());
                }
            }
        }
    }
    set.extend(extra.iter().cloned());
    set.into_iter().collect()
}

/// 確保 CA 與伺服器憑證存在且涵蓋目前的 SAN；缺的才建立。
pub fn ensure(extra_sans: &[String]) -> Result<Material> {
    let dir = tls_dir();
    fs::create_dir_all(&dir).with_context(|| format!("無法建立 {}", dir.display()))?;
    let ca_crt = dir.join("ca.crt");
    let ca_key_path = dir.join("ca.key");
    let cert = dir.join("server.crt");
    let key = dir.join("server.key");
    let san_file = dir.join("server.san");

    // --- CA：已存在就重用，確保使用者匯入過的信任一直有效 ---
    let ca_key = if ca_crt.exists() && ca_key_path.exists() {
        KeyPair::from_pem(&fs::read_to_string(&ca_key_path)?)?
    } else {
        let k = KeyPair::generate()?;
        let c = ca_params()?.self_signed(&k)?;
        fs::write(&ca_crt, c.pem())?;
        fs::write(&ca_key_path, k.serialize_pem())?;
        restrict_permissions(&ca_key_path);
        // 換了 CA，舊的伺服器憑證就不能用了
        let _ = fs::remove_file(&cert);
        k
    };

    // --- 伺服器憑證：SAN 沒涵蓋目前的位址、或快到期才重簽 ---
    let sans = collect_sans(extra_sans);
    let want = sans.join("\n");
    let have = fs::read_to_string(&san_file).unwrap_or_default();
    let fresh = fs::metadata(&cert)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < LEAF_RENEW_AFTER);
    if !(cert.exists() && key.exists() && have == want && fresh) {
        let leaf_key = KeyPair::generate()?;
        let mut p = CertificateParams::new(sans.clone())?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "lobj-view");
        p.distinguished_name = dn;
        p.is_ca = IsCa::NoCa;
        p.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        p.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let now = OffsetDateTime::now_utc();
        p.not_before = now - time::Duration::days(1);
        p.not_after = now + time::Duration::days(LEAF_DAYS);
        // 簽發者只需要 DN 與金鑰；重建的參數與當初的 CA 在這兩點上一致
        let issuer = Issuer::new(ca_params()?, ca_key);
        let leaf = p.signed_by(&leaf_key, &issuer)?;
        fs::write(&cert, leaf.pem())?;
        fs::write(&key, leaf_key.serialize_pem())?;
        restrict_permissions(&key);
        fs::write(&san_file, want)?;
    }

    Ok(Material {
        cert,
        key,
        ca: Some(ca_crt),
        sans,
    })
}

#[cfg(unix)]
fn restrict_permissions(p: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(p, fs::Permissions::from_mode(0o600));
}
#[cfg(not(unix))]
fn restrict_permissions(_: &std::path::Path) {}

/// 把 CA 匯入「這台機器」的信任清單（Windows / macOS）。其他系統回傳說明文字。
pub fn install_ca_locally(ca: &std::path::Path) -> Result<String> {
    if cfg!(windows) {
        // -user：只動目前使用者的存放區，不需要系統管理員，Windows 會跳出確認視窗
        let st = Command::new("certutil")
            .args(["-user", "-addstore", "Root"])
            .arg(ca)
            .status()
            .context("無法執行 certutil")?;
        anyhow::ensure!(st.success(), "certutil 失敗 ({st})");
        Ok("已匯入目前使用者的「受信任的根憑證授權單位」，請重新啟動瀏覽器。".into())
    } else if cfg!(target_os = "macos") {
        let keychain = dirs::home_dir()
            .context("找不到家目錄")?
            .join("Library/Keychains/login.keychain-db");
        let st = Command::new("security")
            .args(["add-trusted-cert", "-r", "trustRoot", "-k"])
            .arg(keychain)
            .arg(ca)
            .status()
            .context("無法執行 security")?;
        anyhow::ensure!(st.success(), "security 失敗 ({st})");
        Ok("已加入登入鑰匙圈並設為信任，請重新啟動瀏覽器。".into())
    } else {
        Ok(client_instructions(ca))
    }
}

/// 給「其他裝置」看的匯入說明（同時用在 CLI 與 `/trust` 頁面）。
pub fn client_instructions(_ca: &std::path::Path) -> String {
    "\
Windows : 下載 ca.crt → 雙擊 → 安裝憑證 → 目前使用者 → 「將所有憑證放入以下存放區」
          → 受信任的根憑證授權單位。（或：certutil -user -addstore Root ca.crt）
macOS   : 雙擊 ca.crt，鑰匙圈選「系統」（不要用預設的「登入」或「本機項目」，否則會出錯），
          輸入本機登入密碼 → 開啟該憑證 → 信任 → 「永遠信任」。
Linux   : Chrome/Edge 到 設定 → 隱私權和安全性 → 安全性 → 管理憑證 → 授權單位 → 匯入。
          Firefox 到 設定 → 隱私權與安全性 → 憑證 → 檢視憑證 → 授權單位 → 匯入。
Android : 設定 → 安全性 → 加密與憑證 → 安裝憑證 → CA 憑證。
iOS     : 開啟 ca.crt → 安裝描述檔，再到 設定 → 一般 → 關於 → 憑證信任設定 開啟完全信任。
匯入完成後請完全重新啟動瀏覽器，再重新整理頁面。"
        .to_owned()
}
