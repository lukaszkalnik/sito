# Slice 33 – Several cameras at one event

**Status:** 🧪 ready for testing (implemented 2026-10-30; what it established is in [docs/domain/grouping.md](../domain/grouping.md))

## Implementation notes

- Implemented as planned, with these differences:
  - Core and UI landed in one commit, because the new `get_series` shape breaks the app until both sides change.
  - The display order comes from the backend (`GroupingDto { order, series }`), and the UI arranges its entries by it (`ui/src/features/timeline/arrange.ts`).
  - `useTimelineEntries` loads the timeline and the series together, and reuses the timeline when only the gap changes, so entries and order always match. A mismatched answer (the timeline changed in between) is logged and keeps the previous view until the change event reloads both.
- `core::series::Grouping` pairs the series with their `Arrangement`, and the Tauri layer caches it per gap and timeline revision.
- A photo dated only by the file between two photos of a series no longer breaks the series. It never joins a series and stays a single photo at its own time.
- Fixtures: `fixtures/cameras/` has two cameras with interleaved shots.

## Testing (for the user)

In „złote gody”:
- no series mixes the R6 and the 5D;
- the 16:59–17:00 R6 („Rodzinne”) and 5D („Tort”) series are separate blocks whose headers name the camera;
- stepping in the viewer goes through a series in one piece;
- rejecting and undoing keep the series and their headers.

## Context

Split out of [slice 12](12-user-feedback-1.md) (item 6) on 2026-10-29: it changes how series are grouped ([Grouping](../domain/grouping.md)), unlike the quick viewer fixes there. From the user's first real test (2026-10-27).

When several cameras at one event have slightly different clocks, their series interleave (2 photos outside, 2 inside from another camera, then back outside).

Example folder: `D:\pulpit\złote gody`, 683 photos from a Canon EOS R6 and a Canon EOS 5D Mark III. At the default 10 s gap, 20 of its 101 series mix both cameras. One of them runs from 16:59 to 17:00 and interleaves 48 photos: R6 shots of „Rodzinne” and 5D shots of „Tort”, which are different scenes. When series are first split by camera, the folder has 99 series.

## Approved plan (2026-10-30)

**Goal:** a series holds the photos of **one camera** taken at most the gap apart, even when another camera's photos fall between them.

**Decided with the user:**
- Overlapping series are shown at the place of their **newest photo**. Single photos stay at their own time.
- A series header names the **camera only when the series overlaps** a series of another camera.
- Photos without camera info (no EXIF Make/Model, or EXIF still pending in the Takeout pass) count as **one more camera**: they form series only with each other.

### Key design: series stay contiguous in a "display order"

Every consumer (grid layout `ui/src/features/timeline/layout.ts`, `crates/core/src/navigation.rs`, the viewer page `src-tauri/src/viewer/page.rs`) assumes that a series is a contiguous range `{start, len}` of the timeline. Interleaved cameras break that assumption. The timeline itself stays sorted by time, because ids, thumbnails, insert and remove depend on that. Instead, grouping adds a **display order**: the timeline positions in time order, with each series pulled together at the place of its newest photo. Series are contiguous ranges in that order, so navigation (`neighbours`, `series_of`, `after_reject`) keeps working on display positions unchanged.

The display order is computed **only in the backend** and sent to the UI with the series, so the rule exists once and the grid and viewer can never disagree. Cost for 50,000 photos (estimated): about 0.8 MB in Rust (order plus inverse, cached like the series today), about 1 ms to build, about 300 KB of JSON per regroup (gap or timeline change only; `get_timeline` is several MB), a few ms to parse in the UI.

### Core (`crates/core`)

1. **`series.rs`: grouping by camera.**
   - `Series` becomes `{ members: Vec<usize> }`: timeline positions, newest first. The series list is sorted by its first member.
   - `find_series(photos, max_gap)` replaces `find_time_series`:
     - Bucket positions by a private `CameraKey` built from `ExifInfo::Read { camera }`. `Pending` and `None` share one "unknown" key.
     - Within each bucket, chain neighbours with the existing `belong_together`. The rule that photos dated by the file never join a series stays.
     - The work is O(n) with a `HashMap`.
   - `without_removed(series, removed)`: map members to positions in the list without removed photos, and drop series with no member left. The rule "a series keeps its shape while one photo is left" stays.
2. **New `series/arrangement.rs`: `Arrangement::new(count, &[Series])`.**
   - `order: Vec<usize>` maps display position to timeline position. A series is emitted whole at its first member; single photos stay in place.
   - `ranges: Vec<SeriesRange { start, len }>` gives the series in display positions. Rename navigation's `Series` input to `SeriesRange`.
   - Helpers: `display_of(timeline_position)` and `timeline_of(display_position)`.
   - Pure and O(n).
