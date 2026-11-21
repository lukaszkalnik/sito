---
status: 📝 planned
---

# Slice 17 – Sharpness suggestion

Planned on 2026-11-21 in a cloud session (Linux container, no access to the user's galleries); revised the same day after the user's corrections (an absolute measure, a star on every series, a score on every photo). Waiting for the user's review: see [Open questions](#open-questions).

## Idea

Every photo of a series gets a **sharpness score** that means the same in every series, and the sharpest photo of each series is marked with a star as the suggestion.

- The app only suggests; the user decides ([Accepting](../domain/accepting.md)). A score or a star never moves, marks or changes a file.
- A relative score (the sharpest photo of a series = 100 %) was dropped: in a series where every shot missed focus it would still show 100 % and a star. An absolute score says when even the best shot is blurred, and stays comparable between series.
- This moves the core of [slice 44](44-out-of-focus.md) (an absolute measure) into this slice; slice 44 then adds single photos, the „Nieostre” view and dismissing marks (see [Related](#related)).

## Behaviour

- **Which photos:** only photos of series with at least two photos are measured (at the current gap, automatic and manual series). Single photos get no score in this slice.
- **The score:** a number from 0 to 100, „ostrość 82” (en „sharpness 82”, de „Schärfe 82“): roughly the share of the photo's sharpest edges that look sharp at the measured size (see [Measure](#measure)). Photos the measure cannot judge (almost no edges: fog, a clear sky, a plain wall; a RAW file whose embedded preview is below 1024 px on its long edge; an unreadable file) show „ostrość: –” with the tooltip „Nie da się ocenić ostrości tego zdjęcia” and take no part in the star.
- **Where the score shows** (open question 1): planned in the viewer bar's line 2 for the shown photo and under every thumbnail of the series strip; in the grid as a small label on the tile, or only in the viewer.
- **The star:** in every series with at least one scored photo, the photo with the highest score carries a star in an accent-coloured round badge in the **bottom-left** corner of its grid tile and of its strip thumbnail (the accepted check stays bottom-right, so both can show). Always shown, also when two photos score alike (ties go to the newer photo). Tooltip and accessible name: „Sugestia: najostrzejsze zdjęcie w serii” (en „Suggestion: the sharpest photo in the series”, de „Vorschlag: das schärfste Foto der Serie“).
- **No sharp photo in a series** (open question 2): when even the starred photo scores below the blur threshold (a setting, default to be tuned on the user's galleries, e.g. 40), its tile and the series header get a „Nieostre” mark (en „Blurry”, de „Unscharf“), so a series with no usable shot stands out.
- **What counts:** only photos shown in the series: rejected, trashed and gone photos never take part, so rejecting the starred photo moves the star to the next best. Accepted photos take part like any other; accepting does not hide the star. The star follows edits, „Cofnij”, the gap and refreshes, since it is worked out from the current series and the cached scores.
- **Background analysis.** After a folder's scan (and after the EXIF pass of a first Takeout scan, which can still change the series), Sito measures the photos in series of two or more at the **largest gap (60 s)**, so changing the gap rarely needs new measurements, newest first. A pill note at the bottom centre ([Main window bar](../domain/timeline.md#main-window-bar)) shows „Ocena ostrości: 1234 z 20 000.” (en „Rating sharpness: 1,234 of 20,000.”, de „Schärfe wird bewertet: 1234 von 20.000.“), about once a second, counting photos already measured, so a resumed pass goes on from where it stopped. Scores and stars appear as they are computed, refreshed at most every 2 s.
- **The viewer's series first.** When the viewer shows a series whose photos are not all measured, those photos are measured next.
- **Staying out of the way:** the pass runs on one thread, waits while the folder is scanned, refreshed or in its EXIF pass, stops when another folder is opened, and yields while thumbnails on screen are waiting.

## Measure

Pure functions in `crates/core` on a grayscale image, no I/O. An absolute measure looks at **how wide edges are**, not how much detail there is: a sharp edge goes from dark to light within 1–2 px at a given size, a blurred one over many, whatever the scene. The approach follows established no-reference blur metrics (edge width, Marziliano et al. 2002; just-noticeable blur and CPBD, Ferzli & Karam 2009, Narvekar & Karam 2011).

1. **Fixed size.** Decode downscaled (the reduced-size JPEG decode of `decode_photo`, RAW through its embedded preview, TIFF's first page) and resize to **2048 px on the long edge**, grayscale. At 1024 px a 12 MP phone photo is shrunk four times and a slight miss of focus disappears; 2048 px keeps it visible at about twice the decode time (open question 4). Photos smaller than that are measured at their own size, if their long edge is at least 1024 px.
2. **Edges.** Sobel gradients; edge pixels are the local maxima of the gradient magnitude above a contrast threshold. For each edge pixel, its **width** is the distance between the intensity minimum and maximum on either side along the gradient direction (rounded to horizontal or vertical).
3. **Per edge: sharp or not.** An edge counts as sharp when its width is at most the just-noticeable blur width for its contrast (wider for low-contrast edges, as in CPBD).
4. **Per tile:** tiles of 128×128 px; a tile with too few edge pixels is skipped. Tile score = share of its edges that are sharp.
5. **Photo score = mean tile score of the best 10 % of tiles with enough edges**, as 0–100. The subject is usually what is in focus, so a blurred background (bokeh) or a plain sky does not pull the score down. With too few judged tiles the photo has no score.

Motion blur widens edges in one direction only; it counts as blur here (the direction could tell it apart from defocus later, for slice 44's reason text).

Considered and dropped: variance of the Laplacian (the first plan; depends on the scene's texture, so only comparable within a series), Tenengrad (the same weakness), a no-reference quality model via ONNX (slice 44's approach 3; a model download and a license to check, possibly later if this measure is not good enough).

### Known limits

- **Focus on the background instead of the subject** (back-focus): sharp background tiles score high, so a shot whose subject is soft but whose background is sharp can get the star. Telling the subject apart needs faces ([slice 21](21-faces-and-eyes.md)) or the camera's focus point from maker notes; until then this is a known limit, named in the domain file once the slice ships.
- **A very small subject** (far below 10 % of the photo) shares the best tiles with blurred ones and scores lower.
- **Strong noise** (high ISO) adds fake narrow edges; the contrast threshold and the downscale damp it.
- **Intentional blur** (panning, long exposures) scores low; that is fine for a star within a series, and is slice 44's concern for marks.

## Storage

- Scores are **derived data** in the photo cache (`photos.db`), in a new table `analysis (path PRIMARY KEY, size, mtime_ns, sharpness INTEGER)` (`NULL` = measured, cannot be judged), so the photos table and its cache version stay as they are. The meta table gets `analysis_version`: a change of the measure empties only `analysis`.
- A row counts only while the photo's size and modification time match. Photos moved or renamed outside Sito ([Photos](../domain/photos.md#moved-and-renamed-photos)) take their score along like their metadata; rows of photos gone from the cache are removed with them.
- The blur threshold is a setting in the action log's settings (key `blur_threshold`); no settings screen until [slice 15](15-settings.md).

## Code

Package by feature: `analysis` (shared by slices 17, 18, 44–46) and `suggestion` (this slice's rule).

- `crates/core/src/analysis.rs` + `analysis/`:
  - `prepare.rs`: decode and normalise to the grayscale image (`PreviewTooSmall` as an error variant); one decode for every measure, so slices 18/45 later run their histogram on the same pixels.
  - `edges.rs`: edge pixels and their widths; `sharpness.rs`: `fn sharpness(image: &GrayImage) -> Option<Sharpness>` (`Sharpness` a newtype over `u8`, 0–100); `tiles.rs`: the tile grid and the best-share selection, generic over a per-tile measure.
  - `cache.rs`: the `analysis` table (scores of a folder, store a batch, move rows with moved photos, version check).
  - `pass.rs`: the order of work (photos in series of two or more at 60 s, newest first, measured ones skipped, a priority set for the viewer's series), batches of 50, cancellable, progress callbacks; no threads (the caller drives it).
- `crates/core/src/suggestion.rs`: `fn suggest(series: &[Series], scores: &Scores, threshold: BlurThreshold) -> Suggestions`, pure: per series `Suggestion { Sharpest { index, blurry: bool } , NoScores }`.
- `src-tauri/src/analysis.rs`: the background thread (own `Generation`, waits for scans and the EXIF pass, yields to visible thumbnails), events `analysis-progress` (throttled with `throttle.rs`) and `suggestions-changed`; command `get_suggestions` (star ids, scores of the current scan's photos in series, blurry series), command `prioritize_analysis(ids)` from the viewer; the viewer's photo details get `sharpness: number | null`.
- UI: `ui/src/features/suggestion/` (`api.ts`, a hook holding the suggestions like the accepted ids, the star badge and the score label shared by tile and strip); the pill note; strings in `pl.json`, `en.json`, `de.json`.
- A dev tool `cargo run -p sito-core --example sharpness -- <folder> [gap]` prints every series of a folder with each photo's score and the star, to check the measure on a real gallery without the app (the user can run it on their own folders).

## Tests

Cases, listed before writing them:

- **Measure (unit, synthetic images):** a step edge blurred with growing Gaussian σ gives growing widths and falling scores (strictly); a sharp pattern scores near 100, the same pattern at σ 2 and 4 well below; the score of a sharp image barely changes with its contrast and texture (a checkerboard, sparse lines, a low-contrast pattern), unlike a Laplacian variance; a sharp subject on a blurred background scores like the subject alone; a blurred subject on a sharp background (back-focus) is recorded as a known limit; motion blur lowers it; a uniform image and too few edges give no score; rotation by 90° gives the same score; images smaller than a tile, 1×1, very wide panoramas; a preview below 1024 px gives `PreviewTooSmall`; added noise on a blurred image does not make it score as sharp.
- **Real photos (integration, `fixtures/sharpness/`):** real photos with a known source and a free license (scikit-image sample photos, CC0 / public domain; and real bursts if a source works, see open question 5), scaled to about 2400 px, each with copies blurred by known amounts (defocus σ 1, 2, 4; motion blur) and with noise; each set is one series by EXIF time. The full pipeline (scan → series → analysis → suggest) stars the unblurred copy in every set, ranks the copies by blur, marks a set of only blurred copies as blurry, and scores the sharp originals of different scenes within a narrow band (the measure is absolute). Sources and licenses go into `fixtures/README.md` and, if any fixture needs attribution, the license notes.
- **Suggestion rule (unit):** series of 0, 1, 2 and many scored photos; the best first, in the middle, last; ties (newer photo); unscored photos mixed in, and a series with none scored; a rejected best photo (the star moves); accepted photos; manual series; a series whose best is below the threshold (blurry) and one just above; several series and single photos side by side.
- **Cache (unit):** store and read back; a changed size or mtime hides a row; moved photos carry their score; a new `analysis_version` empties only `analysis`; `NULL` rows are not measured again.
- **Pass (unit):** only photos in series of two or more at 60 s; newest first; resumes after a stop; the priority set goes first; stops on cancel within a batch.
- **UI (Vitest):** star on tile and strip with its label; score labels, „–” with its tooltip; the star and the accepted check together; the blurry mark; the pill note in all three languages (plural forms).
- **Mocked e2e (Playwright):** suggestions arrive while the grid is shown and stars and scores appear; rejecting the starred photo moves the star; the viewer's strip shows stars and scores.
- **Real app (`e2e-app/`, one spec):** open a gallery from `fixtures/sharpness/`, wait for the pass, the sharp photo of a set carries the star in the grid and in the viewer; reject it, the star moves; undo, it comes back.

## Testing in the cloud session and on the user's machine

- **In the container:** all of the above, including the real-app suite on Linux (Xvfb), and the dev tool on the fixture sets.
- **Real bursts in the container (2026-11-21):** Wikimedia Commons answers file downloads from the container with 429 (its policy wants a contact in the client's User-Agent); a suitable set was found there („Kautex Baustelle 2016-08-25 – 01…11”, 11 shots in 5 s from a Samsung Galaxy S7 Edge, CC BY-SA 4.0). The Princeton Photo Triage dataset (series with human best-shot votes) is offline; Flickr needs an API key. See open question 5.
- **On the user's machine:** the dev tool and the app on real galleries: do the stars match the user's picks; is the blur threshold right (which series get „Nieostre”); how long does the pass take for the Takeout of 34,600 photos on the USB disk at 2048 px, and does the grid stay smooth meanwhile; are small RAW previews of older Olympus cameras skipped.

## Open questions

1. **Where the score shows:** on every tile in the grid and in the viewer (planned: viewer bar and strip; grid tile as a small label), or only in the viewer?
2. **„Nieostre” mark on series with no sharp photo:** in this slice (planned), or left to slice 44?
3. **Scale of the score:** 0–100 as planned, or a few words („ostre”, „lekko nieostre”, „nieostre”) with the number only in the tooltip?
4. **Measuring size:** 2048 px (planned; catches slight misses, about twice the work of 1024 px), or 1024 px?
5. **Real bursts for testing in the cloud:** may the downloader name the public repo URL (https://github.com/lukaszkalnik/sito) as its contact in the User-Agent for Wikimedia, or is a Flickr API key available? Otherwise the fixtures use sample photos only.
6. **Measure only series at the largest gap (60 s)** (planned, so gap changes rarely need new work), or only at the current gap (less work, but a larger gap then waits for the pass)?

## Related

- [Slice 44](44-out-of-focus.md): reuses `analysis` (preparation, edges, tiles, cache, pass) and the absolute score; adds single photos, a per-photo „Nieostre” mark with dismissing, and the view in the „Widok” dropdown. Update its file when this slice ships.
- [Slice 18](18-exposure-weights.md): adds an exposure score and weights; `suggest` then ranks by a weighted score; the analysis table gets the histogram numbers ([slice 45](45-bad-exposure.md) reads them too).
- [Slice 21](21-faces-and-eyes.md): faces tell where the subject is, the fix for back-focus.
- [Slice 16](16-show-rejected.md): rejected photos shown in a series take no part in the star.
- [Slice 63](63-series-of-one.md): a series of one has no star either way.

## Try

Do the stars match your picks? Do series where every shot is blurred get „Nieostre”?
