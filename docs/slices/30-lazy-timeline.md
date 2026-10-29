# Slice 30 – Load the viewer lazily

**Status:** ✅ done (what it established is in [docs/domain/timeline.md](../domain/timeline.md#viewer))

## Context

Both windows load the whole timeline at once: `get_timeline` sends every entry (name, folder, relative path, time, time source, format) and `get_series` every series, and the UI works out everything else from that. For the user's Takeout (34,600 photos, measured 2026-10-29 in the debug build, slice 29a item 7) that is 8.5 MB of JSON and about 3.5 s per call, plus a long task of about 180 ms in the webview to parse it.

- **The viewer** loads it on every open, only to show one photo, its series strip and the way to the previous and next photo; until then it shows a spinner (slice 29a; before, a black window). Opening a photo takes about 3–5 s in a large folder.
- **The grid** loads it after every scan, every refresh and every `timeline-changed`.

**Scope (decided 2026-10-29):** this slice makes only the **viewer** lazy. The grid is [slice 31](31-lazy-grid.md), at the end of the roadmap; it builds on the backend series cache added here.

**Keeping the viewer window alive (hidden) between opens:** decided after measuring. Not needed: with this slice the viewer opens "lightning fast" (user, 2026-10-29), so there is no slice 30a.

## Plan (approved 2026-10-29)

Goal: the viewer asks the backend for one photo's "page" (a few KB). Stepping stays instant because the neighbouring pages are prefetched.

### Core: `crates/core/src/navigation.rs` (new, pure, unit-tested)

Ports the rules from `ui/src/features/viewer/navigation.ts` to positions on the timeline, using binary search over the sorted `series` (no full order vector):
- `pub enum View { Series, All }`: which photos are stepped through.
- `pub fn neighbours(count, series: &[Series], view, position) -> Neighbours { previous: Option<usize>, next: Option<usize> }`. `position` need not be in the view (a single photo opened in series mode).
- `pub fn series_of(series, position) -> Option<Series>`.
- `pub fn after_reject(count, series, view, position) -> Option<usize>`: the same rule as `afterReject`.
- Tests: the cases from `navigation.test.ts`, plus ends, empty series, and a position outside every series.

### src-tauri: series cache in `Scan` (`src-tauri/src/scan/state.rs`)

- Add `revision: u64`, bumped by `refresh`, `update_photos`, `remove` and `restore`. Add `series: Option<(Duration, u64, Arc<Vec<Series>>)>`: the cached series for one gap at one revision.
- `pub(crate) fn cached_series(state: &AppState, max_gap) -> Option<Arc<Vec<Series>>>` (in `src-tauri/src/series.rs`): on a hit, return the cache. On a miss, copy `photos_with_removed` under the lock, compute with `series_with_removed` after releasing it, then store the result only if the revision is unchanged.
- `get_series` uses it too, so the grid and the viewer share the computation (slice 31 builds on this).
- Accessors: `Scan::position_of(id)` and `Scan::entry_at(position) -> Option<(usize, &Photo)>`. `Scan::photo` reuses `position_of`.
- Rust tests: cache hit, and invalidation after remove, restore and update.

### src-tauri: `get_viewer_photo` command (`src-tauri/src/viewer/page.rs`, new)

- Input: `ViewerRequest`. `ViewMode` maps to `core::navigation::View` with `From`.
- Output: `Option<ViewerPhotoDto>`, `None` when the scan id differs or the photo is not on the timeline (yet):
  `{ entry: TimelineEntryDto, previous: Option<id>, next: Option<id>, series: Vec<TimelineEntryDto> (empty if not in a series), afterReject: Option<id> }`.
- Runs in `off_main_thread`. It locks only to look up positions, ids and the few photos it needs, then builds the DTOs. `TimelineEntryDto::new` becomes `pub(crate)`. Register the command in `lib.rs`.
- Tests on a small `Scan`: a page in both modes, a stale scan id, a rejected id.

### UI (viewer)

- `viewer/api.ts`: `ViewerPhoto` type and a typed `getViewerPhoto(request)` wrapper.
- Move `useTimelineRevision` out of `useTimelineEntries.ts` into `ui/src/features/reject/useTimelineRevision.ts` (a separate `refactor:` commit) so the viewer can use it too.
- `viewer/useViewerPhoto.ts` (new): state `{kind:'loading'} | {kind:'ready', photo} | {kind:'missing'} | {kind:'error'}`. It loads the page for the request and keeps the previous photo shown while the next one loads (no spinner flash). A small cache, keyed by id, is cleared when mode, gap, scan or timeline revision changes. After each load it prefetches the `previous` and `next` pages, so stepping is answered from the cache.
- `PhotoViewer.tsx`: props become `photo: ViewerPhoto` and `onShow(id)`. The strip renders `photo.series`, and the step buttons and keys use `photo.previous` / `photo.next`; photo preloading is unchanged.
- `ViewerApp.tsx`: no `useTimelineEntries`. Reject uses `photo.afterReject`. `missing` with a real id shows the spinner (a restored photo whose `timeline-changed` is pending; it reloads on the next revision), and `NO_PHOTO` shows `EmptyViewer` as today.
- Delete `viewer/navigation.ts` and its test (the rules now live in core).
- Update the Vitest tests (`PhotoViewer.test.tsx`, `ViewerApp.test.tsx`) and the e2e mock (`ui/e2e/tauri-mock.ts`: `get_viewer_photo` computed from the mock timeline/series with the same simple rules). `viewer.spec.ts` / `reject.spec.ts` must stay green, plus one check that the viewer no longer invokes `get_timeline`.

### Docs

- After implementing: `docs/domain/timeline.md` describes how the viewer loads (per photo, neighbours prefetched).

### Commits (on `main`, pushed one by one)

1. `docs: plan slice 30 (lazy viewer); grid moves to slice 31`
2. `feat(core): viewer navigation rules`
3. `refactor: cache series per gap in the scan`
4. `feat: get_viewer_photo command`
5. `refactor(ui): share useTimelineRevision`
6. `feat(ui): load the viewer one photo at a time` (incl. the e2e mock and tests)
7. `docs: slice 30 ready for testing` (status 🧪, domain doc)

## Verification

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e` (one heavy job at a time)
- The user, in the debug build on `F:\Google Photos` (34,600 photos): the viewer opens in well under a second, stepping is instant, the series strip is correct in both modes, rejecting continues as before, and undo shows the restored photo. If opening is still slow → slice 30a (keep the window alive).

## Related slices

- [Slice 29a](29a-fast-scan-test-fixes.md) item 7 (measurements), [slice 31](31-lazy-grid.md) (the grid), [slice 14](14-edit-series.md) (editing series changes the layout), [slice 16](16-show-rejected.md) (rejected photos as a view).

## Implementation (2026-10-29)

Done as planned, with two deviations:
- Plan commits 3 and 4 became one (`feat: get_viewer_photo command`): the new `Scan` accessors are only used by the command, so the cache alone would not pass clippy's dead-code check.
- `Scan::len` is called `photo_count`, so clippy does not ask for an `is_empty`.

Commits: `b1f56bd` core navigation, `4bb6072` command and series cache, `bc886a9` shared `useTimelineRevision`, `2bd7633` viewer UI.

**Checks:** cargo fmt, clippy and tests; oxlint, Prettier, `tsc -b`, Vitest (178 tests); Playwright 41 of 42. The failing one, `thumbnails.spec.ts` "loads the visible thumbnails top to bottom before the rows rendered ahead", is flaky and not caused by this slice: it failed 2 of 4 runs on the code from before it (`35fd02c`) too. To look into separately.

## Testing

In the debug build `target\debug\sito.exe`, on `F:\Google Photos`:
1. Opening a photo from the grid: the viewer shows it in well under a second (before, 3–5 s).
2. Stepping with the arrow keys and buttons is instant, in both views; the series strip and stepping across series match the grid.
3. Rejecting in the viewer goes on with the next photo of the series (or the view), and "Cofnij" shows the restored photo.
4. If opening is still slow, keeping the viewer window alive becomes slice 30a.

**Result (2026-10-29):** the user tested the debug build: the viewer "works lightning fast". Slice done; keeping the window alive is not needed.
