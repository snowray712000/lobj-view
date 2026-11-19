//! 伺服器資源狀態（CPU、記憶體、本程式用量、可選的 NVIDIA GPU），給左上角狀態列用。
//!
//! 背景執行緒每 3 秒取樣一次並快取；`/api/stats` 只讀快取，所以不管多少人、多少分頁在輪詢，
//! 取樣成本都一樣。超過 20 秒沒有人讀就暫停取樣（沒人看就不花資源）。
//! GPU 只支援有 `nvidia-smi` 的機器（約每 6 秒一次）；找不到就不顯示。
//! 注意：這些是「伺服器」的狀態。3D 繪圖是用「瀏覽器那台電腦」的 GPU，兩者不一定是同一台。

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Clone, Default)]
pub struct Gpu {
    pub name: String,
    pub util: f32,
    pub mem_used_mb: f32,
    pub mem_total_mb: f32,
}

#[derive(Clone, Default)]
pub struct Stats {
    pub ready: bool,
    pub cpu: f32,
    pub cores: usize,
    pub mem_used: u64,
    pub mem_total: u64,
    pub proc_cpu: f32,
    pub proc_mem: u64,
    pub gpu: Option<Gpu>,
}

static CACHE: OnceLock<Mutex<Stats>> = OnceLock::new();
/// 最後一次有人讀取的時間（unix 秒）
static LAST_READ: AtomicU64 = AtomicU64::new(0);

fn cache() -> &'static Mutex<Stats> {
    CACHE.get_or_init(Default::default)
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// 讀快取（同時告訴取樣執行緒「有人在看」）。
pub fn read() -> Stats {
    LAST_READ.store(now_secs(), Ordering::Relaxed);
    cache().lock().unwrap().clone()
}

pub fn start() {
    std::thread::spawn(sampler);
}

fn sampler() {
    let Ok(mut sys) = System::new() else { return }; // 取不到系統資訊就不顯示（ready 恆為 false）
    let pid = Pid::from_u32(std::process::id());
    let mut gpu_ok = true;
    let mut tick = 0u64;
    loop {
        std::thread::sleep(Duration::from_secs(3));
        if now_secs().saturating_sub(LAST_READ.load(Ordering::Relaxed)) > 20 {
            continue;
        }
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing().with_cpu().with_memory());
        let cores = sys.cpus().len().max(1);
        let (proc_cpu, proc_mem) = sys.process(pid).map(|p| (p.cpu_usage() / cores as f32, p.memory())).unwrap_or((0.0, 0));
        let gpu = if gpu_ok && tick % 2 == 0 {
            let g = query_gpu();
            if g.is_none() {
                gpu_ok = false; // 沒有 nvidia-smi：之後不再嘗試
            }
            g
        } else {
            cache().lock().unwrap().gpu.clone()
        };
        tick += 1;
        *cache().lock().unwrap() = Stats {
            ready: true,
            cpu: sys.global_cpu_usage(),
            cores,
            mem_used: sys.used_memory(),
            mem_total: sys.total_memory(),
            proc_cpu,
            proc_mem,
            gpu,
        };
    }
}

fn query_gpu() -> Option<Gpu> {
    let mut cmd = Command::new("nvidia-smi");
    cmd.args(["--query-gpu=name,utilization.gpu,memory.used,memory.total", "--format=csv,noheader,nounits"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW：不要閃出黑視窗
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_gpu(&String::from_utf8_lossy(&out.stdout))
}

fn parse_gpu(s: &str) -> Option<Gpu> {
    let line = s.lines().next()?;
    let f: Vec<&str> = line.split(',').map(str::trim).collect();
    if f.len() < 4 {
        return None;
    }
    Some(Gpu { name: f[0].to_string(), util: f[1].parse().ok()?, mem_used_mb: f[2].parse().ok()?, mem_total_mb: f[3].parse().ok()? })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_line() {
        let g = parse_gpu("NVIDIA GeForce RTX 3060, 12, 1024, 12288\n").unwrap();
        assert_eq!((g.name.as_str(), g.util, g.mem_total_mb), ("NVIDIA GeForce RTX 3060", 12.0, 12288.0));
        assert!(parse_gpu("garbage").is_none());
    }
}
