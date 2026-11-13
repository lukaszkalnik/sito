---
status: 📝 concept drafted
---

# Slice 54 – Formatted release notes in „Co nowego?”

Since 2026-11-08; plan when the slice starts.

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

## Related

- [Slice 41](41-update-check.md): built the notice and the dialog.
- [Slice 56](56-security-hardening.md): its navigation guard keeps a link from loading a web page into the app window; links here must go to the system browser or stay text.
- [Slice 42](42-zero-click-update.md): zero-click update may show the notes after the restart instead of before it; it reuses this renderer.
- [Slice 49](49-promo-website.md): the promo website shows the changelog too and could share the subset rules (not the code; the site is built separately).
