# Ubuntu：不用 apt 安裝最新的 cargo 與 cargo-binstall

## 為什麼不要 `sudo apt install cargo`

apt 的 cargo 版本偏舊（且與 rustup 管理的版本互相衝突）。本專案使用 `edition = "2024"`，需要 Rust 1.85 以上。
官方建議的安裝方式是 **rustup**，它會裝最新 stable，之後可用 `rustup update` 升級。

## 1. 安裝 rustup（含最新的 cargo）

```bash
sudo apt update
sudo apt install -y curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

安裝時選 `1) Proceed with standard installation`。
完成後讓目前的終端機認得 cargo（之後開新終端機會自動生效）：

```bash
. "$HOME/.cargo/env"
cargo --version
```

> 如果提示「Your path is missing /home/<你>/.cargo/bin」，表示 `~/.cargo/bin` 不在 PATH：
> ```bash
> echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
> source ~/.bashrc
> ```
> 使用 zsh 的話改寫 `~/.zshrc`。

## 2. 安裝最新的 cargo-binstall

binstall 的作用是**直接下載預編譯好的執行檔**，不用在本機編譯。

使用官方安裝腳本，直接下載預編譯好的 binstall（不在本機編譯，因此不需要 C 編譯器）：

```bash
curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
```

驗證：

```bash
cargo binstall --version
```

## 3. 用 binstall 安裝 lobj-view

```bash
cargo binstall lobj-view --git https://github.com/snowray712000/lobj-view --disable-strategies compile
```

- 執行檔會裝到 `~/.cargo/bin/lobj-view`。
- `--disable-strategies compile` 禁止退回從原始碼編譯，確保只用預編譯檔。
- 訊息中應看到從 `github.com/.../releases/download/...` 下載；若出現找不到檔案的錯誤，代表該版本的 GitHub Release 尚未提供對應平台的預編譯檔，需先補上 Release，而不是在本機安裝編譯工具。
- 升級：重新執行同一行指令（有新版時會覆蓋）。

## 4. 之後升級 Rust

```bash
rustup update
```
