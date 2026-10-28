# Analysis (planned)

Signals computed for each photo to compare the shots of a series. None is built yet; each comes with its slice.

## Embedding

A vector describing the image content, from a small image model (e.g. DINOv2-small or CLIP). Used to split series by content ([Grouping](grouping.md#by-content-planned-slice-19)).

## Sharpness (slice 17)

Variance of the Laplacian: higher means sharper. Only comparable between shots of the same scene, which is exactly the case within a series.

## Exposure (slice 18)

Share of clipped highlights and crushed shadows in the histogram: lower is better.

## Faces and eyes (slice 21)

Faces and whether eyes are open or closed, from a small face model (e.g. YuNet + landmarks). A shot with closed eyes is rarely the one to keep.
