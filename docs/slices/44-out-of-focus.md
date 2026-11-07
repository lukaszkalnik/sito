---
status: 📝 concept drafted
---

# Slice 44 – Mark out-of-focus photos

Since 2026-11-07; plan in plan mode when the slice starts.

## Idea

Asked by the user on 2026-11-07: Sito marks photos that are out of focus or blurred (missed focus, camera shake, a moving subject), so the user can find and reject them quickly.

Unlike [slice 17](17-sharpness.md), which compares photos **within a series** to suggest the sharpest one, this mark is **per photo** and applies to every photo of the folder, also to single shots that belong to no series. The app only marks; the user decides.

## Candidate behaviour

- A marked tile in the grid shows a small badge (e.g. „Nieostre”), and the viewer bar shows the mark with a short reason.
- The user can go through the marked photos only and reject them one by one with the usual keys. Suggested by the user on 2026-11-07: the view control „Widok” in the main window bar, today a segmented control „Serie” / „Wszystkie zdjęcia” ([Timeline](../domain/timeline.md)), becomes a **dropdown** with a third view for marked photos (e.g. „Nieostre”). Like the other views, it is kept per folder and pushed to the viewer, so stepping in the viewer goes through the marked photos only.
- A wrong mark can be dismissed („To zdjęcie jest w porządku”); a dismissed photo stays unmarked on later scans unless the file changes.
- Analysis runs in the background after the scan, like the EXIF pass of Takeout folders ([Photos](../domain/photos.md)), with a progress note; the grid stays usable and marks appear as they are computed.
- The threshold is a setting, so the user can make it stricter or looser.

## Candidate approaches

The hard part is an **absolute** measure: variance of the Laplacian (slice 17's measure) depends on the scene, so a sharp photo of a foggy sky scores lower than a blurred photo of a brick wall.

1. **Tiled sharpness.** Split the photo into tiles, measure sharpness per tile (Laplacian variance or gradient energy, normalised by local contrast) and use the sharpest tiles (e.g. the top 5 %). A portrait with a blurred background is still sharp where the subject is; a photo with no sharp tile at all is out of focus. Cheap, no model, but the threshold needs tuning on real galleries.
2. **Edge width.** Measure how wide strong edges are (in pixels at a fixed resolution); blurred photos have wide edges everywhere. Less scene-dependent than variance; motion blur widens edges in one direction only, which also tells it apart from defocus.
3. **No-reference quality model.** A small ONNX model trained for blur or image quality (check the license and size), run through `ort` like the planned models of slices 19 and 21. Likely the most robust; needs the model download.

## To check when planning

- **Resolution:** blur is invisible in a 256 px thumbnail. Which decode size is enough (e.g. 1–2 MP via the downscaled JPEG decode) and how long it takes for 34,600 photos on a USB disk. RAW photos only have their embedded preview ([Photos](../domain/photos.md)), which may be too small on older cameras: skip those rather than mark them.
- **Storage:** scores are derived data, cached in the photo cache keyed by file identity (and rebuilt when the measure changes); dismissals are the user's decisions and need durable storage (a new table via a numbered migration in the action log, like `accepted`).
- **Intentional blur** (panning shots, long exposures, artistic bokeh): how often they get marked on the user's galleries; whether EXIF (exposure time) helps.
- How to tune the threshold: a debug view of scores, as sketched in [the review UI idea](../ideas/review-ui.md).

## Build generically

This slice is the first of three per-photo marks; [slice 45](45-bad-exposure.md) (under- and overexposed) and [slice 46](46-accidental-photos.md) (accidental photos) add their own detectors. Build the shared parts here so they only add a measure:

- a mark **kind** enum (`OutOfFocus` now; `Underexposed`, `Overexposed`, `Accidental` later) with its score and reason;
- the background analysis pass (decode once, run every measure on the same pixels, cache the results);
- the badge, the viewer bar mark, the marked-only review and dismissing, all keyed by kind with their strings in `pl.json`;
- the view dropdown as a list of views (a discriminated union in the UI, one entry per mark kind), so slices 45 and 46 only add their entries. Whether there is one view per kind or one „Oznaczone” view with all kinds is decided when planning (one per kind keeps the dropdown long but each review focused). Other views could join it later, e.g. accepted photos ([slice 32](32-accept-photo.md)) or rejected ones ([slice 16](16-show-rejected.md)).

Slice 17 can then reuse the same sharpness measure and cache to compare photos within a series.

## Try

Open a folder with known blurred shots: are they marked, and are sharp photos with a blurred background left alone? Dismiss a wrong mark and rescan.
