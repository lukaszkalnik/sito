---
status: ⬜ not started
---

# Slice 23 – macOS polish

Moved from the roadmap; concept and plan when the slice starts.

## Idea

Restoring from the macOS trash, macOS-specific fixes. Signing and notarization moved to [slice 50](50-macos-code-signing.md) (2026-11-07), the Mac App Store is [slice 51](51-mac-app-store.md).

## Notes

- Viewer zoom ([slice 13](13-viewer-zoom.md)): WebView2 sends a touchpad pinch as Ctrl+wheel, which the viewer handles. WKWebView sends a pinch as `gesturestart`/`gesturechange`/`gestureend` events instead, so pinch-to-zoom needs those handled on macOS.
- Main window bar (noted by the user on 2026-11-07): „Wybierz inny folder” does not look like a native macOS button. [Slice 48](48-compact-toolbar.md) restyled the main window bar: „Wybierz inny folder” and „Cofnij” are icon buttons, „Widok” is a dropdown, and all its buttons and dropdowns share one bordered look. [Slice 55](55-viewer-toolbar.md) gave the viewer bar the same look (in dark values). What is left here: check that look on macOS, in both bars.

## Builds on

- Auto update ([slice 41](41-update-check.md), [slice 42](42-zero-click-update.md)): the update manifest already lists a macOS bundle; enabling the install path on macOS is part of [slice 50](50-macos-code-signing.md).
- Undo from the trash ([slice 43](43-undo-rejections-only.md)): until macOS can restore from the trash, „Cofnij” skips trash moves there and looks inactive when only those are left (a click shows a toast saying to restore from the Trash by hand). Once restoring works, `TRASH_RESTORE` in `crates/core/src/trash.rs` becomes `Supported` and those entries are undoable again. One way to try: remember where macOS put each file in the Trash and move it back; check whether the Trash's privacy protection allows that without Full Disk Access.
