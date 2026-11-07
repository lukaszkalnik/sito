---
status: ⬜ not started
---

# Slice 23 – macOS polish

Moved from the roadmap; concept and plan when the slice starts.

## Idea

Restoring from the macOS trash, macOS-specific fixes. Signing and notarization moved to [slice 50](50-macos-code-signing.md) (2026-11-07), the Mac App Store is [slice 51](51-mac-app-store.md).

## Notes

- Viewer zoom ([slice 13](13-viewer-zoom.md)): WebView2 sends a touchpad pinch as Ctrl+wheel, which the viewer handles. WKWebView sends a pinch as `gesturestart`/`gesturechange`/`gestureend` events instead, so pinch-to-zoom needs those handled on macOS.
- Main window bar (noted by the user on 2026-11-07): „Wybierz inny folder” does not look like a native macOS button. Check the bar's other buttons too („Cofnij”, the segmented „Serie / Wszystkie zdjęcia” switch) against the macOS look. [Slice 48](48-compact-toolbar.md) reworks the bar's layout for small screens and may cover part of this.

## Builds on

- Auto update ([slice 41](41-update-check.md), [slice 42](42-zero-click-update.md)): the update manifest already lists a macOS bundle; enabling the install path on macOS is part of [slice 50](50-macos-code-signing.md).
