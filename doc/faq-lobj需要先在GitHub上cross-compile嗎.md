# FAQ：lobj-view 在 GitHub Actions cross-compile 時，lobj 要先在 GitHub 上 cross-compile 嗎？

## 答案
**不用。** lobj 只要把原始碼推上 GitHub、打好 tag 就夠了，不需要為 lobj 另外設定 Actions，也不需要先建出任何東西。

## 為什麼
- lobj-view 的 `Cargo.toml` 是 **原始碼相依**：`lobj = { git = "https://github.com/snowray712000/lobj", tag = "v1.1.1" }`。
- Actions 對每個目標平台（Linux x86_64 / aarch64、Windows、macOS Intel / Apple Silicon）各跑一次 `cargo build --release --target <平台>`。
- 每一次 cargo 都會：
  1. 從 GitHub 抓下 lobj 該 tag 的原始碼；
  2. 針對**當次的目標平台**把 lobj 連同 lobj-view 一起編譯；
  3. 靜態連結進同一個執行檔（見 [faq-打包後是單一執行檔還是exe加dll.md](faq-打包後是單一執行檔還是exe加dll.md)）。
- 所以 lobj 是「在每個平台上被重新編譯一次」，而不是先編好再被拿來用。事先為某個平台編好的 lobj 對其他平台沒有用處，Rust 也沒有這種預編譯的 crate 分發方式（不像 C++ 的 `.lib` / `.dll`）。

## 這件事的實際影響
| 項目 | 需要嗎 |
|---|---|
| lobj 設定 GitHub Actions / cross-compile | 不需要 |
| lobj 發佈 Release 檔 | 不需要 |
| lobj 的 tag 先推上 GitHub | **需要**，lobj-view 的 `tag = "v1.1.1"` 才找得到 |
| lobj 額外的 C 編譯器或 cross 設定 | 不需要，lobj 是純 std、沒有相依套件 |

## 發版順序
1. lobj 改完 → commit → 打 tag（例如 `v1.1.2`）→ `git push` 與 `git push origin v1.1.2`。
2. lobj-view 的 `Cargo.toml` 把 `tag` 改成 `v1.1.2` → commit。
3. lobj-view 打 `v<版本>` tag 並 push，觸發 Actions 建置與 Release。

如果第 1 步的 tag 還沒推上去就先推 lobj-view 的 tag，Actions 會因為抓不到 lobj 而失敗。

## 注意
- 這個流程假設 lobj repo 是 **public**。若改成 private，Actions 抓不到原始碼，需要額外設定 token。
- 本機開發時，`rust-workspace/.cargo/config.toml` 的 `[patch]` 會讓 lobj-view 改用旁邊的 `./lobj` 資料夾，CI 看不到這個檔，所以 CI 一定是用 tag 指定的版本。
- 目前尚未實際跑過這個新的 CI 流程，上面是依 cargo 的行為推論，第一次發版時需確認。
