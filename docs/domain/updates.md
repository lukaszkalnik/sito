# Updates

How Sito finds and installs a newer version of itself.

## The check

- Every time the main window opens, Sito asks GitHub for the update manifest of the newest **published** release (`https://github.com/lukaszkalnik/sito/releases/latest/download/latest.json`). Drafts are never offered, so a release whose build or smoke test failed does not reach anyone.
- The check runs in the background: it never delays the start or blocks the grid.
- **„Pomoc → Sprawdź aktualizacje…”** checks again on demand.
- A version is offered when it is newer than the running one (semantic versioning).

## What goes over the network

- The check is a plain GET of `latest.json`; the download is the installer file from the release. Neither request carries anything about the user's photos. GitHub sees the IP address, as with any download from the releases page.
- Requests go out from the Rust side only; the webview has no network access and the CSP is unchanged.
- These two requests are, besides the planned model download, the only network access of Sito.
- „Pobierz” on macOS hands the address of the release page to the system browser, which loads it; Sito itself sends no request. The address is built on the Rust side from the offered version and only ever points into Sito's GitHub repository; the webview has no permission to open addresses.

## The notice

A card at the bottom right of the main window, over the start screen and the grid alike, clear of the bar and the background-work pill. The viewer window shows nothing.

| Situation | Notice |
|---|---|
| Check at start: no update, offline, GitHub unreachable | nothing (the error goes to the debug log) |
| Update found | „Dostępna jest nowa wersja Sito: 0.15.0.” with „Zainstaluj i uruchom ponownie” (macOS: „Pobierz ją ze strony wydań Sito na GitHubie.” and „Pobierz”), „Co nowego?” (when the release has notes) and „Nie teraz” |
| „Pobierz” (macOS) | opens the release page of the offered version (`…/releases/tag/v0.15.0`) in the system browser; the offer stays |
| Release page did not open | the offer again, with „Nie udało się otworzyć strony wydania. System nie otworzył przeglądarki.” |
| „Nie teraz” | hidden until the next start or the next check from the menu |
| „Co nowego?” | a dialog with the release notes of every version newer than the installed one, newest first, in English, formatted (see [Release notes](#release-notes)) |
| Installing | „Pobieranie aktualizacji… 45%” (without a percentage when the size is unknown), then „Instalowanie aktualizacji…”; no buttons |
| Install failed | the offer again, with the reason (no connection, wrong signature, unexpected error) |
| Check from the menu, no update | „Masz najnowszą wersję (0.14.0).” with „Zamknij” |
| Check from the menu failed | „Nie udało się sprawdzić aktualizacji.” with the reason and „Zamknij” |

## Release notes

- A release carries the CHANGELOG sections of every version since 0.15.0, the first with the updater ([Releasing](../RELEASING.md#update-signing-key)). Sito shows those newer than the installed version, so a user who skipped versions sees what changed in all of them. Notes without version headings are shown whole.
- Each version has a heading with its number and release day („Wersja 0.16.0 · 25 listopada 2026”, the day in the UI's language), followed by its New / Improved / Fixed headings, lists, paragraphs, bold text and inline code, in the app's font.
- **No links:** a link shows only its text („slice 32”), without its address; whoever wants more detail looks at the release on GitHub. Anything else the formatting does not know (other headings, unclosed `**` or backticks) stays plain text.
- The notes come over the network unchecked (only the installer is signature-checked), so they are never inserted as HTML: Sito parses them itself and builds the dialog from its own elements.
- The notes stay English in every UI language; only the dialog's own texts are translated.

## Installing

- **Windows:** one click. Sito downloads the installer, checks its signature against the public key built into the app (an update signed with another key is refused), and runs it in passive mode: a small progress window, no questions. The per-user install needs no admin rights. Before the installer closes Sito, the window sizes are saved and a running reject or accept finishes; none starts after that. The installer then starts the new version.
- **macOS:** the notice shows the version and its notes, and „Pobierz” opens the release page of that version in the system browser, where the user downloads the DMG; installing in the app comes with signing ([slice 50](../slices/50-macos-code-signing.md)).
- Only installs that already contain the updater can update themselves; versions before it are updated by running the installer by hand ([Releasing](../RELEASING.md#installing-on-windows)).
- Updating is offered, never forced.
