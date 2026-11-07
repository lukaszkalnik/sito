---
status: 📝 concept drafted
---

# Slice 49 – Promo website

Since 2026-11-07; plan in plan mode when the slice starts.

## Idea

Asked by the user on 2026-11-07: a promo website for Sito on GitHub Pages (`https://lukaszkalnik.github.io/sito/`) that presents the app and links to the newest release, so people can be pointed at one friendly address instead of the GitHub releases page.

The page says in a few lines what Sito does (finds series of similar shots, suggests the best one, only suggests, runs fully offline, never deletes), shows a screenshot or two, and has a prominent download button per platform (Windows installer, macOS DMG) for the newest published release.

## Candidate approaches

### Linking to the newest release

The installer names carry the version (`Sito_X.Y.Z_x64-setup.exe`, `Sito_X.Y.Z_universal.dmg`, see [RELEASING.md](../RELEASING.md)), so a fixed `releases/latest/download/<name>` link does not work for them as it does for `latest.json` ([Updates](../domain/updates.md)).

- **Rebuild the page on every release:** the Release workflow, after publishing, deploys the site with the version and exact asset URLs filled in. Static page, no requests from the visitor's browser beyond GitHub Pages; the page is never ahead of or behind the published release. Needs a `pages` deploy job (`actions/deploy-pages`) with `pages: write` and `id-token: write` permissions on that job only.
- **Fetch at view time:** the page reads `https://api.github.com/repos/lukaszkalnik/sito/releases/latest` in the browser and picks the assets by pattern. No coupling to the release workflow, but depends on the GitHub API (rate limit of 60 requests/hour per visitor IP) and needs a fallback link to the releases page.
- **Unversioned asset copies:** upload `Sito_x64-setup.exe` / `Sito_universal.dmg` alongside the versioned ones, then link `releases/latest/download/...`. Simplest page, but doubles the upload and the installer name no longer shows the version.
- Fallback in every case: a plain link to `https://github.com/lukaszkalnik/sito/releases/latest`.

### Building the site

- **Plain HTML + CSS** in a `site/` folder (no framework, no build step), deployed by a workflow. Small, nothing to maintain.
- **Vite page in the npm workspace:** shares fonts and colors with the app, but adds a second frontend to lint and build.
- Optionally offer the button for the visitor's OS first (detected from the user agent), with the other platform below it.

## To check when planning

- **Language:** the app UI is Polish; decide whether the site is Polish only, or Polish and English (the release notes are English). If several languages, keep strings out of the markup as in the app.
- **Privacy:** no analytics, trackers, cookies or third-party fonts/CDNs, in the spirit of the app's offline promise; host fonts and images in the repo.
- **macOS and Windows warnings:** the builds are unsigned for now ([slice 47](47-windows-code-signing.md), [slice 23](23-macos-polish.md)); the page explains briefly how to get past SmartScreen and Gatekeeper, and the text is dropped once signing lands.
- **Content to reuse:** `README.md` (pitch), `CHANGELOG.md` (what is new in the newest release; link or excerpt), screenshots from `docs/images/` if suitable, the app icon.
- **Pages setup:** GitHub Pages source = GitHub Actions (a repo setting the user switches on); deploy only from `main`/release tags.
- **CI:** HTML/CSS checked by Prettier like the rest of the repo; a check that the link logic picks the right assets (e.g. a small test of the asset-matching function, or the release workflow failing if an expected asset is missing).
- **Licenses:** anything shipped on the page (fonts, scripts) needs a license check; third-party notices if any.
- **Docs:** a short section in `docs/RELEASING.md` on how the site is updated; `README.md` links to the site.

## Related

- [Slice 41](41-update-check.md) / [slice 42](42-zero-click-update.md): existing users get updates in the app; the site is for new users.
- [Slice 22](22-english-german-ui.md) (English and German UI): if the site gets several languages, use the same set.
