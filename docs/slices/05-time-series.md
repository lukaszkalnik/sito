# Slice 5 – Time series

**Status:** 🧪 ready for testing (v0.4.0)

## Context

Slices 3–4 show every photo with its thumbnail on a timeline. Sito's actual job starts here: find **series**, meaning photos shot seconds apart (retakes with a different exposure, angle or facial expression), so later slices can compare them and suggest the best one. Slice 5 groups photos into series by a fixed time gap and shows them. Slice 6 makes the gap adjustable, and slice 7 opens a series for comparison.

User decisions:
- **Two views with a toggle, "Serie" / "Wszystkie zdjęcia":**
  - "Serie" is the default after a scan. It shows only the series; single photos are hidden.
  - "Wszystkie zdjęcia" is the full timeline, with the series **marked inside it**.
- **Photos without an EXIF date are left out of series** for now. Their file dates (WhatsApp, downloads, copies) are unreliable and need extra criteria such as image similarity. That becomes its own new roadmap slice.

## Design

### Core (`crates/core`): new `series.rs`
- `pub struct Series { pub start: usize, pub len: usize }`: a run of consecutive photos in `Timeline::photos` (which is newest first). A series always has `len >= 2`.
- `pub const DEFAULT_MAX_GAP: Duration`: **10 s**, the upper end of the 5–10 s range in CLAUDE.md, so slow retakes still count. Slice 6 makes it adjustable.
- `pub fn find_time_series(photos: &[Photo], max_gap: Duration) -> Vec<Series>`:
  - walks neighbours once, O(n)
  - two neighbours belong together when both have `TimeSource::Exif` and their times differ by at most `max_gap` (inclusive)
  - a file-dated photo always ends a series, so series stay contiguous ranges
  - runs of length 1 are dropped
  - chaining means a series can be long, e.g. 40 photos 5 s apart during an event; slice 12 (split by content) handles that
- Unit tests:
  - empty and single photo
  - a gap of exactly `max_gap` stays together, `max_gap + 1 s` splits
  - singles are dropped
  - a file-dated photo in the middle breaks the series
  - several series in one timeline
- Integration test on the fixtures: the two `pixel_2024-06-01_12-00-0x` photos form the only series.

### Tauri (`src-tauri`)
- New command `get_series(max_gap_seconds: Option<u32>) -> Vec<SeriesDto { start, len }>` on the stored scan. `None` uses `DEFAULT_MAX_GAP`, and slice 6 will pass the slider value. Ranges instead of entries keep the payload tiny, and the UI already has the entries.
- DTO serialization test. The command is registered in `lib.rs`.

### UI (`ui`)
- `api/scan.ts`: `getSeries(): Promise<Series[]>` and the `Series` type.
- `useTimelineEntries` loads entries and series together (`Promise.all`), giving `ready { entries, series }`.
- **`layout.ts`** generalizes `buildRows(entries, series, columns, mode)` with `mode: 'series' | 'all'`:
  - **Units:** each series is one unit; each single photo is its own unit. A unit belongs to the day of its newest photo, so a series that crosses midnight is never split.
  - **Rows:** each day gets a day header. In `all` mode, consecutive single photos are packed into tile rows as today. Each series starts on a new line with a **series header row** (new `kind: 'series'`, about 28 px), followed by its tile rows, which are marked `inSeries` so they get a tinted background and frame.
  - **Mode `series`:** single photos are skipped, and so are days without series.
  - Keys stay unique and stable.
- **Series header text** (pl.json, with plurals): "Seria · 5 zdjęć · 12:00:00–12:00:08", using `timeOf` for the time range.
- **`TimelineView` top bar:**
  - the toggle is a segmented control of two buttons with `aria-pressed`, "Serie" and "Wszystkie zdjęcia"; it lives in `TimelineView` state and defaults to "Serie"
  - the summary adds the series count, e.g. "Znaleziono 1234 zdjęcia. 128 serii (512 zdjęć)." Plurals: 1 seria, 2 serie, 5 serii, 22 serie, 12 serii
- **Empty state** in "Serie": "Nie znaleziono serii zdjęć." with a hint to switch to "Wszystkie zdjęcia".
- `TimelineGrid` takes the rows (or `mode` and `series`) and renders the new row kind. Tile rendering and thumbnails are unchanged.

### Docs
- New roadmap slice **"Series without EXIF date"**, placed after slice 12 (Split by content), because it needs image similarity. It groups file-dated photos by time plus similarity. Later slices are renumbered (13 → 14, 14 → 15). Only slices 1–4 have plan files, so no links break.

## Implementation steps (one commit each, pushed)
0. `docs: add slice 5 plan`: `docs/slices/05-time-series.md`, ROADMAP slice 5 → 🚧, and add the new "Series without EXIF date" slice.
1. `feat(core): find series by time gap`: `series.rs` with unit and integration tests.
2. `feat(tauri): serve series of the current scan`: the `get_series` command, DTO and tests.
3. `feat(ui): series view and series marked in the timeline`:
   - layout and unit tests
   - toggle, header and counts with plural tests
   - e2e: the mock gains `get_series` (explicit ranges, or generated as "every n photos, a series of k"). Tests check:
     - "Serie" shows only series tiles with headers and the right count
     - the toggle switches to all photos with series framed
     - the empty state
     - scrolling 10,000 photos still renders under 200 tiles
4. `chore: release v0.4.0` once checks and CI are green (with your OK for the tag). ROADMAP → 🧪.

## Verification
- **Local**, one heavy job at a time: `cargo fmt --check`, clippy, `cargo test --workspace`, `npm run lint`, `typecheck`, `npm test`, `npm run e2e`.
- **CI** is green, and the v0.4.0 release workflow including the smoke test is green.
- **Manual** (you, with v0.4.0) on your gallery:
  - the series match what you remember shooting
  - single photos are gone in "Serie" and visible again in "Wszystkie zdjęcia", where series are framed
  - the series count looks plausible
  - WhatsApp or downloaded images never form series

## Risks / notes
- **Wrong gap:** a fixed 10 s may merge unrelated shots or split slow retakes. That's expected here; slice 6 adds the slider for tuning.
- **Phone clock jumps** (time zone change while travelling) only shift times; they don't break grouping, because neighbours come from the same phone.
- **Two phones at the same event** (different cameras) can interleave into one series. That's acceptable for now; content splitting comes in slice 12.
