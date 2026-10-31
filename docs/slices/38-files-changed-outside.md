# Slice 38 – Files changed outside Sito

**Status:** 📝 concept drafted (plan in plan mode when the slice starts)

## Context

From the user on 2026-10-31, while testing [slice 36](36-grid-follows-viewer.md): they renamed the open folders from „Zdjęcia z roku …” to just the year. Stepping to the next photo in the viewer then showed „Nie udało się wczytać zdjęcia.”

**Cause:** a scan keeps every photo's path in memory, and nothing watches the disk. After a rename, a move or a delete outside Sito:

- the viewer and the not-yet-loaded thumbnails ask for files at their old paths, which fail with the generic load error ([Timeline](../domain/timeline.md#viewer));
- rejecting such a photo fails as well;
- data kept by path no longer matches: accepted marks ([Accepting](../domain/accepting.md)), the photo cache (the renamed photos are read again in full on the next scan, like new ones) and the undo log of earlier rejections ([Rejecting](../domain/rejecting.md)).

Choosing the folder again („Wybierz inny folder”) fixes the timeline today, but nothing tells the user to do that.

## Idea

Sito notices that photos changed on disk and brings the timeline up to date, or at least says what happened and offers a rescan, instead of showing a load error.

## Candidate approaches

1. **Detect on failure.** When the photo protocol or a reject cannot find a file, it reports a distinct error code (`missing`). The viewer and the grid then show „Plik został przeniesiony lub usunięty poza Sito” with a button „Odśwież folder”, which runs the background refresh that already runs when the last folder is reopened on start („Sprawdzanie zmian…”, [Timeline](../domain/timeline.md#main-window-bar)). Small, and no background work while nothing changes.
2. **Refresh on focus.** When the main window or the viewer gets focus again (the user was in Explorer), run the background refresh. It catches changes before the user hits them, but costs a walk of the folder (about how long for 34,600 photos?) on every return to the app.
3. **Watch the folder** (`notify` crate, ReadDirectoryChangesW on Windows, FSEvents on macOS) and refresh after changes settle. The most immediate, but a new dependency, many events for a large rename, and watcher limits on network drives.

Approaches can combine: 1 as the safety net, plus 2 or 3.

## Open questions

- **Keeping data across a rename:** should accepted marks and the photo cache follow a renamed folder? A refresh could match a vanished file to a new one by size, mtime and name (the cache's file identity without the path) and carry its mark and cached metadata over, so a renamed folder of 34,600 photos is not read again.
- **The undo log:** an earlier rejection's original path may no longer exist (its folder was renamed). Restore it to the old path (recreating the folder), or to the folder's new name when it can be matched?
- **The viewer during a refresh:** the timeline ids change after a refresh, so the viewer needs to find its photo again (by path or the matched identity), like it does after a rejection.
- **The followed photo** ([slice 36](36-grid-follows-viewer.md)) goes away with the old ids; the grid should follow the viewer's photo again once it is found.

## Builds on

- The background refresh of a reopened folder and its note „Sprawdzanie zmian…” ([Timeline](../domain/timeline.md#main-window-bar)).
- Error codes of commands mapped to messages in `pl.json` (CLAUDE.md, Errors).
