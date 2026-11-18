# 透過 GitHub Actions 做 cross-compile 與發佈

這份說明記錄 lobj-view 如何用 GitHub Actions 一次建出 Linux / Windows / macOS 的執行檔，並發佈成 Release。
設定檔：`.github/workflows/release.yml`。

## 為什麼要在 GitHub 上建置

- 專案用了 `ring`（TLS，含 C 程式碼），從 Windows 交叉編譯 Linux musl 會缺 C 編譯器。
- GitHub 提供各平台的 runner，Windows / macOS 直接原生建置；Linux 用 `cross`（Docker 內附 C 編譯器）。
- `web/dist` 已提交進 repo，所以 CI 不需要 Node.js。**改了前端後要先在本機 `npm run build` 並提交 `web/dist`。**

## 建置的目標

| 目標 | 方式 | 壓縮檔 |
|---|---|---|
| `x86_64-unknown-linux-musl` | `cross` | `.tar.gz` |
| `aarch64-unknown-linux-musl` | `cross` | `.tar.gz` |
| `x86_64-pc-windows-msvc` | Windows runner 原生 | `.zip` |
| `x86_64-apple-darwin` | macOS runner | `.tar.gz` |
| `aarch64-apple-darwin` | macOS runner | `.tar.gz` |

檔名：`lobj-view-v<版本>-<target>.tar.gz`（Windows 為 `.zip`），內含同名資料夾，資料夾內是執行檔與 README.md。

## 第一次設定（只做一次）

1. 在 GitHub 建立空的 repo。
2. 本機加入 remote 並推上去：
   ```bash
   git remote add origin https://github.com/<帳號>/lobj-view.git
   git push -u origin master
   ```
3. 到 repo 的 Actions 頁籤確認 workflow 有出現。
4. 若 Release 建立失敗（權限錯誤）：Settings → Actions → General → Workflow permissions 選 Read and write。workflow 檔內已宣告 `contents: write`，通常不需要改。

## 日常發版流程

1. 修改 `Cargo.toml` 的 `version`（例如 `0.0.3`），並執行一次 `cargo build` 讓 `Cargo.lock` 跟著更新。
2. 更新 `doc/Versions.md`，commit 並 push。
3. 推 tag，**tag 必須是 `v` + 版本號**（binstall 靠這個找檔案）：
   ```bash
   git tag v0.0.3
   git push origin v0.0.3
   ```
4. 到 Actions 頁籤看 5 個 build job；全部成功後 `publish release` job 會把檔案附到 Releases，並自動產生 release notes。

只想測試建置、不發佈：Actions → release → **Run workflow**（手動觸發不會建立 Release，產物可在該次 run 的 Artifacts 下載）。

## 重新發佈同一個版本

tag 已存在又要重建時，要先刪掉 tag 再重推：

```bash
git tag -d v0.0.3
git push origin :refs/tags/v0.0.3
git tag v0.0.3
git push origin v0.0.3
```

GitHub 上已存在的 Release 也要先在網頁刪除，否則檔案可能被舊的覆蓋或衝突。

## 使用者怎麼安裝

見 README「安裝」。重點：`Cargo.toml` 的 `[package.metadata.binstall]` 描述了上面的檔名規則，所以 `cargo binstall` 才找得到檔案。**如果改了 workflow 的檔名規則，要同步改這段設定。**

## 常見問題

- **某個 target 失敗**：點進該 job 看 log。Linux 兩個用 `cross`，失敗多半是 C 編譯相關；macOS / Windows 失敗通常是 Rust 程式碼本身的問題。
- **`cargo binstall` 找不到檔案**：確認 tag 是 `v<Cargo.toml 版本>`、Release 是公開的（private repo 需要 token）、檔名沒被改。
- **Release 內容是舊版**：binstall 讀的是該 tag 當下的 `Cargo.toml`，先 push 再打 tag。
