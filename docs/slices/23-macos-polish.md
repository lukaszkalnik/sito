# Slice 23 – macOS polish

**Status:** ⬜ not started (moved from the roadmap; concept and plan when the slice starts)

## Idea

Signing, notarization, restoring from the macOS trash, macOS-specific fixes.

## Notes

- Viewer zoom ([slice 13](13-viewer-zoom.md)): WebView2 sends a touchpad pinch as Ctrl+wheel, which the viewer handles. WKWebView sends a pinch as `gesturestart`/`gesturechange`/`gestureend` events instead, so pinch-to-zoom needs those handled on macOS.
