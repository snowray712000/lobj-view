//! 把 `.zip` 當成資料夾：第一次進入時偷偷解壓到快取資料夾（原 .zip 完全不動），之後照一般資料夾處理。
//!
//! - 快取在 `data_local_dir/lobj-view/zipcache/<key>/`，key 由 zip 的真實路徑 + 大小 + 修改時間決定，
//!   所以 zip 被換掉會自動重新解壓；解壓先寫到 `<key>.partial`，完成才改名，中斷或當機不會留下半套。
//! - 只解出看得到的檔案（模型檔與內層 .zip），其他內容不寫入，省時間與空間。
//! - 可隨時取消：呼叫端把 `AtomicBool` 設為 true，解壓迴圈每個 chunk 都檢查，取消就刪掉 `.partial`。
//! - 防護：路徑穿越（`enclosed_name`）、符號連結不解、解壓後總量與檔案數上限。

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

/// 解壓後總量上限（只計實際會寫出的檔案）。
const MAX_TOTAL: u64 = 8 << 30;
const MAX_FILES: usize = 50_000;
/// 快取超過這麼久沒用就在啟動時清掉。
const KEEP: Duration = Duration::from_secs(7 * 24 * 3600);
const MARKER: &str = ".lobj-view-used";

#[derive(Debug)]
pub enum ZipError {
    Cancelled,
    Msg(String),
}

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZipError::Cancelled => write!(f, "已取消"),
            ZipError::Msg(m) => write!(f, "{m}"),
        }
    }
}

fn msg(e: impl std::fmt::Display) -> ZipError {
    ZipError::Msg(e.to_string())
}

pub fn is_zip_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".zip")
}

pub fn cache_root() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("lobj-view").join("zipcache")
}

/// 啟動時清理：刪掉殘留的 `.partial`，以及很久沒用的快取。
pub fn cleanup_cache() {
    let Ok(rd) = fs::read_dir(cache_root()) else { return };
    let now = SystemTime::now();
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        let stale = if name.ends_with(".partial") {
            true
        } else {
            let t = fs::metadata(p.join(MARKER)).or_else(|_| fs::metadata(&p)).and_then(|m| m.modified());
            t.map(|t| now.duration_since(t).unwrap_or_default() > KEEP).unwrap_or(false)
        };
        if stale {
            let _ = fs::remove_dir_all(&p);
        }
    }
}

fn locks() -> &'static Mutex<HashMap<String, Arc<Mutex<()>>>> {
    static L: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    L.get_or_init(Default::default)
}

fn cache_key(zip: &Path) -> Result<String, ZipError> {
    let m = fs::metadata(zip).map_err(msg)?;
    let mtime = m.modified().ok().and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0);
    let mut h = DefaultHasher::new();
    zip.hash(&mut h);
    m.len().hash(&mut h);
    mtime.hash(&mut h);
    Ok(format!("{:016x}", h.finish()))
}

/// 該 zip 目前的快取資料夾（不解壓；沒有快取回傳 None）。
pub fn cached_dir(zip: &Path) -> Option<PathBuf> {
    let d = cache_root().join(cache_key(zip).ok()?);
    d.is_dir().then_some(d)
}

/// 取得 zip 的解壓資料夾；沒有快取就解壓（會擋住其他要求同一個 zip 的人，直到完成或被取消）。
pub fn ensure_extracted(zip: &Path, cancel: &AtomicBool) -> Result<PathBuf, ZipError> {
    let key = cache_key(zip)?;
    let dest = cache_root().join(&key);
    let touch = |d: &Path| {
        let _ = fs::write(d.join(MARKER), b"");
    };
    if dest.is_dir() {
        touch(&dest);
        return Ok(dest);
    }

    // 同一個 zip 同時只讓一個人解壓；等待的人也要能被取消
    let lock = locks().lock().unwrap().entry(key.clone()).or_default().clone();
    let _guard = loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(ZipError::Cancelled);
        }
        match lock.try_lock() {
            Ok(g) => break g,
            Err(std::sync::TryLockError::Poisoned(p)) => break p.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(50)),
        }
    };
    if dest.is_dir() {
        touch(&dest); // 等待期間別人已解完
        return Ok(dest);
    }

    let part = cache_root().join(format!("{key}.partial"));
    let _ = fs::remove_dir_all(&part);
    fs::create_dir_all(&part).map_err(msg)?;
    match extract(zip, &part, cancel) {
        Ok(()) => {
            fs::write(part.join(MARKER), b"").map_err(msg)?;
            fs::rename(&part, &dest).map_err(msg)?;
            Ok(dest)
        }
        Err(e) => {
            let _ = fs::remove_dir_all(&part);
            Err(e)
        }
    }
}

fn wanted(name: &str) -> bool {
    crate::lobj_api::is_lobj_name(name) || is_zip_name(name)
}

fn extract(zip: &Path, dest: &Path, cancel: &AtomicBool) -> Result<(), ZipError> {
    let file = fs::File::open(zip).map_err(msg)?;
    let mut ar = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|e| ZipError::Msg(format!("無法讀取 zip：{e}")))?;
    if ar.len() > 1_000_000 {
        return Err(ZipError::Msg("zip 內項目過多".into()));
    }

    let (mut total, mut count) = (0u64, 0usize);
    let mut buf = vec![0u8; 256 * 1024];
    for i in 0..ar.len() {
        if cancel.load(Ordering::Relaxed) {
            return Err(ZipError::Cancelled);
        }
        let mut entry = ar.by_index(i).map_err(|e| ZipError::Msg(format!("zip 項目損壞：{e}")))?;
        if entry.is_dir() || entry.is_symlink() {
            continue;
        }
        let Some(rel) = entry.enclosed_name() else { continue }; // 含 .. 或絕對路徑的一律略過
        let name = rel.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !wanted(&name) {
            continue;
        }
        count += 1;
        if count > MAX_FILES {
            return Err(ZipError::Msg("zip 內可用檔案過多".into()));
        }
        let out_path = dest.join(&rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(msg)?;
        }
        let mut out = fs::File::create(&out_path).map_err(msg)?;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(ZipError::Cancelled);
            }
            let n = entry.read(&mut buf).map_err(|e| ZipError::Msg(format!("解壓失敗：{e}")))?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > MAX_TOTAL {
                return Err(ZipError::Msg("解壓後總量超過上限（8 GB）".into()));
            }
            out.write_all(&buf[..n]).map_err(msg)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut w = zip::ZipWriter::new(fs::File::create(path).unwrap());
        let opt = zip::write::SimpleFileOptions::default();
        for (n, d) in entries {
            w.start_file(*n, opt).unwrap();
            std::io::Write::write_all(&mut w, d).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn extract_filters_and_cancel() {
        let base = std::env::temp_dir().join(format!("lobj_view_zip_{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        let z = base.join("a.zip");
        make_zip(&z, &[("d/a.stl", b"solid"), ("readme.txt", b"x"), ("../evil.stl", b"x")]);

        let no = AtomicBool::new(false);
        let part = base.join("out");
        fs::create_dir_all(&part).unwrap();
        extract(&z, &part, &no).unwrap();
        assert!(part.join("d/a.stl").is_file());
        assert!(!part.join("readme.txt").exists());
        assert!(!base.join("evil.stl").exists());

        let yes = AtomicBool::new(true);
        assert!(matches!(extract(&z, &base.join("out2"), &yes), Err(ZipError::Cancelled)));
        fs::remove_dir_all(&base).ok();
    }
}
