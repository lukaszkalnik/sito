---
status: 📝 concept drafted
---

# Slice 28d – Un-zooming after a restart on macOS

Found while testing [slice 28a](28a-macos-small-screen-restore.md) on the dev Mac (2026-11-17). Concept only; no plan yet.

## The bug

On macOS, a window that was zoomed (maximized) when Sito quit comes back zoomed, but un-zooming it puts it in the top-left corner of the screen instead of the frame it had before the zoom. Its size is right; only its position is wrong. Within one run of the app un-zooming is right, because macOS itself keeps the frame from before the zoom. Behaviour today is in [windows.md](../domain/windows.md).

## Cause (as far as known)

- [slice 28a](28a-macos-small-screen-restore.md) places a window from the window-state plugin's file (`window_fit::saved`); for a maximized window it takes the position before maximizing from the plugin's `prev_x`/`prev_y`, as the plugin's own restore did before. It sets that frame, then maximizes, so macOS remembers that frame as the one to un-zoom to.
- The plugin (tauri-plugin-window-state 2.5.0, `src/lib.rs`, its `WindowEvent::Moved` handler) copies `x`/`y` into `prev_x`/`prev_y` on **every** move, not only when the window becomes maximized. The zoom animation on macOS moves the window several times, so `prev` ends up as a point from the animation. Logged in the state file on the dev Mac: before zooming the window was at (308, 556); after zooming the file held `x: 0, y: 60, prev_x: 2, prev_y: 64, maximized: true`.
- The saved inner size (`width`, `height`) is right: the plugin's `Resized` handler and its save on close skip a maximized window.
- So the file holds no correct pre-zoom position; nothing on the restore side alone can fix this.

## Open questions

- Which `Moved`/`Resized` events arrive during the zoom animation, and what does `is_maximized()` (`isZoomed`) answer for each? Decides whether tracking "the last frame while not maximized" from events is reliable on macOS. A short diagnostic log in a debug build answers it.
- Does Windows have the same problem? Maximizing there is not animated by the app; if it sends a single `Moved`, the plugin's `prev` is right. Check on the Windows machine, and Linux on CI if possible.
- Does the viewer window behave the same (it should; same code path)?

## Candidate approaches

1. **Track the normal frame ourselves.** On `Moved`/`Resized` of a window that is not maximized (and not minimized), keep its outer position and inner size; save that with the state on close (our own small file, or our own field next to the plugin's). `placement` then uses our position for a maximized window instead of `prev_x`/`prev_y`. Needs the event question above answered.
2. **Read the frame at the moment of maximizing**: keep the last normal frame and freeze it on the first event where the window reports maximized. Same data as 1, decided by event order.
3. **Replace the plugin's tracking and saving entirely** (noted in 28a's "Settled at the review" as a possible slice of its own). Larger; `placement` and the restore path would stay as they are.

## Tests

- Unit tests for whatever decides the remembered normal frame from a sequence of window events (pure, fed with the event sequence logged on macOS).
- The real-app restart spec (`e2e-app/specs/restart.spec.ts`) could maximize the main window before quitting and check that it comes back maximized and, after un-maximizing, at its earlier frame, if the window plugin's commands can un-maximize reliably on all three platforms.
