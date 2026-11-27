# 1.20.4: flatten by level, folder-name masking

Date: 2026-11-27. Project: `lobj-view`. Version 1.20.4. Follows [261126p](261126p_dir_rename_flatten_v1203.md).

## User request
1. "拿掉這層" on a parent folder, choosing the Nth layer (aa1/lobj, aa2/lobj with R selected and N=2).
2. Port `no_user_info.py` (mask patient folder names with a password) into the UI, implemented in Rust, not Python.
(The second part of the message was cut off; confirmed via a question: web UI + Rust.)

## Decisions & rationale
- Level: select R, N = N levels below R (N=1 children, N=2 grandchildren); default 0 keeps the old "this folder itself" behaviour. The user's "default 1" was resolved to this explicit model.
- All targets are pre-checked (existing names and clashes between targets) and nothing moves on conflict; final deletion is `remove_dir` only.
- Masking is wire-compatible with the Python script (verified with vectors from it). Renames go through temp names so a new name equal to another old name cannot overwrite.
- Password is sent in the POST body (HTTPS on LAN by default), never stored or logged.

## Verification
Unit tests (python vectors, wrong password); curl: level-2 conflict 409, level-2 success (4 folders), missing level 400, mask/unmask round trip, wrong password leaves names untouched. Browser UI NOT clicked through.
