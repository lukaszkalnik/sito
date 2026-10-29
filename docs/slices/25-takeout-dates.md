# Slice 25 – Google Takeout dates

**Status:** ✅ done (tested by the user 2026-10-29) (what it established is in [docs/domain/photos.md](../domain/photos.md) and [docs/domain/rejecting.md](../domain/rejecting.md))

## Context

Photos exported with Google Takeout often have no usable EXIF date (WhatsApp images, screenshots, edited or downloaded copies). After unpacking, their file date is the export date, so Sito would group them all by that meaningless time. Takeout keeps the real capture time in a JSON file next to each photo.

This was split out of slice 20 („Series without EXIF date”) on 2026-10-27, to come right after the README glam shot (slice 11). It doesn't need image similarity.

## What a real Takeout looks like

From the user's own export on `F:\Google Photos`, inspected read-only on 2026-10-27; nothing from it goes into the repo:
- **Five zips:** `takeout-…-1-001.zip` … `-005.zip`, about 50 GB each and 220 GB in total. One zip has about 20,700 entries: 8,100 JPEG, 11,500 JSON, 1,050 Olympus RAW (`.orf`), plus a few TIFF and video files.
- **Paths:** `Takeout/Zdjęcia Google/<album>/<file>`. The folder names are localised (Polish here), and each album has a `metadane.json` (album metadata, localised name).
- **Date files** sit next to each photo: `<name>.<ext>.supplemental-metadata.json`.
  - Duplicate names get numbered date files: `…supplemental-metadata(1).json`, up to `(7)` seen. These presumably belong to `<name>(1).<ext>`; to verify.
  - Older exports and long file names use other forms, e.g. `<name>.<ext>.json` or truncated names.
- **Fields:** `title` (the original file name), `photoTakenTime { timestamp, formatted }` (Unix seconds as a string), `creationTime`, `geoData`, `description`, `url`, `googlePhotosOrigin`.

## Concept

- **Core:** when a photo has no EXIF date, look for its Takeout date file in the same folder and take `photoTakenTime`. That gives a new `TimeSource::Takeout`, which counts like EXIF for series.
  - **Matching:** by the date file's base name first, then the numbered-duplicate rule, then by `title`.
  - **Only the date:** `geoData` and `description` are never read or stored (privacy).
- **Unpacked Takeout only** (decided 2026-10-27): it works through the normal folder scan, and the date files are found next to the photos. The README explains how to unpack a Takeout to a drive with enough room. Reading the zips directly isn't supported, because rejecting couldn't move files inside a zip.
- **A wrong EXIF date is overridden** (decided 2026-10-27). Google takes `photoTakenTime` from EXIF on upload and updates it when someone corrects the date in Google Photos, so comparing the two tells which one to use:
  - **Same, or off by whole hours up to ±14 h:** that's only the time zone (EXIF is local, Takeout is UTC). Keep the EXIF time.
  - **Off by anything else:** the date was most likely corrected in Google Photos (e.g. a camera with a reset clock). Use the Takeout time.
  - **Implausible EXIF date** (before 1990, a camera default such as 2000-01-01 00:00 or 1970-01-01, or in the future): use the Takeout time if there is one; otherwise the photo counts as without a date.
  - The photo's info and the debug panel show which date was used and why.
