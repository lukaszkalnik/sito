---
status: 📝 planned
---

# Slice 28a – Window places on small Mac screens

Found by the real-app restart spec of [slice 28](28-real-app-e2e.md) on the macOS CI runner (2026-11-15). Concept only; the plan is still to be written.

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

## Tests

The restart spec (`e2e-app/specs/restart.spec.ts`) compares window frames after a restart. Until this slice is done it skips that comparison on macOS (the saved-state check still runs there); this slice turns it back on. The macOS runner's screen is small enough to show the bug on every run.
