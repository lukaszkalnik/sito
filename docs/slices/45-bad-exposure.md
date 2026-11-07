---
status: 📝 concept drafted
---

# Slice 45 – Mark under- and overexposed photos

Since 2026-11-07; plan in plan mode when the slice starts.

## Idea

Asked by the user on 2026-11-07: Sito marks photos that are far too dark („Niedoświetlone”) or far too bright („Prześwietlone”), so the user can find and reject them quickly.

Like [slice 44](44-out-of-focus.md), this is a **per-photo** mark for every photo of the folder, not the within-series exposure score of [slice 18](18-exposure-weights.md). The app only marks; the user decides.

## Candidate behaviour

- Two mark kinds, under- and overexposed, shown, reviewed and dismissed the same way as the out-of-focus mark of slice 44.
- Only clearly failed exposures are marked, not photos that are a bit dark or bright: a photo the user could still save in an editor is not the target.
- Thresholds are settings.

## Candidate approaches

1. **Histogram.** From the luminance histogram of the downscaled photo: the share of pixels clipped to black or white and the mean or median brightness. Overexposed = a large share of clipped highlights; underexposed = most pixels in the darkest range. Cheap, runs on the same pixels as slice 44's measure.
2. **Histogram plus EXIF.** Use exposure data (exposure time, aperture, ISO, flash, exposure compensation) to tell a deliberate night or low-key shot from a failed one, and a deliberately bright snow or backlit scene from a blown-out one. Many phone photos and Takeout copies lack some of these tags.
3. **Quality model.** If slice 44 adds a no-reference quality model, check whether it also rates exposure.

## To check when planning

- False marks on night photos, fireworks, concerts, snow, beach and backlit portraits in the user's galleries; whether clipping in large connected areas (a blown-out sky) should count less than clipping all over.
- Whether the measure works on the embedded JPEG preview of RAW photos, which is rendered with the camera's default curve (a RAW file may still be recoverable).
- Sharing the histogram with slice 18, so the within-series exposure score reads the same cached numbers.

## Builds on

- [Slice 44](44-out-of-focus.md): the mark kinds, the background analysis pass and cache, the badge, the marked-only review and dismissing. This slice adds the measure and its two kinds.

## Try

Open a folder with known black and white-out shots: are they marked, and are night and snow photos left alone?