- **Time zone:** `photoTakenTime` is UTC. EXIF capture times are local, so the Takeout time is converted to the local time zone (of the computer, or from the photo's offset when known).
- **Tests:** synthetic fixtures with made-up date files, covering all naming forms. The user's export is used only for a manual check.

## Related slices

- RAW photos (e.g. Olympus `.orf`) are [slice 26](26-raw-photos.md), and TIFF is [slice 27](27-tiff-photos.md). Both come right after this one, so the user's Takeout is fully supported.
- Videos are out of scope.

## Open questions

- Is ±14 h the right limit for "only the time zone"? Daylight-saving shifts and half-hour zones need checking.
- Where to show "date from Takeout" or "date corrected": the viewer's info line, the tile badge (like „data pliku”), or only the debug panel?

## Verification (outline)

- Unit tests for every naming form, including duplicates and long names.
- On a copy of one album from the user's Takeout: photos without EXIF show their real dates, and series form across them.

# Plan (approved 2026-10-28)

## Decisions and context

Photos from an unpacked Google Takeout often have no usable EXIF date (WhatsApp images, downloads, scans), or a wrong one (camera with a reset clock, scans dated when they were scanned). Today they fall back to the file date, which after unpacking is the export date, so they all land on one day and never form series. Takeout keeps the real capture time in a JSON file next to each photo; this slice reads it. Concept and the real export's layout: `docs/slices/25-takeout-dates.md`.

User decisions:
- Unpacked Takeout only; a wrong EXIF date is overridden (2026-10-27, in the concept).
- **Shown in the viewer's info line only** („data z Google Takeout” / „data poprawiona w Google Photos”); the timeline tiles stay clean (2026-10-28).
- **"Only the time zone"** = EXIF minus Takeout UTC is a multiple of 15 minutes within ±14 h (±2 s tolerance) (2026-10-28).
- **Takeout dates are trusted in any year:** the user has scans dated e.g. 1976 in Google Photos. The 1990 limit stays for EXIF only. A scan whose EXIF says 2006 and Takeout 1976 is "corrected" and gets 1976.

Real data for the manual check: `F:\Google Photos\Takeout\Zdjęcia Google\` (being unpacked; read only, never modified). A first look: 17,091 JSON files, of which 15,990 `<name>.<ext>.supplemental-metadata.json`, 1,084 numbered `…supplemental-metadata(n).json`, no truncated names seen yet, plus `metadane.json` per album and two other non-photo JSONs.

## Implementation

### 0. Docs
Save this plan into `docs/slices/25-takeout-dates.md` (keep the concept above it), status 🚧 there and in `docs/ROADMAP.md`. Commit `docs: add slice 25 plan`.

### 1. Refactor: time source as a union in the UI (`refactor:`)
Replace `fromFileDate: boolean` with `timeSource: 'exif' | 'file'` in `TimelineEntryDto` (`src-tauri/src/scan.rs` L80-113), `TimelineEntry` (`ui/src/features/timeline/api.ts`), `TimelineTile.tsx` badge, the e2e mock (`ui/e2e/tauri-mock.ts`), `scripts/make-demo-gallery.py` manifest, and tests. No behaviour change; makes room for new sources without more booleans.

### 2. Core: Takeout sidecars (`crates/core/src/takeout.rs`, new; `feat:`)
- **Domain (pure):**
  - `match_sidecars(photos: &[&OsStr], sidecars: &[&OsStr]) -> Vec<(photo, sidecar)>` for one folder. Rules, in order: `<name>.supplemental-metadata.json`; numbered duplicates (`<stem>(n).<ext>` ↔ `<stem>.<ext>.supplemental-metadata(n).json`); `<name>.json` (older exports); truncated names (the sidecar name without `.json`/`(n)` is a prefix of `<name>.supplemental-metadata`, longest wins, at least the photo stem). Each sidecar is used once.
  - `resolve_time(exif: Option<CaptureTime>, takeout_utc: DateTime<Utc>, now) -> (NaiveDateTime, TimeSource)`:
    - no or implausible EXIF → Takeout (`Takeout(MissingExif)`), converted to local time with the EXIF offset if known, else the computer's zone (`chrono::Local`, which applies the DST of that date);
    - EXIF − Takeout UTC a multiple of 15 min within ±14 h (±2 s) → EXIF;
    - otherwise → Takeout (`Takeout(Corrected)`).
  - **Implausible EXIF** (moved from `metadata.rs` year check into one function): before 1990, exactly January 1 at 00:00:00 (camera defaults), or more than a day after `now`. Without a sidecar such a photo now counts as without a date (file date) — a small behaviour change, noted in the domain doc.
- **Data:** `read_sidecar(path) -> Result<TakeoutTime, TakeoutError>` with `serde`/`serde_json` (add to core; already in the workspace via src-tauri, licenses MIT/Apache already accepted). The struct only has `photoTakenTime.timestamp` and `title`; every other field (geoData, description, people) is ignored by serde and never stored. Timestamp `0` or unparsable → no Takeout date (logged, not fatal).
  - Title fallback: for photos still unmatched after the name rules, read the leftover sidecars of that folder and match `title` exactly.
- **Scan** (`crates/core/src/scan.rs`): the walk already sees every file (L111); also collect `*.json` per folder (except `metadane.json`/`metadata.json`), then attach the matched sidecar to `FoundFile` as `sidecar: Option<FileStamp { path, size, modified }>`.
- **Timeline** (`crates/core/src/timeline.rs`): `TimeSource` becomes `Exif | Takeout(TakeoutUse) | FileModified` with `TakeoutUse { MissingExif, ImplausibleExif, Corrected }`; `to_photo` (L261) calls `resolve_time` when a sidecar exists. `read_photo` (restore from Odrzucone, L102) looks for the sidecar next to the restored file too.
- **Series** (`crates/core/src/series.rs` L70): `belong_together` uses `TimeSource::is_capture_time()` (Exif or Takeout) instead of `== Exif`.
- **Summary:** `without_exif_date` → `without_date` (counts `FileModified`); the UI string becomes „Bez daty zdjęcia: …”.
- **Cache** (`crates/core/src/photo_cache.rs`): `CACHE_VERSION` 1 → 2 (the photo cache is a rebuildable cache, so it is emptied); column `exif INTEGER` → `time_source INTEGER` (0 file, 1 exif, 2/3/4 Takeout uses) plus `sidecar_mtime_ns INTEGER NULL`; `is_unchanged` (`timeline.rs` L205) also compares the sidecar stamp, so adding, changing or removing a sidecar rereads the photo.
- **Rejecting moves the sidecar along** (user, 2026-10-28), so no orphaned sidecar stays behind and the photo keeps its date in „Odrzucone” and after undo:
  - Action log (`crates/core/src/actions.rs`): new migration 2 adds `sidecar_from TEXT NULL, sidecar_to TEXT NULL` to `actions`; `record_reject` takes an optional sidecar pair, and `RejectAction` carries it. Same transaction as today: log entry first, then the moves.
  - Reject: move the photo, then the sidecar, with the same safe-move rules (rename, or copy + verify + remove across drives; free name if taken, keeping the pairing: `name(1).jpg` gets `name.jpg.supplemental-metadata(1).json`-style naming via the same rule as matching, so the pair still matches in „Odrzucone”). If the sidecar move fails, move the photo back and report the error (no half-done reject).
  - Undo: move both back; a missing sidecar in „Odrzucone” only skips the sidecar (logged), the photo is still restored.
  - Tests: reject + undo with and without sidecar, name collision, sidecar move failure rolls the photo back.

### 3. Tauri + UI (`feat:`)
- DTO `timeSource`: `'exif' | 'takeout' | 'takeoutCorrected' | 'file'` (Missing/Implausible both map to `takeout`).
- Viewer info line (`ui/src/features/viewer/PhotoViewer.tsx` L97-103): after the time, a muted note for `takeout` („data z Google Takeout”) and `takeoutCorrected` („data poprawiona w Google Photos”), from `pl.json`. Tile badge unchanged (only `file`).
- Vitest for the viewer note; e2e mock + one viewer e2e case.

### 4. Tests and fixtures
- `scripts/make-fixtures.py` gains `fixtures/takeout/`: small no-EXIF / wrong-EXIF / good-EXIF JPEGs with made-up sidecars for every naming form (plain, numbered duplicate, `.json`, truncated, title-only), a time-zone-only difference, a corrected date, a 1976 scan, a `metadane.json`, and a sidecar with extra fields (`geoData`) that must not appear anywhere. Documented in `fixtures/README.md`.
- Unit tests in `takeout.rs` (matching rules, `resolve_time` incl. ±14 h edges, 15-minute zones, DST, 1976, implausible EXIF), `series.rs` (Takeout photos form series), `photo_cache.rs` (sidecar change rereads), integration test `crates/core/tests/takeout.rs` running a scan on `fixtures/takeout/`.

### 5. Docs
- `docs/domain/photos.md`: capture time rules with Takeout, implausible EXIF. `docs/domain/rejecting.md`: the sidecar moves with its photo.
- Real data note: in the first look, `IM000171.JPG.supplemental-metadata(1).json` had no `IM000171(1).JPG` next to it yet (maybe still in another zip); the survey checks whether numbered sidecars always get their photo once unpacking is done.
- README: short "Google Takeout" section: unpack all zips into one folder on a drive with enough space, open `Takeout/Zdjęcia Google` (or `Google Photos`).
- Slice status 🧪; after CI is green, release (minor bump) and silent install per the usual workflow.

## Verification
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`, `npm run notices:rust` unchanged.
- Read-only survey of `F:\Google Photos` once unpacked: count photos per match rule and unmatched photos/sidecars (a small throwaway check run against core, output to scratchpad) — expect nearly all JPEGs matched.
- Debug build (`npm run tauri build -- --debug --no-bundle`), open a few albums from `F:\Google Photos` (read only, no rejecting there): photos without EXIF show real dates and form series; a 1976 scan sits in 1976 with „data poprawiona w Google Photos”. Rejecting/undo tested on a copy of one album in `D:\Lukasz\tmp`.
