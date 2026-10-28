# Slice 19 – Split by content

**Status:** ⬜ not started (moved from the roadmap; concept and plan when the slice starts)

## Idea

Embedding model (downloaded on first launch, checksum-verified) splits series with different subjects; similarity slider.

## Notes

- A time series can mix subjects (e.g. turning from a person to a view). Split it where neighbours' image embeddings differ: cosine similarity below a threshold the user adjusts.
- Candidate models: a small image model such as DINOv2-small or CLIP (check the license).

## Try

Series with mixed subjects get split.
