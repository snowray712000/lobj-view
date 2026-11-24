# 1.13.2: Right panel edits only the active object

Date: 2026-11-24. Project: `lobj-view`. Version 1.13.1 → 1.13.2.

## Context / User request
Bug report: in the right-hand object panel, changing a property (e.g. color) changed every object, but the user wants it to apply only to the selected (active) object.

## Decisions & rationale
- Confirmed: `buildSettings` edited a global `common: ObjProps`, and `effProps` spread `common` over every object (only an unset color fell back to per-file auto colors).
- Removed `common`; each file's props live in `propsOf` (`getProps(f)`). `effProps(f)` = own props + solo alpha cap. The settings card edits `getProps(activeFile())` and refreshes only that object. With no selection it edits a throwaway default.
- Dev environment moved to Linux: `web/node_modules` had win32-only native packages (typescript, rolldown); deleted and re-ran `npm install`; lockfile unchanged.
- Docs: Versions.md, README, version bumped to 1.13.2; web/dist rebuilt (`index-BB4Z5y4Q.js`).
- Working tree files were CRLF (Windows) while the index is LF; normalized only the files committed here to LF to avoid whole-file diffs.

## Verification
`tsc --noEmit` clean, `npm run build` ok, user tested manually and confirmed.
