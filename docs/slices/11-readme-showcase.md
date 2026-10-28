# Slice 11 – Showcase screenshots for the README

**Status:** 🧪 ready for testing (how to regenerate the screenshots: [docs/images/README.md](../images/README.md))

## Context

The README has no picture of Sito, and the user's own photos can't be shown. This slice builds a small demo gallery from freely licensed stock photos and produces reproducible screenshots of the timeline and the viewer for the README.

User decisions (2026-10-28):
- **Screenshots from the mocked UI:** Playwright against the Vite dev server with mocked Tauri, like the e2e tests. Reproducible, no real app needed.
- **Mixed series:** real multi-shot sequences where stock sites have them; otherwise variations of one photo (slight crop/shift, exposure change, a little blur).
- **Polish UI** in the screenshots; retake after slice 22 adds English.
- **Subjects:** family/party, dog or pet, landscape/sunset, city/travel.

Not in scope: a "best photo" highlight. Scoring arrives in slice 17+, so the viewer shot shows a series with one photo rejected (Odrzuć/Cofnij), not a suggestion.

## Implementation

### 0. Docs
- Save this plan into `docs/slices/11-readme-showcase.md`, status 🚧 there and in `docs/ROADMAP.md`. Commit `docs: add slice 11 plan`.

### 1. Photo sources (`docs/images/demo-gallery/SOURCES.md`)
- Pick about 15–25 originals across the four subjects from Unsplash and/or Pexels (check their current license terms first: free use, attribution optional, documentation use allowed; note the terms and the date checked in the file).
- One row per original: id, subject, author, page URL, direct download URL, license.
- Machine-readable twin `scripts/demo-gallery/sources.json` (same data plus the series recipe: which originals form one real sequence, which get synthetic variations and how many).
- Commit `docs: list the demo gallery photo sources`.

### 2. Gallery builder (`scripts/make-demo-gallery.py`)
- PEP 8, type hints, run with `uv run --with pillow --with piexif scripts/make-demo-gallery.py`, same style as `scripts/make-fixtures.py`.
- Downloads originals into `demo/originals/` (cached; skipped if present), writes the gallery into `demo/gallery/` (both git-ignored, so no photos in the repo).
- Downscales to ~1600 px long edge, writes synthetic variations deterministically (fixed parameters per variation, no randomness).
- Writes EXIF `DateTimeOriginal`, `Make`/`Model` (e.g. Google Pixel 8, samsung SM-S911B) so photos form series seconds apart and several days, plus singles between series.
- Also writes `demo/gallery/manifest.json`: the timeline entries in the shape of `TimelineEntry` (`ui/src/features/timeline/api.ts`): id, name, relativeDir, relativePath, takenAt, fromFileDate, newest first.
- Add `/demo/` to `.gitignore`. Commit `chore: add a demo gallery builder for the README screenshots`.

### 3. Screenshot script (`ui/showcase/`)
- `ui/showcase/readme.spec.ts` with its own `ui/playwright.showcase.config.ts` (same webServer as `ui/playwright.config.ts`, `testDir: ./showcase`), so `npm run e2e` does not run it. Scripts: `"showcase": "playwright test -c playwright.showcase.config.ts"` in `ui/package.json` and `npm run showcase` at the root.
- Reuses `mockTauri` from `ui/e2e/tauri-mock.ts` with `timeline: { entries }` from the manifest, `series: 'byGap'`, and a summary from the manifest count. Routes `http://thumb.localhost/<id>` and `http://photo.localhost/<id>` (URLs from `ui/src/features/timeline/thumbnails.ts` and `ui/src/features/viewer/api.ts`) to files on disk with `route.fulfill({ path })`: photos from the gallery, thumbnails from small copies the builder also writes into `demo/thumbs/<id>.jpg` (outside the gallery so they are not "photos").
- Skips with a clear message if `demo/gallery/manifest.json` is missing.
- Shots, 1280×800, `deviceScaleFactor: 2` for sharpness, light and dark (`colorScheme`):
  - `docs/images/timeline-{light,dark}.png`: the timeline with series blocks and singles.
  - `docs/images/viewer-{light,dark}.png`: the viewer (`?window=viewer` + `viewerRequest`) on a series with one photo rejected.
- Output optimised as WebP if the size win is clear, otherwise PNG; target < 300 KB per image.
- Commit `docs: add README showcase screenshots` (script + images).

### 4. README
- Hero image at the top (`<picture>` with dark/light sources via `prefers-color-scheme`, which GitHub supports), a short feature list (finds series, timeline, viewer, reject with undo, offline) with the viewer shot, a "Photo credits" section linking `SOURCES.md`.
- Section in `docs/RELEASING.md` or a short `docs/images/README.md` on how to regenerate: build gallery, `npm run showcase`.
- Commit `docs: show Sito in the README`.

### 5. Done
- The slice adds tooling, not app behaviour, so nothing moves into `docs/domain/`; the regenerate how-to lives in `docs/images/README.md`. Status 🧪 in both files. No release (the app does not change).

### Changes during implementation

- All photos come from Unsplash: its search page is browsable, while scripted downloads from unsplash.com hit a bot check; the builder downloads from the `images.unsplash.com` CDN instead. 18 originals, 32 shots; the only real sequence is two photos of Krakowskie Przedmieście in Warsaw, all other series are variations.
- `sources.json` holds both photos and shots (time, camera, edit); the builder generates `SOURCES.md` from it, so the credits cannot drift.
- The manifest and thumbnails live next to the gallery (`demo/manifest.json`, `demo/thumbs/`), not inside it, so the gallery folder only holds photos and can be opened in the real app.
- Screenshots are JPEG (quality 88, about 200 KB each) at 1280×800, scale 1: PNG was 0.75–1 MB, and Playwright cannot write WebP.
- The timeline shot shows the default "Serie" view.

## Verification
- Delete `demo/`, run the builder: it downloads, builds the same gallery (manifest identical on rerun).
- `npm run showcase` produces the four images; open them to check they show series blocks, thumbnails of the real photos, correct Polish text and both themes.
- `npm run lint`, `npm run typecheck -w ui`, `npm run e2e` still green (showcase spec not included).
- README renders on GitHub with the hero image in light and dark mode.
