# Windows：用 cmd 安裝最新的 cargo 與 cargo-binstall

對應 Ubuntu 版：[ubuntu-install-rust-and-binstall.md](ubuntu-install-rust-and-binstall.md)

本專案使用 `edition = "2024"`，需要 Rust 1.85 以上，請用 **rustup** 安裝（會裝最新 stable，之後可 `rustup update`）。
全程使用**預編譯好**的 cargo 與 cargo-binstall，不在本機編譯，因此**不需要** Visual Studio / MSVC Build Tools，也**不需要系統管理員權限**。

本文件全部使用 **cmd** 指令，可直接在本機 cmd 或 ssh 到 Windows（OpenSSH 預設 shell 即 cmd）時使用。
只依賴 Windows 10 (1803) 以上內建的 `curl` 與 `tar`，不需要 PowerShell、winget。

## 1. 安裝 rustup（含最新的 cargo）

- 使用 cmd (不需要管理員權限)
> **cmd 請一次貼一行**：貼上多行時，cmd 可能把後一行黏在前一行參數後面（例如 `-ydel`）而出錯。以下每個指令都是獨立區塊。

```bat
curl -sSfL -o "%TEMP%\rustup-init.exe" https://win.rustup.rs/x86_64
```

- 此步驟會有點小久。
- 如果不需要 linux cross-compile(建議不要，直接用 github cross-compile 功能)，但之前有設定過。使用 `rustup target remove x86_64-unknown-linux-musl` 移掉，再執行。 

```bat
"%TEMP%\rustup-init.exe" -y
```

```bat
del "%TEMP%\rustup-init.exe"
```

> 安裝程式若警告找不到 MSVC / Visual Studio，可忽略：那只影響本機編譯，不影響使用預編譯檔。

rustup 會把 `%USERPROFILE%\.cargo\bin` 寫入**使用者 PATH**，所以安裝後**新開**的 cmd 或 PowerShell（ssh 則重新登入）都會認得 cargo。
讓目前這個 cmd 立刻認得：

```bat
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
```

```bat
cargo --version
```

> 若新視窗仍找不到 cargo，用 `echo %PATH%` 檢查是否含 `%USERPROFILE%\.cargo\bin`。
> 若你用 PowerShell，等價指令是 `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`。

## 2. 安裝最新的 cargo-binstall

直接下載官方預編譯的 zip，解壓到 cargo 的 bin 資料夾（已在 PATH 中）：

```bat
curl -sSfL -o "%TEMP%\cargo-binstall.zip" https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-pc-windows-msvc.zip
```

```bat
tar -xf "%TEMP%\cargo-binstall.zip" -C "%USERPROFILE%\.cargo\bin"
```

```bat
del "%TEMP%\cargo-binstall.zip"
```

驗證：
- 出現 `error: a value is required for '--version <VERSION>' but none was supplied` 仍然是正確

```bat
cargo binstall --version
```

## 3. 用 binstall 安裝 lobj-view

```bat
cargo binstall lobj-view --git https://github.com/snowray712000/lobj-view --disable-strategies compile
```

- 執行檔會裝到 `%USERPROFILE%\.cargo\bin\lobj-view.exe`。
- `--disable-strategies compile` 禁止退回從原始碼編譯，確保只用預編譯檔。
- 訊息中應看到從 `github.com/.../releases/download/...` 下載；若出現找不到檔案的錯誤，代表該版本的 GitHub Release 尚未提供 Windows 預編譯檔，需先補上 Release，而不是在本機安裝編譯工具。
- 升級：重新執行同一行指令。

## 4. 之後升級 Rust

```bat
rustup update
```

## 透過 SSH 連到 Windows 安裝

看哪份文件取決於**被安裝的那台機器的作業系統**，與用不用 ssh 無關：連到 Windows 就看本文件，連到 Ubuntu 才看 Ubuntu 版。

- 上面的指令在 ssh 預設的 cmd 下可直接貼上執行。
- PATH 是寫入使用者環境變數，需要**重新 ssh 登入**才會生效；或用 `set "PATH=..."` 在目前連線中暫時設定。
- 若連線進去的 shell 是 PowerShell（管理員改過預設 shell），`%TEMP%`、`%USERPROFILE%` 需寫成 `$env:TEMP`、`$env:USERPROFILE`；或先輸入 `cmd` 切到 cmd 再貼上。
