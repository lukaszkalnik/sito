---
status: 📝 planned
---

# Slice 28b – Viewer stuck on its first photo

Found by the real-app e2e suite of [slice 28](28-real-app-e2e.md) on CI (2026-11-15). Cause found and plan approved by the user on 2026-11-16; nothing is implemented yet, step 1 of the plan comes next.

## The bug

Sometimes the viewer window never shows the photo it was opened for: it stays on „Wczytywanie zdjęcia…”. Seen on Linux and Windows CI, a few times in about a dozen runs of `e2e-app.yml`, in the viewer, reject and restart specs, both for the first photo of a new viewer window and for a photo sent to an open one. Never seen on the Mac. Until this is fixed, `e2e-app.yml` fails now and then for this reason, and a failure in those specs on Linux or Windows is most likely this bug.

## Cause

**Two `start_scan` calls overlap, and the older one finishes last.** Its timeline replaces the scan of the newer call, under a new scan id that no window knows. The main window keeps the id it got from the newer call, so everything it asks for by that id is refused from then on: `get_viewer_photo` returns `null` (the viewer waits forever), and `thumb:` and `photo:` find no photo.

How the two calls come about in the suite:

1. The harness's `openFolder` writes `sito.lastFolder` and reloads the main window. The app's first page is often still starting at that moment, reads the folder the harness just wrote and opens it itself: scan 1, a first scan that reads the photos.
2. The reload starts the page again, which opens the folder too: scan 2. If scan 1 has stored its photos in the photo cache by then, scan 2 shows the folder from the cache at once (`set_timeline`, id A) and returns A to the page.
3. Scan 1 comes back from its blocking thread and calls `set_timeline` as well (id B, later than A). `start_scan` never checks whether another opening started meanwhile (its comment says the UI opens no other folder while it waits, which a reload breaks). The page that asked for scan 1 is gone, so nobody holds B.
4. Scan 2's background refresh finds its scan replaced and ends without a word.

The same happens in the app proper whenever the main window's page reloads during a first scan. Whether a user can make a release build reload (the webview's own Ctrl+R or context menu) was not checked; a first scan of a large gallery takes minutes, so the gap would be wide there.

### Evidence

- **Failed runs** (Windows, run 43768208874, viewer and restart specs): the app log shows the first page load, then the six EXIF reads of scan 1, then the second page load, then one „thumbnail cache cleaned” line only. Each opening ends with that line, except a cached opening whose scan was replaced before its refresh ended (step 4). Passing specs of the same run show either one scan (the first page did not see the folder) or two scans one after the other with two such lines.
- **Diagnostic run** (throwaway branch `debug/slice-28b`, deleted; `workflow_dispatch` run 43948855029): log lines in `start_scan`, the refresh and `viewer_photo`, and the viewer spec 20 times each on Linux and Windows. All 40 passed, but the logs show the mechanism:
  - In 35 of 40 runs the first page opened the folder too, so two `start_scan` calls ran about 40 ms apart.
  - In one Linux run they overlapped: scan 1 (generation 1) installed its timeline when it was no longer the latest opening (`latest: false`), one millisecond before scan 2's cached timeline (ids …651461 and …651462). The other order of those two lines is the bug.
- Not seen on the Mac: there the first page has finished starting before the session begins, or the timing never lines up.

The transient race the concept suspected in `viewer_photo` (the grouping is found before the scan is locked again) exists but heals itself with the next `timeline-changed`; it is not this bug.

## Plan

One commit per step, checks green.

1. **An outdated opening never replaces the scan (`fix:`).** In `src-tauri/src/scan/state.rs` and `scan.rs`:
   - Openings get their own counter and token (`Opening`), apart from `Generation`: `refresh_folder` advances the generation too (to stop the EXIF pass), and a refresh started in the viewer must not cancel an opening that is in flight in the main window.
   - `AppState::set_timeline` takes the opening and installs the timeline only if no later opening has begun, checked under the scan lock (a later opening installs after its own begin, so the check and the lock together leave no gap). It returns `None` otherwise.
   - A first scan stops reading once a later opening has begun (`should_stop` instead of `|| false`), so two scans do not share the disk.
   - `start_scan` answers an outdated opening, cached or not, with a new error code `superseded`. `useScan` ignores that code (the newer call's result is the one to show), so it needs no message.
   - Tests (`state.rs`): the older of two openings finishing last is not installed and the newer one's scan id stays current; an opening is not outdated by a refresh; the newest opening is installed. Regression test in `viewer/page.rs`: after that sequence `viewer_photo` returns the photo for the newer opening's id. `scanReducer`/`useScan` test: `superseded` leaves the state alone.
2. **The viewer says when its scan is gone (`fix:`),** the concept's last point. `get_viewer_photo` tells the two reasons for „nothing” apart: its result becomes a tagged value (`photo` / `missing` / `scanReplaced`) instead of `Option`. For `scanReplaced` the viewer shows a message instead of the spinner, as for a missing folder; proposed text: „Ten folder nie jest już otwarty w oknie głównym. Wybierz zdjęcie w oknie głównym.” This is also what the viewer shows today (as an endless spinner) when another folder is opened while it is open. Tests: `viewer_photo` for another scan id and for a photo off the timeline; `useViewerPhoto` and `ViewerApp` for the new state.
3. **The harness opens the folder once (`test:`).** `openFolder` waits until the first page shows its start screen (the „Wybierz folder ze zdjęciami” button) before it writes `sito.lastFolder` and reloads, so every spec runs exactly one scan and no longer depends on timing. The overlap is covered by the unit tests of step 1, which are deterministic.
4. **Docs (`docs:`).** `docs/domain/timeline.md`: of two openings the later one wins; what the viewer shows when its folder is no longer open. This file and the roadmap get the status.

### Settled at the review

The plan was approved as written: the viewer shows the proposed message (it is not closed when another folder is opened), and no spec reloads the main window during a scan.

## Verification

- `cargo fmt`, `clippy`, `cargo test`, frontend lint, typecheck, Vitest and the mocked e2e green; `npm run e2e:app` on the Mac.
- **Proof on CI,** on a throwaway branch run by hand and deleted afterwards: with a delay before the first scan installs its timeline (so the older opening always finishes last) and the harness of today, the viewer spec fails without step 1 and passes with it. Then the whole suite 20 times on Linux and Windows with the fix, without the delay. The result goes into this file.
- Two consecutive green runs of `e2e-app.yml` on `main`.
