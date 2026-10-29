# Slice 12a – Fixes from testing slice 12 (viewer)

**Status:** ✅ done (tested by the user on 2026-10-29; what it established is in [docs/domain/timeline.md](../domain/timeline.md))

## Context

Found by the user on 2026-10-29 while testing [slice 12](12-user-feedback-1.md) in the debug build:

1. „Odrzuć” at the left end of the viewer bar is not wanted after all. It goes back to the right, before „Cofnij”, but with a clear space of about one button's width between it and „Cofnij”. It stays solid red.
2. The other photos in the series strip are not dimmed; only the current one is marked (thick accent frame with a glow).

Zoom, also asked about in this test, is [slice 13](13-viewer-zoom.md).

## Plan

### 1. `feat(ui): put „Odrzuć” back on the right, spaced from „Cofnij”`
- `PhotoViewer.tsx` `ViewerBar`: the reject button goes after the photo info again.
- `PhotoViewer.css` `.viewer-reject`: `margin-right: 4rem`, which with the bar's 1rem gap is about one button's width.

### 2. `feat(ui): stop dimming the other photos in the series strip`
- `PhotoViewer.css`: drop the `opacity` on `.viewer-strip-item` and its hover/focus/current rule.

### 3. `docs: slice 12a ready for testing`
- `docs/domain/timeline.md` (viewer bar and strip), slice and roadmap → 🧪, debug build.

Checks: `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`.
