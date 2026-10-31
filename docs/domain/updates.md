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

## The notice

A card at the bottom right of the main window, over the start screen and the grid alike, clear of the bar and the background-work pill. The viewer window shows nothing.

| Situation | Notice |
|---|---|
| Check at start: no update, offline, GitHub unreachable | nothing (the error goes to the debug log) |
| Update found | „Dostępna jest nowa wersja Sito: 0.15.0.” with „Zainstaluj i uruchom ponownie”, „Co nowego?” (when the release has notes) and „Nie teraz” |
| „Nie teraz” | hidden until the next start or the next check from the menu |
| „Co nowego?” | a dialog with the release notes: the release's CHANGELOG section, in English, as plain text |
| Installing | „Pobieranie aktualizacji… 45%” (without a percentage when the size is unknown), then „Instalowanie aktualizacji…”; no buttons |
| Install failed | the offer again, with the reason (no connection, wrong signature, unexpected error) |
| Check from the menu, no update | „Masz najnowszą wersję (0.14.0).” with „Zamknij” |
| Check from the menu failed | „Nie udało się sprawdzić aktualizacji.” with the reason and „Zamknij” |

## Installing

- **Windows:** one click. Sito downloads the installer, checks its signature against the public key built into the app (an update signed with another key is refused), and runs it in passive mode: a small progress window, no questions. The per-user install needs no admin rights. Before the installer closes Sito, the window sizes are saved and a running reject or accept finishes; none starts after that. The installer then starts the new version.
- **macOS:** the notice shows the version and its notes but says to download the new version from the releases page; installing in the app comes with signing ([slice 23](../slices/23-macos-polish.md)).
- Only installs that already contain the updater can update themselves; versions before it are updated by running the installer by hand ([Releasing](../RELEASING.md#installing-on-windows)).
- Updating is offered, never forced.
