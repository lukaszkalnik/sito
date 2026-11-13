---
status: 📝 concept drafted
---

# Slice 56 – Security hardening

Since 2026-11-13; findings of a security review of the whole app at commit `a3933f9`. Plan written and approved by the user on 2026-11-13 (with Dependabot for the action pins); not started.

## Idea

Asked by the user on 2026-11-13: review the security of the app and collect the important findings in a slice. The review read the Tauri configuration and capabilities, every command and both protocols, the reject / undo / trash code, the file parsers, the updater and the GitHub workflows.

Nothing found lets a web page, a photo file or the network run code in Sito or reach files outside the opened folder. The findings below are about what Sito ships (the release pipeline), its hard rule that files are never overwritten, and defence in depth.

## Findings

Ordered by importance. Each one names where it is and what would fix it.

### 1. The release build runs actions pinned to movable tags (high)

`.github/workflows/release.yml` builds, signs and publishes the installer that every installed Sito offers as a one-click update (see [Updates](../domain/updates.md)). The job holds `TAURI_SIGNING_PRIVATE_KEY` and a token with `contents: write`, and it uses `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `actions/setup-node@v4` and `tauri-apps/tauri-action@v0`. These are tags or branches their owners can move. If one of those repositories is taken over, its code runs in the job before or during the signing: it can read the key, or change the build so that a backdoored installer gets a valid signature. The signature check in the app does not help then, since the update is signed with the real key.

The same holds for `ci.yml` (`taiki-e/install-action@v2` and the others), with less at stake: no secrets, but it has no `permissions:` block, so the token gets the repository's default rights.

Fix:

- Pin every action in both workflows to a full commit SHA, with the version in a comment.
- Add `.github/dependabot.yml` for the `github-actions` ecosystem, so the pins get update proposals instead of going stale.
- Add `permissions: contents: read` at the top of `ci.yml`.
- Build with the lockfile as committed: pass `--locked` to the cargo build of the release (`args: -- --locked` of `tauri-action`), as CI already does for clippy and tests.

### 2. No check for known vulnerabilities in dependencies (medium)

Sito's main input is untrusted files: photos from anywhere, decoded by `image`, `jpeg-decoder`, `nom-exif` and the TIFF code, and JSON date files. A vulnerability in one of these crates is the most likely way a crafted photo could harm the app. Nothing checks the dependencies against the advisory databases today; the license check (`cargo-about`) does not.

At review time `npm audit --omit=dev` reported 0 vulnerabilities. `cargo audit` (0.23.0, 554 crates in `Cargo.lock`, run on 2026-11-13) reported **no vulnerabilities** and six warnings:

| Crate | Warning | Shipped? |
|---|---|---|
| `rustix` 1.1.5 | unsound ([RUSTSEC-2026-0486](https://rustsec.org/advisories/RUSTSEC-2026-0486)): xattr functions with a zero-length buffer; fixed in 1.1.6 | macOS only, through `tauri-plugin-updater` (`tempfile`, and `tar` → `xattr`) |
| `glib` 0.18.5 | unsound ([RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429)) | no: Linux only |
| `zvariant`, `zvariant_derive`, `zbus_names` | unmaintained | no: Linux only |
| `proc-macro-error` 1.0.4 | unmaintained | no: build time only |

Fix:

- A CI job running `cargo audit` (RustSec) or `cargo deny check advisories`, and `npm audit --omit=dev`, on every push and on a weekly schedule, so a new advisory shows up without a commit.
- ~~Update `rustix` to 1.1.6 or newer.~~ Done on 2026-11-13, ahead of the rest of the slice: only the `rustix` entry of `Cargo.lock` changed. (`cargo update -p rustix@1.1.5` also moved eleven crates to older `windows-sys` versions already in the lockfile, so the entry was changed by hand and checked with `cargo metadata --locked`.) The Linux-only and build-time warnings need nothing; the CI job should not fail on them (it fails on vulnerabilities, or lists these as accepted with the reason).

### 3. A move can overwrite a file (medium: unlikely, but not reversible)

`move_file` in `crates/core/src/reject.rs` uses `fs::rename`, which replaces an existing destination on Windows and macOS alike. Callers check `to.exists()` first (`move_all`, `move_to_free`, `restore`), but between the check and the rename another program can create the destination: a sync client (OneDrive, Dropbox, iCloud) bringing a file back, or a second copy job. The file that was there is then gone for good, with no log entry. This breaks the hard rule that files are never overwritten (see [Rejecting](../domain/rejecting.md)).

Fix:

- Move with a primitive that refuses an existing destination in one step: `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING` on Windows, `renamex_np` with `RENAME_EXCL` on macOS (`renameat2` with `RENAME_NOREPLACE` on Linux). An existing destination becomes `RejectError::Occupied`. Check for a maintained crate before writing the `unsafe` calls by hand.
- Keep the `exists()` checks for picking a free name; the atomic move is the guarantee.
- While there: `prepare` follows a symlink or junction named „Odrzucone” (`dir.is_dir()`), so photos would move out of the gallery to wherever it points. Refuse an „Odrzucone” that is not a real folder (`symlink_metadata`).

### 4. Whole files are read into memory without a size limit (low)

- `display_jpeg` (`crates/core/src/display.rs`) and `read_preview` (`crates/core/src/raw/preview.rs`, for a RAW-named file that is a JPEG) read the whole file with `std::fs::read`. A file of several gigabytes named `.jpg` is read whole for the viewer, up to three at once with the prefetch of the neighbours.
- `read_sidecar` (`crates/core/src/takeout/sidecar.rs`) reads any `.json` date file whole during the scan.

The result is a crash or a frozen machine, not a leak. The embedded-preview and TIFF structure readers already have limits (`MAX_BLOCK`, `MAX_ENTRIES`, `TIFF_MEMORY_LIMIT`); these three paths do not.

Fix: a size limit for each (a date file is a few kilobytes; a JPEG over a few hundred megabytes is not a photo), with a visible error for the photo instead of an out-of-memory crash.

### 5. Content Security Policy and navigation (low, defence in depth)

`src-tauri/tauri.conf.json`:

- `img-src` allows `asset:` and `http://asset.localhost`, but the asset protocol is not enabled and nothing uses it. Remove both.
- Check whether `data:` in `img-src` is still needed; remove it if not.
- `style-src 'unsafe-inline'` is there for React's inline `style` attributes. `style-src-attr 'unsafe-inline'` allows only those and not injected `<style>` elements.
- Add `object-src 'none'; base-uri 'none'; form-action 'none'`.