3. **Overlap label, in `series.rs` or `series/label.rs`.**
   - Two series overlap when their time spans (oldest to newest) intersect and their camera keys differ.
   - `enum SeriesLabel { Plain, Camera(String), UnknownCamera }` per series.
   - The label is computed on the grouped series, before rejected photos are removed, so the header does not change when photos are rejected.
4. **Tests** (unit, in `#[cfg(test)]`):
   - two interleaved cameras give two series;
   - a gap is measured within a camera, so another camera's photo in between does not break a series;
   - photos with no camera and photos with pending EXIF group together;
   - file-dated photos stay single;
   - the arrangement places each series at its newest photo and keeps single photos in place;
   - `without_removed` with members;
   - the overlap labels.
   - Update the navigation tests to `SeriesRange`.
   - Integration: extend `crates/core/tests/series.rs`. Add fixtures from a second camera (two JPEGs with another Make/Model interleaved in time with the pixel pair, generated by `scripts/make-fixtures.py`) and assert two separate series.

### Tauri (`src-tauri`)

- `series.rs`:
  - `cached_series` / `series_with_removed` build and cache a `Grouping { series, arrangement, labels }`, using the existing per-revision cache in `scan/state.rs`.
  - `get_series` returns `GroupingDto { order: number[], series: SeriesDto[] }`. `order` holds timeline positions (which equal the UI's entry indices) in display order. `SeriesDto` keeps `{ start, len }`, now as a range of display positions, and gains a `label` serialized as `{ kind: "plain" } | { kind: "camera", name } | { kind: "unknownCamera" }`.
- `viewer/page.rs`:
  - Map the photo's position to its display position, run `navigation` on `arrangement.ranges`, and map the results back to ids.
  - The strip uses the series members.
  - Rename `navigation`'s module doc and the `Series` imports.
- Update the tests in `series.rs`, `scan/state.rs` and `viewer/page.rs`.

### UI (`ui`)

- `features/timeline/api.ts`: `getSeries` returns `{ order, series }`; `Series` gains a discriminated `label` union. Validate the response there; for example, `order` must be a permutation of the entry indices.
- `useTimelineEntries.ts`: arrange the entries by `order` (one `map`, memoized per grouping) before they reach `buildRows`. The UI has no grouping rule of its own.
- `layout.ts` `buildRows`: unchanged logic, since series stay ranges over the arranged entries. Only the label is passed through to the `series` and `blocks` rows, i.e. to `RowGroup`.
- `TimelineRowView.tsx` `SeriesHeader`:
  - Append the camera, e.g. „3 zdjęcia · 17:00:11–17:00:22 · Canon EOS R6”, through new keys `series.headerCamera` / `series.unknownCamera` („nieznany aparat”) in `i18n/pl.json`.
  - The header is already truncated with a title tooltip, so long camera names fit small blocks.
- `TimelineView.tsx` summary: unchanged (it sums `len`).
- Gap slider hint in `pl.json`: „Zdjęcia z jednego aparatu zrobione w tym odstępie czasu tworzą serię.”
- Tests:
  - Vitest: arranging entries by `order` (`useTimelineEntries.test.ts`), and rejecting an invalid `order` in the api validation.
  - Header test: camera shown only for a camera or unknownCamera label.
  - Update the Playwright mocks in `ui/e2e/` to the new `get_series` shape (`{ order, series }`), and add one e2e case with two overlapping series showing camera names.

### Docs

- `docs/domain/grouping.md`: grouping per camera, the display order, the unknown camera, the overlap label.
- `docs/domain/timeline.md`: the header shows the camera when a series overlaps another.
- This file and the roadmap: 🧪 when done.
- Note in slices 14 and 19 that series are now member lists with a display order, which slice 14's edits can build on.

### Commits (on `main`, each with checks green)

1. `docs: plan slice 33` (this plan into the slice file)
2. `test: fixtures from a second camera`
3. `feat(core): group series per camera with a display order`
4. `feat: series per camera in the grid and viewer` (Tauri, UI, i18n, e2e)
5. `docs: describe grouping per camera, slice 33 ready for testing`

## Related slices

- [Slice 14](14-edit-series.md) (edit series) and [slice 19](19-split-by-content.md) (split by content) also change series boundaries.
- [Slice 31](31-lazy-grid.md) (lazy grid): the grid layout comes from the series.

## Verification

- Checks: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`.
- Analysis of `D:\pulpit\złote gody` (EXIF dumped with Pillow): expect 0 mixed series and about 99 series at 10 s, and compare with the counts in the app.
- Debug build, for the user to test in „złote gody”:
  - no series mixes cameras;
  - the 16:59–17:00 R6 and 5D series are separate blocks with camera names;
  - stepping in the viewer stays within a series;
  - rejecting and undoing keep the series and their headers.
