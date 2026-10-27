# Idea: Sito on Google Photos

Status: concept, not planned. Researched 2026-10-27; Google's Photos APIs changed in 2025, so re-check the linked pages before starting.

## Verdict

**Partially feasible, and only in a limited form.** Since 31 March 2025 the official API no longer lets a third-party app read a user's whole Google Photos library. It can only read photos the user picks by hand in Google's own picker (up to 2,000 per session), and it **cannot trash, delete or archive anything**. So Sito could review a hand-picked batch in the cloud, but "reject" can only mean "the user deletes these later in Google Photos". The only way to work on the whole library and really trash photos is to automate Google's undocumented web API from a browser extension. That works today but is fragile and against the spirit of Google's terms. For whole-library cleanup, the most solid paths are still local copies: the phone's DCIM folder or a Takeout export.

## What the official API allows today

| Capability | Status | Source |
|---|---|---|
| Read the whole library (`photoslibrary.readonly`, `photoslibrary`) | **Removed 31 Mar 2025** (403 since then) | [Updates to the Google Photos APIs](https://developers.google.com/photos/support/updates) |
| Library API read/edit | Only items and albums **the app itself created** (`…readonly.appcreateddata`, `…edit.appcreateddata`, `appendonly`) | same |
| Sharing methods | Discontinued | same |
| **Picker API** (`photospicker.mediaitems.readonly`) | User picks items in the Google Photos UI. The app gets them for this session: max **2,000 items per session**, the session expires, and `baseUrl`s are valid for **60 min** | [Sessions](https://developers.google.com/photos/picker/reference/rest/v1/sessions), [Media items](https://developers.google.com/photos/picker/guides/media-items) |
| Picker metadata | `createTime`, `filename`, `mimeType`, width/height, `cameraMake`/`cameraModel`, focal length, aperture, ISO, exposure time. No location | [PickedMediaItem](https://developers.google.com/photos/picker/reference/rest/v1/mediaItems) |
| Downscaled download | `baseUrl=w400-h400` (fit), `-c` (crop), `=d` (original; EXIF kept **except location**). Needs an OAuth bearer header | [Media items](https://developers.google.com/photos/picker/guides/media-items) |
| Trash / delete / archive / edit description of user items | **Not possible.** It never was for trash or delete; edits only on app-created items | Library API reference |
| Add the user's items to an album | Only to an **app-created album**, and only for items the app created or uploaded. Picked items can't be added to an album, as far as the docs show (verify) | updates page |
| **Ambient API** | Whole library for **smart TVs and photo frames**. Partner programme only, with acceptance, OAuth review and integration review. Not for a cleanup tool | [About Ambient API](https://developers.google.com/photos/ambient/guides/about), [Partner program](https://developers.google.com/photos/partner-program/overview) |
| Quotas | Picker: 100k requests/min and 1M media-byte requests/min per project (ample). Library: 10k requests/day and 75k byte requests/day | [Limits and quotas](https://developers.google.com/photos/overview/api-limits-quotas) |

**OAuth:** a desktop "installed app" client with a loopback redirect works. While the Google Cloud project stays in *Testing* mode it is limited to 100 test users, and refresh tokens expire after 7 days. That is fine for personal use, but a public release needs Google's OAuth app verification (privacy policy, homepage, possibly a review video). I could not confirm whether the Picker scope counts as "sensitive"; check this in the Cloud console.

**Not available any more:** Photos-to-Drive sync ended on [10 July 2019](https://workspaceupdates.googleblog.com/2019/06/google-photos-drive-sync-stopping.html). Since the 2025 change, rclone's Google Photos backend only sees what rclone itself uploaded ([forum](https://forum.rclone.org/t/google-photos-no-longer-allowing-rclone-access-to-all-photos-any-workarounds-why-not-add-this-feature-to-gphotosdl/54310)).

## Options

### A. Picker import into the desktop app (official API)
- **Flow:** "Z Google Photos" → OAuth in the system browser → Sito opens the `pickerUri` → the user selects a day or trip (≤ 2,000 items) → Sito polls the session, lists the items and builds the timeline from `createTime`.
- **Download:** right away, thumbnails at `=w400-h400` (about 30–60 KB each) plus a viewer size of about `=w2048-h2048`, cached locally. They must be fetched within 60 minutes, so download eagerly. That size is enough for time series, sharpness, exposure and embeddings. Originals are not needed.
- **Reject/undo:** Sito can't touch the cloud item. Rejecting is a local decision in SQLite (undoable). At the end Sito shows a list of rejected items and opens them in Google Photos (`productUrl`, if the picker provides it; verify), and the user trashes them there by hand. Optional: "export the list". An album named "Sito – odrzucone" is **not possible**, because picked items can't be added to an app album.
- **Effort:** medium, about 1–2 slices: OAuth (PKCE, loopback, token in the OS keychain), Picker client in `src-tauri` or a new `crates/gphotos`, plus a photo-source abstraction in core (a path today, a remote id plus a cached file tomorrow).
- **Risks:** 2,000-item batches make whole-library cleanup tedious. Manual deletion in Google Photos is error-prone with many items. OAuth verification is needed if distributed. Google has narrowed this API twice already.
- **Rule fit:** good. Only downloads, and nothing is written to Google. The OAuth token is the only new secret.

### B. Local copies: phone DCIM over USB, or Google Takeout (no API)
- **Flow:** the user points Sito at a copy of `DCIM/Camera` (USB/MTP, Windows Explorer, or the Android app below) or at an unpacked Takeout export. Takeout delivers 2/10/50 GB zips with JSON sidecars (`photoTakenTime`).
- **Download:** full originals, once (Takeout means the whole library; a phone copy means only what's on the device).
- **Reject/undo:** works as today on the copy ("Odrzucone"). The cloud is not affected, so the user has to apply the result in Google Photos manually. For DCIM on the phone itself, see option D.
- **Effort:** small. Takeout needs a sidecar reader for photos without EXIF (fits slice 18), plus maybe reading zips directly.
- **Risks:** disk space for 10k+ originals. Only a snapshot: it doesn't sync back.
- **Rule fit:** perfect. This is what Sito already does.

### C. Browser extension on photos.google.com (unofficial)
- **What it can read:** the signed-in page's DOM and its network responses, i.e. Google's undocumented `batchexecute` RPCs. These return item ids, timestamps, dimensions and `lh3.googleusercontent.com` URLs that accept the same `=wN-hN` size suffixes. That covers the whole library with no 2,000 limit. [Google Photos Toolkit](https://github.com/xob0t/Google-Photos-Toolkit) (MIT userscript, ~1.2k stars) proves that listing, "move to trash" and album creation work this way.
- **What it can do:** move items to Google Photos' own trash (undoable there; trash retention reportedly cut from 60 to 30 days in Sept 2026, so verify), add them to an album, or select them in the UI for the user to trash.
- **Core reuse:** series finding (`series.rs`, `timeline.rs` ordering) is pure Rust and compiles to WebAssembly almost as is, once `Photo` is decoupled from `Path`. `image` and later sharpness/exposure also build for wasm32 (no rayon, or `wasm-bindgen-rayon`). Embeddings and faces would use onnxruntime-web instead of `ort`. `walkdir`, `rusqlite` and `nom-exif` file IO stay desktop-only, so this needs a split into `core` (pure) vs. `core-fs`.
- **UI reuse:** most React components (timeline grid, series view, viewer strip, i18n) could be shared if `ui/src/api/*` becomes an interface with Tauri and extension backends. The extension shows them in its own tab or side panel.
- **Chrome Web Store:** Manifest V3 forbids remotely hosted code, but bundled wasm is fine (`wasm-unsafe-eval`). It needs `host_permissions` for photos.google.com and googleusercontent.com, a single-purpose description, a privacy policy and the Limited Use disclosure. Automating another site's private API risks removal, and Google's ToS forbid accessing services "using means other than the interfaces we provide". Self-distribution (unpacked or Firefox) avoids store review but not the fragility.
- **Fragility:** high. RPC ids and payloads change without notice, and a bug in a bulk trash call can hit thousands of items. Every action needs a dry run and a confirmation step.
- **Effort:** large: wasm build of core, a UI backend abstraction, RPC client and maintenance.
- **Rule fit:** analysis is local (in the browser). But **trashing or adding to albums sends changes to Google**, so it must be explicit and per batch. That is still "the user decides" if every write is confirmed. The project gains a component that runs inside a Google page.

### D. Android app (separate build)
- **Flow:** Sito on the phone reads `DCIM/Camera` via MediaStore and finds series. Reject uses `MediaStore.createTrashRequest` (Android 11+), which shows a system confirmation and is undoable for 30 days.
- **Cloud effect:** trashing on the device. Whether Google Photos mirrors it to the cloud depends on Google Photos' sync behaviour; **verify**, since it's often not mirrored.
- **Reuse:** core via JNI or uniffi (Rust builds for Android). UI via Tauri 2 mobile, which reuses the React UI almost entirely.
- **Effort:** large (a new platform, MediaStore permissions, store release). **Rule fit:** perfect, but covers only photos still on the phone.

## Recommendation

> **Decision (2026-10-27):** the user considers **option D, a standalone Android app**, the strongest direction for slice 22. Takeout date files go into slice 18.

1. **Do nothing cloud-specific for now.** Finish slices 9–19 on local folders. Option B already covers "photos from an Android phone" if the user copies `DCIM/Camera` or uses Takeout. The only cheap addition is a Takeout sidecar reader, as part of slice 18.
2. If cloud review is wanted, the **official path is option A** as one roadmap slice, "Import from Google Photos (Picker)". Scope: OAuth desktop flow with keychain storage, a Picker session for ≤ 2,000 items, download and cache of `w400` and `w2048` sizes, the timeline and series on remote items, and reject as a local decision with an "Open in Google Photos" list. It needs a `PhotoSource` abstraction in core first (path vs. remote id). Accept that the user still deletes in Google Photos by hand.
3. **Don't build option C now.** It's the only way to really trash in the cloud at scale, but it rests on an undocumented API and grey ToS. The prerequisite refactors (pure `core` → wasm, a backend-agnostic `ui/src/api`) are worth doing anyway if C or D ever becomes a goal; they also help testing.

## Open questions

- Is the goal to clean the **cloud library** (only C really does that), or to pick the best shots before or after backup (A, B and D are enough)?
- Is a manual "trash these in Google Photos" step acceptable for option A?
- Personal use only, or a public release? Public release means OAuth verification (A) or store review (C, D).
- Are most photos still on the phone, or only in the cloud? (Decides between B/D and A/C.)
- Would you accept a ToS-grey, possibly breaking extension for personal use?
