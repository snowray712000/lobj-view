# 1.14.2: select a .zip, sync to another computer (rsync over ssh) or delete

Date: 2026-11-26. Project: `lobj-view`. Version 1.14.1 → 1.14.2.

- UI: folders / .zip in the left list are now click = select (click again = deselect), double-click = enter; `..` stays single-click. Selecting a real on-disk .zip (not one inside a zip cache) shows "同步到…" / "刪除" (`#zip-actions`, `selectDir` in `main.ts`).
- Sync dialog `#sync`: `user@host` (+ optional password field), "連線" lists remote folders over ssh (`POST /api/remote-ls`, returns `path`, `writable`, `dirs`), click to enter, "同步到這個資料夾" runs `POST /api/zip-sync` (`rsync -a -s --partial`). Host/last dir remembered in localStorage; password only in page memory.
- `src/zipsync.rs`: strict `valid_host` (no leading `-`, no `:`), local-only + `X-Lobj-Requested`, `kill_on_drop`, timeouts. ssh `BatchMode=yes` by default; with a password: `SSH_ASKPASS` = this executable (`askpass_mode()` at top of `main`, env `LOBJ_VIEW_ASKPASS`/`LOBJ_VIEW_PASSPHRASE`), `SSH_ASKPASS_REQUIRE=force`, `NumberOfPasswordPrompts=1`. `StrictHostKeyChecking=accept-new`.
- Gotchas found: user's key had a passphrase not in ssh-agent (BatchMode fails) and the host also accepts password login, so first version (publickey only) was wrong; the final failure was just a non-writable remote folder (`/mnt`) → added `test -w .` check (POSIX shell on remote) and disabled the button. Auth-hint text only for real auth errors.
- Verified: temp sshd (port 2222, passphrase key) end to end through the real server (wrapper `ssh` in PATH); user confirmed it works on their machines. Password-login path itself not tested here.
- Not committed: unrelated line-ending-only diffs (TODO.md, instance.rs, lobj.ts, props.ts, css, older ai-log files).
