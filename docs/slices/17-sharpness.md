---
status: 📝 planned
---

# Slice 17 – Sharpness suggestion

Planned on 2026-11-21 in a cloud session (Linux container, no access to the user's galleries); waiting for the user's review.

## Idea

Sharpness score per photo; the sharpest photo in each series is highlighted as suggested; debug panel shows scores.

- The app only suggests; the user decides ([Accepting](../domain/accepting.md)). A suggestion never moves, marks or changes anything.
- Within a series the photos show the same scene from the same camera seconds apart ([Photos](../domain/photos.md#what-sito-looks-for)), so a relative measure is enough here. [Slice 44](44-out-of-focus.md) later needs an absolute one for single photos; this slice builds the measure, the cache and the background pass generically so slice 44 only adds its threshold and marks.

## Behaviour

- **Mark.** In every series with at least two photos that have a score, the photo with the highest score carries a **suggestion mark**: a small star in an accent-coloured round badge in the **bottom-left** corner of its grid tile and of its thumbnail in the viewer's strip (the accepted check stays bottom-right, so both can show together). Tooltip and accessible name: „Sugestia: najostrzejsze zdjęcie w serii” (en „Suggestion: the sharpest photo in the series”, de „Vorschlag: das schärfste Foto der Serie“). In the viewer bar, line 2 adds „najostrzejsze w serii” after the date source note for the suggested photo.
- **Only a clear winner is suggested.** If the second sharpest photo scores within the **margin** of the sharpest (default 5 %), the series gets no mark: the photos are about equally sharp and other qualities decide (expression, framing; later slices 18 and 21). The margin is a setting (stored in the action log's settings, key `sharpness_margin`; no settings screen until [slice 15](15-settings.md)).
- **What counts:** only photos shown in the series: rejected, trashed and gone photos never take part, so rejecting the suggested photo moves the mark to the next sharpest one (if it still wins clearly). Accepted photos take part like any other; accepting a photo does not hide the mark (it says what the measure thinks, the check says what the user decided).
- **Every series, automatic and manual,** at the current gap; the suggestion follows edits, „Cofnij”, the gap and refreshes, since it is worked out from the current series and the cached scores. Single photos get no mark.
- **Photos without a score** (not measured yet, unreadable, or a RAW file whose embedded preview is too small, below 640 px on its long edge) do not take part; a series with fewer than two scored photos has no mark yet.
- **Background analysis.** After a folder's scan (and after the EXIF pass of a first Takeout scan, which can still change the series), Sito measures the photos that are in a series at the largest gap (60 s), newest first, in the background. Photos outside every series are skipped for now; slice 44 extends the pass to all photos. A pill note at the bottom centre ([Main window bar](../domain/timeline.md#main-window-bar)) shows „Ocena ostrości: 1234 z 20 000.” (en „Rating sharpness: 1,234 of 20,000.”, de „Schärfe wird bewertet: 1234 von 20.000.“), updated about once a second, counting the photos of the folder already measured, so a resumed pass goes on from where it stopped. Marks appear as series get their scores, refreshed at most every 2 s.
- **The viewer's series first.** When the viewer shows a series whose photos are not all measured, those photos are measured next, ahead of the rest, so the mark is there by the time the user looks through it.
- **Staying out of the way:** the pass runs on one thread, never while the folder is scanned, refreshed or in its EXIF pass (it waits and goes on after), and stops when another folder is opened. Visible thumbnails keep priority: the pass yields while thumbnails of the screen are waiting.
- **Scores pane (debug panel).** In the viewer, the key **I** (an action in `keymap.ts`, „Pokaż wyniki ostrości”) toggles a line under each strip thumbnail with its score relative to the series' sharpest (e.g. „100 %”, „82 %”, „–” without a score), and the bar's line 2 adds the raw score of the shown photo („ostrość 1834”). Off by default; the choice is kept per viewer (local storage). This is how the measure and the margin are tuned on real galleries.

## Measure

Implemented in `crates/core`, pure functions on a grayscale image, no I/O:

1. **Fixed resolution.** Decode downscaled (the existing reduced-size JPEG decode in `decode_photo`, RAW through its embedded preview, TIFF first page) and resize to **1024 px on the long edge**, grayscale. A fixed size makes scores of one series comparable even when a manual series mixes cameras, and evens out sensor noise, which would otherwise inflate the measure. Orientation is ignored: the measure does not depend on it.
2. **Laplacian per tile.** Convolve with the 3×3 Laplacian and compute the variance of the response in tiles of 64×64 px (16×12 tiles for a 4:3 photo).
3. **Score = mean variance of the sharpest 10 % of tiles** (at least one tile). The subject is usually what is in focus: this ignores a blurred background (bokeh) and a plain sky, which would drag a whole-image variance down, and it is the per-tile base slice 44 needs for its absolute mark.

Candidate measures considered: whole-image Laplacian variance (simplest, but the background dominates), Tenengrad (Sobel gradient energy; similar results, slightly more work), edge width (slice 44's approach 2; better as an absolute measure, more code). Tiled Laplacian variance is the cheapest that copes with bokeh; if testing on real series shows wrong picks, Tenengrad on the same tiles is the first thing to try.

## Storage

- Scores are **derived data** in the photo cache (`photos.db`), in a new table `analysis (path PRIMARY KEY, size, mtime_ns, sharpness REAL)` (`NULL` = measured, no score, e.g. preview too small), so the photos table and its cache version stay as they are. The meta table gets `analysis_version`: a change of the measure empties only `analysis`.
- A row counts only while the photo's size and modification time match, so a changed file is measured again. Photos moved or renamed outside Sito ([Photos](../domain/photos.md#moved-and-renamed-photos)) take their score along, like their metadata; scores of photos gone from the cache are removed with them.
- Nothing goes into the action log except the margin setting: a suggestion is not a decision.

## Code

Package by feature: a new feature `analysis` (shared by slices 17, 18, 44–46), and `suggestion` for this slice's rule.

- `crates/core/src/analysis.rs` + `analysis/`:
  - `sharpness.rs`: `fn sharpness(image: &GrayImage) -> Sharpness` (newtype over `f32`), the tiled Laplacian; `tiles.rs`: the tile grid and the top-share selection, generic over a per-tile measure (slice 44 and 45 reuse it).
  - `prepare.rs`: decode and normalise to the 1024 px grayscale image, `PreviewTooSmall` as an error variant; one decode for every measure (later the exposure histogram of slices 18/45 runs on the same pixels).
  - `cache.rs`: the `analysis` table in the photo cache (read scores for a folder, store a batch, move rows with moved photos, version check).
  - `pass.rs`: the order of work (photos in series at 60 s, newest first, already measured skipped, a priority set for the viewer's series), batches of 50, cancellable, progress callbacks; no threads (the caller drives it).
- `crates/core/src/suggestion.rs`: `fn suggest(series: &[Series], scores: &Scores, margin: Margin) -> HashSet<usize>`: the suggested photo per series, pure. An enum `Suggestion { Clear(index), TooClose, NotEnoughScores }` per series keeps the reason testable.
- `src-tauri/src/analysis.rs`: the background thread (own `Generation`, waits for scans and the EXIF pass, yields to visible thumbnails), events `analysis-progress` (throttled with `throttle.rs`) and `suggestions-changed`; command `get_suggested_ids` (like `get_accepted_ids`), command `prioritize_analysis(ids)` from the viewer; the viewer's photo details get `sharpness: number | null` and `suggested: boolean`.
- UI: `ui/src/features/suggestion/` (`api.ts` wrappers, a hook keeping the suggested ids like the accepted ones, the badge component shared by the tile and the strip); the pill note in the timeline; the scores line and key I in the viewer. Strings in `pl.json`, `en.json`, `de.json`.
- A dev tool `cargo run -p sito-core --example sharpness -- <folder> [gap]` prints every series of a folder with each photo's score and the suggested one, so the measure can be checked on a real gallery without the app (and run by the user on their own folders).

## Tests

Cases, listed before writing them:

- **Measure (unit, synthetic images):** a sharp pattern scores higher than the same pattern blurred with growing Gaussian σ (strictly decreasing), and than motion blur; a uniform image scores 0; a sharp subject on a blurred background scores like the sharp subject alone (top tiles), whereas a whole-image variance would not; the score does not depend on rotation by 90°; images smaller than one tile; 1×1 and very wide panoramas; a tiny RAW preview gives `PreviewTooSmall`.
- **Real photos (integration, `fixtures/sharpness/`):** a few real photos from scikit-image's sample data (license checked; only CC0 / public domain ones), scaled to about 1200 px, each with copies blurred by known amounts (defocus σ 1, 2, 4 and a motion blur) and with a little added noise; each set is one series by EXIF time. The full pipeline (scan → series → analysis → suggest) must suggest the unblurred copy in every set, and rank the copies by blur.
- **Suggestion rule (unit):** series of 0, 1, 2 and many scored photos; the winner first, in the middle, last; two photos within the margin (no mark) and just outside it; unscored photos mixed in; a rejected winner (mark moves on); an accepted photo; manual series; several series and single photos side by side; ties.
- **Cache (unit):** store and read back; a changed size or mtime hides a row; moved photos carry their score; a new `analysis_version` empties only `analysis`; `NULL` rows are not measured again.
- **Pass (unit):** newest first; only photos in series at 60 s; resumes after a stop; the priority set goes first; stops on cancel within a batch.
- **UI (Vitest):** the badge on tile and strip with its label; no badge without suggestions; the badge and the accepted check together; key I toggles the scores line, „–” for unscored photos; the pill note text in all three languages (plural forms).
- **Mocked e2e (Playwright):** suggestions arrive while the grid is shown and the badges appear; rejecting the suggested photo moves the badge; the viewer's strip shows the badge and the scores with I.
- **Real app (`e2e-app/`, one spec):** open a gallery from `fixtures/sharpness/`, wait for the pass, the sharp photo of a set carries the badge in the grid and in the viewer; reject it, the badge moves; undo, it comes back.

## Testing in this cloud session and on the user's machine

- In the container: all of the above, including the real-app suite on Linux (Xvfb), plus the dev tool on the generated sets. Wikimedia Commons is not reachable here (the network change was declined), so there are no real burst series in the container.
- On the user's machine (after this session): run the dev tool and the app on real galleries: do the suggestions match the user's own picks? How long does the pass take for the Takeout of 34,600 photos on the USB disk, and does the grid stay smooth meanwhile? Are RAW previews of the older Olympus cameras skipped? Tune the margin with the scores pane.

## Decisions for the user

1. **Margin and „no mark” when photos are about equally sharp** (default 5 %), or always mark the sharpest?
2. **Placement of the mark:** a star badge bottom-left, or an accent frame around the tile (clearer, but frames already mean „shown in the viewer”)?
3. **Accepted photos:** keep the mark next to the check (planned), or hide the series' mark once any photo of it is accepted?
4. **Scores pane** behind key I in every build (planned, for tuning on real galleries), or only in debug builds?
5. **Only photos in series are measured** in this slice (planned; less disk work), leaving all photos to slice 44?

## Related

- [Slice 44](44-out-of-focus.md) marks out-of-focus photos with a per-photo measure: it reuses `analysis` (tiles, preparation, cache, pass) and adds an absolute threshold, the badge and the view.
- [Slice 18](18-exposure-weights.md) adds an exposure score and weights: `suggest` then ranks by a weighted score instead of sharpness alone; the analysis table gets the histogram numbers ([slice 45](45-bad-exposure.md) reads them too).
- [Slice 16](16-show-rejected.md): rejected photos shown in a series do not take part in the suggestion.
- [Slice 63](63-series-of-one.md): a series of one has no suggestion either way.

## Try

Do the suggestions match your pick?
