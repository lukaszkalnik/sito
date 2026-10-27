# Slice 11 – Showcase screenshots for the README

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

The README has no picture of Sito, and the user's own photos can't be shown. A „glam shot” made from freely licensed stock photos shows at a glance what Sito does: series of similar shots, side by side, with the best one picked.

## Concept

- **Demo gallery:** a small set of stock photos (a few dozen) under a license that allows use in a project's documentation without per-image fees.
  - Candidates: Unsplash and Pexels licenses (free use, attribution optional), or CC0 / public-domain sources.
  - The license terms are checked for each source and noted in the repo.
  - Stock sites rarely have true bursts. Where real series of one moment aren't available, series are made from one photo: slight crops, small exposure changes, a little blur. That shows what Sito is for (similar shots, one best).
- **Realistic dates:** a small script writes EXIF capture times (`DateTimeOriginal`) and camera names into the copies, so they form series and days like phone photos do.
- **The script lives in `scripts/`,** e.g. `scripts/make-demo-gallery`, and builds the demo folder reproducibly from the downloaded originals. The originals and their source links are listed in `docs/images/demo-gallery/SOURCES.md`.
- **The screenshots** are taken reproducibly with Playwright against the UI (mocked Tauri, like the e2e tests) or from the real app:
  - **the timeline:** series blocks next to single photos
  - **the viewer:** a series with its strip, one photo accepted, Odrzuć/Cofnij
  - optionally both light and dark mode
  - The images are optimised (WebP or PNG, small) and stored under `docs/images/`.
- **README:** a hero image at the top, a short feature list with one or two more shots, and attribution of the photos as their licenses ask (or as a courtesy).

## Open questions

- Which subjects? E.g. a family party, a dog, a landscape at sunset: everyday moments with several shots each.
- Synthetic series (variations of one photo), or only real sequences found on stock sites?
- Screenshots from the mocked UI (fully reproducible) or from the real app (truest)?
- English or Polish UI in the screenshots? The UI is Polish until slice 22 adds English.

## Verification (outline)

- `scripts/make-demo-gallery` rebuilds the demo folder from scratch.
- Opening it in Sito shows the planned series.
- The screenshot script produces the README images.
- The README shows them, with the photo credits.
