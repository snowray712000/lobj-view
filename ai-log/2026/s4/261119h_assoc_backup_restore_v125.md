# 1.2.5: record the original file association before registering, restore on unregister

Date: 2026-11-19. Project: `lobj-view`. Version 1.2.5 (Cargo.toml bumped from 1.2.4). Follows [261119g](261119g_single_instance_file_assoc_icon_v124.md).

## Context / User request
After the user confirmed double-click open and the new icon works (and 1.2.4 was committed), they asked for 1.2.5: when registering, also record the previous state so it can be "restored" — safer.

## Decisions & rationale
- Backup lives in the registry, not a file: `HKCU\Software\lobj-view\AssocBackup\.ext` (survives deleting the data dir, travels with the registration). Values: `ExtKeyExisted` (DWORD), `HadOpenWith` (DWORD), `ClassDefault` (REG_SZ, only if there was one), `PrevEffective` (REG_SZ, the effective default ProgId at that time, omitted when none or when it was already ours).
- Saved **once** per extension, before anything is touched (`backup_save` is a no-op when the key exists) — otherwise a second `register` would record our own registration as "original".
- `unregister` *is* the restore (no separate command): remove our `OpenWithProgids` value unless it pre-existed (and the key if empty), put back `ClassDefault` or delete the default value, delete the `.ext` key if we created it and it is empty, delete the backup; when no backups remain delete `Software\lobj-view`; remove the ProgId when nothing references it.
- `UserChoice` cannot be restored (hash-protected). If the effective default is still lobj-view after restore, `Status.notes` says what the previous default was so the user switches back in Settings. Not testable here (needs a real Settings choice).
- No backup (registered with ≤1.2.4) → assume key existed, no prior value/OpenWithProgids: same behaviour as before.
- `Status` gained `notes`; `ExtStatus` gained `has_backup`/`previous`; CLI/web show "還原為：…"; web button renamed "取消註冊並還原所選".

## Pitfalls
- Git Bash (MSYS) rewrites `/f`, `/ve`, `/d` for `reg.exe` → test registry changes through PowerShell (`HKCU:` provider) instead.
- A running `lobj-view.exe` (the user's double-click test, launched from `target\debug`) locks the exe and fails `cargo build` with "access denied"; I stopped that process. Re-registering (tests) re-pointed the association to `target\release\lobj-view.exe`.

## Implementation (files)
`src/assoc.rs`, `web/src/main.ts` (+ rebuilt `web/dist`), `Cargo.toml` (1.2.5), `doc/Versions.md`, `README.md`.

## Verification
- `cargo test` 7 passed; debug+release build OK; `npm run build` OK.
- Live registry tests (then restored): A) no `.lobj` key before → register/unregister leaves no key, no backup root, no ProgId; B) `.lobj2` default `Foo.file` → backup records it, unregister keeps `Foo.file` and removes `OpenWithProgids`; C) `.stl` with a Store default → state identical after; D) unregister without a backup does not fail.
- NOT verified: the `UserChoice` note branch; web dialog look.

## Open items
- 1.2.6: make lobj-view show up in Explorer's right-click "Open with" list.
