# Ubuntu：不用 apt 安裝最新的 cargo 與 cargo-binstall

## 為什麼不要 `sudo apt install cargo`

apt 的 cargo 版本偏舊（且與 rustup 管理的版本互相衝突）。本專案使用 `edition = "2024"`，需要 Rust 1.85 以上。
官方建議的安裝方式是 **rustup**，它會裝最新 stable，之後可用 `rustup update` 升級。

## 1. 安裝 rustup（含最新的 cargo）

```bash
sudo apt update
sudo apt install -y curl build-essential   # build-essential：之後若要從原始碼編譯才需要 C 編譯器
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

方法 A：官方安裝腳本（下載預編譯的 binstall，快）

```bash
curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
```

方法 B：用 cargo 從原始碼編譯（慢，需要 build-essential）

```bash
cargo install cargo-binstall
```

驗證：

```bash
cargo binstall --version
```

## 3. 用 binstall 安裝 lobj-view

```bash
cargo binstall lobj-view --git https://github.com/snowray712000/lobj-view
```

- 執行檔會裝到 `~/.cargo/bin/lobj-view`。
- 訊息中若看到從 `github.com/.../releases/download/...` 下載，代表用的是預編譯檔；若看到 compiling，代表找不到對應的 Release 檔而退回從原始碼編譯。
- 升級：重新執行同一行指令（有新版時會覆蓋）。

## 4. 之後升級 Rust

```bash
rustup update
```
