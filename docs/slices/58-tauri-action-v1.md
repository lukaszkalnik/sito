---
status: 📝 concept drafted
---

# Slice 58 – Move the release build to tauri-action v1

Since 2026-11-13; plan when the slice starts. Not urgent: do it together with a release, which is its only real test.

## Idea

Asked by the user on 2026-11-13, after [slice 56](56-security-hardening.md) pinned `tauri-apps/tauri-action` to the newest v0 release (`action-v0.6.2`) instead of v1.0.0 and told Dependabot to ignore its major updates (`.github/dependabot.yml`).

v0 works and is pinned to a commit, so nothing is at risk today. But v0 will stop getting fixes, and the ignore rule hides that. The release build should move to v1 deliberately, with the update path tested.

## Why v1 was not taken in slice 56

v1 changes what is written into `latest.json`, the manifest every installed Sito reads to find its update (see [Updates](../domain/updates.md)). A manual run of the Release workflow uploads no `latest.json`, so only a published release shows whether updates still work.

From the v1.0.0 release notes (the action's code was not read):

- **Download addresses:** "The download urls in `latest.json` will now use the github url instead of the browser download url" ([#1315](https://github.com/tauri-apps/tauri-action/pull/1315)). The change answers a request to support private repositories, so it most likely means `api.github.com/repos/…/releases/assets/<id>` instead of `github.com/…/releases/download/…`. Not confirmed.
- **macOS update bundles:** `.app.tar.gz` and its `.sig` now carry the app version in their names. This matters once macOS installs updates in the app ([slice 50](50-macos-code-signing.md)).
- **Releases:** the action fails if `releaseDraft: true` meets a release that is not a draft, and it now updates the name and body of an existing release. Both look harmless for the workflow as it is.
- **Inputs:** `includeUpdaterJson` became `uploadUpdaterJson`, `assetNamePattern` became `releaseAssetNamePattern`; the workflow uses neither.

## Open questions

- **What exactly is the new download address,** and does it need a request header? An API asset address returns the file only for `Accept: application/octet-stream`. Check what `tauri-plugin-updater` sends, in the version in `Cargo.lock`.
- **Rate limit:** unauthenticated requests to `api.github.com` are limited to 60 per hour per IP address. Does a download count against it, and what does the user see when it is hit (the notice's "no connection" text would be wrong)?
- **Can v1 keep the old addresses?** If the action has an option for the browser download address, taking it changes nothing for installed apps and is the simplest route.
- **Privacy text:** [Updates](../domain/updates.md) says the download is the installer file from the release; it needs updating if the host changes.

## Candidate approach

1. Read the v1 code and answer the questions above.
2. Switch the pin to the newest v1 release (full commit SHA, version comment) and remove the `ignore` entry from `.github/dependabot.yml`; adjust `docs/RELEASING.md` ("`tauri-action` stays on v0").
3. Extend `scripts/check-update-manifest.mjs` to check the form of the download addresses, so a manifest with unexpected ones stays in the draft.
4. Run the Release workflow by hand (build and smoke test), then make the next release with it: read `latest.json` in the draft before it is published, then update an installed copy on Windows from the published release.
5. If the update fails: installed apps keep working, and the next release, built with v0 again, repairs it, since apps always read the newest published release. Write that fallback into the plan.

## Builds on

- [Slice 56](56-security-hardening.md): the pinned workflows, Dependabot and the `zizmor` check; the review steps for an action update in `docs/RELEASING.md` ("Pinned actions").
- [Slice 41](41-update-check.md): the updater, `latest.json` and its check before publishing.

## Related

- [Slice 42](42-zero-click-update.md) and [slice 50](50-macos-code-signing.md): both change how updates are installed and are easier to test on one known manifest format.
