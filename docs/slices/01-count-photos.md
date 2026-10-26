# Slice 1 – Count photos

**Status:** 🧪 ready for testing

## Context

First vertical slice of Sito (see `docs/ROADMAP.md`). The repo currently has only a skeleton: `crates/core/src/lib.rs` (just `VERSION`), `src-tauri/src/lib.rs` (one `core_version` command), and a static UI (`ui/src/App.tsx` with a disabled "Wybierz folder ze zdjęciami" button, i18n in `ui/src/i18n/`).

Goal: the user picks a folder in the running app and sees how many photos it contains, with live progress while scanning. This proves the full path core → Tauri → UI (folder dialog, long-running command, progress streaming, typed errors, Polish plurals) that every later slice builds on.

User decisions:
- **Only JPEG** (`.jpg`, `.jpeg`, case-insensitive) counts as a photo. HEIC, PNG and DNG are ignored for now.
- **Skip hidden folders** (names starting with `.`, e.g. `.thumbnails`, `.trashed-*`) and folders that contain a `.nomedia` file (the Android convention for "not gallery media").

This plan lives in `docs/slices/01-count-photos.md`; every slice gets its own `docs/slices/NN-name.md`.

## Implementation

### 0. Docs
- Create `docs/slices/01-count-photos.md` with this plan.
- `docs/ROADMAP.md`: slice 1 → 📝 then 🚧; update its description to "JPEG only, hidden and `.nomedia` folders skipped"; add a line that every slice plan lives in `docs/slices/`.
- `CLAUDE.md` Roadmap section: mention `docs/slices/`.
- Commit `docs: add slice 1 plan`.

### 1. Core: scanner (`crates/core`)
New module `crates/core/src/scan.rs`, re-exported from `lib.rs`.

```rust
pub struct ScanProgress { pub entries_seen: u64, pub photos_found: u64 }
pub struct ScanSummary  { pub photos_found: u64, pub entries_seen: u64, pub unreadable_entries: u64 }

#[derive(thiserror::Error)]
pub enum ScanError { NotFound(PathBuf), NotADirectory(PathBuf), Io { path, source } }

pub fn is_photo(path: &Path) -> bool;            // .jpg/.jpeg, case-insensitive
pub fn scan(root: &Path, on_progress: impl FnMut(ScanProgress)) -> Result<ScanSummary, ScanError>;
```

- `walkdir::WalkDir` with `follow_links(false)` and `filter_entry` that prunes hidden dirs and dirs containing `.nomedia` (root itself is never pruned).
- Unreadable entries (permission denied etc.) are counted in `unreadable_entries`, not fatal. Only a missing or non-directory root is an error.
- Progress callback throttled: at most every 500 entries plus once at the end, so the UI isn't flooded.
- Only the file list is returned as a count for now; slice 2 will extend `scan` to return photo records. Keep the walker in one place so that change is small.
- Deps: `walkdir`, `thiserror`; dev-dep `tempfile`.
- Tests:
  - Unit (`scan.rs`): `is_photo` for `a.jpg`, `B.JPEG`, `c.Jpg`, `d.png`, `e.heic`, `noext`, `.jpg`-named dir.
  - Integration `crates/core/tests/scan.rs`: temp tree with nested folders, uppercase extensions, non-JPEG files, a `.thumbnails` dir and a `.nomedia` dir → exact count; progress callback called and final values match summary; missing root → `NotFound`; file as root → `NotADirectory`; empty folder → 0.
- Commit `feat(core): count JPEG photos in a folder tree`.

### 2. Tauri layer (`src-tauri`)
- Add `tauri-plugin-dialog` (Rust + `@tauri-apps/plugin-dialog` in `ui`), register it in `run()`, add `dialog:allow-open` to `src-tauri/capabilities/default.json`.
- New `src-tauri/src/commands/scan.rs` (keep `lib.rs` as wiring only):
  - `#[tauri::command] async fn start_scan(path: PathBuf, on_progress: Channel<ScanProgressDto>) -> Result<ScanSummaryDto, CommandError>`
  - Runs `sito_core::scan::scan` in `tauri::async_runtime::spawn_blocking`, forwards progress through the Tauri 2 `Channel` (the recommended way to stream progress to one caller).
  - DTOs with `#[serde(rename_all = "camelCase")]` live in `src-tauri`, so `core` has no serde/UI coupling.
  - `CommandError` serializes as `{ code: "notFound" | "notADirectory" | "io" | "internal" }`. No English text is sent, and the UI maps codes to Polish messages.
- Remove the placeholder `core_version` command (unused).
- Unit tests for the DTO mapping and `ScanError → CommandError` codes.
- Commit `feat(tauri): expose folder scan with progress channel`.

### 3. UI (`ui`)
- `ui/src/api/scan.ts`: typed wrappers `pickFolder(): Promise<string | null>` (dialog `open({ directory: true })`) and `scanFolder(path, onProgress): Promise<ScanSummary>` (invoke + `Channel`). Types mirror the DTOs. Only this file touches `@tauri-apps/*`.
- `ui/src/features/scan/useScan.ts`: hook with a reducer, `idle → picking → scanning(progress) → done(summary) | error(code)`.
- `ui/src/features/scan/ScanScreen.tsx`: button "Wybierz folder ze zdjęciami" (disabled while scanning); shows the chosen path; while scanning "Przeszukano {{count}} plików…" and "Znaleziono do tej pory: N zdjęć"; when done "Znaleziono 1 234 zdjęcia" plus, if >0, "Nie udało się odczytać N elementów"; error messages per code. `App.tsx` renders `ScanScreen`.
- New strings in `ui/src/i18n/pl.json` with `_one/_few/_many/_other` plural keys and `{{count, number}}` (pl-PL formatting).
- Tests (Vitest + Testing Library, `api/scan` mocked with `vi.mock`):
  - `useScan` reducer transitions, including cancelled dialog (null path → back to idle).
  - `ScanScreen`: progress text, final plural text (1/2/5/22), error message per code, button disabled while scanning.
- E2E (`ui/e2e/scan.spec.ts`): `page.addInitScript` installs a fake `window.__TAURI_INTERNALS__` (`invoke` + `transformCallback`) that answers `plugin:dialog|open` with a path and `start_scan` by sending two progress messages to the channel and then a summary. The test clicks the button and checks progress and final count. The helper goes in `ui/e2e/tauri-mock.ts` for reuse in later slices.
- Commit `feat(ui): pick a folder and show photo count`.

### 4. Finish
- Run every check (below), push after each commit, and check CI with `gh run list`.
- ROADMAP slice 1 → 🧪 (ready for testing). It becomes ✅ only after you've tested it.
- Commands section in `CLAUDE.md` stays valid (no new commands).

## Verification

Locally, one heavy job at a time (low RAM):
1. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (core integration tests are the pipeline e2e for this slice).
2. `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`, `npm run build`.
3. Manual: `npm run tauri dev` → click "Wybierz folder ze zdjęciami" → pick a folder → progress, then the count. Cross-check: in Explorer, search the folder for `*.jpg OR *.jpeg` (hidden folders excluded) and compare. Also try a folder with no photos and cancel the dialog.
4. CI green on Windows and macOS.

Risks / notes:
- First `tauri dev` with the dialog plugin recompiles some crates (a few minutes with 2 jobs).
- Scanning network drives or very large trees is not cancellable yet. Acceptable for slice 1, and cancellation can come with slice 2.
