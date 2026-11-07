---
status: ⬜ not started
---

# Slice 18 – Exposure and weights

Moved from the roadmap; concept and plan when the slice starts.

## Idea

Exposure score added; score weights adjustable in settings.

## Notes

- Candidate measure: share of clipped highlights and crushed shadows in the histogram (lower = better).
- Each photo gets a weighted score from its signals (sharpness, exposure, later faces and eyes); the best score is suggested. Weights and thresholds are tuned in the app itself (settings plus the debug panel), not in a separate prototype.

## Try

Tune weights on series with exposure differences.

## Related

- [Slice 45](45-bad-exposure.md) marks under- and overexposed photos from the same histogram. Compute the histogram numbers once and cache them for both.
