//! Windows：把 `assets/lobj-view.ico` 嵌進 .exe（檔案總管、工作列、副檔名關聯的圖示都來自這裡）。
//! 圖示由 `assets/lobj-view.png` 產生（多尺寸 16–256）。其他平台不做任何事。

fn main() {
    println!("cargo:rerun-if-changed=assets/lobj-view.ico");
    println!("cargo:rerun-if-changed=build.rs");
    // 線上更新要知道該下載哪個平台的 Release 檔（src/update.rs）
    println!("cargo:rustc-env=LOBJ_VIEW_TARGET={}", std::env::var("TARGET").unwrap_or_default());
    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/lobj-view.ico");
            res.set("FileDescription", "lobj-view");
            if let Err(e) = res.compile() {
                // 沒有 rc.exe（Windows SDK）時不要讓整個建置失敗，只是少了圖示
                println!("cargo:warning=無法嵌入圖示: {e}");
            }
        }
    }
}
