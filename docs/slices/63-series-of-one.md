---
status: 📝 concept drafted
---

# Slice 63 – Series of one photo

Drafted on 2026-11-20 in the second review of [slice 59](59-edit-series-in-viewer.md); plan when the slice starts.

## Idea

Today a series with one photo left is still shown as a series: an automatic series after its other photos were rejected, and a manual series after rejections or edits ([Grouping](../domain/grouping.md#series-edited-by-hand)). The grid then offers „Rozdziel serię” on it, which turns one photo into one photo: nothing a user would want, since a series of one is effectively a single photo already. Reconsider whether a series of one should be shown as a series at all.

## Questions

- **What a series of one is good for today**: it shows that the photo is what is left of a series (its frame, the header, an accepted keeper), and in „Serie” it stays in the view. Does the user need that, e.g. to see that a series was sifted down to its keeper, or for [slice 16](16-show-rejected.md) (showing the rejected photos of a series)?
- **Candidate approaches**:
  1. Show a series of one as a single photo (grid, viewer strip, „Serie” view, the summary's series count), keeping its membership underneath so it is a series again when a rejected photo comes back with „Cofnij”.
  2. Keep showing it as a series, but offer no edit on it (no „Rozdziel serię”) and mark it as sifted down.
  3. Leave it as it is.
- **What changes with it**: the „Serie” view (would the last photo of a sifted series drop out of it?), stepping in the viewer, rejecting the second-last photo of a series, „Cofnij” bringing a photo back, manual series with one photo left, accepted marks.

## Related slices

- [Slice 59](59-edit-series-in-viewer.md) offers no edit on a series of one in the viewer until this is decided.
- [Slice 62](62-undo-series-edits.md): undoing edits and rejections can turn a single photo back into a series.
