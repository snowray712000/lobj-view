# 1.19.2: --admin flag

Date: 2026-11-26. Project: `lobj-view`. Version 1.19.2. Follows [261126m](261126m_stat_download_v1183.md).

## User request
With `--admin` (e.g. on 0.0.0.0), allow the operations that were local-only: auto update, compress, sync, delete.

## Decisions & rationale
- All privileged endpoints share `assoc::guard_with`; `--admin` is honoured there (non-local origin and Host allowed, `X-Lobj-Requested` still required).
- The registry/file-association endpoints use `guard_impl(allow_admin=false)`: system settings stay local-only.
- Browsing scope (`unrestricted`) is unchanged: still confined to `--dir`.
- No authentication yet; SSO login is planned (see doc/TODO.md).

## Verification
`guard_rules` unit test extended and passing; build passes. Not exercised over a LAN. Pre-existing `lobj_api::tests::zip_as_folder` failure is unrelated.
