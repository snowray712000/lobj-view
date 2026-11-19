# FAQ：打包好的 lobj-view 是 exe + dll，還是單一檔案？

## 問題
以前寫 C++，常是一個 `lobj.dll` 加一個 `lobj-view.exe`。打包好的 lobj-view 也是這樣嗎？還是 lobj-view 一個檔案就包含兩者？

## 答案
**一個檔案就包含兩者。** 發佈的只有一個執行檔（Windows 是 `lobj-view.exe`，Linux / macOS 是 `lobj-view`），不需要 `lobj.dll`，也不需要另外放前端檔案或安裝 Node.js。

## 為什麼
1. **lobj 是「靜態連結」進來的**
   `lobj` 是 Rust 的 library crate，不是 DLL。編譯 lobj-view 時，編譯器把用到的 lobj 程式碼直接編進同一個執行檔，效果等同 C++ 的 static library（`.lib` / `.a`）。Rust 的 crate 預設就是這樣，除非特別設定成 `cdylib` / `dylib` 才會產生 DLL。
2. **其他 Rust 相依套件也一樣**
   axum、tokio、rustls 等都編進同一個檔案，所以不會有 DLL 版本衝突（DLL hell）的問題。
3. **前端網頁也在 exe 裡**
   `rust-embed` 在編譯時把 `web/dist`（HTML / JS / CSS）嵌進執行檔，執行時從記憶體提供給瀏覽器。
4. **Linux 版連 C 執行庫都不依賴**
   Linux 版以 `x86_64-unknown-linux-musl` 靜態連結，不依賴系統的 glibc，所以複製到任何 Linux 都能直接執行。Windows 版預設用 MSVC 執行庫，一般 Windows 都已具備。

## 對照表

| | C++ 傳統做法 | lobj-view |
|---|---|---|
| 共用程式庫 | `lobj.dll` + `lobj-view.exe` | 編進 `lobj-view.exe`（靜態連結） |
| 前端資源 | 另外放資料夾 | 嵌入 exe（rust-embed） |
| 發佈內容 | exe + dll + 資源檔 | 一個檔案 |

## 開發時 lobj 在哪裡
- 原始碼層面，lobj 是獨立的 GitHub repo（https://github.com/snowray712000/lobj），lobj-view 的 `Cargo.toml` 以 git tag 相依。
- 編譯時 cargo 把它抓下來（或本機用 `rust-workspace/.cargo/config.toml` 的 `[patch]` 改用旁邊的 `./lobj`），編進 exe 後就和 lobj-view 合為一體。
- 因此「lobj 與 lobj-view 是兩個 repo」只影響開發與編譯，**不影響打包結果**。

## 如果真的想要 DLL
可以，但需要額外工作：把 lobj 設成 `crate-type = ["cdylib"]` 並匯出 C 介面（`extern "C"`）。Rust 沒有穩定的 ABI，不能像 C++ 那樣直接匯出類別與泛型。目前沒有這個需求，所以沒有做。

## 如何自行確認
- Windows：用 `dumpbin /dependents lobj-view.exe`，只會看到系統 DLL（如 `KERNEL32.dll`），沒有 `lobj.dll`。
- Linux：`ldd lobj-view` 會顯示 `not a dynamic executable`（靜態連結）。