The webview never navigates today: the UI has no links. Nothing enforces that, though, and a page loaded into the window from elsewhere could request `thumb://` and `photo://` images of the open folder. Add an `on_navigation` handler that refuses every address outside the app. [Slice 54](54-release-notes-markdown.md) brings links into the release notes and must not let them navigate the window.

## What the review found in good shape

Recorded so a later review can start from here; details are in the code and the domain files.

- **Webview permissions:** only `core:default`, window title and close, and the folder dialog. No file system, shell or HTTP plugin; the opener is called from Rust only, with an address built and checked there (`update/release_page.rs`).
- **Protocols:** `thumb://` and `photo://` take a scan id and a photo index, never a path, so they serve only photos of the open folder.
- **Commands:** reject, undo and accept take ids of the current scan; no command takes a path except opening a folder.
- **Updater:** HTTPS only, minisign signature checked against the built-in key before the installer runs, the endpoint override exists only in debug builds, drafts are never offered.
- **Parsers:** the TIFF and RAW preview readers check every offset against the file length and cap counts and block sizes; TIFF decoding has a memory limit.
- **Scan:** symlinks are not followed (`follow_links(false)`).
- **SQLite:** every statement uses bound parameters.
- **UI:** no `dangerouslySetInnerHTML`; release notes are shown as plain text; `localStorage` values are validated when read.
- **Privacy:** no network access besides the update check and download; the logger runs only in debug builds.

