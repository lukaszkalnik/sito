---
status: 📝 planned
---

# Slice 54 – Formatted release notes in „Co nowego?”

Since 2026-11-08; planned on 2026-11-21 (see [Plan](#plan)), waiting for the user's approval.

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

Own renderer for the subset (no dependency), links open in the system browser through a Rust command that checks the address, and the dialog shows every version since the installed one.

### Decisions

- **Links stay clickable.** They are the same links as on the GitHub release page, and the slice files are where the details are. A click goes to a new command, `open_github_page(url)`, which opens the address in the system browser only if it is a page of Sito's repository (`release_page::github_page`, already used for „Pobierz”); anything else is refused. The command uses the opener's free function like `open_release_page`, so the webview still gets no permission to open addresses and no capability changes. The UI shows a link only for an `https://github.com/lukaszkalnik/sito/…` address and renders any other link as its text, so a refused click cannot happen in normal use; the Rust check is the guarantee.
- **Skipped versions are shown.** The notes in `latest.json` become every CHANGELOG section from the tag back to v0.15.0 (the first version with the updater), each with its `## vX.Y.Z – date` heading. The Rust side keeps only the sections newer than the running version before sending the notes to the UI (`notes_of`, with the `semver` crate the updater already uses), so a user on 0.15.1 offered 0.17.0 sees 0.17.0 and 0.16.0. Notes without version headings (a local test manifest, an older release) pass unchanged. Installed versions up to this slice show the longer notes as plain text, which is acceptable.
- **Language:** the notes stay English; the renderer knows nothing of the language. Only the dialog's own texts (title, version headings, the error) are translated.

### Domain: `ui/src/features/update/releaseNotes.ts`

`parseReleaseNotes(notes: string): ReleaseNotes`, pure, into a typed tree:

- `ReleaseNotes = readonly NotesSection[]`; a section is `{ version: string | null; date: string | null; blocks }` (`null` for notes without version headings, a single section).
- Blocks: `{ kind: 'heading'; inline }` (`###`), `{ kind: 'list'; items: readonly Inline[][] }` (`- ` items; an indented line continues the item), `{ kind: 'paragraph'; inline }` (other lines up to a blank line).
- Inline: `text`, `bold` (whose content may hold `code` and `link`), `code` (taken literally), `link` (`{ text; url }`, `url` only when it is a repository page as above, otherwise the link is plain text).
- Anything it does not know stays text: an unclosed `**` or backtick, `[text](` without its `)`, `#`/`####` headings, a `## ` line that is not `vX.Y.Z`. CRLF line endings are accepted.

### Rust: `src-tauri/src/update.rs`

- `notes_of(body, running)`: splits at `## v` headings and drops sections whose version is not newer than `running`; a heading whose version does not parse is kept. Unit tests: no headings, all newer, some older, equal to the running one, unparseable version, blank result (`None`).
- `open_github_page(url: String)`: parses the address, refuses it with a new `UpdateError::NotAllowed` unless `github_page` accepts it, then opens it; a failed opening is `OpenFailed` as today. Registered in `lib.rs`; the error code gets its translations.

### UI

- `ReleaseNotes.tsx` renders the tree with React elements only (never HTML strings): each version gets a heading „Wersja 0.16.0 · 12.11.2026” (date through `useFormats`; the heading is left out for a section without a version), `###` headings below it, lists, paragraphs, `<strong>`, `<code>` and links. A link is an `<a href>` (so it reads and focuses as a link) whose click and Enter call `openGithubPage` in `api.ts` instead of navigating.
- `ReleaseNotesDialog.tsx`: the `<pre>` becomes a scrollable, focusable region in the app's font; inline code in the monospaced font on the tile background. A failed opening shows „Nie udało się otworzyć strony. System nie otworzył przeglądarki.” in the dialog, which stays open.
- New strings in `pl.json`, `en.json`, `de.json` (version heading, link error).

### Release side

- `scripts/release-notes.mjs`: `--changes-only` becomes `--update-notes`: the sections from the tag back to v0.15.0, headings included. The Release workflow passes the new flag; `check-update-manifest.mjs` also checks that the notes start with the tag's section.
- `scripts/local-update-manifest.mjs` writes formatted sample notes (two versions, a list, bold, code, a link), so a local update test shows the formatting.
- `docs/RELEASING.md` and [Updates](../domain/updates.md) describe the new notes.

### Tests

- Vitest: the parser (each block and inline kind, sizes 0/1/many, unclosed markers, unknown headings, links inside and outside the repository, CRLF, one real CHANGELOG section as a whole); the dialog (headings and lists by role, a click on a link calls `openGithubPage` with the address and does not navigate, a failed opening shows the error, Escape still closes).
- Rust: `notes_of` and the address check, as above.
- Mocked e2e (`ui/e2e/update.spec.ts`): the notes of two versions are shown formatted, a link click invokes `open_github_page`.
- Real app, here on Linux: a debug build with `SITO_UPDATE_ENDPOINT` pointing at a local manifest with a `linux-x86_64` entry shows the offer; check the dialog's formatting with a screenshot (no browser is installed here, so a link click shows the error). The user checks it later on Windows/macOS (font, a link opening in the browser), and the release after this slice is the real test of the new `latest.json` notes.

### Out of scope

- Translated notes; the promo website's changelog ([slice 49](49-promo-website.md)).
- Showing the notes after an update ([slice 42](42-zero-click-update.md)) reuses `ReleaseNotes.tsx` and the parser.

## Related

- [Slice 41](41-update-check.md): built the notice and the dialog.
- [Slice 56](56-security-hardening.md): its navigation guard keeps a link from loading a web page into the app window; links here must go to the system browser or stay text.
- [Slice 42](42-zero-click-update.md): zero-click update may show the notes after the restart instead of before it; it reuses this renderer.
- [Slice 49](49-promo-website.md): the promo website shows the changelog too and could share the subset rules (not the code; the site is built separately).
