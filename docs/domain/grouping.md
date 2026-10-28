# Grouping

## By time

- Photos are sorted by capture time, newest first.
- Two neighbouring photos belong to the same series when both have an EXIF capture time and were taken at most the **time gap** apart. A series has at least two photos.
- The default gap is 10 s; the user adjusts it per folder with a slider (1–60 s).
- Photos dated only by the file never join a series (see [Photos](photos.md#capture-time)); slice 20 explores series without an EXIF date.
- Rejected photos still count when series are found, so a series keeps its shape after its weaker shots were rejected.

## Not built yet

Splitting time series by content: [slice 19](../slices/19-split-by-content.md).
