# 261118d — User verification of HTTPS, README.md, version 0.0.2

Date: 2026-11-18
Project root: `D:\coding\ccu-cadcam\rust-workspace\lobj-view\`
Follows: [261118c](261118c_https_local_ca.md)

## Context / User request

The user tried the HTTPS / local-CA feature from 261118c and reported:
- On the Windows dev machine it worked. Gotcha: on the server machine `ca.crt` need not be downloaded (`trust --install` is enough), but the **browser must be fully restarted** afterwards.
- A Mac mini on the LAN also worked, but the macOS import instructions were incomplete: when double-clicking `ca.crt`, the keychain prompt offers 登入 / 本機項目 (default) / 系統; **"系統" must be chosen** and the local login password entered, otherwise an error occurs.
- Ubuntu (headless) was NOT tested. The user wants the reverse role later: Ubuntu as server, Windows as client. Deferred to the next session.
- Asked for a `README.md` in lobj-view documenting these operations, and to call this stage version **0.0.2**.

## Decisions & rationale

- README written in Traditional Chinese (end-user doc) covering: quick start, options, why HTTPS (WebGPU secure context), the semi-automatic LAN workflow (`trust --install`, `--host 0.0.0.0`, `/trust`, `/ca.crt`), per-OS import table, common pitfalls, cert file locations, ssh -L alternative, build notes.
- `Cargo.toml` version bumped 0.0.1 → 0.0.2 at the user's explicit request; `readme = "README.md"` and README added to `include` so `cargo package` carries it.
- `src/tls.rs` `client_instructions()` macOS line updated with the "系統" keychain tip (this text is shown by `lobj-view trust` and the `/trust` page).

## Pitfalls & gotchas (user-confirmed)

- Restart the browser fully after importing the CA, including on the server machine.
- macOS: keychain must be "系統", requires login password.

## Verification

- Confirmed by the user (not by me): Windows server + Windows browser OK; Windows server + Mac mini client OK.
- Not verified by me: the README/tls.rs text edits were not rebuilt in this session unless noted in the final report; Ubuntu server/Linux binary not tested.

## Open items

- Next session: Ubuntu as server, Windows as client. Needs a Linux binary (musl cross-compile from Windows fails because of `ring`; see 261118c) — build on Linux or via CI.
