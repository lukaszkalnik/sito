---
status: ⬜ not started
---

# Slice 17 – Sharpness suggestion

Moved from the roadmap; concept and plan when the slice starts.

## Idea

Sharpness score per photo; the sharpest photo in each series is highlighted as suggested; debug panel shows scores.

## Notes

- Candidate measure: variance of the Laplacian (higher = sharper). It is only comparable between shots of the same scene, which is exactly the case within a series.
- The app only suggests; the user decides.

## Try

Do the suggestions match your pick?

## Related

- [Slice 44](44-out-of-focus.md) marks out-of-focus photos with a per-photo sharpness measure cached in the photo cache. Whichever comes first builds the measure and the background analysis pass; the other reuses them.
