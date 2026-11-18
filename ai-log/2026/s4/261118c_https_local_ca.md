# 261118c — HTTPS for `--host 0.0.0.0` (auto local CA + trust helper)

Date: 2026-11-18
Project root: `D:\coding\ccu-cadcam\rust-workspace\lobj-view\`
Follows: [261118b](261118b_release_build_and_linux_cross.md)

## Context / User request

`--host 0.0.0.0` over plain http loses WebGPU (non-secure context). User wanted HTTPS so 0.0.0.0 works, and a small install guide or small Rust program to make the manual certificate-trust steps semi-automatic.

## Decisions & rationale

1. **Local CA + leaf cert, not a bare self-signed cert.** The CA is created once (10 y); the server cert (1 y, renewed after 300 d or when the SAN set changes) is re-signed automatically. A client imports the CA once and never again, even when the LAN IP changes.
2. **HTTPS is automatic for non-loopback hosts** (`--http` forces plain, `--https` forces on for loopback). Loopback stays http because `localhost` is already a secure context.
3. **rustls with the `ring` backend** (`axum-server` feature `tls-rustls-ring`, `rcgen` default ring). Avoids aws-lc-rs (needs cmake/nasm on Windows).
4. **Semi-automation:**
   - `lobj-view trust` generates certs, prints paths/SANs and per-OS import steps for other devices.
   - `lobj-view trust --install` imports the CA on *this* machine (Windows `certutil -user -addstore Root`, macOS `security add-trusted-cert`; Linux prints instructions).
   - Server routes `/ca.crt` (download) and `/trust` (instruction page) so a phone/other PC can fetch the CA (one-time browser warning) then import it.
5. Own certs: `--cert` + `--key`. Extra names: `--san`.

## Implementation

- `src/tls.rs` (new): `ensure()`, `collect_sans()`, `install_ca_locally()`, `client_instructions()`. Files in `%LOCALAPPDATA%\lobj-view\tls\` (`ca.crt ca.key server.crt server.key server.san`); keys chmod 600 on unix.
- Issuer for re-signing is rebuilt from the same DN + stored CA key (`Issuer::new(ca_params(), ca_key)`), so no `x509-parser` needed.
- `src/main.rs`: new flags, `trust` subcommand, `axum_server::from_tcp_rustls(listener.into_std()?, ...)` reusing the existing port-fallback `bind()`.
- New deps: axum-server, rustls (ring), rcgen, dirs, local-ip-address, time.

## Pitfalls & gotchas

- **Linux musl cross-compile from Windows no longer works**: `ring` has C/asm and needs `x86_64-linux-musl-gcc` (cc-rs `ToolNotFound`). The "all pure Rust" premise from 261118b is gone. Options: build on Linux / GitHub Actions, or `zig cc` as CC. Not solved.
- Windows `curl` (schannel) needs `--ssl-no-revoke` with a private CA; use `openssl s_client -CAfile` to verify instead.
- Leaf SANs include IPv4 of all interfaces (including VPN/WSL adapters) and the hostname; IPv6 is only `::1`.
- A browser that has not imported the CA still shows a warning; a click-through page may or may not get WebGPU depending on the browser.

## Verification

- `cargo build` (debug) OK, no warnings.
- `--host 0.0.0.0 --port 18443`: `/api/health` = `ok` over https via `localhost` (curl + CA); openssl `Verification: OK` for `192.168.0.153` and `127.0.0.1`; without CA, connection refused as expected; `/ca.crt` 200 `application/x-x509-ca-cert`; `/trust` returns the page.
- NOT verified: `trust --install` (would change the user's trust store, not run); a real browser getting WebGPU over the LAN IP; macOS/Linux paths; release build on Windows.

## Open items

- Linux binary build path (see above).
- Run `lobj-view trust --install` and test in a real browser / second device.
