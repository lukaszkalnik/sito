# Security

What protects the user's photos and machine today. Sito's main input is untrusted: photo files from anywhere. What it ships is the other half: the installer every installed copy offers as an update.

## The window (webview)

- **Permissions:** the webview may only use Tauri's defaults, set the window title, close the window and open the folder dialog. It has no file system, shell or network access. Opening the release page is done on the Rust side, with an address built and checked there ([Updates](updates.md)).
- **Commands and protocols take ids, never paths.** Reject, undo and accept name a photo by its id in the current scan; `thumb://` and `photo://` take a scan id and a photo index, so they serve only photos of the open folder. Opening a folder is the one command that takes a path.
- **Content Security Policy:** scripts, styles and fonts load only from the app itself; images also from `thumb:` and `photo:`. Inline styles are allowed only as `style` attributes (React sets them), not as injected `<style>` elements. Plugins (`object-src`), `<base>` and form targets are forbidden. The policy applies to the bundled app (release builds and `tauri build --debug`), not to `tauri dev`, where the page comes from the Vite dev server.
- **No navigation:** a window never leaves the app's own pages. Any attempt to load another address into it is refused (and logged in debug builds), so no page from elsewhere can run where `thumb://` and `photo://` are reachable.
- The UI never inserts HTML from data (`dangerouslySetInnerHTML`); release notes are shown as plain text; values from `localStorage` are validated when read.

## Photo files

- **Parsers check everything against the file.** The TIFF and RAW preview readers check every offset and length against the file's size and cap entry counts, value counts and block sizes; TIFF decoding has a memory limit. A crafted file ends in "not readable" or "no preview", never in a crash: in these modules, indexing and arithmetic that could panic or overflow are compile errors.
- **Size limits:** a JPEG (also one under a RAW name) over 512 MB is not read for the viewer, which shows its error text for that photo; a Takeout date file over 1 MB is ignored like an unreadable one (the photo keeps its EXIF time). An embedded preview is at most 64 MB.
- **Scan:** symbolic links are not followed.

## Moving files

- A move never replaces a file, and `Odrzucone` must be a real folder, not a link: see [Rejecting](rejecting.md).
- Every statement of the action log and the photo cache (SQLite) uses bound parameters.

## Network and privacy

- No network access besides the update check and download, HTTPS only, from the Rust side ([Updates](updates.md)). The endpoint can be overridden only in debug builds.
- An update is installed only if its minisign signature matches the public key built into the app; drafts are never offered.
- The logger runs only in debug builds; logs never contain image data.

## The release pipeline

- Every action in the GitHub workflows is pinned to a commit; workflows run with a read-only token unless a job needs more; the signed build restores no cache and uses the committed lockfiles (`--locked`). A checker (`zizmor`) in CI keeps the workflows that way. See [Releasing](../RELEASING.md#pinned-actions).
- The shipped Rust crates and npm packages are checked against the advisory databases on every push and weekly ([Releasing](../RELEASING.md#security-checks)).

## Known gaps

- The installers are not code-signed yet ([slice 47](../slices/47-windows-code-signing.md), [slice 50](../slices/50-macos-code-signing.md)).
- Thumbnails of photos stay in the cache folder until it is cleaned ([slice 15](../slices/15-settings.md)).
- On macOS volumes without an exclusive rename (exFAT and FAT cards, some network shares), a move checks the target and then renames, with a short gap in between ([Rejecting](rejecting.md)).
- The file parsers are not fuzzed yet ([slice 57](../slices/57-fuzz-parsers.md)).
