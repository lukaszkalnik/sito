# Slice 33 – Several cameras at one event

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

Split out of [slice 12](12-user-feedback-1.md) (item 6) on 2026-10-29: it changes how series are grouped ([Grouping](../domain/grouping.md)), unlike the quick viewer fixes there. From the user's first real test (2026-10-27).

When several cameras at one event have slightly different clocks, their series interleave (2 photos outside, 2 inside from another camera, then back outside).

## Concept

- Split series by camera (EXIF make and model), even when their times overlap. Core already reads `camera`.

## Open questions

- How overlapping series are ordered (e.g. by their newest photo, as the user suggested).
- How the header shows the overlap (e.g. „nakłada się z inną serią”, or the camera name).
- Photos without camera info.

## Related slices

- [Slice 14](14-edit-series.md) (edit series) and [slice 19](19-split-by-content.md) (split by content) also change series boundaries.
- [Slice 31](31-lazy-grid.md) (lazy grid): the grid layout comes from the series.

## Verification (outline)

- The party folder shot with several cameras keeps each camera's series together.
