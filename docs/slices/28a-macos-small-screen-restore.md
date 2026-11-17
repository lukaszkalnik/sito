---
status: 🧪 testing
---

# Slice 28a – Window places on small Mac screens

Found by the real-app restart spec of [slice 28](28-real-app-e2e.md) on the macOS CI runner (2026-11-15). The plan below was approved on 2026-11-16 and implemented on 2026-11-17 (`7395a76`); what it established is in [windows.md](../domain/windows.md).

## The bug

On a Mac whose screen is smaller than the default window (1280×800), Sito forgets where the main window was: after a restart it fills the screen's visible area instead of coming back at its remembered size and position. The saved state is right (`.window-state.json` holds 920×620 at 20,40); only restoring it fails. Larger screens (the dev Mac) are not affected, nor are Windows and Linux.

## Cause

`window_fit::show_placed` restores the remembered state through the window-state plugin and then reads the window back to keep it on screen (`fit_to_screen`). On macOS the window's setters are dispatched asynchronously (tao), so right after the restore the window still reports its default frame. Logged on the runner (1024×768 screen, work area 1024×674 at 0,31):

```
main before restore: (-128, -32) 1280×800
main after restore:  (-128, -32) 1280×800   ← restore not applied yet
main after fit:      (-128, -32) 1280×800   ← fit computed from the default frame
```

The default frame does not fit, so the fit moves and shrinks the window to the work area, and that change lands after the restore and undoes it. On a screen where the default frame fits, the fit does nothing, so the restore stands.

Queuing the fit and show as a main-thread task after the restore was tried: it kept the restored size but lost the position on the dev Mac (the queued task and the dispatched setters are not ordered, and resizing an NSWindow keeps its bottom-left corner), so it is not the fix.

## Candidate approaches

