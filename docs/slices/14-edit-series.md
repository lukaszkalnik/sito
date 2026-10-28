# Slice 14 – Edit series

**Status:** ⬜ not started (moved from the roadmap; concept and plan when the slice starts)

## Idea

Fix false positives and misses by hand: remove a photo from a series, add a photo to a neighbouring series, ungroup a series completely, or "cut here" to split a series that chains several moments (e.g. a long event shot in quick succession) into two at a chosen photo. Edits are stored in SQLite (slice 9), survive rescans and win over automatic grouping.

## Try

Cut a chained series into its real moments, ungroup a wrong series and remove a stray photo, rescan, and check that the edits are kept.
