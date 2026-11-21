---
status: 🧪 ready for testing
---

# Slice 54 – Formatted release notes in „Co nowego?”

Since 2026-11-08; planned and implemented on 2026-11-21 (see [Plan](#plan) and [Done](#done)); ready for testing. What it established is in [Updates](../domain/updates.md#release-notes).

## Idea

Asked by the user on 2026-11-08, with a screenshot of „Co nowego w wersji 0.15.1”: the dialog shows the release notes as raw Markdown in a monospaced box (`### Fixed`, `- **Hidden macOS files…**`, backticks). They should be rendered: headings, bullet lists, bold text, inline code and links, in the app's normal font.

Today's behaviour is in [Updates](../domain/updates.md) („Co nowego?” shows the release's CHANGELOG section as plain text); the dialog is `ui/src/features/update/ReleaseNotesDialog.tsx`.

## What the notes contain

The notes are the tag's `CHANGELOG.md` section, written by us, carried in `latest.json` ([Releasing](../RELEASING.md#update-signing-key)). They use a small Markdown subset: `###` headings (New / Improved / Fixed), `-` bullet items, `**bold**`, `` `code` `` and links to slice files (`[slice 32](https://github.com/…)`).

## Candidate approaches

- **Own renderer for the subset (preferred).** A pure function in the feature's domain layer parses the notes into a small typed tree (heading, list, paragraph; inline text, bold, code, link) and a component renders it as React elements. No dependency, easy to unit-test, and anything it does not know stays plain text.
- **A Markdown library** (`react-markdown`, `marked`, …). Full CommonMark, but a new dependency (license review, bundle size) for a handful of constructs.

Either way, **never render the notes as HTML** (`dangerouslySetInnerHTML`): they come over the network, and only the installer is signature-checked, not the notes.

## To decide when planning

- **Links:** open them in the system browser (needs the opener plugin or a Rust command, a new capability that needs a stated reason) or show the link text only. Slice links mean little to users; dropping them from the notes (in `scripts/release-notes.mjs` or when rendering) is an option.
- **Skipped versions:** a user several versions behind sees only the newest section. Consider showing all sections since the installed version (would need the notes of every release, e.g. from the GitHub release list, or all of `CHANGELOG.md` in the manifest).
- **Language:** the notes are English in a Polish UI. [Slice 22](22-english-german-ui.md) (English and German UI) may want translated notes; keep the renderer independent of the language.

## Plan

Own renderer for the subset (no dependency), links shown as their text only, and the dialog shows every version since the installed one.

### Decisions

- **Links are text only** (decided by the user on 2026-11-21): the in-app notes have no clickable links. A link `[slice 32](https://…)` is shown as its text, „slice 32”, in the normal text style; the address is dropped. Users who want more detail go to GitHub themselves. So the slice adds no command, no permission and no way to open an address from the notes.
- **Skipped versions are shown.** The notes in `latest.json` become every CHANGELOG section from the tag back to v0.15.0 (the first version with the updater), each with its `## vX.Y.Z – date` heading. The Rust side keeps only the sections newer than the running version before sending the notes to the UI (`notes_of`, with the `semver` crate the updater already uses), so a user on 0.15.1 offered 0.17.0 sees 0.17.0 and 0.16.0. Notes without version headings (a local test manifest, an older release) pass unchanged. Installed versions up to this slice show the longer notes as plain text, which is acceptable.
- **Language:** the notes stay English; the renderer knows nothing of the language. Only the dialog's own texts (title, version headings) are translated.

### Domain: `ui/src/features/update/releaseNotes.ts`

`parseReleaseNotes(notes: string): ReleaseNotes`, pure, into a typed tree:

- `ReleaseNotes = readonly NotesSection[]`; a section is `{ version: string | null; date: string | null; blocks }` (`null` for notes without version headings, a single section).
- Blocks: `{ kind: 'heading'; inline }` (`###`), `{ kind: 'list'; items: readonly Inline[][] }` (`- ` items; an indented line continues the item), `{ kind: 'paragraph'; inline }` (other lines up to a blank line).
- Inline: `text`, `bold` (whose content may hold `code`), `code` (taken literally). A link `[text](url)` becomes the `text` it shows (inside bold too); the parser keeps no addresses, so the tree cannot carry one to the UI.
- Anything it does not know stays text: an unclosed `**` or backtick, `[text](` without its `)`, `#`/`####` headings, a `## ` line that is not `vX.Y.Z`. CRLF line endings are accepted.

### Rust: `src-tauri/src/update.rs`

- `notes_of(body, running)`: splits at `## v` headings and drops sections whose version is not newer than `running`; a heading whose version does not parse is kept. Unit tests: no headings, all newer, some older, equal to the running one, unparseable version, blank result (`None`).

### UI

- `ReleaseNotes.tsx` renders the tree with React elements only (never HTML strings): each version gets a heading „Wersja 0.16.0 · 12.11.2026” (date through `useFormats`; the heading is left out for a section without a version), `###` headings below it, lists, paragraphs, `<strong>` and `<code>`. No `<a>` elements.
- `ReleaseNotesDialog.tsx`: the `<pre>` becomes a scrollable, focusable region in the app's font; inline code in the monospaced font on the tile background.
- New strings in `pl.json`, `en.json`, `de.json` (version heading).

### Release side

- `scripts/release-notes.mjs`: `--changes-only` becomes `--update-notes`: the sections from the tag back to v0.15.0, headings included. The Release workflow passes the new flag; `check-update-manifest.mjs` also checks that the notes start with the tag's section.
- `scripts/local-update-manifest.mjs` writes formatted sample notes (two versions, a list, bold, code, a slice link), so a local update test shows the formatting.
- `docs/RELEASING.md` and [Updates](../domain/updates.md) describe the new notes.

### Tests

- Vitest: the parser (each block and inline kind, sizes 0/1/many, unclosed markers, unknown headings, links shown as their text and their addresses gone (also inside bold, and a link-like `[text]` without an address), CRLF, one real CHANGELOG section as a whole); the dialog (headings and lists by role, no link in the dialog and the link text shown, Escape still closes).
- Rust: `notes_of`, as above.
- Mocked e2e (`ui/e2e/update.spec.ts`): the notes of two versions are shown formatted, with no links.
- Real app, here on Linux: a debug build with `SITO_UPDATE_ENDPOINT` pointing at a local manifest with a `linux-x86_64` entry shows the offer; check the dialog's formatting with a screenshot and that it has no links. The user checks it later on Windows/macOS (font, Polish and German dialog texts), and the release after this slice is the real test of the new `latest.json` notes.

### Out of scope

- Translated notes; the promo website's changelog ([slice 49](49-promo-website.md)).
- Showing the notes after an update ([slice 42](42-zero-click-update.md)) reuses `ReleaseNotes.tsx` and the parser.

## Done

Implemented as planned, in a cloud session without the user's machine:

- Rust: `src-tauri/src/update/notes.rs` (`notes_for`) keeps the sections newer than the running version; `semver` became a direct dependency (it was already in the tree through the updater).
- UI: `releaseNotes.ts` (parser, `releaseDay`), `ReleaseNotes.tsx`, the dialog's region and styles; a `day` format in `ui/src/i18n/formats.ts`; `update.notesVersion` / `update.notesVersionDated` in all three languages.
- Release: `release-notes.mjs --update-notes` (replaces `--changes-only`), the Release workflow, the manifest check (notes must start with the tag's heading), the sample notes of `local-update-manifest.mjs`.
- Checked in the real app on Linux (WebKitGTK under Xvfb) with a local manifest offering 99.0.0 with the notes of 99.0.0, 0.16.0 (no date) and the real 0.15.1 and 0.15.0 sections: 99.0.0 and 0.16.0 are shown formatted, without links, 0.15.1 and 0.15.0 (not newer than the running 0.15.1) are not; in English and Polish; long notes (the older CHANGELOG sections renamed to 50.x) scroll with the wheel inside the dialog, which keeps its title and „Zamknij” in view; Escape closes it. Keys other than Escape did not reach the webview in that X session without a window manager, so keyboard scrolling is checked in the mocked e2e suite (Chromium) instead.

### Fixes from the real-app check

- The notes region collapsed to one line in WebKitGTK: as a flex item with `flex: 1` (a zero basis) in the auto-height dialog, WebKit sized the dialog from the region's minimum height. The region now takes its content's height up to its own maximum (`max-height: calc(85vh - 9rem)`) and scrolls past it. Chromium did not show the problem, so the mocked e2e tests could not catch it.
- Opened again, the dialog showed the notes at the scroll position it was closed at (the dialog stays mounted). They now start at the top each time; the scroll is reset after `showModal()`, since a hidden dialog has no layout. Covered by the e2e test „scrolls long notes with the keyboard, from the top each time”.

## Testing

Still to test by the user, on Windows (and macOS when at hand):

1. A debug build against a local manifest ([Releasing](../RELEASING.md#testing-an-update-locally)): „Co nowego?” shows the sample notes formatted, in the app's font, with „Wersja 99.0.0 · 1 grudnia 2026” and no link; the 0.0.1 section is not shown.
2. The same in English and German (version heading „Version 99.0.0 · 1 December 2026” / „Version 99.0.0 · 1. Dezember 2026”).
3. Long notes scroll, also with the keyboard after focusing them (Tab).
4. The first release after this slice: its `latest.json` notes hold the sections since v0.15.0, and an installed 0.15.x shows them (as plain text, its old dialog).

## Related

- [Slice 41](41-update-check.md): built the notice and the dialog.
- [Slice 56](56-security-hardening.md): its navigation guard keeps a link from loading a web page into the app window; links here must go to the system browser or stay text.
- [Slice 42](42-zero-click-update.md): zero-click update may show the notes after the restart instead of before it; it reuses this renderer.
- [Slice 49](49-promo-website.md): the promo website shows the changelog too and could share the subset rules (not the code; the site is built separately).
