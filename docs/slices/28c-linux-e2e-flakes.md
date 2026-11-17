---
status: 🚧 in progress
---

# Slice 28c – Flaky real-app tests on Linux

Found while verifying [slice 28b](28b-viewer-stuck-loading.md) (2026-11-16): with 28b fixed, the real-app suite (`e2e-app/`) still fails now and then on the Linux runner (WebKitGTK, `tauri-driver`). Cause found with a diagnostic run (throwaway branch `debug/slice-28c`, deleted; `workflow_dispatch` run 43968893663, the suite 40 times on Linux) and a local run on the Mac. Plan written on 2026-11-16 and approved by the user on 2026-11-17 as written. Steps 1 to 4 implemented on 2026-11-17; the [Verification](#verification) comes next. What the grid fix established is in [Timeline: Staying responsive](../domain/timeline.md#staying-responsive).

## The failures

Seen on Linux only, in about one suite run of ten; Windows and macOS passed every run (28b's [Result](28b-viewer-stuck-loading.md#result-2026-11-16)):

1. **A click on a tile is lost** (viewer spec: „the viewer does not show canon_12-00-02.jpg”; reject spec: „no viewer window”). The app never gets the request: the backend still holds the previous one, or no viewer opens.
2. **A tile is never found** (scan spec: „the tile of canon_12-00-06.jpg is not shown”), while the screenshot shows it with its thumbnail.
3. **The restart spec finds the main window elsewhere:** „x 20 instead of 0, y 40 instead of 112”.

## Cause

### 1. The grid builds its rows twice when it opens (an app bug)

`TimelineGrid` builds its rows for the width of the scroll area (`useElementWidth`, `useGridScroll.ts`). That width is 0 until the first `ResizeObserver` callback, which comes after the first render. So a grid that opens first builds its rows for width 0, where series stack as rows of their own (header row plus tile rows), and then again for the real width, where they sit side by side as blocks. The row keys differ (`tiles-…` vs `blocks-…`, `layout.ts`), so every row and every tile button is replaced.

- The diagnostic run's event recorder in the page shows the lost click: `pointerdown`, `mousedown` and `mouseup` reached „Otwórz canon_12-00-06.jpg” 266 ms after the page loaded, but no `click` followed. A browser fires no click when the pressed button is replaced before the release.
- A `MutationObserver` in the real app on the Mac shows the swap: about 60 ms after the page loads, all rows `timeline-row--series` are removed. Both layouts show the two series headers, so the harness's `expectCounts` (summary text and `h3` count) can already pass on the first one, and a fast click lands mid-swap. Linux is the platform where it lines up.
- A user never clicks within those milliseconds, but the first layout is wasted work: every tile is built twice when a folder opens.

### 2. WebKitWebDriver cannot hand out the tile (a driver quirk)

In the failed scan spec, all 100 polls of `waitForDisplayed` failed with WebKit's „Stale element found when trying to create the node handle”, raised while finding the element (`elements`), not while using an old one. WebKit's automation script raises it for a found node that is not connected or not in the document it holds (`WebAutomationSessionProxy.js`, `_createNodeHandle`), so a fresh lookup fails too, here for 10 s. Why the driver's view of the document went out of date is not found; the first trigger is most likely the grid's swap of point 1. The harness already works around this quirk for the series headers (`expectCounts`) and the viewer photo (`expectViewerShows`): they check inside the page by selector instead of through element handles.

### 3. `placeWindow` reads the frame back before the move has landed (a test bug)

The restart spec moves the main window to x 20, y 40, reads its frame back as the expected value, and compares after the restart. In the failure the main window *was* at x 20, y 40 after the restart; the expected value, x 0, y 112, is the main window's default place on the Linux runner (the frame of every failed scan spec). So that read came before GTK had applied the move, which it does asynchronously. In the diagnostic run all 80 placements read back right at once, so the window is narrow.

## Plan

One commit per step, checks green.

1. **The grid builds its rows once (`fix:`).** `useElementWidth` measures the scroll area in a layout effect (content width: `clientWidth` minus the padding, as `contentRect` gives it) and keeps following it with the `ResizeObserver`. Until that first measure it reports „not measured” (`null`), and `TimelineGrid` builds no rows then, so the first rows are built before the first paint and for the real width; nothing is laid out twice. Where nothing can be measured (jsdom), the measure is 0 and the grid behaves as today, so the unit tests stay as they are.
   - Tests: `useElementWidth` reports the measured content width after mount and follows a resize (stubbed `clientWidth`, padding and `ResizeObserver`). Mocked Playwright regression test (`ui/e2e/`): open a folder with series and record removed tile buttons with a `MutationObserver` (added by an init script); none is removed while the grid opens. It fails before the fix.
2. **The harness finds tiles inside the page (`test:`).** `waitForDisplayed` for tiles becomes an in-page check by selector, as `expectViewerShows` does (the button exists, is connected and has a size; `checkVisibility()` where available), so WebKit's element handles are not needed to see a tile. Clicks stay WebDriver clicks (a real pointer click is what the specs test); with step 1 the button no longer changes under them.
3. **`placeWindow` waits until the move has landed (`test:`).** After setting size and position it polls the frame until it has left the frame from before the move and two reads in a row agree (at most a few seconds, with a clear timeout message), and returns that frame.
4. **Docs (`docs:`).** `docs/domain/timeline.md` („Staying responsive” or the grid section): the grid lays out its rows once, for the measured width, before the first paint. This file and the roadmap get the status.

[Slice 28a](28a-macos-small-screen-restore.md) changes the restart spec too (it turns the frame comparison on for macOS); step 3 only changes `placeWindow` in `e2e-app/support/windows.ts`, so the two do not touch the same lines, and step 3 makes 28a's comparison on macOS just as safe.

## Verification

- `cargo fmt`, `clippy`, `cargo test` (nothing in Rust changes), frontend lint, typecheck, Vitest and the mocked e2e green; `npm run e2e:app` on the Mac.
- **Proof on CI,** on a throwaway branch run by hand and deleted afterwards: the suite 40 times on Linux and 20 times on Windows and macOS with the fix. No failure of the three kinds above; any other failure is looked into before this slice is done. The result goes into this file.
- Two consecutive green runs of `e2e-app.yml` on `main`.
