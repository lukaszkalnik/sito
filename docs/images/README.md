# README images

The screenshots in the README show the UI with mocked Tauri commands (like the e2e tests) and a demo gallery of stock photos. To regenerate them, from the repo root:

```bash
uv run --with pillow --with piexif scripts/make-demo-gallery.py
npm run showcase
```

1. `scripts/make-demo-gallery.py` downloads the photos listed in `scripts/demo-gallery/sources.json` and builds the gallery in `demo/` (git-ignored): photos with EXIF capture times and cameras, thumbnails and the timeline as `get_timeline` returns it. It also rewrites the credits in [demo-gallery/SOURCES.md](demo-gallery/SOURCES.md). Shots with an `edit` are variations of one photo (crop, exposure, blur), which turns a single stock photo into a series.
2. `npm run showcase` runs `ui/showcase/readme.spec.ts` (its own Playwright config, not part of `npm run e2e`) and writes `timeline-{light,dark}.jpg` and `viewer-{light,dark}.jpg` here.

To change what the screenshots show, edit `sources.json` (photos, times, cameras, variations) or the spec (window size, which series the viewer opens). Only use photos whose license allows this; note the license and the date it was checked in `sources.json`. The demo gallery can also be opened in the real app: `demo/gallery`.