1. Fit the remembered frame instead of reading the window back: take the saved state (the plugin's file, or keep our own copy when saving), fit it into the work area, and set position and size once. No read-back after a set.
2. Fit later: on the first `Moved`/`Resized` event after the restore, or once the window is shown. Simpler, but the window may visibly jump.

## Findings from the code (2026-11-16)

- **Confirmed:** tao 0.37.2 implements `set_outer_position`, `set_inner_size` and `set_maximized` on macOS by dispatching to the main queue (`set_frame_top_left_point_async`, `set_content_size_async`, `set_maximized_async`), also when called on the main thread. Nothing the plugin's `restore_state` sets is readable when it returns.
- **A remembered maximized window is hit too:** `fit_to_screen` skips maximized windows, but `is_maximized()` is still false right after the restore, so on a small screen the fit runs on the default frame and its change lands after the maximize.
- **The fit picks its monitor from the stale frame:** `current_monitor()` answers for the default, centred window, not for where the window is about to go. With two monitors the remembered frame may be fitted into the wrong screen's work area.
- **The plugin keeps its state private** (`WindowStateCache`, `WindowState`); only the file's name is public (`AppHandleExt::filename()`). The file is current whenever a window opens: the plugin reads it at startup, and `save_on_close` rewrites it for all windows whenever one closes (so also for a viewer reopened in the same run). It holds, per window label, the outer position (`x`, `y`), the inner size (`width`, `height`), `maximized`, and the position before maximizing (`prev_x`, `prev_y`), all in physical pixels.

## Plan

Approach 1, on all platforms: `show_placed` works out the frame from the remembered state and sets it once. Nothing is read back from the window after a set, so the order in which the system applies the setters no longer matters. Approach 2 still depends on the order of events (what broke the queued task) and may show the window jumping.

One commit per step, checks green.

1. **Windows are placed from their remembered state (`fix:`).** In `src-tauri/src/window_fit.rs`, with submodules in `window_fit/` so the file stays small:
   - **Data (`window_fit/saved.rs`):** reads the plugin's state file (the app config folder + `filename()`) and returns the remembered frame of one window label, as our own type with only the fields we use: `Saved { position, inner_size, maximized }`. For a maximized window the position is the one before maximizing (`prev_x`, `prev_y`), as in the plugin's restore. No entry, an entry with a zero size (the plugin's placeholder for a window it has not seen yet) and a missing file all mean "nothing remembered"; an unreadable file is logged as a warning and treated the same.
   - **Domain (pure, in `window_fit.rs`):** `placement(saved, frame_extra, monitors, primary) -> Placement`, where `frame_extra` is the title bar and borders (outer size minus inner size) and each monitor comes with its full area and its work area.
     - The remembered outer frame is the saved position with the saved inner size plus `frame_extra`.
     - Its monitor is the one whose area overlaps that frame the most; if none does (a disconnected monitor), the primary one.
     - A frame that lies within that monitor's work area (with `BORDER_SLACK`) is kept as it is; otherwise it goes through the existing `fit_into`.
     - The result is the outer position and the inner size to set, and whether to maximize afterwards.
   - **`show_placed`:**
     - With a remembered state: read `frame_extra` from the window before anything is set (it still has its settled default frame then), compute the placement, set the position, then the size (the order of the plugin's restore, which holds the remembered place on the dev Mac), maximize if it was maximized, and show.
     - With nothing remembered (first launch): nothing is set, so reading the window is safe; today's `fit_to_screen` stays for this case and keeps the default frame on a screen smaller than it.
     - The plugin's `restore_state` is no longer called. The plugin keeps tracking and saving the state (`skip_initial_state` already stops its own restore).
   - **Tests** (unit, on `placement` and on parsing):
     - the runner's case: 920×620 at 20,40 on a 1024×768 screen with work area 1024×674 at 0,31 stays where it was;
     - a frame larger than the work area is shrunk and moved into it, and the inner size leaves room for the frame's extra;
     - a frame on a disconnected monitor comes back onto the primary one;
     - with two monitors, the frame is fitted into the one it lies on, not the primary;
     - a maximized state uses the position before maximizing and asks to maximize;
     - parsing: a real state file, a missing label, a zero-size entry, a broken file.
   - **Real-app spec:** `e2e-app/specs/restart.spec.ts` drops `CHECKS_FRAMES`, so macOS compares the frames again. The macOS runner's screen shows the bug on every run, which makes it the regression test; the Windows and Linux runs guard the platforms that worked before.
2. **Docs (`docs:`).** A new `docs/domain/windows.md` (indexed in the README): both windows come back at their remembered size and place, a maximized one maximized; a window that would lie outside the visible part of its screen is moved and shrunk into it; one remembered on a disconnected monitor comes back on the primary one. This file and the roadmap get the status.

### Settled at the review

- **The plugin stays, and its file is read.** After this slice the plugin only loads, tracks and saves the state; restoring is ours. It stays because the saving half holds the platform quirks (a minimized window on Windows reports -32000, a maximized one the screen's frame) and works today. Its file format is not a public API, but `Cargo.lock` pins the plugin, so the format changes only with a deliberate update, and then the fallback (a warning, the window at its default place) and the restart spec on all platforms show it. Replacing the plugin with our own tracking and saving would be a slice of its own; `placement` and the restore path would stay as they are.
- **One path on all platforms**, not a macOS-only branch, as proposed. On Windows and Linux the setters apply at once, so the result should not change, and the monitor choice gets better there too.

## Verification

- `cargo fmt`, `clippy`, `cargo test` green; `npm run e2e:app` on the Mac with the frame comparison on.
- By hand on the dev Mac: move and resize both windows, restart, both come back; zoom (maximize) the main window, restart, it comes back zoomed and un-zooms to its earlier frame; remove the state file, start, the window opens centred.
- **Proof on CI:** `e2e-app.yml` green on `main` on all three platforms, twice in a row. The macOS job fails today as soon as the comparison is on, so its passing is the proof. The result goes into this file.

## Tests

The restart spec (`e2e-app/specs/restart.spec.ts`) compares window frames after a restart. Until this slice is done it skips that comparison on macOS (the saved-state check still runs there); this slice turns it back on. The macOS runner's screen is small enough to show the bug on every run.

## Verification result (2026-11-17)

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, e2e-app lint and typecheck: green.
- `npm run e2e:app` on the dev Mac with the frame comparison on: 4 of 4 specs pass, the restart spec included.
- By hand on the dev Mac: pending (move and resize both windows and restart; zoom the main window and restart; remove `.window-state.json` and start).
- CI: pending, `e2e-app.yml` green on `main` on all three platforms twice in a row.
