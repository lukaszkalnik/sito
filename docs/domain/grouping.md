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

## Not built yet

Splitting time series by content: [slice 19](../slices/19-split-by-content.md).
