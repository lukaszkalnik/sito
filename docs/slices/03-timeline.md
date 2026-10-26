# Slice 3 – Timeline

**Status:** ✅ done (tested by the user on 2026-10-27 with v0.2.0)

## Context

Slices 1–2 can count JPEGs in a folder (`crates/core/src/scan.rs` → `src-tauri/src/commands/scan.rs` → `ui/src/features/scan/`) and ship as an installable app. Slice 3 is the first step towards finding series: every photo needs a **capture time**. The timeline shows all photos of the chosen folder sorted by that time, so the user can check that dates and order are right before grouping (slice 5) relies on them.

Requirements:
- Capture time from EXIF (`DateTimeOriginal`, plus `OffsetTimeOriginal` if present). Fallback without EXIF: the file's modification time, marked as such.
- Also read EXIF orientation and camera model now (used by thumbnails in slice 4 and series later).
- **Must stay smooth with 10,000+ photos:** metadata is read in parallel with progress, and the UI renders only the visible rows (virtualization).

User decisions:
- **Newest first** (like the phone gallery / Google Photos).
- **Grid with day headers:** tiles in rows under a Polish day header ("poniedziałek, 26 października 2026"). In this slice a tile is a placeholder showing time and file name; slice 4 puts the thumbnail into the same tile.

## Design

### Core (`crates/core`)
- `scan.rs`: change the walker to collect `Vec<FoundFile { path, size, modified }>` instead of only counting (keeps hidden/`.nomedia` skipping, unreadable counting, throttled progress). `scan()` stays as the slice 1 API on top of it, or is replaced if nothing else needs it.
- New `metadata.rs`: `read_photo_metadata(path) -> PhotoMetadata { captured: Option<CaptureTime>, orientation: Orientation, camera: Option<String> }` using `nom-exif` (crate from the tech stack). Invalid/missing EXIF → `None`/defaults, never an error that stops the scan.
- New `timeline.rs`:
  - `Photo { path, size, modified, taken_at: NaiveDateTime, time_source: TimeSource::{Exif, FileModified}, orientation, camera }`
  - `build_timeline(root, on_progress) -> Result<Timeline, ScanError>`: walk (phase 1, "searching"), then read metadata **in parallel with `rayon`** (phase 2, "reading", progress = done/total), then sort **newest first** with a stable tie-breaker (path).
  - Progress type: `TimelineProgress { phase: Searching { found } | Reading { done, total } }`, throttled.
  - Capture times are local wall-clock times (EXIF has no time zone); mtime fallback is converted to local time. This is consistent for photos from one phone.
- Deps: `nom-exif`, `rayon`, `chrono`.

### Fixtures (`fixtures/`)
- Small JPEGs (a few KB) with known EXIF: different `DateTimeOriginal`, one with an offset, one with orientation 6, one without EXIF, one with a broken date. Generated once by `scripts/make-fixtures.py` (run with `uv run --with pillow --with piexif`), committed together with the script, and listed in `fixtures/README.md`.

### Tauri (`src-tauri`)
- Managed state `AppState { timeline: Mutex<Option<Timeline>> }`. Keeping it in Rust lets slice 4 serve thumbnails for the same photos.
- `start_scan(path, on_progress)` now builds the timeline, stores it in state and returns a summary (`photosFound`, `unreadableEntries`, `withoutExif`).
- New `get_timeline() -> Vec<TimelineEntryDto>`, a compact DTO: `id` (index), `name`, `relativeDir`, `takenAt` (ISO local string), `fromFileDate` (bool). That's roughly 100 bytes per photo, so 10,000 photos is about 1 MB, sent once. Paging can be added later if profiling shows a need; the UI already only renders what's visible.
- Progress DTO with `phase` tag (camelCase), errors unchanged.

### UI (`ui`)
- `@tanstack/react-virtual` for virtualization (small, well maintained).
- `features/timeline/layout.ts` (pure, unit-tested): turns entries + container width into virtual rows: `{ kind: 'day', date }` or `{ kind: 'tiles', entries[] }` with N tiles per row from the width; recomputed on resize.
- `features/timeline/Timeline.tsx`: virtualized list of those rows; day header formatted with `Intl.DateTimeFormat('pl-PL', { weekday, day, month, year })`; tile shows time (`HH:mm:ss`), file name (truncated, full name in `title`), and a small "data pliku" badge when the time came from the file date.
- Flow: after the scan the app switches to the timeline: top bar with folder, Polish count ("1234 zdjęcia, 12 bez daty EXIF") and "Wybierz inny folder". The scan screen shows the two progress phases ("Wyszukiwanie zdjęć…: 1234", "Odczytywanie dat: 500 z 1234").
- All new strings in `pl.json`, with plurals.

## Implementation steps (one commit each, pushed)
0. `docs: add slice 3 plan`: save as `docs/slices/03-timeline.md`, ROADMAP slice 3 → 🚧.
1. `test: add EXIF fixture photos`: script plus fixtures.
2. `feat(core): read capture time, orientation and camera from EXIF`: `metadata.rs` with unit tests against fixtures.
3. `feat(core): build a timeline sorted by capture time`: walker refactor and `timeline.rs`, with integration tests in `crates/core/tests/timeline.rs` covering fixtures, fallback, order, progress phases, and a 10,000-file performance smoke test (empty files → mtime fallback, must finish in a few seconds).
4. `feat(tauri): keep the timeline in app state and serve it`: commands, state, DTO tests.
5. `feat(ui): virtualized timeline grid with day headers`: layout unit tests, component tests, e2e with a mocked 10,000-photo timeline (asserts only a small number of tiles in the DOM, scrolling to the end shows the oldest day), scan progress phases.
6. `chore: release v0.2.0`: tag after checks and CI are green (tag push needs your OK). ROADMAP → 🧪.

## Verification
- Local: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace` (incl. fixture tests), `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`, one heavy job at a time.
- CI green on Windows and macOS. The v0.2.0 release workflow incl. the smoke test is green.
- Manual (you, with the installed v0.2.0):
  - pick a real phone gallery folder (10,000+ photos): progress shows both phases; scrolling is smooth from top to bottom
  - newest day at the top; dates and times match the phone's gallery for a few spot checks
  - photos without EXIF (e.g. WhatsApp images, screenshots saved as JPEG) show the "data pliku" badge.

Risks / notes:
- Reading EXIF of 10,000+ files on a spinning disk could take a while. Parallel reads and progress make it visible; caching comes in slice 9 (Persistence).
- `nom-exif` API details are checked against its docs while implementing; if it can't read a common Android file, fall back to `kamadak-exif`.
