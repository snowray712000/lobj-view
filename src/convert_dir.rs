//! `lobj-view convert-dir <資料夾>`：批次把檔名明確標示 upper / lower 的 `.lobj` 轉成同名 `.lobj2`。
//!
//! 檔名沒有（或同時有）upper / lower 的檔案無法確定上下顎，一律略過並列出，不猜。
//! 同名 `.lobj2` 已存在也略過（不覆蓋）。單檔轉換邏輯與網頁的「轉 lobj2」共用 `lobj_api::convert_real`。

use std::path::{Path, PathBuf};

use lobj::Jaw;

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let ft = e.file_type()?;
        let p = e.path();
        if ft.is_dir() {
            collect(&p, out)?; // 符號連結的 is_dir() 為 false，不會跟進
        } else if ft.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lobj")) {
            out.push(p);
        }
    }
    Ok(())
}

pub fn run(dir: &Path, delete_old: bool, dry_run: bool) -> anyhow::Result<()> {
    let mut files = Vec::new();
    collect(dir, &mut files)?;
    let (mut ok, mut skipped, mut failed) = (0usize, 0usize, 0usize);
    for (i, f) in files.iter().enumerate() {
        let tag = format!("[{}/{}] {}", i + 1, files.len(), f.strip_prefix(dir).unwrap_or(f).display());
        let Some(jaw) = Jaw::guess_from_path(f) else {
            println!("{tag}: 略過（檔名無法判斷上 / 下顎）");
            skipped += 1;
            continue;
        };
        if f.with_extension("lobj2").exists() {
            println!("{tag}: 略過（.lobj2 已存在）");
            skipped += 1;
            continue;
        }
        if dry_run {
            println!("{tag}: 將轉換（{jaw:?}）");
            ok += 1;
            continue;
        }
        match crate::lobj_api::convert_real(f, Some(jaw), delete_old) {
            Ok(_) => ok += 1,
            Err((_, m)) => {
                println!("{tag}: 失敗：{m}");
                failed += 1;
            }
        }
    }
    println!(
        "完成：{}{ok} 個，略過 {skipped} 個，失敗 {failed} 個（共 {} 個 .lobj）",
        if dry_run { "將轉換 " } else { "轉換 " },
        files.len()
    );
    if failed > 0 {
        anyhow::bail!("{failed} 個檔案轉換失敗");
    }
    Ok(())
}
