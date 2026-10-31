# Slice 42 – Zero-click update

**Status:** 📝 concept drafted (2026-10-31; plan in plan mode when the slice starts)

## Idea

Asked by the user on 2026-10-31: updates install themselves without a click. Stage 2 of auto update, built on [slice 41](41-update-check.md) (signing, manifest, check on every start, the update notice and its state machine).

## Feasibility

Yes on Windows. The updater's download and install are separate steps, so Sito can download a found update in the background and install it at a moment that never interrupts work:

1. **On close (preferred):** when the user closes Sito and a downloaded, signature-checked update is waiting, Sito runs the installer silently (`installMode` `quiet`) as it exits. The next start is the new version. No click and no interruption; the cost is that the update only takes effect one start later.
2. **On start:** if a download from the previous session is waiting, install it before the main window opens, then restart. Applies the update sooner, but delays that start by the install (a few seconds) and needs the installer to be kept on disk between sessions.
3. **Right away** while Sito is open: interrupts the user's work; ruled out.

The installer is per-user and needs no admin rights, so a silent install has no UAC prompt. macOS waits for [slice 23](23-macos-polish.md) (signing), as in slice 41.

## Candidate UI

- While downloading: at most a quiet hint („Pobieranie aktualizacji…” in the bottom note), nothing to click.
- Ready: „Aktualizacja do wersji 0.15.0 zostanie zainstalowana po zamknięciu Sito” with „Zainstaluj teraz” (slice 41's one-click path) for whoever wants it sooner.
- After an update: on the first start of the new version, a one-time note „Zaktualizowano do wersji 0.15.0” with „Co nowego?”.
- **Opt-out:** „Aktualizuj automatycznie” (on by default), e.g. in the Pomoc menu until [slice 15](15-settings.md) brings a settings page. Off means slice 41's behaviour (check and offer). Whether to also allow turning off the check itself is open.

## To check when planning

- Installing during exit: the app must have finished all writes (action log transaction, photo cache, window state) before the installer replaces the files; hook into the same shutdown path that stops background work today.
- What if the user starts Sito again while the silent install still runs (single instance, or the installer closes the app)?
- A failed install or a bad update: never loop (remember the failed version and fall back to the slice 41 notice), and keep the old version usable.
- Download only once per version, resume or restart on a broken connection, store it in the app's cache folder, delete nothing the user owns (CLAUDE.md: only Sito's own temporary download may be cleaned up).
- Release discipline: since every published release now reaches users without their action, a bad release spreads faster. The smoke test (and [slice 28](28-real-app-e2e.md)) are the gate; consider a test of updating from the previous release in the release workflow.
- `docs/domain/updates.md` and `docs/RELEASING.md` describe the automatic path and the opt-out.

## Try

With auto update on, start the previous version while a newer one is published: after a while the note says it will install on close. Close Sito, wait a moment, start it: it is the new version and says so once. Turn auto update off: only the slice 41 notice appears.
