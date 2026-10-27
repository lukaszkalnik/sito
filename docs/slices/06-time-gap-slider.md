# Slice 6 – Time-gap slider

**Status:** ✅ done (tested by the user on 2026-10-27 with v0.6.4)

## Context

Slice 5 groups photos into series using a fixed gap of 10 s. In real galleries (e.g. "Złote gody") this has two failure modes:
- events shot in quick succession chain into one long series
- slow retakes get split apart

Slice 6 lets the user change the gap; the series then regroup instantly. That way the user can find the gap that suits their photos. Manual fixes ("cut here", add/remove/ungroup) are slice 10.

User decisions:
- **Range:** 1–60 s in 1 s steps, default 10 s (`DEFAULT_MAX_GAP` in core).
- **The gap is remembered per selected top-level folder**, so each gallery keeps its own value. It is stored locally in the UI for now; slice 9 moves settings to SQLite.

## Design

### Core / Tauri
No change needed:
- `get_series(max_gap_seconds: Option<u32>)` in `src-tauri/src/commands/series.rs` already takes the gap.
- `find_time_series` in `crates/core/src/series.rs` is O(n) and fast enough to call on every slider step for 10,000+ photos (a few ms plus IPC).
- Grouping logic stays in core; the UI never re-implements it.

### UI (`ui/src`)
- **`api/scan.ts`:** `getSeries(maxGapSeconds?: number)` passes `{ maxGapSeconds }`.
- **`features/timeline/gapSetting.ts`** (new):
  - `MIN_GAP = 1`, `MAX_GAP = 60`, `DEFAULT_GAP = 10`
  - `loadGap(folder)` / `saveGap(folder, seconds)` store a `{ [folderPath]: seconds }` map in `localStorage` under `sito.timeGap`
  - every access is wrapped in try/catch; invalid or missing values fall back to the default and are clamped to 1–60
  - unit tests cover a round trip, per-folder isolation, broken JSON, out-of-range values, and storage that throws
- **Data loading:** split `useTimelineEntries` into two parts:
  - entries load once per scan, as today
  - series reload per `(scanId, gap)`, and the latest request wins (stale responses are ignored)
  - while new series load, the previous ones stay on screen, so there is no "loading" flash while dragging
  - the state stays `ready { entries, series }` for the view; the hook takes `gap` as a parameter
- **`GapSlider` component** in the top bar, next to the mode toggle:
  - a labelled `<input type="range" min=1 max=60 step=1>`
  - the label "Odstęp w serii" shows the current value as "10 s" in an `<output>`
  - the value lives in `TimelineView` state, initialised from `loadGap(path)` and saved on change
  - the summary line ("128 serii (512 zdjęć).") updates with each regroup
  - the empty state "Nie znaleziono serii zdjęć." also reacts to the gap
- **Keep the view in place while regrouping.** Without this, dragging the slider makes the grid jump, which makes tuning on a specific event impossible.
  - `TimelineGrid` remembers the first visible photo (from the virtualizer range)
  - when `series` changes, a new `rowIndexForEntry(rows, entryId)` in `layout.ts` finds the row with that photo, or in "Serie" mode the next row after it if the photo is now a single
  - the grid then scrolls there with `scrollToIndex(…, { align: 'start' })`
  - switching between "Serie" and "Wszystkie zdjęcia" uses the same anchoring (requested during implementation): the grid stays mounted and the photo at the top stays put
- **pl.json strings:** `gap.label` "Odstęp w serii", `gap.value` "{{seconds}} s", and `gap.hint` (tooltip) "Zdjęcia zrobione w tym odstępie czasu tworzą serię."

### Tests
- **Vitest:**
  - `gapSetting` as listed above
  - `rowIndexForEntry`: photo found, photo became a single, past the end
  - `TimelineView`: the slider shows the stored value for the folder; changing it calls `getSeries(n)`, updates the count and saves the value
  - the hook ignores a stale response
- **Playwright:**
  - the mock's `get_series` gains a `byGap` mode that groups the generated timeline by the requested gap, mirroring core (EXIF only, inclusive)
  - spec `gap.spec.ts`:
    - moving the slider changes the series count
    - the value survives a reload for the same folder, and another folder starts at 10 s
    - after scrolling down and changing the gap, the photo at the top stays visible

### Docs
- `docs/slices/06-time-gap-slider.md` gets this plan. ROADMAP slice 6 goes 🚧 → 🧪.

## Implementation steps (one commit each, pushed)
0. `docs: add slice 6 plan`: the plan file, and ROADMAP slice 6 → 🚧.
1. `feat(ui): time-gap slider remembered per folder`: API param, `gapSetting`, hook split, slider, strings, unit tests, mock `byGap`, e2e for count and persistence.
2. `feat(ui): keep the visible photos in place when series regroup`: `anchoredScrollTop` (replaces the planned `rowIndexForEntry`), scroll anchoring for gap and view changes, unit tests and e2e.
3. `chore: release v0.5.0` after checks and CI are green, with your OK for the tag. Then the installer download and ROADMAP → 🧪.

## Verification
- **Local**, one heavy job at a time: `npm run lint`, `typecheck`, `npm test`, `npm run e2e`. Run `cargo fmt/clippy/test` too, even though only the UI changes, since CI runs them.
- **CI** is green, and the v0.5.0 release workflow including the smoke test is green.
- **Manual** (you, with v0.5.0):
  - open "Złote gody" and drag the slider down until the chained groups split into their moments
  - restart the app, pick the same folder and check the value is kept
  - pick another folder and check it starts at 10 s

## Risks / notes
- **Per-folder key** is the exact path string picked. `D:\Desktop` and `D:\Desktop\Złote gody` are different folders with separate values, which is what "per selected folder" means.
- **Storage location:** `localStorage` lives in the WebView2 profile of the installed app. It survives updates; it is lost on uninstall, which is acceptable until slice 9.
