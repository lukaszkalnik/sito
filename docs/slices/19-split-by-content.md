# Slice 19 – Split by content

**Status:** ⬜ not started (moved from the roadmap; concept and plan when the slice starts)

## Idea

Embedding model (downloaded on first launch, checksum-verified) splits series with different subjects; similarity slider.

## Notes

- A time series can mix subjects (e.g. turning from a person to a view). Split it where neighbours' image embeddings differ: cosine similarity below a threshold the user adjusts.
- Candidate models: a small image model such as DINOv2-small or CLIP (check the license).

## Try

Series with mixed subjects get split.

## Builds on

- [Slice 33](33-split-by-camera.md): a series is a list of its photos (`Series::members`, timeline positions) with a display order (`Arrangement`) built from it, so series need not be contiguous in time. Edits and splits only need to produce member lists; the grid and viewer follow the arrangement.
