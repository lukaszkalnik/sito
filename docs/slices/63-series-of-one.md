---
status: 📝 concept drafted
---

# Slice 63 – Series of one photo

Drafted on 2026-11-20 in the second review of [slice 59](59-edit-series-in-viewer.md); plan when the slice starts. Postponed on 2026-11-20 at the user's request: the user experiments with the UX on their own machine first.

## Idea

Today a series with one photo left is still shown as a series: an automatic series after its other photos were rejected, and a manual series after rejections or edits ([Grouping](../domain/grouping.md#series-edited-by-hand)). The grid then offers „Rozdziel serię” on it, which turns one photo into one photo: nothing a user would want, since a series of one is effectively a single photo already. Reconsider whether a series of one should be shown as a series at all.

## Questions

- **What a series of one is good for today**: it shows that the photo is what is left of a series (its frame, the header, an accepted keeper), and in „Serie” it stays in the view. Does the user need that, e.g. to see that a series was sifted down to its keeper, or for [slice 16](16-show-rejected.md) (showing the rejected photos of a series)?
- **Candidate approaches**:
  1. Show a series of one as a single photo (grid, viewer strip, „Serie” view, the summary's series count), keeping its membership underneath so it is a series again when a rejected photo comes back with „Cofnij”.
  2. Keep showing it as a series, but offer no edit on it (no „Rozdziel serię”) and mark it as sifted down.
  3. Leave it as it is.
- **Inconsistency between edits and rejections** (noticed by the user in slice 62's testing, 2026-11-20): an edit that leaves a part with one photo (a cut before the second photo, „Wyjmij z serii” or a join out of a series of two) pins that photo as a single photo, with the pinned mark, not as a series of one (`part` in `crates/core/src/series/edit.rs`). A rejection that leaves one photo keeps it a series of one (`without_removed` in `crates/core/src/series.rs`), and so does a manual series whose other photos were trashed or went missing. So the same situation, one photo left of a series, looks different depending on how it came about. Whatever this slice decides should make both paths agree (e.g. approach 1 would match what edits already do; approach 2 would mean edits keep a series of one too).
- **What changes with it**: the „Serie” view (would the last photo of a sifted series drop out of it?), stepping in the viewer, rejecting the second-last photo of a series, „Cofnij” bringing a photo back, manual series with one photo left, accepted marks.

## Related slices

- [Slice 59](59-edit-series-in-viewer.md) offers no edit on a series of one in the viewer until this is decided.
- [Slice 62](62-undo-series-edits.md): undoing edits and rejections can turn a single photo back into a series.
