# Grouping

## By time (built)

- Photos are sorted by capture time, newest first.
- Two neighbouring photos belong to the same series when both have an EXIF capture time and were taken at most the **time gap** apart. A series has at least two photos.
- The default gap is 10 s; the user adjusts it per folder with a slider (1–60 s).
- Photos dated only by the file never join a series (see [Photos](photos.md#capture-time)); slice 20 explores series without an EXIF date.
- Rejected photos still count when series are found, so a series keeps its shape after its weaker shots were rejected.

## By content (planned, slice 19)

A time series can mix different subjects (e.g. turning from a person to a view). It is then split where the image embeddings of neighbours differ (cosine similarity below a threshold the user can adjust). See [Analysis](analysis.md#embedding).
