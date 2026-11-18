# Grouping

## By time and camera

- Photos are sorted by capture time, newest first.
- A series holds the photos of **one camera** (EXIF make and model) taken in quick succession: two photos of the same camera that follow each other among that camera's photos belong to the same series when both have a capture time and were taken at most the **time gap** apart. Another camera's photos taken in between do not break the series. A series has at least two photos.
- Photos without a known camera (no EXIF make and model, or EXIF not read yet during the background pass of a first Takeout scan) are grouped like one more camera. Once the background pass reads their EXIF, they move to their camera's series.
- The default gap is 10 s; the user adjusts it per folder with a slider (1–60 s).
- Photos dated only by the file never join a series (see [Photos](photos.md#capture-time)); slice 20 explores series without an EXIF date.
- Rejected photos still count when series are found, so a series keeps its shape after its weaker shots were rejected.

## Order on screen

- Each series is shown in one piece at the place of its **newest photo**. Photos outside series keep their place in time, so a single photo taken during a series of another camera comes after that series.
- The viewer steps through photos in the same order, so it goes through a series without jumping into another camera's photos.

## Several cameras at one event

- When a series overlaps in time (between its oldest and newest photo) with a series of another camera, its header also names its camera (e.g. „2 zdjęcia · 17:00:11–17:00:22 · Canon EOS R6”, or „nieznany aparat”). Other headers stay without it.
- Example: a golden wedding shot with a Canon EOS R6 and a Canon EOS 5D Mark III (683 photos). Grouping by time alone mixed both cameras in 20 of 101 series at 10 s; grouped per camera, there are 99 series and none mixes cameras.

## Series edited by hand

The user fixes series in the grid ([Timeline](timeline.md#editing-series)); Sito only suggests the automatic ones.

- **Edits:** „Podziel serię przed tym zdjęciem” (the photos before it stay one series, it and the photos after it form another), „Wyjmij z serii” (the photo becomes a single photo, the rest stays one series), „Dołącz do poprzedniej/następnej serii” (the photo joins the series of the photo shown before/after it; a series it came from keeps the rest), „Rozdziel serię” (every photo becomes a single photo). A part left with one photo becomes a single photo; rejected photos count, as they still belong to their series.
- **Pins:** an edit pins every photo it touches, to a manual series or as a single photo. An edited series stays exactly as the user left it, whatever the gap; pinned single photos stay single.
- **Pinned photos are barriers:** automatic grouping runs on the unpinned photos as before, but never links two photos of a camera across a pinned photo of that camera. So no automatic series forms "around" an edited one at a larger gap, and a pinned single photo inside a burst splits the burst around it at every gap.
- **Only what differs from automatic grouping stays edited:** after each edit, every part of the touched stretch of edited photos that automatic grouping would form anyway (with the stretch unpinned) is unpinned, as long as the series stay as the edit left them: a series with exactly the photos automatic grouping gives them, or a photo that would be single anyway. So joining a single photo to a series and taking it out again leaves nothing edited, joining it and ungrouping the series leaves only the series' photos edited (not the photo that was single anyway), and a cut joined back photo by photo ends unedited. Unpinned photos follow the gap again.
- **„Przywróć automatyczne grupowanie”** gives the whole stretch of edited photos around a photo back to automatic grouping: the run of pinned photos around it among its camera's photos, every photo of an edited series among them (also its photos that are gone for now), and so on. So it takes back a photo taken out of a series together with the rest of that series, and both parts of a cut, from any photo or header of them. The photos are grouped again at the current gap, so they may end up differently than before the edit, and series on both sides may join.
- Edits last: pins are kept by path in the action log, so they survive refreshes, reopening and restarts. Rejected photos stay in their manual series; photos trashed or gone leave it (and come back into it when restored). A manual series with one photo left shows as a series of one; with none left it is gone. Photos moved or renamed outside Sito keep their pin, like accepted marks ([Accepting](accepting.md)), also when „Cofnij” restores a photo under a new name.
- Labels (camera names on overlapping series) are worked out over manual and automatic series together. A manual series may mix cameras; it is labelled by the camera of its newest photo.
- Edits are not part of „Cofnij”, which stays for rejections ([Rejecting](rejecting.md)).

## Not built yet

Splitting time series by content: [slice 19](../slices/19-split-by-content.md). Editing series in the viewer: [slice 59](../slices/59-edit-series-in-viewer.md).
