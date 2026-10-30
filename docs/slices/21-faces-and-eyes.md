# Slice 21 – Faces and eyes

**Status:** ⬜ not started (moved from the roadmap; concept and plan when the slice starts)

## Idea

Face and eye-state score (closed eyes penalized).

## Notes

- Candidate models: a small face detector with landmarks, e.g. YuNet (check the license before adding it).
- A shot with closed eyes is rarely the one to keep.

## Try

Series of portraits pick shots with open eyes.

## Can build on

- [Slice 13](13-viewer-zoom.md): the viewer's zoom reducer (`ui/src/features/viewer/zoom.ts`) zooms to 100 % at a given spot (`actual` with `at`), so "zoom to this face" only needs the face's position.