Known and tracked elsewhere: the installers are not code-signed ([slice 47](47-windows-code-signing.md), [slice 50](50-macos-code-signing.md)); thumbnails of photos stay in the cache folder until it is cleaned ([slice 15](15-settings.md)).

## Plan

Approved on 2026-11-13. Five steps, one commit each, in this order. Steps 1 and 2 touch only workflows, steps 3 and 4 only `core` and its error mapping, step 5 the Tauri layer and the configuration.

### Step 1 – Pin the workflow actions (finding 1)

- In `release.yml` and `ci.yml`, replace every `uses: owner/action@tag` with the full commit SHA of the newest release of that action, keeping the version as a comment (`# v4.2.2`). Look the SHAs up with `gh api repos/<owner>/<action>/git/ref/tags/<tag>` (for an annotated tag, follow it to its commit).
- `dtolnay/rust-toolchain@stable` is a branch that installs the stable toolchain. Pin the action to a commit of its `master` branch and pass `toolchain: stable`, so the action is fixed and the toolchain still follows stable.
- `ci.yml`: add `permissions: contents: read` at the top.
- `release.yml`: add the same at the top; the two jobs keep their own `contents: write`.
- `release.yml`: `args` of `tauri-action` become `-- --locked` (Windows) and `--target universal-apple-darwin -- --locked` (macOS), so cargo refuses a lockfile that does not match.
- `docs/RELEASING.md`: a short section on the pins: why they are there and how Dependabot updates them.
- **Updating the pins:** Dependabot (decided by the user on 2026-11-13). New `.github/dependabot.yml` with only the `github-actions` ecosystem, checked monthly, all updates grouped into one pull request. Dependabot keeps the SHA pin and its version comment in step. No `cargo` or `npm` entries: those dependencies stay updated by hand, and the audit job of step 2 reports the urgent ones.
- These pull requests are the one exception to working on `main` only: the user reviews and merges them on GitHub once CI is green. `CLAUDE.md` (Development workflow) gets a line saying so, and `docs/RELEASING.md` says what to look at before merging one (the action's release notes, and that the new SHA belongs to the tagged release).
- **Check:** CI is green on the push. Then a manual run of the Release workflow (`workflow_dispatch`): it builds and smoke-tests the installer and uploads it as an artifact, without publishing anything.

### Step 2 – Audit job (finding 2)

- New workflow `.github/workflows/audit.yml`, on push to `main`, weekly (`schedule`) and by hand. One job on `ubuntu-latest`, `permissions: contents: read`:
  - `cargo audit` (installed with the pinned `taiki-e/install-action`, a fixed `cargo-audit` version). It fails on vulnerabilities and only prints warnings such as "unmaintained", which fits the Linux-only warnings listed under finding 2.
  - `npm ci` and `npm audit --omit=dev`.
- A failed scheduled run sends GitHub's usual email to the repository owner; nothing else is needed.
- `CLAUDE.md`: add `cargo audit` to the commands and one line to the Dependencies rules (run it after changing Rust dependencies).

### Step 3 – Moves that cannot overwrite (finding 3)

- New private module `crates/core/src/reject/exclusive.rs` with `move_exclusive(from, to) -> io::Result<()>`: moves a file and fails with `AlreadyExists` if `to` exists, in one system call.
  - Windows: `MoveFileExW` with no flags (without `MOVEFILE_REPLACE_EXISTING` it refuses an existing destination; without `MOVEFILE_COPY_ALLOWED` it stays a same-drive move, as today). Paths go through as wide strings, so long and non-ASCII paths work.
  - macOS: `renamex_np` with `RENAME_EXCL`. Volumes that do not support it (exFAT and FAT cards, some network shares) answer `ENOTSUP`; only then fall back to today's check followed by `fs::rename`. The gap stays on those volumes and is documented.
  - Linux (tests and development only): `renameat2` with `RENAME_NOREPLACE`, same fallback.
  - The calls are declared through `windows-sys` and `libc`, both already in `Cargo.lock`, each `unsafe` block with `#[expect(unsafe_code, reason = "...")]` like `src-tauri/src/reject/drive.rs`. No new crate: `renamore` does this, but it is a small crate of unknown upkeep and the code is about thirty lines per system.
- `move_file` in `reject.rs` calls it and maps `AlreadyExists` to `RejectError::Occupied`. The `exists()` checks stay for choosing free names and for early, clear errors.
- **„Odrzucone” must be a real folder:** `prepare` checks `symlink_metadata`; a symlink or a Windows junction gives the new `RejectError::RejectedDirIsLink`, mapped to a new command error code with a Polish message in `pl.json` (wording to settle while implementing, e.g. „Folder „Odrzucone” jest skrótem do innego miejsca. Usuń skrót, aby odrzucać zdjęcia do folderu.”).
- **Tests** (unit, in temp folders): a move onto an existing file fails and leaves both files with their contents; a move to a free name works, also with a Polish name; rejecting with a symlinked „Odrzucone” is refused and moves nothing (Unix test; the junction case is covered by the same check).
- Restoring from the system trash is not affected: the `trash` crate does that move.

### Step 4 – Size limits (finding 4)

- `display.rs`: `MAX_DISPLAY_BYTES` (512 MB; the largest JPEG the format allows is far smaller in practice) checked from the file's metadata before reading. Over it: new `DisplayError::TooLarge`, answered by `photo://` with `413`; the viewer already shows its error text for a photo that does not load.
- `raw/preview.rs`: the same limit for a RAW-named file that is really a JPEG.
- `takeout/sidecar.rs`: `MAX_SIDECAR_BYTES` (1 MB); over it: a `SidecarError`, which the scan already treats like an unreadable date file (the photo keeps its EXIF time).
- **Tests:** each limit with a sparse file just over it (created with `set_len`, so nothing large is written).

### Step 5 – CSP and navigation guard (finding 5)

- `tauri.conf.json`: remove `asset:` and `http://asset.localhost`; remove `data:` from `img-src` (nothing in `ui/src` uses a data address; confirm in the running app); add `object-src 'none'; base-uri 'none'; form-action 'none'`.
- `style-src`: try `style-src 'self'; style-src-attr 'unsafe-inline'`. `ui/index.html` has an inline `<style>` block; Tauri adds a hash for it at build time, and if that does not hold, the block moves into a CSS file. If either webview (WebView2, WKWebView) rejects `style-src-attr`, keep `style-src 'self' 'unsafe-inline'` and note why.
- New `src-tauri/src/navigation.rs`: a small Tauri plugin (`tauri::plugin::Builder::on_navigation`), which covers both windows, including the viewer created at run time. It allows only the app's own origin (`tauri://localhost`, `http://tauri.localhost`, and the dev server address in debug builds) and logs a refused address. The pure part, `is_app_page(url)`, gets unit tests (the app pages, a web page, a look-alike host, another scheme).
- **Check in the app** (debug build on macOS; Windows when a Windows session or a release allows): styles, thumbnails, viewer photo, zoom, licenses dialog and update notice look as before; the webview console shows no CSP errors.

### When the steps are done

- `docs/domain/security.md` (new, linked from the index): what protects the user today, taken from "What the review found in good shape" plus what this slice adds (no-overwrite moves, size limits, CSP, navigation guard, pinned release pipeline, audits).
- `docs/domain/rejecting.md`: the exclusive move, its gap on volumes without support, and the refused „Odrzucone” link.
- Status 🧪; the user tests step 5 in the debug build.

### What the user tests

1. Open a folder, scroll the grid, open the viewer, zoom, step through photos: everything looks and works as before.
2. Reject a photo into „Odrzucone” and undo it; reject into the trash.
3. „Pomoc → Licencje…” and „Pomoc → Sprawdź aktualizacje…” work.
4. On GitHub: the CI and Audit runs are green; the manual Release run built an installer.

## Related

- [Slice 41](41-update-check.md): built the updater and its signing.
- [Slice 54](54-release-notes-markdown.md): links in release notes; builds on the navigation guard of finding 5.
- [Slice 38](38-files-changed-outside.md): also about other programs changing files under Sito; finding 3 is the part that can lose a file.
