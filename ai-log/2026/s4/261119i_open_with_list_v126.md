# 1.2.6: show lobj-view in Explorer's "Open with" list for all five extensions

Date: 2026-11-19. Project: `lobj-view`. Version 1.2.6 (Cargo.toml bumped from 1.2.5). Follows [261119h](261119h_assoc_backup_restore_v125.md).

## Context / User request
After 1.2.5 was committed, the user asked for 1.2.6: when right-clicking a file in Windows and choosing "Open with", lobj-view should also be in that list.

## Findings
- Inspected the real list with `SHAssocEnumHandlers` (the API behind the menu; C# via `Add-Type` in PowerShell, `IAssocHandler`/`IEnumAssocHandlers`). Before the change lobj-view appeared only for the registered `.lobj/.lobj2`, never for `.stl/.ply/.obj`, and with the bare name `lobj-view.exe`.

## Decisions & rationale
- Add `HKCU\Software\Classes\Applications\lobj-view.exe`: `FriendlyAppName` ("lobj-view 3D 檢視器"), `DefaultIcon`, `shell\open\command`, `SupportedTypes\.ext`. The key name is the exe file name (constant `APP_EXE`), because Windows matches the "Open with" entry by it.
- Per extension also create `.ext\OpenWithList\lobj-view.exe` so it is in the recommended list, not only under "Choose another app".
- `register` with no `--ext` now covers **all five** extensions (was `.lobj/.lobj2`): registering is harmless for the ones that already have a default because the class default is only set when no effective default exists. Removed `--all` and `DEFAULT_EXTS`; the web dialog is all-checked by default.
- Backup gained `HadOpenWithList`; `unregister` removes our `OpenWithList` entry (and the key if empty), our `SupportedTypes` value, and finally `Applications\lobj-view.exe` + ProgId when no extension references them.

## Pitfalls
- `Test-Path HKCU:\...\.ply\OpenWithProgids` is True because Store apps have their own values there; check the *value names*, not the key, to confirm our entry is gone.
- PowerShell: `"$x : "` parses `$x :` as a scoped variable → build strings with `+`/`$($x)`.
- The user's own `target\debug\lobj-view.exe` was running again and locked the debug build; tested with `--release` instead and did not kill it. The association now points at `target\release\lobj-view.exe`.

## Implementation (files)
`src/assoc.rs`, `src/main.rs` (CLI), `web/src/main.ts` (+ rebuilt `web/dist`), `Cargo.toml` (1.2.6), `doc/Versions.md`, `README.md`.

## Verification
- `cargo test --release` 7 passed; release build + `npm run build` OK.
- `SHAssocEnumHandlers` (recommended) lists "lobj-view 3D 檢視器" for `.lobj .lobj2 .stl .ply .obj` after `register`; after `unregister --ext stl` only `.stl` loses it; after full `unregister` Applications, ProgId and backup keys are gone and only Store-app values remain in `OpenWithProgids`.
- NOT verified: the actual right-click menu rendering (only the API behind it).

## Open items
Linux (`.desktop` + `xdg-mime`) and macOS (needs `.app` bundle) still unsupported; multi-select open shows only the last file.
